//! PLAN-121 W2.3 / BUG-245 — the `@form <kind> --name` surface, marker design.
//!
//! `@form` is SIP-001c's one declaration for a named reusable chunk. Its KIND
//! is an explicit leading marker — `@form motion --rise`, never inferred from
//! the body and never trailing — because BUG-245 measured that body-based
//! inference is registration order in disguise (`properties` and `keyframes`
//! accept the same input, and the matcher takes the first successful
//! candidate). User ruling 2026-07-26: the marker is always required; an
//! uninferrable or ambiguous body is a compile-time error, never a default.
//!
//! The six kinds are six sibling macros in `stdlib/macros/form.st`
//! (kind-is-the-macro, the `data-kind.st` idiom), discriminated by a
//! ONE-ALTERNATIVE UNION capture (`$kind:("motion")`) rather than a bare
//! literal: a literal miss yields LiteralMismatch, which the reporter
//! deliberately suppresses; a union miss yields CaptureTypeMismatch, which is
//! reported — so a misspelt kind is a hard error naming the full vocabulary.
//!
//! What is asserted as CURRENT (incomplete) behavior: nothing — with BUG-241
//! fixed, every W2.3 gap is closed or filed. The splice's EXPANSION (the
//! declared body spliced into the host scope) is W3 scope, tested there.

use std::process::Command;

fn run_cli(source: &str, args: &[&str]) -> (bool, String) {
    let dir = std::env::temp_dir().join(format!(
        "sip001-w2-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let file = dir.join("index.st");
    std::fs::write(&file, source).expect("write source");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_spacetime"));
    cmd.arg(args[0]).arg(&file).args(&args[1..]);
    let out = cmd.output().expect("run spacetime CLI");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
    (out.status.success(), combined)
}

/// Build and return the emitted (JS, CSS). Used instead of `inspect` wherever
/// the question is "did this construct actually take effect?" — see BUG-244.
fn build_out(source: &str) -> (String, String) {
    let dir = std::env::temp_dir().join(format!(
        "sip001-w2js-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let file = dir.join("index.st");
    std::fs::write(&file, source).expect("write source");
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("build")
        .arg(&file)
        .output()
        .expect("run spacetime build");
    assert!(
        out.status.success(),
        "build failed:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let js = std::fs::read_to_string(dir.join("spacetime.js")).unwrap_or_default();
    let css = std::fs::read_to_string(dir.join("spacetime.css")).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    (js, css)
}

/// Every kind-word parses, at file level and inside a selector. Body
/// captures are deliberately lenient for most kinds (see form.st's
/// tight/lenient table): the MARKER is the discriminator. The exception is
/// `score` — PLAN-128 tightened it to `$entries:score_entry+`, so its case
/// below is a real score line, not a keyframe placeholder.
#[test]
fn all_six_kinds_parse() {
    let cases: &[(&str, &str)] = &[
        ("style", "background: #121722;"),
        ("value", "result: 1;"),
        ("motion", "opacity: 0 -> 1;"),
        ("easing", "cubic-bezier(0.16, 1, 0.3, 1)"),
        // PLAN-128: the score body is a TIGHT capture — real score lines,
        // spliceable into a `@score` body. Keyframes no longer parse here.
        ("score", "&a for 50%;"),
        ("markup", "<span class=\"badge\">New</span>"),
    ];
    for (kind, body) in cases {
        for (pos, src) in [
            ("file", format!("@form {kind} --chunk {{ {body} }}\n")),
            (
                "scope",
                format!(".host {{\n  @form {kind} --chunk {{ {body} }}\n}}\n"),
            ),
        ] {
            let (passed, out) = run_cli(&src, &["check"]);
            assert!(passed, "@form {kind} must parse at {pos} level:\n{out}");
        }
    }
}

/// A parameterized name (`--rise($distance = 24px)`) parses and the params are
/// captured -- the article's first fence depends on this.
#[test]
fn parameterized_form_name_parses() {
    let src = ".host {\n  @form motion --rise($distance = 24px, $duration = 700ms) { opacity: 0 -> 1; }\n}\n";
    let (passed, out) = run_cli(src, &["check"]);
    assert!(passed, "a parameterized form name must parse:\n{out}");
    let expansion = run_cli(src, &["inspect", "--layer", "expansion"]).1;
    assert!(
        expansion.contains("--rise") && expansion.contains("param"),
        "the captured name and its params must appear in the expansion.\n{expansion}"
    );
}

/// A `@form` declaration parses, and its name MUST carry the sigil.
#[test]
fn form_declaration_requires_a_dashed_name() {
    let ok = ".host {\n  @form style --card-surface { background: #121722; }\n}\n";
    let (passed, out) = run_cli(ok, &["check"]);
    assert!(passed, "a dashed form name must be accepted:\n{out}");

    let bad = ".host {\n  @form style cardsurface { background: #121722; }\n}\n";
    let (passed, out) = run_cli(bad, &["check"]);
    assert!(
        !passed,
        "an UNDASHED form name must be rejected -- otherwise the sigil is \
         decoration.\n{out}"
    );
    assert!(
        out.contains("DashedIdent") || out.contains("dashed_ident"),
        "the rejection must name the expected capture type.\n{out}"
    );
}

/// A misspelt kind-word is a HARD ERROR naming the full vocabulary -- the
/// whole point of expressing the marker as a union capture. A bare literal
/// would have been a suppressed LiteralMismatch and a silent drop.
#[test]
fn typo_kind_is_a_hard_error_naming_the_vocabulary() {
    let src = ".host {\n  @form mootion --rise { opacity: 0 -> 1; }\n}\n";
    let (passed, out) = run_cli(src, &["check"]);
    assert!(!passed, "a misspelt kind must fail:\n{out}");
    assert!(out.contains("E0946"), "the failure must be E0946.\n{out}");
    for word in ["style", "value", "motion", "easing", "score", "markup"] {
        assert!(
            out.contains(word),
            "the error must name `{word}` -- siblings merge their one-word \
             unions into the full vocabulary (match_sink.rs).\n{out}"
        );
    }
}

/// The TRAILING shape (`@form --fade-in motion`) was considered and rejected:
/// declaration-site modifiers lead (`@data inline`), and a trailing word
/// collides with the form's own parens. It must not parse -- otherwise two
/// grammars are live and the marker's position is a coin flip.
#[test]
fn trailing_kind_word_is_rejected() {
    let src = ".host {\n  @form --fade-in motion { opacity: 0 -> 1; }\n}\n";
    let (passed, out) = run_cli(src, &["check"]);
    assert!(
        !passed,
        "the trailing kind shape must be rejected -- the marker leads, always.\n{out}"
    );
}

/// BUG-245, inverted: THE MARKER SELECTS THE MACRO -- not the body, not
/// registration order. A keyframes body next to a declarations body, in one
/// scope: each is claimed by the macro its marker names.
///
/// This replaces `form_kind_inference_does_not_discriminate_today`, whose
/// failure message instructed updating test, bug and spec together when the
/// fix landed. SIP-001c §1 (v3) and BUG-245 are updated; this is the test.
#[test]
fn the_marker_selects_the_macro() {
    let src = ".host {\n  @form motion --rise { opacity: 0 -> 1; }\n  @form style --card-surface { background: #121722; }\n}\n";
    let expansion = run_cli(src, &["inspect", "--layer", "expansion"]).1;
    assert!(
        expansion.contains("kind: motion") && expansion.contains("body: [keyframes]"),
        "the motion marker must select the keyframes macro.\n{expansion}"
    );
    assert!(
        expansion.contains("kind: style") && expansion.contains("body: [properties]"),
        "the style marker must select the properties macro -- in the SAME \
         scope, so selection cannot be registration order.\n{expansion}"
    );
}

/// BUG-244 — `inspect` omits FILE-LEVEL matches, so it cannot answer "did this
/// match?" for a top-level directive.
///
/// This test originally asserted that a top-level user directive was DROPPED.
/// That was wrong, and the way it was wrong is worth keeping: the evidence was
/// `inspect --layer expansion` printing nothing. But `inspect` iterates only
/// `ast.scopes`/`scope.matches` (`src/cli/inspect.rs:2026`), while file-level
/// matches live in the flat `ast.matches` the compiler actually consumes
/// (`src/parser/mod.rs:2205`, `src/compiler.rs:929`). The construct worked all
/// along.
///
/// So the assertion is now made against an OBSERVABLE COMPILE EFFECT — emitted
/// output — which is the only evidence that distinguishes "matched" from
/// "silently dropped". Asserting against a reporting tool measured the tool.
#[test]
fn a_top_level_user_directive_really_matches() {
    let src = r#"
%macro tl-probe {
  %order 700
  %form { @tl-probe $name:ident { $body:properties } }
  %binds { css-property(selector: ".probe-target", property: "color", value: "red") }
}

@tl-probe alpha { background: blue; }
"#;
    let (js, css) = build_out(src);
    assert!(
        css.contains("color: red"),
        "a TOP-LEVEL user directive must expand and emit. If this fails, the \
         construct really is dropped and BUG-244 should be re-filed as a \
         compiler bug rather than an inspector bug.\nCSS:\n{css}\nJS:\n{js}"
    );
}

/// BUG-241 — FIXED: statement position now dispatches. A `--name;` splice is
/// recognized (FORM_REF), and a name no `@form` registers is a hard E0947 —
/// the `%form { --splice … }` grammar below is never consulted (dispatch is a
/// parser arm, not registry coverage), so the proof of dispatch is the ERROR
/// CHANGING KIND: silent drop → E0947 naming the unknown form.
///
/// The full suite is tests/bug_241_form_splice_test.rs; this guards the
/// inversion at the surface this file owns. SIP-001c's position table and the
/// BUG-241 file are updated with it, as this test's predecessor instructed.
#[test]
fn statement_position_splice_now_dispatches() {
    let src = ".card {\n  --splice card-surface;\n  color: white;\n}\n";
    let (passed, out) = run_cli(src, &["check"]);
    assert!(
        !passed,
        "an unregistered statement-position form must be a hard error, not a \
         silent drop. If this passes WITHOUT an E0947, the silent drop is back \
         — reopen BUG-241.\n{out}"
    );
    assert!(
        out.contains("E0947") && out.contains("--splice"),
        "dispatch must route the splice to registry validation:\n{out}"
    );
}
