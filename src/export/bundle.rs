//! Per-route bundle compilation.
//!
//! Stubbed in Wave 0; populated TDD-style in Wave 1 by the BUNDLE task.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct CompiledBundle {
    /// Directory relative to site_dir (empty PathBuf for root).
    pub(crate) dir: PathBuf,
    pub(crate) js: String,
    pub(crate) css: String,
    /// Compiler-produced body markup (file-scope `<tag>` literals + FEAT-078 hydration markers,
    /// PLAN-023 W1). Empty unless the `.st` has file-scope HTML. Used to synthesize a page for a
    /// full-Spacetime (`.st`-only) route that has no authored `.html` (FUP-039).
    pub(crate) html: String,
    /// Non-fatal compile diagnostics surfaced as warnings.
    pub(crate) diagnostics: Vec<String>,
}

/// Compile every discovered page entry (`route dir -> entry .st`, see
/// `discover::SiteLayout::st_entries`). Per-route errors degrade to
/// diagnostics; the function never returns an error for compile failures
/// (the caller surfaces them as warnings).
#[allow(dead_code)]
pub(crate) fn compile_bundles(
    site_dir: &Path,
    entries: &std::collections::BTreeMap<PathBuf, PathBuf>,
) -> Vec<CompiledBundle> {
    // BUG-227: `from_file`'s second argument is the WORKSPACE ROOT (what
    // `resolve_imports` joins `stdlib/` onto), NOT the site. Passing `site_dir`
    // here made every `@import "stdlib/..."` look inside the user's project,
    // fail, and degrade to an EMPTY bundle behind a mere warning -- which is
    // how a push shipped a deployment whose /spacetime.js was 0 bytes. It only
    // ever appeared to work because a CWD-relative `./stdlib` also resolves
    // when the build runs from the toolchain checkout, which `build` and
    // `push` (run from the user's project) never do.
    let workspace_root =
        crate::toolchain::workspace_root_for(site_dir).unwrap_or_else(|| site_dir.to_path_buf());

    let mut bundles = Vec::new();
    for (dir, entry_rel) in entries {
        let st_path = site_dir.join(entry_rel);
        match crate::compiler::Compiler::from_file(&st_path, &workspace_root) {
            Ok(compiler) => {
                // `with_site_dir` is what lets build-time file reads resolve
                // against the SERVED directory: `@doc(src:)` inlines a markdown
                // file's bytes, `@data`/static-each unrolls a JSON collection.
                // The dev server sets it (`server.rs`'s compile path); without
                // it here the export silently produced "Document not found"
                // pages while `spacetime serve` rendered the real doc --
                // the same dev/deploy divergence BUG-218 is about.
                let compiled = compiler
                    .with_site_dir(Some(site_dir.to_path_buf()))
                    .compile();
                bundles.push(CompiledBundle {
                    dir: dir.clone(),
                    js: compiled.js,
                    css: compiled.css,
                    html: compiled.html,
                    diagnostics: compiled
                        .pipeline_errors
                        .iter()
                        .map(|e| e.message.clone())
                        .collect(),
                });
            }
            Err(e) => {
                bundles.push(CompiledBundle {
                    dir: dir.clone(),
                    js: String::new(),
                    css: String::new(),
                    html: String::new(),
                    diagnostics: vec![format!("Compile failed: {e}")],
                });
            }
        }
    }
    bundles
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn minimal_st_content() -> &'static str {
        ".test-page {\n    display: block;\n}\n\nbody {\n    @on &.click { $opacity <- 1; }\n}\n"
    }

    #[test]
    fn empty_bundle_dirs_returns_empty_vec() {
        let tmp = tempfile::tempdir().unwrap();
        let site_dir = tmp.path();
        let entries: BTreeMap<PathBuf, PathBuf> = BTreeMap::new();
        let result = compile_bundles(site_dir, &entries);
        assert!(result.is_empty());
    }

    #[test]
    fn compiles_single_root_bundle() {
        let tmp = tempfile::tempdir().unwrap();
        let site_dir = tmp.path();
        std::fs::write(site_dir.join("index.st"), minimal_st_content()).unwrap();

        let mut entries = BTreeMap::new();
        entries.insert(PathBuf::new(), PathBuf::from("index.st"));

        let result = compile_bundles(site_dir, &entries);
        assert_eq!(result.len(), 1);
        let bundle = &result[0];
        assert_eq!(bundle.dir, PathBuf::new());
        assert!(!bundle.js.is_empty(), "js should not be empty");
        assert!(!bundle.css.is_empty(), "css should not be empty");
        assert!(
            bundle.diagnostics.is_empty(),
            "diagnostics should be empty for valid st"
        );
    }

    #[test]
    fn compiles_per_route_bundle() {
        let tmp = tempfile::tempdir().unwrap();
        let site_dir = tmp.path();
        std::fs::write(site_dir.join("index.st"), minimal_st_content()).unwrap();
        std::fs::create_dir(site_dir.join("about")).unwrap();
        std::fs::write(
            site_dir.join("about").join("index.st"),
            minimal_st_content(),
        )
        .unwrap();

        let mut entries = BTreeMap::new();
        entries.insert(PathBuf::new(), PathBuf::from("index.st"));
        entries.insert(PathBuf::from("about"), PathBuf::from("about/index.st"));

        let result = compile_bundles(site_dir, &entries);
        assert_eq!(result.len(), 2);
        // BTreeMap iteration order: root first, then "about"
        assert_eq!(result[0].dir, PathBuf::new());
        assert!(!result[0].js.is_empty());
        assert!(!result[0].css.is_empty());
        assert_eq!(result[1].dir, PathBuf::from("about"));
        assert!(!result[1].js.is_empty());
        assert!(!result[1].css.is_empty());
    }

    #[test]
    fn compile_failure_becomes_diagnostic_not_panic() {
        let tmp = tempfile::tempdir().unwrap();
        let site_dir = tmp.path();
        // Create root index.st so the site is valid, but add a bundle dir
        // whose entry .st does NOT exist.
        std::fs::write(site_dir.join("index.st"), minimal_st_content()).unwrap();

        let mut entries = BTreeMap::new();
        entries.insert(PathBuf::new(), PathBuf::from("index.st"));
        entries.insert(
            PathBuf::from("missing-route"),
            PathBuf::from("missing-route/index.st"),
        );

        let result = compile_bundles(site_dir, &entries);
        assert_eq!(result.len(), 2);
        // Root compiled OK
        let root = result.iter().find(|b| b.dir == PathBuf::new()).unwrap();
        assert!(root.diagnostics.is_empty());
        // Missing route has diagnostics
        let missing = result
            .iter()
            .find(|b| b.dir == PathBuf::from("missing-route"))
            .unwrap();
        assert!(
            !missing.diagnostics.is_empty(),
            "missing route should have compile diagnostics"
        );
        assert!(missing.js.is_empty());
        assert!(missing.css.is_empty());
    }
}
