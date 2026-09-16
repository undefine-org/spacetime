//! A bare relative import resolves beside the file that WROTE it.
//!
//! `@import "_helpers.st"` is the ordinary way to share declarations across a
//! project's pages. It was searched for ONLY under the workspace root, never
//! beside the importing file:
//!
//! ```text
//! Could not resolve import '_helpers.st' from projects/animations/index.st.
//! Searched: ["/home/user/code/ora/verse/_helpers.st"]
//! ```
//!
//! The importing file lives in `projects/animations/`; `_helpers.st` sits right
//! next to it. `./_helpers.st` resolved (there is an explicit relative-import
//! branch), but the bare spelling — which is what everyone writes — did not.
//!
//! Same class as BUG-227 and BUG-326: a path anchored to the wrong base. The
//! only base under which an author's relative path means what they wrote is the
//! DIRECTORY OF THE FILE CONTAINING THE IMPORT.
//!
//! Driven through `resolve_imports`, the entry point the compiler itself uses,
//! with the workspace root DELIBERATELY set elsewhere — that mismatch between
//! "where the toolchain is" and "where the project is" is the whole bug.
//!
//! These gates were seen RED before the fix.

use spacetime::parser::resolve_imports;
use std::fs;
use std::path::{Path, PathBuf};

/// A throwaway tree under `target/`, outside every source dir a corpus check
/// walks.
struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("import-fixtures")
            .join(name);
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("fixture root");
        Self { root }
    }

    fn write(&self, rel: &str, body: &str) -> PathBuf {
        let path = self.root.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("fixture parent");
        }
        fs::write(&path, body).expect("fixture write");
        path
    }

    /// Resolve `page`'s imports with the workspace root pointed at the
    /// TOOLCHAIN, not at the fixture — the real situation for any project that
    /// is not the compiler's own checkout.
    fn resolve(&self, page: &Path) -> Result<usize, String> {
        let src = fs::read_to_string(page).expect("read page");
        let ast = spacetime::parse(&src).map_err(|e| format!("parse: {e}"))?;
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let merged = resolve_imports(&ast, page, &workspace_root)?;
        // What matters is that resolution SUCCEEDED and the imported file's
        // content came across. `@form easing --x { ... }` lands in `matches`
        // (it is a form MATCH, not a `%macro`/`%primitive` meta_def), so count
        // every carrier an imported declaration can land in rather than
        // guessing one — the contract is "something arrived", not "it arrived
        // in this particular field".
        Ok(merged.matches.len()
            + merged.meta_defs.len()
            + merged.scopes.len()
            + merged.presets.len())
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// The reported bug, reduced.
#[test]
fn a_bare_sibling_import_resolves_beside_the_importing_file() {
    let fx = Fixture::new("sibling");
    fx.write("_helpers.st", "@form easing --x { linear }\n");
    let index = fx.write("index.st", "@import \"_helpers.st\";\n");

    let defs = fx
        .resolve(&index)
        .expect("a sibling import must resolve — it sits next to the importing file");
    assert!(
        defs > 0,
        "the imported file's declarations must come across, got {defs}"
    );
}

/// A subdirectory beside the importing file, the other common layout
/// (`projects/hermes-agent/patterns/_reveals.st`).
#[test]
fn a_bare_subdirectory_import_resolves_beside_the_importing_file() {
    let fx = Fixture::new("subdir");
    fx.write("patterns/_reveals.st", "@form easing --y { linear }\n");
    let index = fx.write("index.st", "@import \"patterns/_reveals.st\";\n");

    fx.resolve(&index)
        .expect("a subdirectory import must resolve beside the importing file");
}

/// An import from a file in a SUBDIRECTORY resolves against ITS directory —
/// "beside the importing file" must mean the file doing the importing, at
/// whatever depth it sits, not the project's top level.
#[test]
fn the_base_is_the_importing_files_own_directory() {
    let fx = Fixture::new("depth");
    fx.write("pages/_local.st", "@form easing --nearest { linear }\n");
    fx.write("_local.st", "@form easing --toplevel { linear }\n");
    let deep_page = fx.write("pages/about.st", "@import \"_local.st\";\n");

    fx.resolve(&deep_page)
        .expect("must resolve against the importing file's own directory");
}

/// The fix must not turn every typo into a silent pass.
#[test]
fn a_missing_import_still_fails_loudly() {
    let fx = Fixture::new("missing");
    let index = fx.write("index.st", "@import \"_nope.st\";\n");

    let err = fx
        .resolve(&index)
        .expect_err("an import with no file anywhere must still fail");
    assert!(
        err.contains("_nope"),
        "the failure must name the import that could not be found, got: {err}"
    );
}

/// A project-local `stdlib/` directory must not capture a stdlib import: the
/// new step is ADDITIVE and sits after the stdlib branch, not before it.
#[test]
fn a_sibling_directory_does_not_shadow_the_stdlib() {
    let fx = Fixture::new("shadow");
    // A decoy that would break a page if it were preferred over the real module.
    fx.write("stdlib/md/index.st", "// decoy — not the real md module\n");
    let index = fx.write("index.st", "@import \"stdlib/md\";\n");

    fx.resolve(&index)
        .expect("a stdlib import must keep resolving to the stdlib");
}
