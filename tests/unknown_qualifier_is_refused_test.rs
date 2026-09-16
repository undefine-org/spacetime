//! A directive with an unbound module qualifier must be refused. (BUG-301)
//!
//! # Why this file exists
//!
//! `@typo/badge` names a directive `badge` in a module `typo` that was never
//! imported. Both `check` and `build` accepted it:
//!
//! ```text
//! $ spacetime check index.st
//! warning[W0714]: `@typo/badge` is not a known directive and was ignored
//! 0 error(s), 1 warning(s)     exit 0
//!
//! $ spacetime build .          exit 0, dist/index.html written
//! ```
//!
//! The page ships with the directive **silently removed**. Whatever `@typo/badge`
//! was supposed to do — render a badge, bind an event, mount a component —
//! simply does not happen, and the build says everything is fine.
//!
//! ## Why a warning is the wrong severity here
//!
//! A typo in a qualifier is not a style question with a defensible other side.
//! There is no program for which "you wrote a directive I do not recognize, so I
//! deleted it" is the intended outcome. The author asked for behavior and got a
//! page without it.
//!
//! E0926 (unbound import qualifier) already exists in `src/diagnostics/codes.rs`
//! for exactly this. It was not reaching this path.
//!
//! ## The measurement trap this sits next to
//!
//! While reproducing this, `spacetime check f.st 2>&1 | tail -3; echo $?`
//! reported `exit=0` for a command that exits 1 — `$?` after a pipeline is the
//! exit status of `tail`. Every gate below runs the binary directly and reads
//! its real status, because a test that measures the wrong thing is worse than
//! no test: it reports confidently.
//!
//! Written RED: the refusal gates were seen failing (warning + exit 0) first.

use std::path::{Path, PathBuf};
use std::process::Command;

fn run(subcommand: &str, target: &Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg(subcommand)
        .arg(target)
        .output()
        .expect("spacetime binary should run");
    let mut text = String::from_utf8_lossy(&out.stdout).to_string();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.success(), text)
}

fn scratch(tag: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scratch")
        .join(format!("qual-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

const UNKNOWN_QUALIFIER: &str = "<div class=\"a\">hello</div>\n\n.a { @typo/badge; }\n";

/// `check` must refuse a directive whose qualifier was never imported.
#[test]
fn check_refuses_an_unbound_qualifier() {
    let dir = scratch("check");
    let file = dir.join("index.st");
    std::fs::write(&file, UNKNOWN_QUALIFIER).expect("write");
    let (ok, out) = run("check", &file);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        !ok,
        "`@typo/badge` names a module that was never imported. Accepting it \
         drops the directive and ships a page missing the behavior the author \
         asked for:\n{out}"
    );
}

/// …and so must `build`, which is the one that actually ships the page.
#[test]
fn build_refuses_an_unbound_qualifier() {
    let dir = scratch("build");
    std::fs::write(dir.join("index.st"), UNKNOWN_QUALIFIER).expect("write");
    let (ok, out) = run("build", &dir);
    let shipped = dir.join("dist").join("index.html").exists();
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        !ok,
        "build accepted an unbound qualifier and exited 0:\n{out}"
    );
    assert!(
        !shipped,
        "worse, it WROTE the page with the directive silently removed \u{2014} a \
         deploy of this build serves a page missing the behavior:\n{out}"
    );
}

/// The diagnostic must name the qualifier, so the typo is findable.
#[test]
fn the_refusal_names_the_unknown_qualifier() {
    let dir = scratch("names");
    let file = dir.join("index.st");
    std::fs::write(&file, UNKNOWN_QUALIFIER).expect("write");
    let (_, out) = run("check", &file);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        out.contains("typo"),
        "the diagnostic must name the offending qualifier so the author can \
         find the typo:\n{out}"
    );
}

/// Over-widening guard: an ordinary directive with no qualifier still compiles.
#[test]
fn a_plain_directive_still_compiles() {
    let dir = scratch("plain");
    let file = dir.join("index.st");
    std::fs::write(&file, "<div class=\"a\">hello</div>\n\n.a { color: red; }\n").expect("write");
    let (ok, out) = run("check", &file);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        ok,
        "refusing unknown qualifiers must not refuse ordinary source:\n{out}"
    );
}
