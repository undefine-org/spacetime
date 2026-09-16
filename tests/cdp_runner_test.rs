//! W2 CDP substrate proof (PLAN-027). Requires --features cdp + a local Chrome.
#![cfg(feature = "cdp")]

use spacetime::cdp;
use std::io::Write;

fn write_tmp(name: &str, src: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(name);
    let mut f = std::fs::File::create(&p).unwrap();
    f.write_all(src.as_bytes()).unwrap();
    p
}

#[test]
fn cdp_runs_logic_test() {
    if !cdp::is_available() {
        eprintln!("SKIP: no Chrome found");
        return;
    }
    let f = write_tmp(
        "cdp_logic.test.st",
        r#"@import "stdlib/testing/test"
@test "exists on CDP" { @fixture { <div class="x">X</div> } @then .x should exist }
"#,
    );
    let r = cdp::run_test_files(&[f.clone()], None).expect("cdp run");
    std::fs::remove_file(&f).ok();
    assert_eq!(r.failed, 0, "logic test should pass on CDP: {:?}", r.errors);
    assert_eq!(r.passed, 1, "expected 1 pass, got {}", r.passed);
}

#[test]
fn cdp_runs_layout_test_that_v8_refuses() {
    if !cdp::is_available() {
        eprintln!("SKIP: no Chrome found");
        return;
    }
    // A layout assertion (be_visible -> real offsetParent). On V8 this REFUSES
    // (W1); on CDP it RUNS for real. The element is visible, so it passes.
    let f = write_tmp(
        "cdp_layout.test.st",
        r#"@import "stdlib/testing/test"
@test "visible on CDP" needs layout { @fixture { <div class="v" style="display:block">V</div> } @then .v should be_visible }
"#,
    );
    let r = cdp::run_test_files(&[f.clone()], None).expect("cdp run");
    std::fs::remove_file(&f).ok();
    assert_eq!(
        r.failed, 0,
        "layout test should PASS on CDP (real layout): {:?}",
        r.errors
    );
    assert_eq!(r.passed, 1, "expected 1 pass, got {}", r.passed);
}
