//! BUG-367 — the argument type check runs on the LIVE surface.
//!
//! # What this file is really about
//!
//! FEAT-109 W4 added `check_param_types` and proved it with unit gates. Then
//! BUG-367 asked whether the validator it was *planned* for runs at all, and
//! the measurement said no — `ast.presets` is empty for every file in the
//! corpus. Chasing that found something worse: `Value::FunctionCall`, the enum
//! variant the whole `src/validator/functions.rs` walk pattern-matches on, is
//! NEVER CONSTRUCTED ANYWHERE. Two dead layers, one on top of the other.
//!
//! Then a third: `check_param_types` itself had no caller outside its own unit
//! test. It was correct, tested, and unreachable — the exact defect BUG-367 was
//! filed about, reproduced by the fix for it.
//!
//! So this file asserts through the CLI-visible diagnostic path (the same
//! `pipeline` seam that emits BUG-352's unknown-param refusal), not against a
//! function called directly. A gate that calls the checker itself cannot
//! distinguish "checks correctly" from "checks correctly and never runs" —
//! which is how this bug got here.
//!
//! # RED before GREEN
//!
//! Before the fix, `wrong_type_for_a_declared_param_is_refused` and
//! `the_refusal_names_param_expected_and_found` FAIL (zero diagnostics: the
//! wrong type sails through). Every other test in this file PASSES before and
//! after — they pin the silence, which is the property most at risk from a
//! checker that just started running for the first time.

use spacetime::diagnostics::Diagnostic;

/// Compile a source through the same diagnostic entry point `cargo run -- check`
/// uses, and return everything it reports.
fn diagnostics_for(src: &str) -> Vec<Diagnostic> {
    let dir = std::env::temp_dir().join(format!(
        "st-bug367-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("index.st");
    std::fs::write(&file, src).unwrap();
    let ast = spacetime::parse(src).expect("fixture must parse");
    let (meta, _errs) = spacetime::compiler::cached_stdlib_registry();
    let out = spacetime::analysis::diagnostics::collect_document_diagnostics(&ast, Some(dir.as_path()), Some(&meta));
    let _ = std::fs::remove_dir_all(&dir);
    out
}

fn messages(src: &str) -> Vec<String> {
    diagnostics_for(src)
        .iter()
        .map(|d| format!("[{}] {}", d.code.as_str(), d.message))
        .collect()
}

/// A type mismatch reported by THIS check, identified by its code, so a
/// coincidental unrelated diagnostic cannot make the gate pass.
fn type_mismatches(src: &str) -> Vec<String> {
    diagnostics_for(src)
        .iter()
        .filter(|d| d.code == spacetime::diagnostics::DiagnosticCode::E173)
        .map(|d| d.message.clone())
        .collect()
}

// ═══════════════════════════════════════════════════════════════════
// THE KEYSTONE — RED before the fix
// ═══════════════════════════════════════════════════════════════════

/// `start` is declared `number` by `%primitive scroll-driver`. A colour is not
/// a number, and inference says so with no annotation anywhere.
///
/// Before BUG-367 this compiled clean. The param NAME was checked (BUG-352) and
/// the param TYPE was not, so `start: #FF0020` was silently bound and shipped.
#[test]
fn wrong_type_for_a_declared_param_is_refused() {
    let out = type_mismatches(
        r#"
div {
    background: #ffffff;
    @on &.scroll(start: #FF0020, end: 1) { opacity: 0 -> 1; }
}
"#,
    );
    assert_eq!(
        out.len(),
        1,
        "a colour in a `number` param must be refused exactly once, got: {out:?}"
    );
}

/// A diagnostic that does not say WHICH param, what was EXPECTED and what was
/// FOUND makes the author hunt. All three are already in
/// `ValidationErrorKind::ParameterTypeMismatch`; this pins that they survive to
/// the rendered message.
#[test]
fn the_refusal_names_param_expected_and_found() {
    let out = type_mismatches(
        r#"
div {
    background: #ffffff;
    @on &.scroll(start: #FF0020, end: 1) { opacity: 0 -> 1; }
}
"#,
    );
    let msg = out.first().expect("must report").to_lowercase();
    assert!(msg.contains("start"), "must name the param: {msg}");
    assert!(msg.contains("number"), "must name the expected type: {msg}");
    assert!(msg.contains("color"), "must name what was found: {msg}");
}

/// The check is registry-driven, not scroll-specific: three different drivers,
/// backed by three different primitives, are refused by the same code path with
/// no Rust arm per driver. A newly registered driver is checked the moment it
/// names a primitive.
///
/// FIXTURE NOTE. This first read `@on load(delay: ...)` and failed — not because
/// the check is special-cased, but because that spelling is a grammar error
/// (E0965), so compilation never reached the argument loop. The gate was
/// measuring the fixture, not the mechanism. Verified by hand against the CLI
/// before rewriting: `.time`, `.loop` and `.hover` all report E173, `.mouse`
/// reports E0960 for an undeclared name, `.intersect` is not a registered
/// driver at all. Three drivers, one code path.
#[test]
fn the_check_is_not_special_cased_to_one_driver() {
    for (driver, param) in [("time", "duration"), ("loop", "duration"), ("hover", "duration")] {
        let src = format!(
            "div {{\n    background: #ffffff;\n    @on &.{driver}({param}: #FF0020) {{ opacity: 0 -> 1; }}\n}}\n"
        );
        let out = type_mismatches(&src);
        assert_eq!(
            out.len(),
            1,
            "`{driver}`'s numeric `{param}` must refuse a colour too, got: {out:?}"
        );
        assert!(
            out[0].contains(driver),
            "the message must name the driver it is about: {out:?}"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════
// THE SILENCE — green before AND after. A checker that just came alive
// for the first time is most dangerous here.
// ═══════════════════════════════════════════════════════════════════

/// The ordinary spelling. If this ever reports, the check is worse than useless.
#[test]
fn a_correct_argument_is_silent() {
    let out = type_mismatches(
        r#"
div {
    background: #ffffff;
    @on &.scroll(start: 0, end: 1) { opacity: 0 -> 1; }
}
"#,
    );
    assert!(out.is_empty(), "correct args must not warn: {out:?}");
}

/// A REFERENCE names a value defined elsewhere — possibly in another file the
/// checker cannot see. Refusing it would block a build over correct code.
#[test]
fn a_reference_argument_is_silent() {
    let out = type_mismatches(
        r#"
@form value --my-start { 0 }

div {
    background: #ffffff;
    @on &.scroll(start: --my-start, end: 1) { opacity: 0 -> 1; }
}
"#,
    );
    assert!(out.is_empty(), "a reference must not be refused: {out:?}");
}

/// A `string` param takes words. Inference abstains or answers `string`, and
/// either way there is no contradiction to report.
#[test]
fn a_string_param_taking_a_word_is_silent() {
    let out = type_mismatches(
        r#"
div {
    background: #ffffff;
    @on &.scroll(scope: cover, start: 0, end: 1) { opacity: 0 -> 1; }
}
"#,
    );
    assert!(out.is_empty(), "a word in a string param is fine: {out:?}");
}

/// THE STAGGER RULE, at the call site. `duration: number = 1000` is declared
/// `number` and authors write `600ms` — the slot's unit is milliseconds either
/// way. A duration literal in a numeric time slot is NOT a mismatch to report;
/// treating it as one would refuse the corpus's own idiom.
#[test]
fn a_duration_literal_in_a_numeric_time_slot_is_silent() {
    let out = type_mismatches(
        r#"
div {
    background: #ffffff;
    @on load(duration: 600ms) { opacity: 0 -> 1; }
}
"#,
    );
    assert!(
        out.is_empty(),
        "600ms for a numeric duration must not be refused: {out:?}"
    );
}

/// A value the grammar does not model is an ABSTENTION, never evidence of a
/// mismatch. This is the property that decides whether the whole check is
/// usable, because the corpus is 30% unmodelled values.
#[test]
fn an_unmodelled_value_is_silent() {
    let out = type_mismatches(
        r#"
div {
    background: #ffffff;
    @on &.scroll(start: calc(100% - 3px), end: 1) { opacity: 0 -> 1; }
}
"#,
    );
    assert!(out.is_empty(), "an abstention must stay silent: {out:?}");
}

/// The corpus itself must stay clean. If the newly-live check fires anywhere in
/// the shipped examples, it is wrong — those pages compile and work today.
#[test]
fn the_shipped_corpus_reports_no_type_mismatches() {
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                let n = p.file_name().unwrap().to_string_lossy().to_string();
                if n == "dist" || n == "node_modules" || n == "vendor" {
                    continue;
                }
                walk(&p, out);
            } else if p.extension().map(|x| x == "st").unwrap_or(false) {
                out.push(p);
            }
        }
    }
    let mut files = Vec::new();
    for d in ["examples", "demos"] {
        walk(std::path::Path::new(d), &mut files);
    }
    assert!(!files.is_empty(), "corpus must be found from the crate root");

    let (meta, _e) = spacetime::compiler::cached_stdlib_registry();
    let mut offenders = Vec::new();
    for f in &files {
        let Ok(src) = std::fs::read_to_string(f) else {
            continue;
        };
        let Ok(ast) = spacetime::parse(&src) else {
            continue;
        };
        let dir = f.parent().unwrap();
        for d in spacetime::analysis::diagnostics::collect_document_diagnostics(&ast, Some(dir), Some(&meta)) {
            if d.code == spacetime::diagnostics::DiagnosticCode::E173 {
                offenders.push(format!("{}: {}", f.display(), d.message));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "the check must not fire on working pages:\n{}",
        offenders.join("\n")
    );
}

// ═══════════════════════════════════════════════════════════════════
// THE DEAD LAYER — asserted gone, so it cannot come back
// ═══════════════════════════════════════════════════════════════════

/// `@preset` is RETIRED (SIP-001c / BUG-263) and produces no `PresetDef`. That
/// is not the bug — the bug was a validator walking the empty list it leaves
/// behind and reporting nothing, for years, while looking like coverage.
///
/// This pins the measurement so a future reader does not "fix" the walk.
#[test]
fn preset_definitions_are_genuinely_never_produced() {
    let ast = spacetime::parse(
        r#"
@preset animation &reveal {
    opacity: 0 -> 1;
    duration: 600ms;
}
"#,
    )
    .unwrap();
    assert_eq!(
        ast.presets.len(),
        0,
        "@preset is retired — if this ever produces a def again, the dead \
         validator BUG-367 deleted must be reconsidered, not resurrected blindly"
    );
}

/// The retired spelling still fails loudly. Deleting the validator must not
/// weaken the one diagnostic `@preset`'s retirement actually depends on.
#[test]
fn the_retired_tilde_spelling_still_fails_loudly() {
    let err = spacetime::parse("@preset easing ~smooth: cubic-bezier(0.4, 0, 0.2, 1);")
        .expect_err("`~name` must still be refused");
    let text = format!("{err:?}");
    assert!(
        text.contains("retired"),
        "the retirement diagnostic must survive: {text}"
    );
}

/// An unknown directive is still caught, by the parser pass that genuinely runs.
/// The deleted validator's "did you mean?" was never reachable; this proves the
/// capability itself did not leave with it.
#[test]
fn an_unknown_directive_is_still_reported() {
    let out = messages(
        r#"
div {
    background: #ffffff;
    @nosuchdirectiveatall(a: 1)
}
"#,
    );
    assert!(
        out.iter().any(|m| m.to_lowercase().contains("nosuchdirectiveatall")),
        "unknown directives must still be reported: {out:?}"
    );
}
