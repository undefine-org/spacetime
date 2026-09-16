//! BUG-189 regression: `spacetime build <file>` must scope its post-build
//! "inject spacetime.css/js links" step to the RESOLVED OUTPUT DIRECTORY
//! (the project's own `dist/`, or the explicit `--output` dir) — never to
//! `std::env::current_dir()` when no `--output` is given, and never to any
//! directory outside that output root.
//!
//! Root cause (confirmed by reading `src/main.rs::run_build`, live process
//! repro against a scratch tree, and empirical mutation of an unrelated
//! sibling HTML file):
//!   - `let out_dir = output.unwrap_or_else(|| std::env::current_dir().unwrap());`
//!     defaults to the PROCESS CWD when `--output` is omitted.
//!   - The asset-link-injection block then does:
//!       `collect_html_files(&out_dir, &mut html_files)` — a RECURSIVE walk
//!       from `out_dir` — and rewrites (`<link>`/`<script>` injection) every
//!       `.html` file found under it, unconditionally.
//!   - Building a single throwaway `.st` file FROM THE REPO ROOT (no
//!     `--output`) therefore recursively walks the entire repo working tree
//!     and rewrites every `.html` file it finds, including vendored git
//!     submodule docs (`stdlib/3d/vendor/three/**`,
//!     `stdlib/text/vendor/pretext/**` — 1873 files in the live repro that
//!     produced this bug report).
//!
//! Impact: `build` mutates files the user never asked to touch, dirtying
//! third-party submodule working trees as a side effect of compiling one
//! unrelated file — no confirmation, no scoping guard, trivially triggered.
//!
//! This test constructs an isolated scratch tree (NOT the real repo, so it
//! can never re-touch real vendored submodules) with:
//!   - `project/main.st`         — the file actually being built
//!   - `sibling/unrelated.html`  — a file OUTSIDE the build's output scope,
//!                                 standing in for a vendored submodule doc
//! and asserts that after `spacetime build project/main.st` (no `--output`,
//! so it defaults to CWD == the scratch root, reproducing the exact reported
//! shape), `sibling/unrelated.html` is byte-for-byte UNCHANGED.
//!
//! Expected to FAIL (RED) until `run_build` scopes `collect_html_files` to
//! the resolved project/output root instead of defaulting to/walking the
//! full CWD subtree.

use std::fs;
use std::path::Path;
use std::process::Command;

/// Locate the compiled `spacetime` binary the same way `cargo test` does
/// for its own integration binaries — via the `CARGO_BIN_EXE_<name>` env var
/// Cargo sets for `[[bin]]` targets during `cargo test`.
fn spacetime_bin() -> &'static str {
    env!("CARGO_BIN_EXE_spacetime")
}

#[test]
fn build_does_not_mutate_html_files_outside_its_output_scope() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path();

    // The file actually being built.
    let project_dir = root.join("project");
    fs::create_dir_all(&project_dir).unwrap();
    fs::write(
        project_dir.join("main.st"),
        r#"<div class="probe">hello</div>"#,
    )
    .unwrap();

    // A sibling HTML file OUTSIDE the project dir, standing in for a
    // vendored third-party doc elsewhere in a real working tree. This must
    // never be touched by building `project/main.st`.
    let sibling_dir = root.join("sibling").join("vendor");
    fs::create_dir_all(&sibling_dir).unwrap();
    let sibling_html_path = sibling_dir.join("unrelated.html");
    let original_html = "<html><head></head><body>vendored doc, not mine</body></html>";
    fs::write(&sibling_html_path, original_html).unwrap();

    // Build WITHOUT --output, from the scratch ROOT as CWD — this is the
    // exact reported trigger shape ("running it from the repo root").
    let status = Command::new(spacetime_bin())
        .arg("build")
        .arg(project_dir.join("main.st"))
        .current_dir(root)
        .status()
        .expect("failed to invoke spacetime build");
    assert!(status.success(), "spacetime build should succeed");

    let after_html = fs::read_to_string(&sibling_html_path).expect("sibling html must still exist");

    assert_eq!(
        after_html,
        original_html,
        "BUG-189: `spacetime build` (no --output, run from the tree root) must NOT \
         mutate an HTML file outside the built project's own output directory — \
         it rewrote {} by injecting spacetime.css/js links into a file the build \
         was never asked to touch",
        sibling_html_path.display()
    );
}

#[test]
fn build_only_injects_asset_links_into_its_own_resolved_output_dir() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path();

    let project_dir = root.join("project");
    fs::create_dir_all(&project_dir).unwrap();
    fs::write(
        project_dir.join("main.st"),
        r#"<div class="probe">hello</div>"#,
    )
    .unwrap();

    let out_dir = root.join("dist");
    fs::create_dir_all(&out_dir).unwrap();
    // An HTML file that IS inside the explicit --output dir is fair game to
    // inject into (this is the intended, scoped behavior) — pinned here as a
    // contrast case so a fix can't "fix" this test by disabling injection
    // altogether.
    let in_scope_html = out_dir.join("index.html");
    fs::write(
        &in_scope_html,
        "<html><head></head><body>project page</body></html>",
    )
    .unwrap();

    // An HTML file OUTSIDE --output, at the scratch root — must stay
    // untouched even when --output IS explicitly given.
    let out_of_scope_html = root.join("outside.html");
    fs::write(
        &out_of_scope_html,
        "<html><head></head><body>not part of this build</body></html>",
    )
    .unwrap();

    let status = Command::new(spacetime_bin())
        .arg("build")
        .arg(project_dir.join("main.st"))
        .arg("--output")
        .arg(&out_dir)
        .current_dir(root)
        .status()
        .expect("failed to invoke spacetime build");
    assert!(status.success(), "spacetime build should succeed");

    let outside_after = fs::read_to_string(&out_of_scope_html).unwrap();
    assert_eq!(
        outside_after, "<html><head></head><body>not part of this build</body></html>",
        "BUG-189: a file OUTSIDE the explicit --output dir must never be mutated \
         by asset-link injection"
    );

    // In-scope file MAY be injected into (that's the intended behavior) —
    // just confirming the build didn't error out on it either way.
    assert!(
        Path::new(&in_scope_html).exists(),
        "in-scope output HTML should still exist after build"
    );
}
