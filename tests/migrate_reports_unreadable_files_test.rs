//! `migrate` must not report success over a file it could not read. (AUD-010)
//!
//! # Why this file exists
//!
//! With an unparseable `.st` in the tree, `spacetime migrate <dir>` printed:
//!
//! ```text
//! No pending migrations — the project is current.     exit 0
//! ```
//!
//! It cannot know that. A file it failed to parse is a file it did not inspect,
//! so "the project is current" is a claim about evidence it never had. The
//! sentence an author reads is "everything is migrated"; the truth is "some files
//! were skipped, and one of them may be the one holding retired syntax".
//!
//! This is the same defect class as `check` printing `✓ All 0 file(s) passed` and
//! `build` printing `✓ Export complete: Pages: 0` — a tool emitting its success
//! output over work it did not do. It is the third instance found in one session,
//! which is why it is worth naming as a class rather than fixing three times:
//!
//!   **A tool must not report on units it could not examine.**
//!
//! The migration case is the most dangerous of the three. `check` and `build`
//! fail visibly later; a missed migration leaves retired syntax in a file that
//! nothing will revisit, and the next wave silently widens the gap.
//!
//! ## Why report-and-fail rather than refuse
//!
//! Refusing to migrate anything while one file is unparseable would block a
//! legitimate migration of a large tree containing one broken file — and that
//! broken file may be broken precisely BECAUSE it needs the migration. So the
//! migration still runs on every file it can read; what changes is that the
//! skipped files are named and the exit code is non-zero, so no script can
//! mistake a partial pass for a complete one.
//!
//! Written RED: the gates were seen failing against the unfixed binary.

use std::path::{Path, PathBuf};
use std::process::Command;

/// An `.st` file that genuinely fails to PARSE.
///
/// NB the distinction that matters here: E0946 ("directive does not match its
/// declared grammar") is a FORM-MATCH failure, and `parser::parse()` succeeds on
/// it — those files ARE read and migrated. Only a file the parser itself
/// rejects leaves the plan list, which is the case this gate is about. The first
/// version of this fixture used an E0946 shape and stayed red against a working
/// fix, which is a good reminder that a RED gate can be red for the wrong reason
/// too.
const UNPARSEABLE: &str = "this is not {{{ spacetime at all\n";
const CLEAN: &str = "<div class=\"a\">hi</div>\n\n.a { color: red; }\n";

fn migrate(dir: &Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("migrate")
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
        .join(format!("mig-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// A file that could not be parsed must be NAMED, not silently skipped.
#[test]
fn migrate_names_a_file_it_could_not_parse() {
    let dir = scratch("names");
    std::fs::write(dir.join("bad.st"), UNPARSEABLE).expect("write");
    std::fs::write(dir.join("good.st"), CLEAN).expect("write");
    let (_, out) = migrate(&dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        out.contains("bad.st"),
        "migrate skipped a file it could not parse without naming it, so the \
         author has no way to know it was never inspected:\n{out}"
    );
}

/// …and must not claim the project is current when it skipped something.
#[test]
fn migrate_does_not_claim_current_when_it_skipped_a_file() {
    let dir = scratch("current");
    std::fs::write(dir.join("bad.st"), UNPARSEABLE).expect("write");
    std::fs::write(dir.join("good.st"), CLEAN).expect("write");
    let (ok, out) = migrate(&dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        !ok,
        "migrate exited 0 over a file it never read; a CI step cannot tell this \
         from a complete migration:\n{out}"
    );
    assert!(
        !out.contains("the project is current"),
        "\"the project is current\" is a claim about files that were inspected — \
         it must not be printed when some were skipped:\n{out}"
    );
}

/// Over-widening guard: a tree with no unparseable file still reports current
/// and still exits 0.
#[test]
fn a_clean_tree_still_reports_current() {
    let dir = scratch("clean");
    std::fs::write(dir.join("good.st"), CLEAN).expect("write");
    let (ok, out) = migrate(&dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        ok,
        "reporting skipped files must not make a clean migration fail:\n{out}"
    );
}
