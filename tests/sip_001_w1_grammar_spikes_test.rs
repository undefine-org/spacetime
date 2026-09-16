//! PLAN-121 W1.1 — SIP-001 grammar spikes, with negatives that MUST fail.
//!
//! # Why this file exists
//!
//! A grammar "spike" that only checks the happy path proves nothing. The first
//! attempt at these spikes (2026-07-26) reported success while a deliberately
//! malformed body ALSO passed `check` — because a directive that matched no form
//! was silently dropped (BUG-229). Every spike here therefore asserts BOTH:
//!
//!   * the positive case EXPANDS (via `inspect --layer expansion`, not merely a
//!     zero exit code — a dropped directive also exits zero), and
//!   * each negative case is REJECTED with a diagnostic.
//!
//! These grammars are the ones SIP-001 depends on. Until BUG-229's gate landed
//! (`E0946`), none of them could be verified at all.

use std::process::Command;

fn run_cli(source: &str, args: &[&str]) -> (bool, String) {
    let dir = std::env::temp_dir().join(format!(
        "sip001-w1-{}-{}",
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

/// Assert a source compiles AND the named directive actually matched.
fn assert_accepts(source: &str, directive: &str, case: &str) {
    let (ok, output) = run_cli(source, &["check"]);
    assert!(
        ok,
        "[{case}] expected to compile, but `check` failed:\n{output}"
    );

    let expansion = run_cli(source, &["inspect", "--layer", "expansion"]).1;
    assert!(
        expansion.contains(directive),
        "[{case}] `check` passed but `{directive}` does NOT appear in the expansion -- \
         the directive was silently DROPPED, which is exactly the failure mode a \
         happy-path-only spike cannot see.\nExpansion:\n{expansion}"
    );
}

/// Assert a source is rejected with a real diagnostic — and NOT via E0947:
/// the arm fixtures declare their forms (BUG-241 made undeclared forms an
/// error), so a rejection that names E0947 is the FORM REGISTRY talking, not
/// the arm grammar this spike exists to prove.
fn assert_rejects(source: &str, case: &str) {
    let (ok, output) = run_cli(source, &["check"]);
    assert!(
        !ok,
        "[{case}] expected REJECTION, but `check` passed. A grammar that accepts \
         malformed input is not a grammar.\nOutput:\n{output}"
    );
    assert!(
        !output.contains("E0947"),
        "[{case}] rejected, but via E0947 (unknown form) — vacuously. The arm \
         grammar itself must be what rejects this input.\nOutput:\n{output}"
    );
}

// ===========================================================================
// Spike 1 — driver expressions:  &card.visible
// ===========================================================================

const DRIVER_GRAMMAR: &str = r#"
%capture_type drv_ref { "&" $name:ident? "." $member:ident }

%macro spike-driver {
  %order 700
  %form {
    @spike-driver $driver:drv_ref {
      $props:properties
    }
  }
  %binds { noop(&self, driver: $driver, props: $props) }
}
"#;

fn driver_source(driver: &str) -> String {
    format!("{DRIVER_GRAMMAR}\n.stage {{\n  @spike-driver {driver} {{\n    opacity: 1;\n  }}\n}}\n")
}

#[test]
fn driver_expression_accepts_both_sip_surfaces() {
    // SIP-001 uses BOTH: `&card.visible` (a named subject) and `&.visible`
    // (self, the far more common form -- it is the first example in the launch
    // article). An earlier revision of this spike required the subject and so
    // listed `&.visible` as a NEGATIVE, i.e. it asserted that the SIP's own
    // primary surface must be rejected. `$name:ident?` admits both.
    assert_accepts(
        &driver_source("&card.visible"),
        "@spike-driver",
        "named subject",
    );
    assert_accepts(&driver_source("&.visible"), "@spike-driver", "self subject");
}

#[test]
fn driver_expression_rejects_malformed() {
    // The MEMBER is required (that is what makes it a driver rather than a bare
    // reference); the SUBJECT is optional (`&.visible` = self).
    for (src, case) in [
        ("&card", "missing member"),
        (
            "card.visible",
            "missing & -- that is a bare ident, not a reference",
        ),
        ("&card.", "trailing dot, no member"),
        ("&.", "dot with neither subject nor member"),
    ] {
        assert_rejects(&driver_source(src), case);
    }
}

// ===========================================================================
// Spike 2 — arms:  `pattern => consequence;`, where a pattern may be a
// two-ended transition.  THE load-bearing grammar for `@on`.
// ===========================================================================

const ARM_GRAMMAR: &str = r#"
%capture_type on_leaf { ( $lit:string ) | ( $wild:ident ) }
%capture_type on_transition { $from:on_leaf "->" $to:on_leaf }
%capture_type on_pat { ( $transition:on_transition ) | ( $leaf:on_leaf ) }
%capture_type form_application { $form:ident ( $params:params )? }
%capture_type on_arm { $pat:on_pat "=>" $form:form_application ";" }

%macro spike-on {
  %order 700
  %form {
    @spike-on $subject:binding {
      $arms:on_arm+
    }
  }
  %binds { dispatch-mount(&self, subject: $subject, arms: $arms, reactive: false) }
}
"#;

fn arm_source(arms: &str) -> String {
    // The consequences are statement-position form splices; since BUG-241 an
    // undeclared form is E0947, so the fixtures declare their forms (exactly
    // like the launch article's fences do). This also keeps the reject tests
    // HONEST: they must fail on the ARM grammar, never vacuously on E0947.
    const DECLS: &str = "@form motion --cut { opacity: 1 -> 1; }\n\
                         @form motion --dissolve($duration = 200ms) { opacity: 0 -> 1; }\n\
                         @form motion --slide-open($duration = 280ms) { opacity: 0 -> 1; }\n";
    format!("{DECLS}{ARM_GRAMMAR}\n.stage {{\n  @spike-on $take {{\n{arms}\n  }}\n}}\n")
}

#[test]
fn arms_accept_the_launch_articles_real_surfaces() {
    // These are verbatim from SIP-001-LAUNCH-ARTICLE, not simplified stand-ins.
    // An earlier revision used bare `cut`/`dissolve` consequences and so proved
    // nothing about the surface authors actually write.
    //
    // A BARE pattern is the 90% case ("arrived here from anywhere"); the
    // `from -> to` form is opt-in, so N states never force N^2 arms.
    assert_accepts(
        &arm_source("    a -> b => --cut;"),
        "@spike-on",
        "transition arm",
    );
    assert_accepts(
        &arm_source("    b => --dissolve(6f);"),
        "@spike-on",
        "form WITH params",
    );
    assert_accepts(&arm_source("    _ => --cut;"), "@spike-on", "wildcard arm");
    assert_accepts(
        &arm_source("    \"billing\" => --slide-open(280ms);"),
        "@spike-on",
        "string-literal pattern (a signal value)",
    );
    assert_accepts(
        &arm_source("    a -> b => --cut;\n    b => --dissolve(6f);"),
        "@spike-on",
        "mixed arms",
    );
}

/// A malformed arm AFTER a valid one must not be swallowed.
///
/// Found by review of the first spike. `$arms:on_arm+` stopped at the first
/// element that failed to extract and returned the prefix collected so far, so
/// `a -> b => --cut; a -> b -> c => --cut;` captured only the FIRST arm, the
/// form matched, and `check` went green while the second arm vanished from the
/// program.
///
/// That is BUG-229's silent-drop class one level down: there a whole directive
/// disappeared, here one element of a repeat did. Position must not decide
/// whether a malformed arm is reported.
#[test]
fn arms_reject_a_malformed_arm_in_any_position() {
    assert_rejects(
        &arm_source("    a -> b -> c => --cut;\n    b => --dissolve(6f);"),
        "malformed arm FIRST",
    );
    assert_rejects(
        &arm_source("    a -> b => --cut;\n    a -> b -> c => --cut;"),
        "malformed arm LAST -- the regression this test exists for",
    );
}

#[test]
fn arms_reject_chained_transition() {
    // THE negative. `a -> b -> c` must not parse: a transition has exactly two
    // ends, which is why `on_leaf` is a separate capture type from `on_pat` --
    // transitions cannot nest.
    //
    // The original spike could not make this fail, and that inability is what
    // BUG-229 was filed for. It failing here is the proof the gate works.
    assert_rejects(
        &arm_source("    a -> b -> c => --cut;"),
        "chained transition",
    );
}

#[test]
fn arms_reject_malformed() {
    for (src, case) in [
        ("    a -> b => ;", "empty consequence"),
        ("    a -> b => --cut", "missing semicolon"),
        ("    => --cut;", "consequence with no pattern"),
    ] {
        assert_rejects(&arm_source(src), case);
    }
}

// ===========================================================================
// Spike 3 (W1.2) — dashed-ident POSITION rule.
//
// This is the assumption the whole `--` sigil decision rested on, and it did
// NOT hold. Recorded here as executable truth rather than prose, because the
// SIP asserted the opposite for two revisions.
// ===========================================================================

/// Read the emitted CSS for a source, via a real `build`.
fn emitted_css(source: &str) -> String {
    let dir = std::env::temp_dir().join(format!(
        "sip001-pos-{}-{}",
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

    let css = std::fs::read_to_string(dir.join("spacetime.css")).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    css
}

#[test]
fn dashed_ident_property_position_is_a_custom_property() {
    let css = emitted_css(".card {\n  --brand: red;\n  color: var(--brand);\n}\n");
    assert!(
        css.contains("--brand: red"),
        "a dashed-ident in PROPERTY position is an ordinary CSS custom property \
         and must survive untouched.\nCSS:\n{css}"
    );
}

#[test]
fn dashed_ident_value_position_passes_through() {
    // `--double(20px)` is a css-mixins-1 custom FUNCTION call. It already works,
    // which is why @form's function kind needs no parser change.
    let css = emitted_css(".card {\n  width: --double(20px);\n}\n");
    assert!(
        css.contains("--double(20px)"),
        "a dashed-ident in VALUE position must pass through verbatim.\nCSS:\n{css}"
    );
}

/// BUG-241, FIXED: statement position dispatches. An undeclared `--name;` is
/// a hard E0947 (never the silent drop this test's predecessor asserted as
/// "today's wrong behavior"); a declared one compiles. BUG-241's file and
/// SIP-001c's verified position table are updated with it, as instructed.
/// The predecessor's framing is kept one line up in git history: it asserted
/// the drop deliberately so the fix would force test, bug and spec to be
/// updated together — which is exactly what happened.
#[test]
fn dashed_ident_statement_position_now_dispatches() {
    let (ok, out) = run_cli(".card {\n  --card-surface;\n  color: red;\n}\n", &["check"]);
    assert!(!ok, "an undeclared splice must fail:\n{out}");
    assert!(out.contains("E0947"), "must be E0947, not silence:\n{out}");

    let declared = "@form style --card-surface { background: #121722; }\n\n.card {\n  --card-surface;\n  color: red;\n}\n";
    let (ok, out) = run_cli(declared, &["check"]);
    assert!(ok, "a declared splice must compile:\n{out}");
}

// ===========================================================================
// Spike 4 (W1.3) — `dashed_ident`, the capture type BUG-242 showed was required.
// ===========================================================================

const FORM_APP_GRAMMAR: &str = r#"
%capture_type form_application { $form:dashed_ident ( $params:params )? }

%macro spike-form {
  %order 700
  %form {
    @spike-form $form:form_application {
      $props:properties
    }
  }
  %binds { noop(&self, form: $form, props: $props) }
}
"#;

fn form_app_source(form: &str) -> String {
    format!("{FORM_APP_GRAMMAR}\n.stage {{\n  @spike-form {form} {{\n    opacity: 1;\n  }}\n}}\n")
}

/// SIP-001's `form_application` works — but ONLY via `dashed_ident`.
///
/// The SIP declared `{ "--" $form:ident ... }`, which can never match: `--rise`
/// is lexed as ONE IDENT by the `RawToken::Minus` ladder, so no separate `--`
/// token exists for the literal to consume (BUG-242). The launch article's very
/// first example did not parse under the SIP's own grammar, and BUG-229 is why
/// nobody noticed: the failure was silent.
#[test]
fn form_application_accepts_a_dashed_form_reference() {
    assert_accepts(&form_app_source("--rise"), "@spike-form", "bare form");
    assert_accepts(
        &form_app_source("--rise(distance: 12px)"),
        "@spike-form",
        "form with params",
    );
    assert_accepts(
        &form_app_source("--ease-out-expo"),
        "@spike-form",
        "multi-dash form name",
    );
}

/// The reason `dashed_ident` must exist as its own capture type.
///
/// `$form:ident` would accept a BARE `rise` too, which would make the `--` sigil
/// decoration rather than meaning. A sigil that can be omitted is not a sigil.
#[test]
fn form_application_rejects_an_undashed_name() {
    assert_rejects(
        &form_app_source("rise"),
        "bare ident is not a form reference",
    );
    assert_rejects(
        &form_app_source("-webkit-thing"),
        "a SINGLE leading dash is a vendor prefix, not a form reference",
    );
}
