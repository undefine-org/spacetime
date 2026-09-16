//! W4/R1 — a `@score` placement must be written in ITS DRIVER'S time domain.
//!
//! # The rule being gated
//!
//! A score never declares a duration of its own; it inherits the domain of
//! whatever drives it (SIP-001 §Proposal/4, ruling R1). That is registry data —
//! a `domain:` column on the driver row:
//!
//! | driver              | domain     | placements read |
//! |---------------------|------------|-----------------|
//! | `time`, `loop`      | declared   | `at 2s for 3s`  |
//! | `scroll`, `visible` | normalized | `for 30%`       |
//! | `clip`, `playback`  | inherited  | from the parent |
//! | `steps`             | derived    | `at step 3`     |
//!
//! So `@score &.scroll(cover) { &x at 2s; }` is a compile error: scroll progress
//! is 0..1, and "2 seconds into a scroll" names nothing. Under `&.time(60s)` the
//! identical placement is correct.
//!
//! # Why the NEGATIVE is the point
//!
//! Both directions are asserted here, and the accepting case is the one that
//! carries the weight. A domain check that rejects EVERY placement satisfies a
//! positive-only suite — and a positive-only suite is exactly why five event
//! drivers shipped emitting the wrong trigger for months (BUG-253). A gate that
//! cannot fail in both directions is not a gate.
//!
//! # Status: RED by design
//!
//! Written before `@score` exists, per PLAN-128 t1. Every assertion here must be
//! SEEN to fail for the right reason before implementation begins.

use std::process::Command;

/// Compile a source string through the real `check` path.
///
/// NB `check` on a directory with no page compiles nothing and still reports
/// success, so the fixture is always a single concrete `index.st` file.
fn check_source(source: &str) -> (bool, String) {
    run_cli(source, &["check"])
}

/// Run `inspect --layer expansion`. This is what proves a directive MATCHED:
/// exiting 0 does not distinguish "compiled correctly" from "vanished silently",
/// and the vanishing case is the more dangerous one (BUG-216, BUG-229).
fn expansion_of(source: &str) -> String {
    run_cli(source, &["inspect", "--layer", "expansion"]).1
}

fn run_cli(source: &str, args: &[&str]) -> (bool, String) {
    let dir = std::env::temp_dir().join(format!(
        "score-domain-{}-{}",
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
    // `inspect` takes the path BEFORE its flags; `check` takes it last. Passing
    // the path first satisfies both.
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

const PAGE: &str = r#"@version 2026-06-09;

<div class="stage">
  <div class="title">Title</div>
  <div class="body">Body</div>
</div>
"#;

/// A time-driven score measures in time. This is the ACCEPTING half, and it is
/// what stops a blanket-reject implementation from passing this file.
#[test]
fn seconds_are_valid_under_a_time_driver() {
    let source = format!(
        r#"{PAGE}
.stage {{
  @score &.time(60s) {{
    &title at 2s for 3s;
  }}
}}
"#
    );

    let (ok, output) = check_source(&source);
    assert!(
        ok,
        "W4/R1: `at 2s for 3s` is CORRECT under a declared-domain driver \
         (&.time(60s)). Rejecting it means the domain check rejects everything, \
         which a positive-only suite would never catch.\nOutput:\n{output}"
    );

    // Accepting is NOT enough, and this is the half that makes the test real.
    // Today `check` accepts this source only because an unknown directive
    // vanishes silently — so without the match assertion below, this test
    // passes whether or not the feature exists, which is precisely the
    // vacuous-green class BUG-229 was filed for.
    let expansion = expansion_of(&source);
    assert!(
        expansion.contains("@score"),
        "W4/R1: the accepted score must also MATCH. It did not appear in the \
         expansion layer, so `check` passed by DROPPING the directive rather \
         than by understanding it.\nExpansion:\n{expansion}"
    );
}

/// A scroll-driven score measures in percentages.
#[test]
fn percentages_are_valid_under_a_scroll_driver() {
    let source = format!(
        r#"{PAGE}
.stage {{
  @score &.scroll(cover) {{
    &title for 30%;
  }}
}}
"#
    );

    let (ok, output) = check_source(&source);
    assert!(
        ok,
        "W4/R1: `for 30%` is CORRECT under a normalized-domain driver \
         (&.scroll). Output:\n{output}"
    );

    let expansion = expansion_of(&source);
    assert!(
        expansion.contains("@score"),
        "W4/R1: the accepted score must also MATCH — see the sibling test. A \
         green `check` on a directive that vanished proves nothing.\n\
         Expansion:\n{expansion}"
    );
}

/// The rule itself: seconds under a normalized driver name nothing.
#[test]
fn seconds_under_a_scroll_driver_are_rejected() {
    let source = format!(
        r#"{PAGE}
.stage {{
  @score &.scroll(cover) {{
    &title at 2s;
  }}
}}
"#
    );

    let (ok, output) = check_source(&source);
    assert!(
        !ok,
        "W4/R1: scroll's domain is `normalized` (0..1), so `at 2s` is \
         meaningless and must be a compile error, not a silently-ignored \
         number. Silent acceptance is the banned class.\nOutput:\n{output}"
    );
}

/// A diagnostic that does not say WHICH domain, or does not offer the right
/// spelling, leaves the author to guess. The registry knows the domain, so the
/// message can state it — and a did-you-mean is derivable from the column
/// rather than hand-written per driver.
#[test]
fn the_rejection_names_the_domain_and_suggests_a_percentage() {
    let source = format!(
        r#"{PAGE}
.stage {{
  @score &.scroll(cover) {{
    &title at 2s;
  }}
}}
"#
    );

    let (ok, output) = check_source(&source);
    assert!(
        !ok,
        "must be rejected — see the test above.\nOutput:\n{output}"
    );
    assert!(
        output.contains("E0951"),
        "W4/R1: the rejection must carry E-MEASURE-DOMAIN (E0951), not an \
         incidental unrelated error — otherwise a future refactor satisfies \
         the test above by failing for the wrong reason.\nOutput:\n{output}"
    );
    assert!(
        output.contains('%'),
        "W4/R1: the diagnostic must suggest the domain's own spelling (a \
         percentage). The registry knows the driver's domain, so the \
         did-you-mean is derivable, not hand-written.\nOutput:\n{output}"
    );
}

/// The BUG-229 lesson, applied forward.
///
/// A malformed body does not usually produce a WRONG match — it produces NO
/// match, and the directive ceases to exist. `check` then exits 0 and the score
/// is simply absent from the page. So the well-formed case must be proven to
/// MATCH, not merely to avoid erroring.
///
/// NB for the implementer: write the score body as a direct repeated body
/// capture (`{ $lines:score_line+ }`), NOT as a group (`"{" ( $lines:score_line
/// )+ "}"`). A group compiles against stdlib capture types only, so a
/// same-file `%capture_type` is invisible there and the directive never
/// matches at all.
#[test]
fn a_wellformed_score_actually_matches() {
    let source = format!(
        r#"{PAGE}
.stage {{
  @score &.time(60s) {{
    &title at 2s for 3s;
  }}
}}
"#
    );

    let expansion = expansion_of(&source);
    assert!(
        expansion.contains("@score"),
        "W4: the well-formed score must actually MATCH, not merely fail to \
         error. `inspect --layer expansion` shows no @score, so it was dropped \
         silently — indistinguishable from never having been written.\n\
         Expansion:\n{expansion}"
    );
}

/// Harness self-check — proves the DETECTOR works.
///
/// Every assertion in this file that reads "the score did not appear in the
/// expansion" is only meaningful if `expansion_of` can see a directive that IS
/// there. Without this, a broken detector that returns empty output forever
/// would make the whole file fail RED for the wrong reason, and later make it
/// pass GREEN for the wrong reason too.
///
/// `@cursor` is a shipped stdlib directive, so it must always be visible here.
/// If this test ever fails, the other failures in this file mean nothing until
/// it is fixed.
#[test]
fn the_expansion_detector_sees_a_directive_that_exists() {
    let source = format!(
        r#"{PAGE}
.stage {{
  @cursor(size: "40px")
}}
"#
    );

    let expansion = expansion_of(&source);
    assert!(
        expansion.contains("@cursor"),
        "HARNESS BROKEN: `inspect --layer expansion` cannot see @cursor, a \
         shipped stdlib directive. Every 'the score did not match' assertion \
         in this file is therefore untrustworthy — fix this first.\n\
         Expansion:\n{expansion}"
    );
}

// --- Review findings (2026-08-02 swarm) -------------------------------------
//
// Three reviewers ran against the t2 slice and all three found real defects.
// Each is locked here, because a defect found once by review and not gated is
// a defect that returns.

/// A spatial length names no position in ANY time domain.
///
/// `score_measure` admits `length` because that is the capture type carrying
/// `%`, so `at 100px` parsed happily and the check — which only recognized
/// Time and percent-Length — ignored it. The placement was accepted and then
/// discarded: the silent class this whole diagnostic exists to abolish.
#[test]
fn a_spatial_length_is_not_a_placement() {
    for unit in ["100px", "4em", "20vh"] {
        let source = format!(
            r#"{PAGE}
.stage {{
  @score &.time(60s) {{
    &title at {unit};
  }}
}}
"#
        );
        let (ok, output) = check_source(&source);
        assert!(
            !ok,
            "W4/R1: `at {unit}` measures DISTANCE. A score places clips in time \
             or in progress, so no domain admits it and it must not compile \
             silently.\nOutput:\n{output}"
        );
    }
}

/// A driver with no `domain:` column cannot drive a score, and must say so.
///
/// The first cut `continue`d silently here, so `@score &.click { … }` compiled
/// clean while every placement inside it went unchecked — a score attached to
/// a driver that has no timeline at all.
#[test]
fn a_driver_without_a_domain_cannot_drive_a_score() {
    let source = format!(
        r#"{PAGE}
.stage {{
  @score &.click {{
    &title at 2s;
  }}
}}
"#
    );

    let (ok, output) = check_source(&source);
    assert!(
        !ok,
        "W4/R1: `.click` is an EVENT — it marks a moment and has no timeline to \
         place clips in. Driving a score with it must be an error, not a score \
         whose contents are silently unchecked.\nOutput:\n{output}"
    );
    assert!(
        output.contains(".time") && output.contains(".scroll"),
        "W4/R1: the hint must list the score-capable drivers, and that list must \
         come from the registry (the `domain:` column) rather than a hand-written \
         constant that goes stale the first time a driver is added.\n\
         Output:\n{output}"
    );
}

/// `@score … as &film` names a TIMELINE ENTITY.
///
/// The first cut reused `@on`'s `on_as`, which captures `$name:binding` — a
/// `$`-sigil. `&film` therefore failed the optional capture and the alias
/// VANISHED with no diagnostic, which is why this asserts the capture is
/// PRESENT rather than merely that the source compiles: `as &film` compiled
/// green throughout the defect.
///
/// The sigils are doing exactly their job here. `@on … as $name` names a
/// progress SIGNAL (`$` = data that flows); `@score … as &film` names a
/// TIMELINE (`&` = identity, a thing you can point at). Two categories, two
/// capture types.
#[test]
fn the_score_alias_is_an_identity_and_is_actually_captured() {
    let source = format!(
        r#"{PAGE}
.stage {{
  @score &.time(60s) as &film {{
    &title at 2s;
  }}
}}
"#
    );

    let (ok, output) = check_source(&source);
    assert!(ok, "`as &film` must compile.\nOutput:\n{output}");

    let expansion = expansion_of(&source);
    assert!(
        expansion.contains("film"),
        "W4: the `as &film` alias must reach the expansion. It compiled green \
         while silently dropping the name — a timeline nothing can reference \
         is indistinguishable from one never named.\nExpansion:\n{expansion}"
    );
}
