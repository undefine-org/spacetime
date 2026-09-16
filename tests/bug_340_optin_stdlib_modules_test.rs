//! BUG-340: every opt-in stdlib module must be reachable by `@import`.
//!
//! `stdlib/md`, `stdlib/text`, `stdlib/3d` are OPT-IN: they are deliberately
//! absent from `STDLIB_DIRS` (the always-on scan) and load from disk when a
//! page imports them. `resolve_stdlib` checked the EMBEDDED module index
//! first, and `build_stdlib_index` indexes every `.st` under `stdlib/` by bare
//! name — so `stdlib/md/index.st` registered the name `"md"`, the embedded
//! branch matched, and the import returned `ResolvedImport::Stdlib`. That
//! variant is a dead end for these modules: `resolve_imports` skips it
//! ("handled by MetaRegistry") and `MetaRegistry` never loads them.
//!
//! Result: every opt-in module went dark at once — `@doc` (and therefore EVERY
//! literate `.st.md` page, since the tangler synthesizes `@import "stdlib/md"`
//! for prose), `@balance`, `@stage` — each failing with "unknown primitive".
//!
//! Nothing in the suite compiled a page through one of these modules, which is
//! why a whole authoring rail could break without a single red test. This file
//! is that missing coverage: one page per module, built the way a user builds.

use std::process::Command;

fn build_page(name: &str, source: &str) -> (bool, String) {
    let dir = std::env::temp_dir().join(format!("bug340-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(dir.join("index.st"), source).expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("build")
        .arg(&dir)
        // The resolver falls back to `<cwd>/stdlib`, and a user builds from the
        // repo root, so reproduce that exactly.
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run build");
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
    (out.status.success() && !log.contains("Export failed"), log)
}

#[test]
fn stdlib_md_is_importable() {
    let (ok, log) = build_page(
        "md",
        "@import \"stdlib/md\"\n\n<div class=\"d\"></div>\n.d { @doc(content: \"# hi\"); }\n",
    );
    assert!(
        ok,
        "`@import \"stdlib/md\"` must make `@doc` resolvable — this failing means \
         the import returned the embedded-stdlib dead end instead of the file on \
         disk, and every literate `.st.md` page in the repo is dead with it:\n{log}"
    );
}

#[test]
fn stdlib_text_is_importable() {
    let (ok, log) = build_page(
        "text",
        "@import \"stdlib/text\"\n\n<h1 class=\"t\">Hello world</h1>\n.t { @balance; }\n",
    );
    assert!(
        ok,
        "`@import \"stdlib/text\"` must make `@balance` resolvable:\n{log}"
    );
}

#[test]
fn stdlib_3d_is_importable() {
    let (ok, log) = build_page(
        "3d",
        "@import \"stdlib/3d\"\n\n<div class=\"s\"></div>\n.s { @stage; }\n",
    );
    assert!(
        ok,
        "`@import \"stdlib/3d\"` must make `@stage` resolvable:\n{log}"
    );
}

/// A literate document imports nothing by hand — the tangler synthesizes
/// `@import "stdlib/md"` for its prose. So this is the same rail as the first
/// test, reached the way an author actually reaches it, and it is the one that
/// would have caught the regression on the day it landed.
#[test]
fn a_literate_page_builds_end_to_end() {
    let dir = std::env::temp_dir().join(format!("bug340-lit-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(
        dir.join("index.st.md"),
        "# Title\n\nSome prose that must render.\n\n```st\n<p class=\"x\">x</p>\n.x { color: #111; }\n```\n",
    )
    .expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("build")
        .arg(&dir)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run build");
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let html = std::fs::read_to_string(dir.join("dist").join("index.html")).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        out.status.success() && !log.contains("Export failed"),
        "a literate `.st.md` page must build — the `stdlib/md` import is \
         synthesized for its prose, so a broken module rail kills every one of \
         them at once:\n{log}"
    );
    assert!(
        html.contains("data-lit-seg"),
        "the prose segment must reach the document — building green while \
         emitting no prose would be the same silence one layer down:\n{html}"
    );
}
