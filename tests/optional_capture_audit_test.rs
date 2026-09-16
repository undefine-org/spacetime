//! Audit sentinel for optional directive captures (BUG-264).
//!
//! An optional capture whose value is a NEAR-MISS must be refused loudly, not
//! silently dropped. Two mechanisms make that true, and both are asserted here:
//!
//! 1. CLEAN-MATCH RANKING — a LENIENT productive match (one that dropped the
//!    clause it could not bind) ranks BELOW a matching error sibling. `portal`
//!    is declared BEFORE `portal-invalid-close` in stdlib/macros/portal.st, so
//!    the productive form is tried first; for `close: close` it matches
//!    leniently (drops the bare name) and the ranking refuses it in favour of
//!    the error sibling. DISCRIMINATION GATE: disable the lenient defer in
//!    match_sink (accept lenient matches immediately) and `close: close`
//!    silently compiles — this test fails. That proves the test exercises the
//!    ranking, not the sibling's declaration order.
//! 2. STRICT CAPTURES — the productive `:duration` body capture rejects a
//!    bare number, so `refresh: 5` never matches `data-fetch-opts`; the
//!    `data-fetch-opts-invalid-refresh` error sibling (declared after it) then
//!    reports the near-miss with its data-supplied message.

use std::process::Command;

const REPO_ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn check(source: &str) -> String {
    let dir = tempfile::tempdir().expect("tempdir");
    let entry = dir.path().join("index.st");
    std::fs::write(&entry, source).expect("fixture writes");

    let output = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .current_dir(REPO_ROOT)
        .args(["check", entry.to_str().expect("utf-8 path")])
        .output()
        .expect("spacetime check runs");
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn portal_close_with_a_bare_name_is_refused_not_dropped() {
    // The productive `portal` form matches `close: close` LENIENTLY (it drops the
    // bare name it cannot bind as `$closeSignal:binding`); the clean-match ranking
    // must make the `portal-invalid-close` error sibling win and emit E0955.
    let text = check(
        r#"@version 2026-06-09;
$open bool: false;
<div class="stage">A</div>
.stage { @portal(when: $open, close: close); }
"#,
    );
    assert!(
        text.contains("E0955") && text.contains("close: $close"),
        "a bare close name must be refused with E0955 naming the `$close` repair; \
         otherwise the lenient productive match silently drops the portal. Got:\n{text}"
    );
}

#[test]
fn portal_close_with_a_signal_is_valid() {
    let text = check(
        r#"@version 2026-06-09;
$open bool: false;
$close bool: false;
<div class="stage">A</div>
.stage { @portal(when: $open, close: $close); }
"#,
    );
    assert!(
        !text.contains("E0955"),
        "a `$`-sigil close must compile clean; the error sibling must not steal a \
         valid portal. Got:\n{text}"
    );
}

#[test]
fn fetch_refresh_with_a_bare_number_is_refused_not_dropped() {
    // STRICT CAPTURES: `:duration` rejects a unit-less number, so `data-fetch-opts`
    // never matches `refresh: 5`; the `data-fetch-opts-invalid-refresh` error sibling
    // reports it with the `5s` repair instead of silently zeroing the refresh.
    let text = check(
        r#"@version 2026-06-09;
<div class="stage">A</div>
@data fetch $x Product[] : "/api/products" { refresh: 5; }
"#,
    );
    assert!(
        text.contains("E0955") && text.contains("refresh: 5s"),
        "a bare-number refresh must be refused with E0955 naming the `5s` repair; \
         otherwise the strict capture silently drops the refresh clause. Got:\n{text}"
    );
}

#[test]
fn fetch_refresh_with_a_duration_is_valid() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("data.json"), "[{\"a\":1}]").expect("data file");
    let entry = dir.path().join("index.st");
    std::fs::write(
        &entry,
        r#"@version 2026-06-09;
@data fetch $x : "./data.json" { refresh: 5s; }
@each item in $x { <div class="stage">A</div> }
"#,
    )
    .expect("fixture writes");

    let output = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .current_dir(REPO_ROOT)
        .args(["check", entry.to_str().expect("utf-8 path")])
        .output()
        .expect("spacetime check runs");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !text.contains("E0955"),
        "a `refresh: 5s` duration must compile clean; the error sibling must not \
         steal a valid refresh. Got:\n{text}"
    );
}

#[test]
fn fetch_refresh_with_a_wrong_unit_length_is_refused_not_dropped() {
    // Reviewer 67.0: the strict guard used to reject only f64-PARSEABLE values,
    // so `refresh: 5px` converted to Expr, bound cleanly, and silently dropped
    // (the timer never started). Now any non-duration, non-`$` value rejects and
    // the error sibling reports it.
    let text = check(
        r#"@version 2026-06-09;
<div class="stage">A</div>
@data fetch $x Product[] : "/api/products" { refresh: 5px; }
"#,
    );
    assert!(
        text.contains("E0955") && text.contains("refresh: 5s"),
        "a wrong-unit refresh (`5px`) must be refused with E0955 naming the `5s` repair; \
         otherwise it silently drops. Got:\n{text}"
    );
}

#[test]
fn fetch_refresh_with_a_bare_ident_is_refused_not_dropped() {
    // Reviewer 67.0: `refresh: foo` (a bare ident, not a `$` signal) used to
    // convert to Expr and silently drop. It must be refused loudly instead.
    let text = check(
        r#"@version 2026-06-09;
<div class="stage">A</div>
@data fetch $x Product[] : "/api/products" { refresh: foo; }
"#,
    );
    assert!(
        text.contains("E0955") && text.contains("refresh: 5s"),
        "a bare-ident refresh (`foo`) must be refused with E0955 naming the `5s` repair; \
         otherwise it silently drops. Got:\n{text}"
    );
}

#[test]
fn fetch_refresh_with_a_signal_is_valid() {
    // A `$`-sigiled signal reference is the productive form's OTHER legal
    // value (the error sibling's own hint: "a duration … or a `$signal`"). It
    // must keep binding cleanly — never routed to the error sibling.
    let text = check(
        r#"@version 2026-06-09;
$poll bool: false;
<div class="stage">A</div>
@data fetch $x Product[] : "/api/products" { refresh: $poll; }
"#,
    );
    assert!(
        !text.contains("E0955"),
        "a `$`-sigiled refresh signal must compile clean; the error sibling must not \
         steal a valid refresh. Got:\n{text}"
    );
}

#[test]
fn fetch_refresh_with_zero_is_valid() {
    // Reviewer 67.0: ZERO needs no unit (CSS convention) and is the stdlib's
    // own no-auto-refresh sentinel (`refresh: 0` — data-kind.st:82,482). It
    // must bind cleanly, not trip the strict bare-number rejection.
    let text = check(
        r#"@version 2026-06-09;
<div class="stage">A</div>
@data fetch $x Product[] : "/api/products" { refresh: 0; }
"#,
    );
    assert!(
        !text.contains("E0955"),
        "a `refresh: 0` no-auto-refresh sentinel must compile clean; the strict \
         capture must not reject the stdlib's own disable value. Got:\n{text}"
    );
}
