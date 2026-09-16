//! BUG-107 regression guard: per-file isolation in the `--headless` V8 runner.
//!
//! The `--headless` runner used to execute EVERY test file in ONE shared V8
//! context with no reset between files. The `st.js` page-global MutationObserver
//! (installed once, observing `document.body` for the whole run, never
//! disconnected) plus shared singletons (`ST._dataRegistry`,
//! `Spacetime.templates`, `ST._selectorInitializers`, accumulated DOM) leaked
//! across files. Once a SECOND file mounted elements / drove `@each` data, the
//! leaked observer fired on the FIRST file's stale nodes and threw
//! `Cannot read properties of null (reading 'nodeType')` — turning passing tests
//! RED purely by run ORDER.
//!
//! The fix runs each file in its OWN fresh V8 context (src/main.rs
//! `new_headless_runtime` + the per-file loop in `run_headless_tests`), so no
//! observer/registry/template/DOM state can cross a file boundary.
//!
//! This guard mounts + drives `@each` in TWO sibling files and runs them THROUGH
//! the multi-file harness in one invocation, asserting BOTH all-pass in BOTH
//! orders. Pre-fix this FAILS with the `nodeType` error; it is the regression
//! pin that makes a future re-sharing of harness state fail loudly.

#![cfg(feature = "headless")]

use std::path::Path;
use std::process::Command;

/// Run `spacetime test <files…> --headless --format json` and return
/// `(passed, failed)` parsed from the emitted JSON.
fn run_headless(files: &[&str]) -> (u64, u64, String) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut args: Vec<String> = vec!["test".to_string()];
    for f in files {
        args.push(root.join(f).to_str().unwrap().to_string());
    }
    args.push("--headless".to_string());
    args.push("--format".to_string());
    args.push("json".to_string());

    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .args(&args)
        .output()
        .expect("run headless");
    let stdout = String::from_utf8_lossy(&out.stdout);
    // The JSON object is pretty-printed; slice from first `{` to last `}`.
    let json = match (stdout.find('{'), stdout.rfind('}')) {
        (Some(a), Some(b)) if b > a => &stdout[a..=b],
        _ => "{}",
    };
    let v: serde_json::Value = serde_json::from_str(json).unwrap_or_default();
    let passed = v["passed"].as_u64().unwrap_or(0);
    let failed = v["failed"].as_u64().unwrap_or(0);
    (passed, failed, stdout.to_string())
}

const FILE_A: &str = "tests/fixtures/isolation/iso-file-a.test.st";
const FILE_B: &str = "tests/fixtures/isolation/iso-file-b.test.st";

#[test]
fn each_file_passes_in_isolation() {
    let (pa, fa, outa) = run_headless(&[FILE_A]);
    assert_eq!(fa, 0, "file A alone must not fail; stdout={outa}");
    assert_eq!(pa, 1, "file A alone must report 1 pass; stdout={outa}");

    let (pb, fb, outb) = run_headless(&[FILE_B]);
    assert_eq!(fb, 0, "file B alone must not fail; stdout={outb}");
    assert_eq!(pb, 1, "file B alone must report 1 pass; stdout={outb}");
}

#[test]
fn two_mount_files_do_not_cross_contaminate_a_then_b() {
    let (passed, failed, stdout) = run_headless(&[FILE_A, FILE_B]);
    assert_eq!(
        failed, 0,
        "BUG-107: A then B must not cross-contaminate (leaked observer nodeType throw); \
         got passed={passed} failed={failed}; stdout={stdout}"
    );
    assert_eq!(passed, 2, "both isolation tests must pass; stdout={stdout}");
}

#[test]
fn two_mount_files_do_not_cross_contaminate_b_then_a() {
    let (passed, failed, stdout) = run_headless(&[FILE_B, FILE_A]);
    assert_eq!(
        failed, 0,
        "BUG-107: B then A must not cross-contaminate; got passed={passed} failed={failed}; \
         stdout={stdout}"
    );
    assert_eq!(passed, 2, "both isolation tests must pass; stdout={stdout}");
}
