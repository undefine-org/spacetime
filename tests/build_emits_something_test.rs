//! A build that produced no pages must not report success. (BUG-351)
//!
//! # Why this file exists
//!
//! `spacetime build <dir>` on a project whose source emits no page printed:
//!
//! ```text
//! ✓ Export complete:
//!   Pages: 0
//!   Files copied: 0
//!   Total size: 0 bytes
//! ```
//!
//! …and exited 0, leaving `dist/` empty. A deploy step reading that exit code
//! ships an empty site and reports a green build.
//!
//! This is the same defect class as `check` reporting `✓ All 0 file(s) passed`:
//! **an instrument that produces its success output by doing no work.** The two
//! are worth fixing together precisely because the shape recurs — it is not one
//! bug in one command, it is a habit of treating "nothing went wrong" as
//! equivalent to "something went right". They are different claims, and only the
//! second is what a `✓` promises.
//!
//! The realistic trigger is small and easy to hit: a page whose markup is
//! commented out, a file that is all styles, a project mid-refactor. Each builds
//! green and deploys nothing.
//!
//! Written RED: the first two gates were seen failing against the unfixed
//! binary.

use std::path::{Path, PathBuf};
use std::process::Command;

fn build(dir: &Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("build")
        .arg(dir)
        .output()
        .expect("spacetime binary should run");
    let mut text = String::from_utf8_lossy(&out.stdout).to_string();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.success(), text)
}

fn scratch(tag: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scratch")
        .join(format!("emit-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// A project whose only source emits no page must fail the build.
#[test]
fn a_build_that_emits_no_page_is_not_a_success() {
    let dir = scratch("styles-only");
    std::fs::write(dir.join("index.st"), ".hero { color: red; }\n").expect("write");
    let (ok, out) = build(&dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        !ok,
        "a build that exported zero pages left dist/ empty; exiting 0 tells a \
         deploy step to ship nothing and call it green:\n{out}"
    );
}

/// The refusal must say what went wrong, not just fail.
#[test]
fn the_empty_build_says_no_pages_were_emitted() {
    let dir = scratch("says");
    std::fs::write(dir.join("index.st"), ".hero { color: red; }\n").expect("write");
    let (_, out) = build(&dir);
    let _ = std::fs::remove_dir_all(&dir);

    let lower = out.to_lowercase();
    assert!(
        lower.contains("no page") || lower.contains("0 page") || lower.contains("zero page"),
        "the diagnostic must name the actual problem — that nothing was \
         emitted — rather than failing generically:\n{out}"
    );
}

/// Over-widening guard: a project WITH markup still builds and still passes.
///
/// Without this, "never succeed on zero pages" could be satisfied by never
/// succeeding.
#[test]
fn a_build_that_emits_a_page_still_succeeds() {
    let dir = scratch("real");
    std::fs::write(
        dir.join("index.st"),
        "<div class=\"hero\">hello</div>\n\n.hero { color: red; }\n",
    )
    .expect("write");
    let (ok, out) = build(&dir);
    let emitted = dir.join("dist").join("index.html").exists();
    let _ = std::fs::remove_dir_all(&dir);

    assert!(ok, "a real page must still build:\n{out}");
    assert!(
        emitted,
        "and must still actually write the page to dist/:\n{out}"
    );
}
