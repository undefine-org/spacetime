//! A scan that examined nothing must not report success.
//!
//! # Why this file exists
//!
//! `spacetime check <dir>` printed `✓ All 0 file(s) passed` and exited 0 when it
//! found no source files at all. That output is indistinguishable from a clean
//! scan of a real tree, which makes it a *vacuous success*: the strongest
//! possible claim ("everything passed") produced by doing no work.
//!
//! This is not hypothetical tidiness. It cost real correctness twice in one
//! session:
//!
//!   1. A fallout measurement reported "zero hits across 576 files". The scan
//!      had crashed on the first file; the zero came from a dead process, not a
//!      clean tree. A regression shipped behind that zero.
//!   2. The same shape reappeared one layer along, in a build that emitted no
//!      pages and still printed `✓`.
//!
//! The defect class is *an instrument that reports success by not measuring*.
//! Any such tool will eventually be believed, because its success output is
//! byte-identical to a real one. So the contract is:
//!
//!   **Zero units examined is never success. It is a distinct, loud outcome.**
//!
//! These gates were written RED — each was seen failing against the unfixed
//! binary before the fix existed — because a gate never seen to fail is exactly
//! the kind of vacuous measurement it is here to prevent.

use std::path::Path;
use std::process::Command;

/// Run `spacetime check <path>` and return (exit_ok, combined output).
fn check(path: &Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("check")
        .arg(path)
        .output()
        .expect("spacetime binary should run");
    let mut text = String::from_utf8_lossy(&out.stdout).to_string();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.success(), text)
}

/// A unique scratch dir under the repo (never /tmp — repo convention).
fn scratch(tag: &str) -> std::path::PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scratch")
        .join(format!("vacuous-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// Checking a directory containing no source at all must NOT report success.
///
/// This is the generator case: the tool did zero work and made the maximal
/// claim.
#[test]
fn a_directory_with_no_source_is_not_a_pass() {
    let dir = scratch("empty");
    let (ok, out) = check(&dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        !ok,
        "checking a directory with no source examined nothing, so it must not \
         exit 0 — that is the vacuous success that lets a crashed or empty scan \
         masquerade as a clean one:\n{out}"
    );
    assert!(
        !out.contains("All 0 file(s) passed"),
        "`All 0 file(s) passed` states a conclusion drawn from no evidence; the \
         output must say that nothing was found instead:\n{out}"
    );
}

/// The message must name the problem, not merely fail.
///
/// A bare non-zero exit would satisfy the gate above while leaving the author
/// guessing. The whole point is that a zero-unit scan is *legible*.
#[test]
fn the_empty_scan_says_it_found_nothing() {
    let dir = scratch("says");
    let (_, out) = check(&dir);
    let _ = std::fs::remove_dir_all(&dir);

    let lower = out.to_lowercase();
    assert!(
        lower.contains("no ") && (lower.contains("file") || lower.contains("source")),
        "the diagnostic must state that no source files were found, so the \
         author can tell an empty scan from a clean one:\n{out}"
    );
}

/// Over-widening guard: a directory that DOES contain a clean file still passes.
///
/// Without this, "never report success" could be satisfied by never succeeding,
/// which would be a worse bug than the one being fixed.
#[test]
fn a_directory_with_a_clean_file_still_passes() {
    let dir = scratch("clean");
    std::fs::write(dir.join("ok.st"), "<div class=\"a\">hi</div>\n").expect("write");
    let (ok, out) = check(&dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        ok,
        "refusing an empty scan must not refuse a real one — this is the \
         over-widening guard:\n{out}"
    );
    assert!(
        out.contains("1 file"),
        "a real scan should still report the count it examined:\n{out}"
    );
}

/// Over-widening guard: a directory with a BROKEN file still fails, and for the
/// original reason (the file), not for emptiness.
#[test]
fn a_directory_with_a_broken_file_still_fails_for_that_file() {
    let dir = scratch("broken");
    std::fs::write(dir.join("bad.st"), "notvalidspacetime {{{{\n").expect("write");
    let (ok, out) = check(&dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(!ok, "a broken file must still fail the scan:\n{out}");
    assert!(
        out.contains("bad.st"),
        "the failure must still name the offending file rather than being \
         absorbed into an emptiness complaint:\n{out}"
    );
}
