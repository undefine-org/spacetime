//! Vendored-dependency bundling — lib wrapper over the std-only core.
//!
//! The bundle mechanism lives ONCE in `src/vendor_core.rs` (std-only), shared by
//! `build.rs` (the `cargo build` embed trigger) and this wrapper (the
//! `vendor build` CLI / hot-reload trigger). This file adds the crate-aware
//! surface: mapping a parsed [`VendorDefAst`] into a [`vendor_core::BundleSpec`]
//! and computing the artifact's blake3 content hash. See `docs/stdlib/VENDORING.md`.

use crate::metasystem::MetaRegistry;
use crate::parser::meta_ast::{VendorBundleStrategy, VendorDefAst};
use std::path::{Path, PathBuf};

#[path = "vendor_core.rs"]
mod core_mod;
pub use core_mod::vendor_core::{BundleError as VendorBuildError, Strategy};
use core_mod::vendor_core::{BundleSpec, bundle};

/// Outcome of a vendor bundle attempt (core outcome + content hash).
#[derive(Debug, Clone)]
pub struct VendorBuildResult {
    /// Absolute-ish path to the generated artifact.
    pub out_path: PathBuf,
    /// Byte length of the artifact.
    pub bytes: usize,
    /// blake3 content hash of the artifact (hex).
    pub content_hash: String,
    /// Submodule HEAD commit the artifact was built from, if resolvable.
    pub submodule_commit: Option<String>,
    /// True when the lazy escape fired and no rebundle happened.
    pub skipped: bool,
}

/// Build (or lazily skip) the artifact for one `%vendor` declaration.
///
/// `module_dir` is the directory the `%vendor` paths are relative to (e.g.
/// `stdlib/text`). `force` bypasses the lazy escape.
pub fn build_vendor(
    module_dir: &Path,
    def: &VendorDefAst,
    force: bool,
) -> Result<VendorBuildResult, VendorBuildError> {
    let strategy = match def.bundle {
        VendorBundleStrategy::Direct => Strategy::Direct,
        VendorBundleStrategy::Bun => Strategy::Bun,
    };
    let spec = BundleSpec {
        name: def.name.clone(),
        module_dir: module_dir.to_path_buf(),
        source: def.source.clone(),
        entry: def.entry.clone(),
        out: def.out.clone(),
        strategy,
    };
    let outcome = bundle(&spec, force)?;
    let data = std::fs::read(&outcome.out_path).map_err(|e| VendorBuildError::Io(e.to_string()))?;
    Ok(VendorBuildResult {
        content_hash: blake3::hash(&data).to_hex().to_string(),
        bytes: outcome.bytes,
        out_path: outcome.out_path,
        submodule_commit: outcome.submodule_commit,
        skipped: outcome.skipped,
    })
}

/// Hot-reload signature for vendored blobs (PLAN-024 W2).
///
/// Returns the most recent mtime across all `stdlib/**/vendor/*.bundle.js`
/// artifacts, or `None` when none exist (embedded mode / pre-build). Folding
/// this into the dev compile cache key makes `vendor build` a hot-reload
/// concern: rewriting a blob bumps the signature, busting the cache so the next
/// request recompiles with the fresh blob — no binary recompile, no site edit.
pub fn vendor_blobs_signature() -> Option<std::time::SystemTime> {
    let mut latest: Option<std::time::SystemTime> = None;
    collect_bundle_mtimes(Path::new("stdlib"), &mut latest, 0);
    latest
}

fn collect_bundle_mtimes(dir: &Path, latest: &mut Option<std::time::SystemTime>, depth: usize) {
    // Bound recursion; stdlib module trees are shallow.
    if depth > 8 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_bundle_mtimes(&path, latest, depth + 1);
        } else if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with(".bundle.js"))
            && let Ok(mtime) = std::fs::metadata(&path).and_then(|m| m.modified())
        {
            *latest = Some(match *latest {
                Some(cur) if cur >= mtime => cur,
                _ => mtime,
            });
        }
    }
}

/// Demand-driven vendored-blob injection (the "lean rail", PLAN-024 W2).
///
/// Seed a vendor registry for the TEST paths (headless V8 + CDP/HTML test
/// runners) with every `%vendor` declared under the stdlib tree. Test files
/// resolve macros against the cached CORE registry (STDLIB_DIRS), whose
/// import statements (e.g. timeline.st's `@import "stdlib/text"`) are never
/// traversed for vendor declarations — so a test using a vendored primitive
/// (@value-change -> pretext) without importing its module gets the driver
/// code but NOT the vendored blob it references (runtime "pretext is not
/// defined", the value-change headless failures triaged 2026-07-19). The
/// per-file reference check in inject_vendor_preludes keeps unused-vendor
/// pages lean; embedded-stdlib mode skips gracefully (no filesystem tree).
pub fn stdlib_test_vendor_registry() -> crate::metasystem::MetaRegistry {
    let mut registry = crate::metasystem::MetaRegistry::new();
    if std::path::Path::new("stdlib").exists() {
        let seed = registry.load_stdlib_collecting_errors(std::path::Path::new("stdlib"));
        if !seed.errors.is_empty() {
            log::debug!(
                "test vendor seed: {} stdlib file(s) skipped (best-effort): {:?}",
                seed.errors.len(),
                seed.errors.iter().map(|e| &e.file_path).collect::<Vec<_>>()
            );
        }
    }
    registry
}

/// Demand-driven vendored-blob injection (the "lean rail", PLAN-024 W2).
///
/// For each registered `%vendor`, prepend its bundled IIFE to `site_js` IFF the
/// vendored global name is actually referenced in the generated site code. A page
/// that imports a module but never invokes a vendored primitive ships ZERO of
/// that vendor's bytes.
///
/// The blob is read from disk at `parent(source_file) / out`. In embedded-stdlib
/// mode (no filesystem source), the blob is absent and the vendor is skipped
/// gracefully (embedding the artifact is a later wave). Returns `site_js`
/// unchanged when nothing is injected.
///
/// Order: the returned string is `vendor_iife(s) ++ site_js`. Callers wrap the
/// result with the core runtime AFTER, yielding `runtime -> vendor -> site`.
pub fn inject_vendor_preludes(site_js: &str, registry: &MetaRegistry) -> String {
    if site_js.is_empty() {
        return site_js.to_string();
    }

    let mut preludes: Vec<String> = Vec::new();
    for def in registry.vendors() {
        if !vendor_is_referenced(site_js, &def.name) {
            continue;
        }
        let Some(blob_path) = vendor_blob_path(def) else {
            continue;
        };
        let Ok(blob) = std::fs::read_to_string(&blob_path) else {
            // Blob not on disk (embedded mode / not yet built) — skip gracefully.
            continue;
        };
        preludes.push(format!(
            "// vendored: {} ({})\n{}",
            def.name,
            def.license.as_deref().unwrap_or("unspecified license"),
            blob.trim_end()
        ));
    }

    if preludes.is_empty() {
        return site_js.to_string();
    }
    format!("{}\n\n{}", preludes.join("\n\n"), site_js)
}

/// The bundled IIFE source of the vendor named `name`, when it is registered
/// and its blob is on disk. The BUILD-TIME counterpart of
/// [`inject_vendor_preludes`]: a compile pass that wants to run the very
/// engine the browser runs (SSG markdown, `ssg_unroll::unroll_static_docs`)
/// reads it from here rather than carrying a second copy.
pub fn vendor_blob_source(registry: &MetaRegistry, name: &str) -> Option<String> {
    let def = registry.vendors().find(|d| d.name == name)?;
    std::fs::read_to_string(vendor_blob_path(def)?).ok()
}

/// Resolve the on-disk path of a vendor's bundled artifact: the `out` path is
/// relative to the module dir, which is the directory containing the declaring
/// `.st` file.
fn vendor_blob_path(def: &VendorDefAst) -> Option<PathBuf> {
    let source_file = def.source_file.as_deref()?;
    let module_dir = Path::new(source_file).parent()?;
    Some(module_dir.join(&def.out))
}

/// Whether the vendored global `name` is referenced in `js` as a real token
/// (word-boundary match), not a coincidental substring.
fn vendor_is_referenced(js: &str, name: &str) -> bool {
    let bytes = js.as_bytes();
    let nb = name.as_bytes();
    if nb.is_empty() {
        return false;
    }
    let is_word = |c: u8| c == b'_' || c == b'$' || c.is_ascii_alphanumeric();
    let mut i = 0usize;
    while let Some(off) = js[i..].find(name) {
        let start = i + off;
        let end = start + nb.len();
        let before_ok = start == 0 || !is_word(bytes[start - 1]);
        let after_ok = end >= bytes.len() || !is_word(bytes[end]);
        if before_ok && after_ok {
            return true;
        }
        i = start + 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::meta_ast::VendorDefAst;

    fn def(strategy: VendorBundleStrategy, source: &str, entry: &str, out: &str) -> VendorDefAst {
        VendorDefAst {
            name: "fixture".to_string(),
            source: source.to_string(),
            entry: entry.to_string(),
            bundle: strategy,
            exports: vec!["foo".to_string()],
            out: out.to_string(),
            license: Some("MIT".to_string()),
            span: Default::default(),
            source_file: None,
        }
    }

    fn unique_tmp(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "vendor_{}_{}_{}",
            tag,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn direct_strategy_copies_entry_to_out() {
        let tmp = unique_tmp("copy");
        let sub = tmp.join("vendor/dep");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("index.js"), "globalThis.fixture = { foo: 1 };").unwrap();

        let d = def(
            VendorBundleStrategy::Direct,
            "vendor/dep",
            "vendor/dep/index.js",
            "vendor/dep.bundle.js",
        );
        let res = build_vendor(&tmp, &d, true).unwrap();
        assert!(res.out_path.exists());
        assert!(!res.skipped);
        assert_eq!(res.content_hash.len(), 64); // blake3 hex
        let content = std::fs::read_to_string(&res.out_path).unwrap();
        assert!(content.contains("globalThis.fixture"));

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn direct_lazy_skips_when_artifact_present_and_no_commit() {
        let tmp = unique_tmp("lazy");
        let sub = tmp.join("vendor/dep");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("index.js"), "A").unwrap();
        let d = def(
            VendorBundleStrategy::Direct,
            "vendor/dep",
            "vendor/dep/index.js",
            "vendor/dep.bundle.js",
        );
        build_vendor(&tmp, &d, true).unwrap();
        // Mutate source; a LAZY call must NOT pick it up (no git commit to
        // compare, artifact already present).
        std::fs::write(sub.join("index.js"), "B").unwrap();
        let res = build_vendor(&tmp, &d, false).unwrap();
        assert!(res.skipped, "expected lazy skip");
        let content = std::fs::read_to_string(&res.out_path).unwrap();
        assert_eq!(content, "A", "stale artifact preserved by lazy escape");

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn direct_force_rebuilds_over_lazy() {
        let tmp = unique_tmp("force");
        let sub = tmp.join("vendor/dep");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("index.js"), "A").unwrap();
        let d = def(
            VendorBundleStrategy::Direct,
            "vendor/dep",
            "vendor/dep/index.js",
            "vendor/dep.bundle.js",
        );
        build_vendor(&tmp, &d, true).unwrap();
        std::fs::write(sub.join("index.js"), "B").unwrap();
        let res = build_vendor(&tmp, &d, true).unwrap();
        assert!(!res.skipped);
        let content = std::fs::read_to_string(&res.out_path).unwrap();
        assert_eq!(content, "B", "force rebuild picks up new source");

        std::fs::remove_dir_all(&tmp).ok();
    }

    fn registry_with_vendor(blob_dir: &Path, blob_name: &str, global: &str) -> MetaRegistry {
        use crate::metasystem::MetaRegistry;
        std::fs::create_dir_all(blob_dir).unwrap();
        std::fs::write(
            blob_dir.join(blob_name),
            format!("globalThis.{} = {{ measure: function(){{}} }};", global),
        )
        .unwrap();
        let mut reg = MetaRegistry::new();
        let def = VendorDefAst {
            name: global.to_string(),
            source: "vendor/dep".to_string(),
            entry: "x".to_string(),
            bundle: VendorBundleStrategy::Direct,
            exports: vec!["measure".to_string()],
            out: blob_name.to_string(),
            license: Some("MIT".to_string()),
            span: Default::default(),
            // source_file lives in the module dir; vendor_blob_path uses its parent.
            source_file: Some(blob_dir.join("MODULE.st").to_string_lossy().to_string()),
        };
        reg.register_vendor(def).unwrap();
        reg
    }

    #[test]
    fn inject_includes_blob_when_global_referenced() {
        let tmp = unique_tmp("inj_yes");
        let reg = registry_with_vendor(&tmp, "pretext.bundle.js", "pretext");
        let site = "const r = pretext.measure(el);";
        let out = inject_vendor_preludes(site, &reg);
        assert!(
            out.contains("globalThis.pretext"),
            "blob should be injected"
        );
        assert!(
            out.contains("vendored: pretext (MIT)"),
            "provenance header present"
        );
        // Order: vendor IIFE precedes site code.
        assert!(out.find("globalThis.pretext").unwrap() < out.find("const r =").unwrap());
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn inject_omits_blob_when_global_absent() {
        let tmp = unique_tmp("inj_no");
        let reg = registry_with_vendor(&tmp, "pretext.bundle.js", "pretext");
        let site = "const x = ST.signal(1); document.body;";
        let out = inject_vendor_preludes(site, &reg);
        assert_eq!(out, site, "unused vendor must ship zero bytes");
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn inject_word_boundary_avoids_substring_false_positive() {
        let tmp = unique_tmp("inj_wb");
        let reg = registry_with_vendor(&tmp, "pretext.bundle.js", "pretext");
        // `pretextual` contains `pretext` as a substring but is a different token.
        let site = "const pretextual = 1; foo.pretextField;";
        let out = inject_vendor_preludes(site, &reg);
        assert_eq!(out, site, "substring match must not trigger injection");
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn inject_skips_when_blob_missing_on_disk() {
        // Embedded mode: registry has the vendor but no blob file exists.
        let mut reg = MetaRegistry::new();
        reg.register_vendor(VendorDefAst {
            name: "pretext".to_string(),
            source: "vendor/dep".to_string(),
            entry: "x".to_string(),
            bundle: VendorBundleStrategy::Direct,
            exports: vec![],
            out: "pretext.bundle.js".to_string(),
            license: None,
            span: Default::default(),
            source_file: Some("/nonexistent/dir/MODULE.st".to_string()),
        })
        .unwrap();
        let site = "pretext.measure(el);";
        let out = inject_vendor_preludes(site, &reg);
        assert_eq!(out, site, "missing blob skips gracefully");
    }

    #[test]
    fn missing_submodule_without_artifact_errors() {
        let tmp = unique_tmp("miss");
        std::fs::create_dir_all(&tmp).unwrap();
        let d = def(
            VendorBundleStrategy::Direct,
            "vendor/absent",
            "index.js",
            "vendor/absent.bundle.js",
        );
        let err = build_vendor(&tmp, &d, true).unwrap_err();
        assert!(matches!(err, VendorBuildError::SubmoduleMissing(_)));

        std::fs::remove_dir_all(&tmp).ok();
    }
}
