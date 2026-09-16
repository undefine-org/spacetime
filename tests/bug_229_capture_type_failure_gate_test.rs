//! BUG-229 — `check` must gate on capture-type failure inside a directive body.
//!
//! # The bug
//!
//! A directive body that could NOT match its declared `%capture_type` still
//! compiled green. Two independent defects produced it:
//!
//! (A) `parser::rematch_with_user_macros_in` registered same-file `%macro`
//!     definitions into the augmented registry, but never same-file
//!     `%capture_type` definitions. A user grammar was therefore ALWAYS absent
//!     from the registry used to enforce it.
//!
//! (B) With the extractor absent, `form_compiler`'s Custom body-capture branch
//!     fell back to `Some(CapturedValue::String(inner))` — "preserve historical
//!     lenient behavior" — so the capture *succeeded* with unparsed raw text.
//!     The form matched, and `check` reported success on malformed source.
//!
//! Then, even when a failure WAS produced, `parser::cst_to_stfile_with_registry`
//! discarded every match diagnostic except a hardcoded `@data subscribe` case
//! (BUG-234), so the failure never reached `StFile::diagnostics`.
//!
//! # Why it mattered
//!
//! This is the BUG-216 hazard on a different path: a body that fails to parse is
//! indistinguishable from one that legitimately matched. Its practical cost was
//! that **grammar spikes could not be validated** — a green `check` proved
//! nothing, so any design claiming "spiked, works" was unfounded.
//!
//! # What is asserted here
//!
//! The positive AND the negative. A regression test that only asserts the good
//! case would have passed throughout the entire lifetime of this bug.

use std::process::Command;

/// Compile a source string through the real `check` path and report whether it
/// was accepted, plus the diagnostics produced.
fn check_source(source: &str) -> (bool, String) {
    run_cli(source, &["check"])
}

/// Run `inspect --layer expansion` and return its output.
///
/// This is what proves a directive actually MATCHED. `check` exiting 0 does not:
/// for the entire life of BUG-229 a malformed body exited 0 precisely BECAUSE the
/// directive silently produced no match. A positive test that asserts only the
/// exit code therefore cannot distinguish "worked" from "vanished".
fn expansion_of(source: &str) -> String {
    run_cli(source, &["inspect", "--layer", "expansion"]).1
}

fn run_cli(source: &str, args: &[&str]) -> (bool, String) {
    let dir = std::env::temp_dir().join(format!(
        "bug229-{}-{}",
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
    // `inspect` takes the path BEFORE its flags; `check` takes it last. Passing the
    // path first satisfies both.
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

/// The grammar under test: a tiny arm language where an arm is
/// `pattern => consequence;` and a pattern is either a bare leaf or a
/// two-ended transition `a -> b`.
/// NB the `%form` shape. `$arms:spike_arm+` is a direct repeated BODY CAPTURE,
/// which resolves through the augmented extractor registry and therefore sees a
/// same-file `%capture_type`. Writing it as a parenthesized group
/// (`"{" ( $arms:spike_arm )+ "}"`) makes it a FEAT-103 body GROUP instead, and
/// group matching compiles against stdlib definitions only — so a user grammar
/// is invisible there and the directive never matches at all. Mirrors the real
/// `@match` macro in `stdlib/enum/dispatch.st`.
const GRAMMAR: &str = r#"
%capture_type spike_leaf { ( $lit:string ) | ( $wild:ident ) }
%capture_type spike_transition { $from:spike_leaf "->" $to:spike_leaf }
%capture_type spike_pat { ( $transition:spike_transition ) | ( $leaf:spike_leaf ) }
%capture_type spike_arm { $pat:spike_pat "=>" $form:ident ";" }

%macro spike {
  %order 700

  %form {
    @spike $subject:binding {
      $arms:spike_arm+
    }
  }

  %binds {
    dispatch-mount(&self, subject: $subject, arms: $arms, reactive: false)
  }
}
"#;

#[test]
fn wellformed_arm_body_still_compiles() {
    let source = format!(
        r#"{GRAMMAR}
.stage {{
  @spike $take {{
    a -> b => cut;
    b => dissolve;
  }}
}}
"#
    );

    let (ok, output) = check_source(&source);
    assert!(
        ok,
        "BUG-229 regression: a WELL-FORMED body must still compile. Output:\n{output}"
    );

    // Exiting 0 is NOT enough. A silently-dropped directive also exits 0 — that IS
    // the bug. Prove the directive matched and its custom capture type ran.
    let expansion = expansion_of(&source);
    assert!(
        expansion.contains("@spike"),
        "BUG-229: the well-formed directive must actually MATCH, not merely fail to \
         error. `inspect --layer expansion` shows no @spike, so it was dropped \
         silently — the exact failure mode this bug is about.\nExpansion:\n{expansion}"
    );
    assert!(
        expansion.contains("arms:"),
        "BUG-229: the `$arms:spike_arm+` capture must be populated, proving the \
         same-file %capture_type was registered and ran.\nExpansion:\n{expansion}"
    );
}

/// Valid CSS must not become a compile error.
///
/// `@media` / `@supports` / `@keyframes` are passed through VERBATIM as raw CSS
/// (the `raw_css_blocks` arm in `cst_to_stfile`), so they are under no obligation
/// to match a `%form`. `@media (prefers-reduced-motion: reduce)` DOES fail
/// `@media($query:string)` — correctly, that form wants a quoted string — and an
/// earlier revision of the BUG-229 gate reported that failure, turning ordinary
/// CSS into an error. Found by sweeping `examples/` and `demos/` by hand; locked
/// here so it cannot regress silently.
#[test]
fn valid_css_at_rules_are_not_grammar_errors() {
    let source = r#".box { color: red; }

@media (prefers-reduced-motion: reduce) {
  .box { color: blue; }
}

@supports (display: grid) {
  .box { display: grid; }
}

@keyframes pulse {
  from { opacity: 0; }
  to { opacity: 1; }
}
"#;

    let (ok, output) = check_source(source);
    assert!(
        ok,
        "BUG-229: plain CSS at-rules must compile. A form failure on a construct the \
         CSS layer owns is not an author error.\nOutput:\n{output}"
    );
    assert!(
        !output.contains("E0946"),
        "BUG-229: no grammar diagnostic may be emitted for a CSS passthrough at-rule.\nOutput:\n{output}"
    );
}

/// The case that defines the bug. Before the fix this passed `check` exactly
/// like the well-formed one above, which is what made grammar work unverifiable.
#[test]
fn malformed_arm_body_is_rejected() {
    let source = format!(
        r#"{GRAMMAR}
.stage {{
  @spike $take {{
    a -> -> b => ;
    => cut
  }}
}}
"#
    );

    let (ok, output) = check_source(&source);
    assert!(
        !ok,
        "BUG-229: a body that CANNOT match its declared %capture_type must be \
         rejected by `check`. It was accepted, so a malformed grammar is again \
         indistinguishable from a working one.\nOutput:\n{output}"
    );
}

/// A transition has exactly two ends. `a -> b -> c` is the negative that the
/// original (inconclusive) spike could not make fail — the reason BUG-229 was
/// filed at all. It is asserted here so the grammar's shape is enforced, not
/// merely intended.
#[test]
fn chained_transition_is_rejected() {
    let source = format!(
        r#"{GRAMMAR}
.stage {{
  @spike $take {{
    a -> b -> c => cut;
  }}
}}
"#
    );

    let (ok, output) = check_source(&source);
    assert!(
        !ok,
        "BUG-229: `a -> b -> c` must not parse as a transition pattern -- a \
         transition has exactly two ends.\nOutput:\n{output}"
    );
}

/// The bug's actual shape, asserted directly.
///
/// A malformed body did not produce a wrong match — it produced NO match: the
/// augmented parse returned `0 matches, 0 diagnostics` and the directive simply
/// ceased to exist. That is worse than a bad error message, because a construct
/// vanishing silently is indistinguishable from one that was never written.
///
/// This asserts the diagnostic actually NAMES the failure, so a future refactor
/// cannot satisfy the tests above by failing for some unrelated reason.
#[test]
fn rejection_names_the_grammar_failure() {
    let source = format!(
        r#"{GRAMMAR}
.stage {{
  @spike $take {{
    a -> b -> c => cut;
  }}
}}
"#
    );

    let (ok, output) = check_source(&source);
    assert!(!ok, "must be rejected— see other tests. Output:\n{output}");
    assert!(
        output.contains("E0946"),
        "BUG-229: rejection must carry E0946 (directive body vs declared grammar), \
         not an incidental unrelated error.\nOutput:\n{output}"
    );
    assert!(
        output.contains("spike_arm") || output.contains("grammar"),
        "BUG-229: the diagnostic must point at the grammar that rejected the body, \
         otherwise an author cannot act on it.\nOutput:\n{output}"
    );
}

#[test]
fn comments_are_informational_even_when_the_scanner_warns() {
    let source = r#"
//@todo: preserve the authorship rail

//@> this continuation is deliberately orphaned
.stage { color: red; }
"#;

    let (ok, output) = check_source(source);
    assert!(
        ok,
        "comment trivia must not fail `check`, even with scanner warnings. Output:\n{output}"
    );
    assert!(
        output.contains("Comments"),
        "check must render its comments section. Output:\n{output}"
    );
    assert!(
        output.contains("open") && output.contains("todo"),
        "the live open record must be listed. Output:\n{output}"
    );
    assert!(
        output.contains("continuation without header"),
        "scanner warnings must remain visible but informational. Output:\n{output}"
    );
}
