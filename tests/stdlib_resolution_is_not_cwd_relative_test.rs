//! Where the stdlib lives must not depend on where you were standing. (BUG-326)
//!
//! # Why this file exists
//!
//! The same file, compiled by the same binary, passed or failed depending on the
//! shell's current directory:
//!
//! ```text
//! $ cd /repo        && spacetime check scratch/sl/i.st   # ✓ passed
//! $ cd /repo/scratch/sl && spacetime check i.st          # ✓ passed
//! $ cd /home/user   && spacetime check /repo/scratch/sl/i.st  # ✗ failed
//! ```
//!
//! The file imports `stdlib/md`. The resolver searched exactly two roots — a
//! configured `stdlib_path`, then `$PWD/stdlib` — so an absolute path compiled
//! from an unrelated directory could not find the stdlib that sits right next to
//! the file being compiled.
//!
//! This makes the compiler's answer a function of the invoking environment
//! rather than of the source. It breaks any caller that doesn't chdir first:
//! editors and language servers (whose cwd is arbitrary), pre-commit hooks, CI
//! steps that build several projects, and any script using absolute paths. It
//! also fails in the confusing direction — "unknown primitive" or "module not
//! found" for an import that is plainly there.
//!
//! ## The rule
//!
//! An import is resolved relative to **the file that wrote it**, by walking up
//! from that file to find the enclosing project, then to the toolchain. `$PWD`
//! is not an input. A file's meaning belongs to the file.
//!
//! Written RED: the cwd-independence gate was seen failing before the fix.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Run `spacetime check <file>` from an explicit working directory.
fn check_from(cwd: &Path, file: &Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("check")
        .arg(file)
        .current_dir(cwd)
        .output()
        .expect("spacetime binary should run");
    let mut text = String::from_utf8_lossy(&out.stdout).to_string();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.success(), text)
}

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn scratch(tag: &str) -> PathBuf {
    let dir = repo()
        .join("scratch")
        .join(format!("stdlibres-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// A page importing a stdlib module compiles the same from any cwd.
///
/// This is the whole contract. The cwd is deliberately a directory with no
/// `stdlib/` anywhere beneath or beside it.
#[test]
fn a_stdlib_import_resolves_regardless_of_cwd() {
    let dir = scratch("anycwd");
    let file = dir.join("i.st");
    std::fs::write(&file, "@import \"stdlib/md\"\n\n<div class=\"a\">hi</div>\n").expect("write");

    // Baseline: from the repo root, where `./stdlib` happens to exist.
    let (ok_root, out_root) = check_from(&repo(), &file);

    // The real test: from a directory that has no stdlib relationship at all.
    // `/` is guaranteed to exist and guaranteed not to contain our stdlib.
    let (ok_elsewhere, out_elsewhere) = check_from(Path::new("/"), &file);

    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        ok_root,
        "baseline: the file must compile from the repo root:\n{out_root}"
    );
    assert!(
        ok_elsewhere,
        "the same file, same binary, must compile from an unrelated cwd — the \
         stdlib belongs to the FILE's project, not to the shell's location. \
         An editor, language server, hook or CI step does not chdir first:\n\
         {out_elsewhere}"
    );
}

/// The failure mode was specifically that the stdlib was not found. Pin the
/// symptom so a future regression is recognizable, not just red.
#[test]
fn the_stdlib_is_actually_found_not_merely_silent() {
    let dir = scratch("found");
    let file = dir.join("i.st");
    std::fs::write(&file, "@import \"stdlib/md\"\n\n<div class=\"a\">hi</div>\n").expect("write");

    let (_, out) = check_from(Path::new("/"), &file);
    let _ = std::fs::remove_dir_all(&dir);

    let lower = out.to_lowercase();
    assert!(
        !lower.contains("could not resolve") && !lower.contains("not found"),
        "resolution must succeed rather than report a missing module:\n{out}"
    );
}

/// Over-widening guard: a genuinely missing module must STILL fail.
///
/// Widening the search until everything resolves would "fix" this bug by making
/// the resolver unable to say no.
#[test]
fn a_module_that_does_not_exist_still_fails() {
    let dir = scratch("missing");
    let file = dir.join("i.st");
    std::fs::write(
        &file,
        "@import \"stdlib/definitely-not-a-real-module\"\n\n<div class=\"a\">hi</div>\n",
    )
    .expect("write");

    let (ok, out) = check_from(&repo(), &file);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        !ok,
        "a nonexistent module must still be refused — resolution must keep the \
         ability to say no:\n{out}"
    );
}

/// The BARE aggregate import must be cwd-independent too.
///
/// `@import "stdlib"` and `@import "stdlib/md"` are the same author-level
/// question — "give me the stdlib" — so they must not have different answers
/// depending on where the shell was standing.
///
/// These travel through two DIFFERENT layers, which is why fixing one did not
/// fix the other:
///   - `stdlib/<module>` goes through `ImportResolver::resolve_stdlib`, which
///     now anchors to the importing file.
///   - bare `stdlib` additionally needs the METASYSTEM REGISTRY, loaded once per
///     process via a `LazyLock` over `STDLIB_DIRS` — relative paths joined
///     against the process cwd. When that misses, the load silently falls back
///     to the EMBEDDED registry and the failure surfaces as an unrelated
///     internal complaint (`unknown primitive: element-refs`) rather than
///     "stdlib not found".
///
/// That misleading symptom is the real cost: the diagnostic blames a primitive
/// the author never wrote.
#[test]
fn a_bare_stdlib_import_also_resolves_regardless_of_cwd() {
    let dir = scratch("bare");
    let file = dir.join("i.st");
    std::fs::write(&file, "@import \"stdlib\"\n\n<div class=\"a\">hi</div>\n").expect("write");

    let (ok_root, out_root) = check_from(&repo(), &file);
    let (ok_elsewhere, out_elsewhere) = check_from(Path::new("/"), &file);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        ok_root,
        "baseline: bare stdlib import must compile from the repo root:\n{out_root}"
    );
    assert!(
        ok_elsewhere,
        "bare `@import \"stdlib\"` must resolve from an unrelated cwd, exactly \
         like `@import \"stdlib/md\"` does. It currently falls back to the \
         embedded registry and reports a confusing internal-primitive error \
         instead:\n{out_elsewhere}"
    );
}

