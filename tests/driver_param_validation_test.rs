//! BUG-352 — an unknown driver param must be REFUSED, not silently absorbed.
//!
//! `bind_driver_args` bound a named argument by applying the registry's
//! `param_map` rename and then pushing it with `.unwrap_or(name)` — with no
//! check that the resulting name is a param the primitive DECLARES. An
//! unrecognized name was bound to nothing and evaporated, so every one of these
//! compiled clean and did nothing:
//!
//! ```text
//! @on &.click(from: ".item")   -> delegation that never delegates
//! @on &.visible(once: true)    -> a one-shot that repeats
//! @on &.click(nonsense: 1)     -> a pure typo, accepted
//! ```
//!
//! Proven silent by A/B emit diff: building WITH the clause and WITHOUT it
//! produced byte-identical output. These gates assert on the CLI RUN, because a
//! source-string assertion cannot distinguish "refused" from "absorbed".

use std::process::Command;

fn write_temp(source: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("st_param_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let file = dir.join(format!(
        "p{}.st",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::write(&file, source).expect("write temp source");
    file
}

fn check(source: &str) -> (bool, String) {
    let file = write_temp(source);
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("check")
        .arg(&file)
        .output()
        .expect("run spacetime CLI");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_file(&file);
    (out.status.success(), combined)
}

/// The core refusal: a param no primitive declares is an ERROR.
#[test]
fn an_undeclared_driver_param_is_refused() {
    let src = "$n number: 0;\n\n<div class=\"a\">hi</div>\n\n.a { @on &.click(nonsense: 1) { $n <- 1; } }\n";
    let (passed, out) = check(src);
    assert!(
        !passed,
        "`nonsense:` is not a param of any primitive — it must be refused, not absorbed:\n{out}"
    );
    assert!(
        out.contains("nonsense"),
        "the diagnostic must NAME the offending param:\n{out}"
    );
}

/// The diagnostic earns its place by saying what IS accepted — a refusal that
/// does not tell you the right spelling just moves the guesswork.
///
/// A MOTION body, because the two rails legitimately declare different params:
/// a motion body drives `event-driver` (`duration`, `delay`, `threshold`, …)
/// while a mutation body drives `on-mutation-handler` (`event`, `actions`,
/// `target`). The enumeration is per-PRIMITIVE, which is what makes it true
/// rather than a hardcoded list — so the gate pins the primitive it is actually
/// asking about.
#[test]
fn the_refusal_lists_the_params_that_are_accepted() {
    let src = "<div class=\"a\">hi</div>\n\n.a { @on &.click(nonsense: 1) { opacity: 0 -> 1; } }\n";
    let (_, out) = check(src);
    assert!(
        out.contains("duration"),
        "a motion body's driver declares `duration` — the hint must enumerate it:\n{out}"
    );
}

/// The mutation rail takes NO author-supplied named params, and says so.
///
/// Both mutation sites hand-build `event` and `actions` and forward nothing the
/// author wrote, so the honest accepted-set is what the registry row MAPS —
/// today, nothing. This gate pins the refusal AND the absence of a misleading
/// enumeration; the sibling gate pins that `target` specifically is not offered.
#[test]
fn the_mutation_rail_reports_that_it_takes_no_named_params() {
    let src = "$n number: 0;\n\n<div class=\"a\">hi</div>\n\n.a { @on &.click(nonsense: 1) { $n <- 1; } }\n";
    let (passed, out) = check(src);
    assert!(!passed, "must still be refused on the mutation rail:\n{out}");
    assert!(
        out.contains("nonsense"),
        "the diagnostic must still NAME the offending param:\n{out}"
    );
    assert!(
        !out.contains("actions"),
        "`actions` is hand-built, never author-supplied — offering it misleads:\n{out}"
    );
}

/// BUG-334's spelling. Delegation is expressed by `&name` + selector scope, not
/// a param, so `from:` on an event driver is a genuine mistake and must say so.
#[test]
fn the_delegation_param_from_bug_334_is_refused() {
    let src = "$n number: 0;\n\n<div class=\"list\"><span class=\"item\">hi</span></div>\n\n.list { @on &.click(from: \".item\") { $n <- 1; } }\n";
    let (passed, out) = check(src);
    assert!(
        !passed,
        "`from:` is not a param of the click driver — silence here is BUG-334:\n{out}"
    );
}

/// The refusal must not over-widen: a DECLARED param still compiles.
#[test]
fn a_declared_driver_param_still_compiles() {
    let src = "<div class=\"a\">hi</div>\n\n.a { @on &.click(duration: 200ms) { opacity: 0 -> 1; } }\n";
    let (passed, out) = check(src);
    assert!(
        passed,
        "`duration:` IS declared and must keep working — this guards over-widening:\n{out}"
    );
}

/// The STATEMENT form (`@on &.<driver>(...): <mutation>;`) is a different
/// hand-built site from the braced-body form.
///
/// Reviewers proved this was uncovered: deleting the refusal in `expand_one`
/// left all five original gates green, because every one of them used a braced
/// body and therefore routed through `expand_body`'s call instead. Two sites,
/// one shared refusal, and only one of them was pinned — exactly the asymmetry
/// that let BUG-346's nested case keep dropping after the top level was fixed.
#[test]
fn the_statement_form_refuses_an_unknown_param_too() {
    let src = "$n number: 0;\n\n<div class=\"a\">hi</div>\n\n.a { @on &.click(nonsense: 1): $n <- 1; }\n";
    let (passed, out) = check(src);
    assert!(
        !passed,
        "the statement form binds through a SECOND hand-built site — it must refuse too:\n{out}"
    );
}

/// A refusal must never ADVERTISE a param that is then dropped.
///
/// The first version of this fix validated the mutation rail against the
/// primitive's own signature, so `target:`, `event:` and `actions:` all passed
/// — and the hint listed them as accepted. But both mutation sites hand-build
/// `event`/`actions` and forward nothing the author wrote: `@on &.click(target:
/// ".item") { $n <- 1; }` compiled and still emitted `const delegateSelector =
/// null`. A diagnostic that recommends a silent drop is worse than no
/// diagnostic, because it converts a guess into false confidence.
///
/// `target` is the sharpest case (it is BUG-334's whole subject), so it is the
/// one pinned here.
#[test]
fn a_param_the_rail_never_forwards_is_not_advertised_as_accepted() {
    let src = "$n number: 0;\n\n<div class=\"list\"><span class=\"item\">hi</span></div>\n\n.list { @on &.click(target: \".item\") { $n <- 1; } }\n";
    let (passed, out) = check(src);
    assert!(
        !passed,
        "`target:` is never forwarded on the mutation rail — accepting it is the silent drop:\n{out}"
    );
    assert!(
        !out.contains("declared parameters: actions"),
        "the hint must not advertise params the rail hand-builds and ignores:\n{out}"
    );
}


/// A driver whose registry row names NO real primitive still refuses.
///
/// `clip` registers `primitive: none` — a sentinel, not a declaration. The first
/// version of this fix bailed whenever the declared-param list came back empty,
/// which conflated "this primitive takes no params" with "there is no primitive
/// to ask", and let `@on &.clip(bogus: 1)` through silently. Reviewers found it.
///
/// The two answers are now distinct (`Option<Vec<_>>`): absent = permissive,
/// present-but-empty = refuse everything.
#[test]
fn a_driver_with_no_declared_params_still_refuses_one() {
    let src = "<div class=\"a\">hi</div>\n\n.a { @on &.clip(bogus: 1) { opacity: 0 -> 1; } }\n";
    let (passed, out) = check(src);
    assert!(
        !passed,
        "`clip` declares no params, so `bogus:` must be refused, not absorbed:\n{out}"
    );
}

/// … and the valid spelling of that same driver keeps working.
#[test]
fn a_paramless_driver_still_compiles_without_params() {
    let src = "<div class=\"a\">hi</div>\n\n.a { @on &.clip { opacity: 0 -> 1; } }\n";
    let (passed, out) = check(src);
    assert!(
        passed,
        "refusing a param must not refuse the paramless spelling:\n{out}"
    );
}

