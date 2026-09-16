//! Shared CDP infrastructure for running `.test.st` files in real Chromium.
//!
//! PLAN-027 W2: migrated off the `playwright` npm-driver crate to the pure-Rust
//! `chromiumoxide` CDP client (`spacetime::cdp`). One warm browser is shared
//! across every `#[test]` (process-global in `spacetime::cdp`); each file runs in
//! a fresh page. The module name is kept (`playwright_st`) for test-path
//! stability, but there is no Playwright dependency anymore.
//!
//! Requires `--features cdp` and a discoverable Chrome/Chromium.

use spacetime::cdp;
use std::path::PathBuf;

/// Compile + run the given `.test.st` files in real Chromium and assert all
/// JS-level tests pass. Skips gracefully (with an eprintln) when no Chrome is
/// available so the suite is a no-op on machines without a browser rather than a
/// hard failure.
pub fn run_st_tests(paths: &[&str]) {
    if !cdp::is_available() {
        eprintln!(
            "SKIP: no Chrome/Chromium found (set SPACETIME_CHROME); skipping {:?}",
            paths
        );
        return;
    }

    let files: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
    let result = cdp::run_test_files(&files, None)
        .unwrap_or_else(|e| panic!("CDP run failed for {paths:?}: {e}"));

    if result.failed > 0 {
        let mut msg = format!(
            "\n{} passed, {} failed, {} skipped\n\nFailures:\n",
            result.passed, result.failed, result.skipped
        );
        for err in &result.errors {
            msg.push_str(&format!("  FAIL: {}\n    {}\n", err.name, err.message));
            if let Some(ref stack) = err.stack {
                for line in stack.lines().take(5) {
                    msg.push_str(&format!("    {line}\n"));
                }
            }
        }
        panic!("{msg}");
    }

    assert!(
        result.passed > 0 || result.skipped > 0,
        "no tests were executed (passed=0, skipped={})",
        result.skipped
    );
}
