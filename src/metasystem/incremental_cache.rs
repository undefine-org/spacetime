//! Incremental stdlib cache with mtime-based invalidation.
//!
//! Replaces the static `LazyLock<(MetaRegistry, Vec<StdlibLoadError>)>` with
//! an mtime-aware cache that re-parses only changed stdlib files on each access.
//! This gives both fast reuse AND stdlib hot-reload during development.
//!
//! Additionally, a binary cache (via bincode) is written to disk so that
//! subsequent startups can deserialize the registry instead of re-parsing
//! 161+ `.st` files.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::metasystem::registry::MetaRegistry;
use crate::pipeline::StdlibLoadError;

/// Version stamp for binary cache compatibility
const CACHE_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Canary string — change this if MetaRegistry struct layout changes
// v2: pre-v2 caches could have been written by an ERRORING load (the write
// was unconditional before the clean-load gate) — they share the v1 identity
// and would be accepted on hit, load errors dropped. Invalidate them.
const CACHE_STRUCT_HASH: &str = "MetaRegistry_v2_2026";
/// Mtime-based incremental cache for the stdlib registry.
///
/// On each `refresh_if_needed()` call:
/// - If using embedded stdlib (`file_mtimes` is `None`): no-op
/// - Otherwise: scan stdlib dirs for changed/new/deleted `.st` files,
///   re-parse only those, and update the registry in-place
pub struct IncrementalStdlibCache {
    registry: MetaRegistry,
    errors: Vec<StdlibLoadError>,
    /// Per-file mtime tracking. `None` = using embedded stdlib (no incremental).
    file_mtimes: Option<HashMap<PathBuf, SystemTime>>,
    /// The stdlib dirs that were loaded from filesystem
    stdlib_dirs: Vec<PathBuf>,
    /// Path to the binary cache file on disk
    cache_path: Option<PathBuf>,
}

impl IncrementalStdlibCache {
    /// Create a cache from the full stdlib load (used by the global static).
    ///
    /// Calls `load_stdlib_registry()` logic internally, then populates
    /// `file_mtimes` by scanning the loaded files.
    pub fn from_full_load() -> Self {
        // BUG-350: anchor the stdlib dirs to the TOOLCHAIN ROOT, not the process's
        // working directory. `STDLIB_DIRS` are relative (`stdlib/primitives`, …), so a
        // bare `PathBuf::from` resolves them against wherever the shell happened to be.
        // Run from the repo root they exist and the on-disk stdlib loads; run from any
        // subdirectory (`src/`, `tests/`, a fixture folder) NONE of them exist, the
        // `any_dir_exists` probe below says "no stdlib on disk", and the load silently
        // falls back to the EMBEDDED registry — grammar frozen at the last `cargo build`.
        // Same site, same binary, different cwd, different stdlib: a page that builds
        // from the root failed from a subdirectory with a confusing diagnostic about an
        // unrelated internal primitive (`Missing required parameter 'match' in bind:
        // derive-match`).
        //
        // Mirrors what BUG-227 fixed for the SyntaxRegistry — the same class of bug, one
        // layer down. The embedded fallback stays correct for a distributed binary that
        // genuinely has no stdlib on disk; it just must not be reached because of where
        // the shell was.
        // The binary's OWN checkout is the last resort: when the process runs from
        // somewhere with no enclosing toolchain (`cd /tmp && spacetime build <site>`),
        // the cwd walk finds nothing, and bare relative dirs would resolve against /tmp
        // and miss too. Falling back to the checkout that contains this executable keeps
        // a dev binary pointed at its own stdlib from ANY directory; a genuinely
        // distributed binary has no such checkout and still lands on the embedded copy.
        // The exe fallback now lives INSIDE `workspace_root_for` (BUG-326), so
        // every layer that asks "where is the stdlib" gets one answer. It was
        // bolted on here and nowhere else, which is exactly how this loader and
        // `stdlib_registry` came to disagree.
        let root = crate::toolchain::anchored_workspace_root();
        let dirs: Vec<PathBuf> = super::registry::STDLIB_DIRS
            .iter()
            .map(|dir| match &root {
                Some(root) => root.join(dir),
                None => PathBuf::from(dir),
            })
            .collect();

        let any_dir_exists = dirs.iter().any(|dir| dir.exists());

        if any_dir_exists {
            Self::from_dirs(dirs)
        } else {
            // Embedded stdlib — no incremental possible
            let mut registry = MetaRegistry::new();
            let mut errors = Vec::new();
            crate::compiler::load_embedded_stdlib(&mut registry, &mut errors);
            registry.rebuild_provider_index();
            // PLAN-076: chain checks on the embedded path too.
            errors.extend(crate::compiler::migration_chain_load_errors(&registry));
            Self {
                registry,
                errors,
                file_mtimes: None,
                stdlib_dirs: Vec::new(),
                cache_path: None,
            }
        }
    }

    /// Create a cache by loading `.st` files from the given directories.
    ///
    /// Used both by `from_full_load()` and by tests that use temp dirs.
    pub fn from_dirs(dirs: Vec<PathBuf>) -> Self {
        // 1. Determine cache path
        let cache_path = binary_cache_path(&dirs);

        // 2. Collect current file mtimes
        let mut file_mtimes = HashMap::new();
        for dir in &dirs {
            if dir.exists() {
                collect_file_mtimes(dir, &mut file_mtimes);
            }
        }

        // 3. Try loading from binary cache (FAST PATH)
        if let Some(ref cp) = cache_path
            && let Some(cached) = load_binary_cache(cp)
        {
            if cached.file_mtimes == file_mtimes {
                log::debug!(
                    "[PERF] Stdlib binary cache HIT ({} files)",
                    file_mtimes.len()
                );
                return Self {
                    registry: cached.registry,
                    errors: Vec::new(),
                    file_mtimes: Some(file_mtimes),
                    stdlib_dirs: dirs,
                    cache_path: Some(cp.clone()),
                };
            } else {
                log::debug!("[PERF] Stdlib binary cache stale, re-parsing");
            }
        }

        // 4. Full parse (SLOW PATH)
        let mut registry = MetaRegistry::new();
        let mut all_errors = Vec::new();

        for dir in &dirs {
            if !dir.exists() {
                continue;
            }
            let result = registry.load_stdlib_collecting_errors(dir.as_path());
            all_errors.extend(result.errors);
        }

        registry.rebuild_provider_index();

        // PLAN-076: post-load migration chain validation (same rail as
        // compiler::load_stdlib_registry — must not diverge).
        all_errors.extend(crate::compiler::migration_chain_load_errors(&registry));

        // I4 / gh-18: the capture-consumption lint (E0964) runs here too —
        // a lint the fast/cached path skips is a lint that dies the day
        // builds get fast. The binary-cache gate below (all_errors.is_empty())
        // therefore also refuses to persist a registry carrying an unconsumed
        // capture, so a cache HIT (fast path, errors cleared) can only ever
        // observe a registry the lint already passed.
        all_errors.extend(crate::compiler::capture_consumption_load_errors(&registry));

        // 5. Write binary cache for next startup — ONLY when the load was
        // clean. A registry that loaded with errors (e.g. a stdlib file saved
        // mid-edit) must NEVER be persisted: cache hits drop load errors
        // (`errors: Vec::new()`), so a poisoned write would silently stick
        // until the next stdlib mtime change and surface as far-removed
        // expansion failures (E0804 at stdlib %binds spans) instead of the
        // real load error.
        if let Some(ref cp) = cache_path {
            if all_errors.is_empty() {
                let bin_cache = BinaryStdlibCache {
                    version: CACHE_VERSION.to_string(),
                    struct_canary: CACHE_STRUCT_HASH.to_string(),
                    file_mtimes: file_mtimes.clone(),
                    registry: registry.clone(),
                };
                write_binary_cache(cp, &bin_cache);
            } else {
                log::debug!(
                    "[CACHE] skipping binary cache write: {} stdlib load error(s)",
                    all_errors.len()
                );
            }
        }

        Self {
            registry,
            errors: all_errors,
            file_mtimes: Some(file_mtimes),
            stdlib_dirs: dirs,
            cache_path,
        }
    }

    /// Check for stdlib changes and re-parse only modified/new/deleted files.
    ///
    /// Performance characteristics:
    /// - No changes (common case): ~1ms for mtime scan of 151 files
    /// - One file changed: ~2ms (scan + re-parse + rebuild provider index)
    /// - Embedded stdlib: zero overhead (returns immediately)
    pub fn refresh_if_needed(&mut self) {
        let file_mtimes = match &mut self.file_mtimes {
            Some(mtimes) => mtimes,
            None => return, // Embedded stdlib — nothing to check
        };

        let mut current_files: HashMap<PathBuf, SystemTime> = HashMap::new();
        for dir in &self.stdlib_dirs {
            if dir.exists() {
                collect_file_mtimes(dir, &mut current_files);
            }
        }

        let mut changed = false;

        // Detect modified files (mtime differs)
        let mut modified_files = Vec::new();
        for (path, new_mtime) in &current_files {
            match file_mtimes.get(path) {
                Some(old_mtime) if old_mtime == new_mtime => {
                    // Unchanged — skip
                }
                Some(_) => {
                    // Modified
                    modified_files.push(path.clone());
                    changed = true;
                }
                None => {
                    // New file
                    modified_files.push(path.clone());
                    changed = true;
                }
            }
        }

        // Detect deleted files
        let mut deleted_files = Vec::new();
        for path in file_mtimes.keys() {
            if !current_files.contains_key(path) {
                deleted_files.push(path.clone());
                changed = true;
            }
        }

        if !changed {
            return;
        }

        // Process deleted files
        for path in &deleted_files {
            let source_file = path.to_string_lossy().to_string();
            log::debug!("Stdlib file deleted, unregistering: {}", source_file);
            self.registry.unregister_file(&source_file);
            file_mtimes.remove(path);
        }

        // Remove stale errors for modified/deleted files
        let affected_paths: Vec<PathBuf> = modified_files
            .iter()
            .chain(deleted_files.iter())
            .cloned()
            .collect();
        self.errors
            .retain(|e| !affected_paths.contains(&e.file_path));

        // Process modified/new files
        for path in &modified_files {
            let source_file = path.to_string_lossy().to_string();
            log::debug!("Stdlib file changed, reloading: {}", source_file);

            // Unregister old defs from this file (no-op for new files)
            self.registry.unregister_file(&source_file);

            // Re-parse and register
            match self.registry.load_file_for_stdlib(path) {
                Ok(()) => {}
                Err(e) => {
                    self.errors.push(e);
                }
            }

            // Update mtime
            if let Some(mtime) = current_files.get(path) {
                file_mtimes.insert(path.clone(), *mtime);
            }
        }

        // Rebuild provider index after any changes
        self.registry.rebuild_provider_index();

        // PLAN-076: re-run the order-independent migration chain checks after
        // every incremental change (the per-file validate_migration at
        // registration cannot see cross-migration facts).
        let chain_errors = crate::compiler::migration_chain_load_errors(&self.registry);
        // Chain errors REPLACE any prior chain errors (they are global, not
        // per-file): drop old ones by their shared sentinel path first.
        self.errors
            .retain(|e| e.file_path != std::path::Path::new("stdlib/migrations/entries"));
        self.errors.extend(chain_errors);

        // I4 / gh-18: the capture-consumption lint (E0964) is recomputed after
        // every incremental change (a modified stdlib macro may drop a capture
        // it used to forward, or vice versa). It is a per-file, deterministic
        // scan over the WHOLE registry, so drop the prior lint errors (keyed by
        // their stable message marker) and re-extend — a fast/cached path that
        // skipped this would let a half-implemented macro ship the moment an
        // incremental reload happens.
        self.errors.retain(|e| !e.message.contains("never consumes"));
        self.errors
            .extend(crate::compiler::capture_consumption_load_errors(&self.registry));

        // Re-serialize to binary cache after changes — same clean-load gate
        // as the from_dirs write: never persist a registry carrying errors.
        if let Some(ref cp) = self.cache_path {
            if self.errors.is_empty() {
                let bin_cache = BinaryStdlibCache {
                    version: CACHE_VERSION.to_string(),
                    struct_canary: CACHE_STRUCT_HASH.to_string(),
                    file_mtimes: file_mtimes.clone(),
                    registry: self.registry.clone(),
                };
                write_binary_cache(cp, &bin_cache);
            } else {
                log::debug!(
                    "[CACHE] skipping binary cache write after refresh: {} load error(s)",
                    self.errors.len()
                );
            }
        }
    }

    /// Return a clone of the current registry and errors.
    pub fn get(&self) -> (MetaRegistry, Vec<StdlibLoadError>) {
        (self.registry.clone(), self.errors.clone())
    }
}

/// Serializable snapshot of the stdlib registry for binary caching.
#[derive(Serialize, Deserialize)]
struct BinaryStdlibCache {
    version: String,
    struct_canary: String,
    file_mtimes: HashMap<PathBuf, SystemTime>,
    registry: MetaRegistry,
}

/// Determine the binary cache file path from the stdlib directories.
///
/// Uses the parent of the first existing stdlib dir: `<parent>/.cache/registry.bin`
fn binary_cache_path(dirs: &[PathBuf]) -> Option<PathBuf> {
    dirs.iter()
        .find(|d| d.exists())
        .and_then(|d| d.parent())
        .map(|parent| parent.join(".cache").join("registry.bin"))
}

/// Atomically write a binary cache to disk (write to temp, then rename).
fn write_binary_cache(path: &Path, cache: &BinaryStdlibCache) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let temp_path = path.with_extension("bin.tmp");
    match std::fs::File::create(&temp_path) {
        Ok(file) => {
            if bincode::serialize_into(&file, cache).is_ok() {
                let _ = std::fs::rename(&temp_path, path);
            } else {
                let _ = std::fs::remove_file(&temp_path);
            }
        }
        Err(_) => { /* silently fail — cache is optional */ }
    }
}

/// Try to load and validate a binary cache from disk.
fn load_binary_cache(path: &Path) -> Option<BinaryStdlibCache> {
    let metadata = std::fs::metadata(path)
        .map_err(|e| log::debug!("[CACHE] stat failed: {e}"))
        .ok()?;
    // Reject files that are suspiciously large (>100MB) or tiny (<32 bytes)
    let size = metadata.len();
    if !(32..=100 * 1024 * 1024).contains(&size) {
        log::debug!("[CACHE] file size {size} out of range, skipping");
        return None;
    }
    // Read entire file into memory first, then deserialize.
    // This prevents bincode from trying unbounded allocations on corrupt data.
    let bytes = std::fs::read(path)
        .map_err(|e| log::debug!("[CACHE] read failed: {e}"))
        .ok()?;
    let cache: BinaryStdlibCache = bincode::deserialize(&bytes)
        .map_err(|e| log::debug!("[CACHE] deserialize failed: {e}"))
        .ok()?;
    if cache.version != CACHE_VERSION || cache.struct_canary != CACHE_STRUCT_HASH {
        log::debug!("[CACHE] version/canary mismatch");
        return None;
    }
    Some(cache)
}

/// Recursively collect `.st` file paths and their mtimes from a directory,
/// applying the same filtering rules as `load_dir_recursive_collecting`
/// (skip `examples/` dirs and `.test.st` files).
fn collect_file_mtimes(dir: &Path, mtimes: &mut HashMap<PathBuf, SystemTime>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Skip examples directories
            if path.file_name().is_some_and(|name| name == "examples") {
                continue;
            }
            collect_file_mtimes(&path, mtimes);
        } else if path.extension().is_some_and(|ext| ext == "st") {
            // Skip test files
            let file_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if file_name.ends_with(".test") {
                continue;
            }
            if let Ok(metadata) = std::fs::metadata(&path)
                && let Ok(mtime) = metadata.modified()
            {
                mtimes.insert(path, mtime);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper: create a temp dir with a .st file containing a primitive definition
    fn create_stdlib_file(dir: &Path, filename: &str, primitive_name: &str) -> PathBuf {
        let file_path = dir.join(filename);
        let content = format!(
            r#"%primitive {name} {{
    %emit js {{
        // {name} primitive
        console.log("{name}");
    }}
}}"#,
            name = primitive_name
        );
        fs::write(&file_path, content).unwrap();
        file_path
    }

    // ===== A. unregister_file tests =====

    #[test]
    fn test_unregister_removes_primitives() {
        let mut registry = MetaRegistry::new();

        let prim_a = crate::parser::meta_ast::PrimitiveDefAst {
            name: "prim_a".to_string(),
            params: vec![],
            body: Default::default(),
            uses: vec![],
            span: Default::default(),
            source_file: Some("file_a.st".to_string()),
            doc: None,
        };
        let prim_b = crate::parser::meta_ast::PrimitiveDefAst {
            name: "prim_b".to_string(),
            params: vec![],
            body: Default::default(),
            uses: vec![],
            span: Default::default(),
            source_file: Some("file_b.st".to_string()),
            doc: None,
        };

        registry.register_primitive(prim_a).unwrap();
        registry.register_primitive(prim_b).unwrap();

        registry.unregister_file("file_a.st");

        assert!(registry.get_primitive("prim_a").is_none());
        assert!(registry.get_primitive("prim_b").is_some());
    }

    #[test]
    fn test_unregister_removes_macros() {
        let mut registry = MetaRegistry::new();

        let macro_a = crate::parser::meta_ast::MacroDefAst {
            retired: None,
            name: "macro_a".to_string(),
            form: None,
            binds: vec![],
            derives: vec![],
            states: None,
            registers: None,
            imports: None,
            order: None,
            resolves: None,
            scopes: vec![],
            scope_within: Vec::new(),
            scope_element: Vec::new(),
            body: vec![],
            requires: vec![],
            span: Default::default(),
            source_file: Some("file_a.st".to_string()),
            module: None,
            doc: None,
            ..Default::default()
        };
        let macro_b = crate::parser::meta_ast::MacroDefAst {
            retired: None,
            name: "macro_b".to_string(),
            form: None,
            binds: vec![],
            derives: vec![],
            states: None,
            registers: None,
            imports: None,
            order: None,
            resolves: None,
            scopes: vec![],
            scope_within: Vec::new(),
            scope_element: Vec::new(),
            body: vec![],
            requires: vec![],
            span: Default::default(),
            source_file: Some("file_b.st".to_string()),
            module: None,
            doc: None,
            ..Default::default()
        };

        registry.register_macro(macro_a).unwrap();
        registry.register_macro(macro_b).unwrap();

        registry.unregister_file("file_a.st");

        assert!(registry.get_macro("macro_a").is_none());
        assert!(registry.get_macro("macro_b").is_some());
    }

    #[test]
    fn test_unregister_removes_capture_types() {
        use crate::parser::meta_ast::*;

        let mut registry = MetaRegistry::new();

        let ct_a = CaptureTypeDefAst {
            name: "type_a".to_string(),
            pattern: CapturePatternAst::Capture {
                var_name: "x".to_string(),
                capture_type: CaptureType::Ident,
                modifier: CaptureModifier::Required,
            },
            span: Default::default(),
            source_file: Some("file_a.st".to_string()),
        };
        let ct_b = CaptureTypeDefAst {
            name: "type_b".to_string(),
            pattern: CapturePatternAst::Capture {
                var_name: "x".to_string(),
                capture_type: CaptureType::Ident,
                modifier: CaptureModifier::Required,
            },
            span: Default::default(),
            source_file: Some("file_b.st".to_string()),
        };

        registry.register_capture_type(ct_a).unwrap();
        registry.register_capture_type(ct_b).unwrap();

        registry.unregister_file("file_a.st");

        assert!(registry.get_capture_type("type_a").is_none());
        assert!(registry.get_capture_type("type_b").is_some());
    }

    #[test]
    fn test_unregister_removes_runtime_registries() {
        use crate::parser::meta_ast::*;

        let mut registry = MetaRegistry::new();

        let rr_a = RuntimeRegistryDef {
            name: "reg_a".to_string(),
            targets: vec![],
            span: Default::default(),
            source_file: Some("file_a.st".to_string()),
        };
        let rr_b = RuntimeRegistryDef {
            name: "reg_b".to_string(),
            targets: vec![],
            span: Default::default(),
            source_file: Some("file_b.st".to_string()),
        };

        registry.register_runtime_registry(rr_a).unwrap();
        registry.register_runtime_registry(rr_b).unwrap();

        registry.unregister_file("file_a.st");

        assert!(registry.get_runtime_registry("reg_a").is_none());
        assert!(registry.get_runtime_registry("reg_b").is_some());
    }

    #[test]
    fn test_unregister_noop_for_unknown_source() {
        let mut registry = MetaRegistry::new();

        let prim = crate::parser::meta_ast::PrimitiveDefAst {
            name: "prim".to_string(),
            params: vec![],
            body: Default::default(),
            uses: vec![],
            span: Default::default(),
            source_file: Some("file_a.st".to_string()),
            doc: None,
        };
        registry.register_primitive(prim).unwrap();

        // Unregister a file that contributed nothing — should not panic or change anything
        registry.unregister_file("nonexistent.st");

        assert!(registry.get_primitive("prim").is_some());
    }

    #[test]
    fn test_unregister_then_reregister_no_duplicate_error() {
        let mut registry = MetaRegistry::new();

        let prim = crate::parser::meta_ast::PrimitiveDefAst {
            name: "prim".to_string(),
            params: vec![],
            body: Default::default(),
            uses: vec![],
            span: Default::default(),
            source_file: Some("file_a.st".to_string()),
            doc: None,
        };
        registry.register_primitive(prim).unwrap();

        registry.unregister_file("file_a.st");

        // Re-register same-named primitive — should not get DuplicatePrimitive error
        let prim2 = crate::parser::meta_ast::PrimitiveDefAst {
            name: "prim".to_string(),
            params: vec![],
            body: Default::default(),
            uses: vec![],
            span: Default::default(),
            source_file: Some("file_a.st".to_string()),
            doc: None,
        };
        assert!(registry.register_primitive(prim2).is_ok());
    }

    #[test]
    fn test_unregister_mixed_def_types_same_file() {
        use crate::parser::meta_ast::*;

        let mut registry = MetaRegistry::new();

        let prim = PrimitiveDefAst {
            name: "mixed_prim".to_string(),
            params: vec![],
            body: Default::default(),
            uses: vec![],
            span: Default::default(),
            source_file: Some("mixed.st".to_string()),
            doc: None,
        };
        let mac = MacroDefAst {
            retired: None,
            name: "mixed_macro".to_string(),
            form: None,
            binds: vec![],
            derives: vec![],
            states: None,
            registers: None,
            imports: None,
            order: None,
            resolves: None,
            scopes: vec![],
            scope_within: Vec::new(),
            scope_element: Vec::new(),
            body: vec![],
            requires: vec![],
            span: Default::default(),
            source_file: Some("mixed.st".to_string()),
            module: None,
            doc: None,
            ..Default::default()
        };
        let ct = CaptureTypeDefAst {
            name: "mixed_type".to_string(),
            pattern: CapturePatternAst::Capture {
                var_name: "x".to_string(),
                capture_type: CaptureType::Ident,
                modifier: CaptureModifier::Required,
            },
            span: Default::default(),
            source_file: Some("mixed.st".to_string()),
        };

        registry.register_primitive(prim).unwrap();
        registry.register_macro(mac).unwrap();
        registry.register_capture_type(ct).unwrap();

        registry.unregister_file("mixed.st");

        assert!(registry.get_primitive("mixed_prim").is_none());
        assert!(registry.get_macro("mixed_macro").is_none());
        assert!(registry.get_capture_type("mixed_type").is_none());
    }

    #[test]
    fn test_unregister_with_none_source_file() {
        let mut registry = MetaRegistry::new();

        // Defs with source_file: None should NOT be removed
        let prim = crate::parser::meta_ast::PrimitiveDefAst {
            name: "orphan_prim".to_string(),
            params: vec![],
            body: Default::default(),
            uses: vec![],
            span: Default::default(),
            source_file: None,
            doc: None,
        };
        registry.register_primitive(prim).unwrap();

        registry.unregister_file("any_file.st");

        assert!(registry.get_primitive("orphan_prim").is_some());
    }

    // ===== B. source_file tagging tests =====

    #[test]
    fn test_load_file_tags_all_def_types() {
        // Load a .st file containing a primitive and verify source_file is set
        let tmp = TempDir::new().unwrap();
        let file_path = create_stdlib_file(tmp.path(), "test.st", "tagged_prim");

        let mut registry = MetaRegistry::new();
        registry.load_file_for_stdlib(&file_path).unwrap();

        let prim = registry.get_primitive("tagged_prim").unwrap();
        assert_eq!(
            prim.source_file.as_deref(),
            Some(file_path.to_string_lossy().as_ref())
        );
    }

    // ===== C. IncrementalStdlibCache tests =====

    #[test]
    fn test_from_dirs_populates_mtimes() {
        let tmp = TempDir::new().unwrap();
        create_stdlib_file(tmp.path(), "a.st", "cache_prim_a");
        create_stdlib_file(tmp.path(), "b.st", "cache_prim_b");
        create_stdlib_file(tmp.path(), "c.st", "cache_prim_c");

        let cache = IncrementalStdlibCache::from_dirs(vec![tmp.path().to_path_buf()]);

        let mtimes = cache.file_mtimes.as_ref().unwrap();
        assert_eq!(mtimes.len(), 3);
        assert!(cache.registry.get_primitive("cache_prim_a").is_some());
        assert!(cache.registry.get_primitive("cache_prim_b").is_some());
        assert!(cache.registry.get_primitive("cache_prim_c").is_some());
    }

    #[test]
    fn test_refresh_noop_when_unchanged() {
        let tmp = TempDir::new().unwrap();
        create_stdlib_file(tmp.path(), "a.st", "noop_prim");

        let mut cache = IncrementalStdlibCache::from_dirs(vec![tmp.path().to_path_buf()]);
        let count_before = cache.registry.primitive_count();

        cache.refresh_if_needed();

        assert_eq!(cache.registry.primitive_count(), count_before);
        assert!(cache.registry.get_primitive("noop_prim").is_some());
    }

    #[test]
    fn test_refresh_detects_modified_primitive() {
        let tmp = TempDir::new().unwrap();
        let file_path = create_stdlib_file(tmp.path(), "mod.st", "old_prim");

        let mut cache = IncrementalStdlibCache::from_dirs(vec![tmp.path().to_path_buf()]);
        assert!(cache.registry.get_primitive("old_prim").is_some());

        // Wait a moment to ensure mtime difference
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Overwrite with a different primitive name
        let content = r#"%primitive new_prim {
    %emit js {
        console.log("new");
    }
}"#;
        fs::write(&file_path, content).unwrap();

        cache.refresh_if_needed();

        assert!(cache.registry.get_primitive("old_prim").is_none());
        assert!(cache.registry.get_primitive("new_prim").is_some());
    }

    #[test]
    fn test_refresh_detects_new_file() {
        let tmp = TempDir::new().unwrap();
        create_stdlib_file(tmp.path(), "existing.st", "existing_prim");

        let mut cache = IncrementalStdlibCache::from_dirs(vec![tmp.path().to_path_buf()]);
        assert_eq!(cache.registry.primitive_count(), 1);

        // Add a new file
        create_stdlib_file(tmp.path(), "new.st", "new_prim");

        cache.refresh_if_needed();

        assert_eq!(cache.registry.primitive_count(), 2);
        assert!(cache.registry.get_primitive("new_prim").is_some());
    }

    #[test]
    fn test_refresh_detects_deleted_file() {
        let tmp = TempDir::new().unwrap();
        let file_a = create_stdlib_file(tmp.path(), "a.st", "prim_a");
        create_stdlib_file(tmp.path(), "b.st", "prim_b");

        let mut cache = IncrementalStdlibCache::from_dirs(vec![tmp.path().to_path_buf()]);
        assert_eq!(cache.registry.primitive_count(), 2);

        // Delete file_a
        fs::remove_file(&file_a).unwrap();

        cache.refresh_if_needed();

        assert_eq!(cache.registry.primitive_count(), 1);
        assert!(cache.registry.get_primitive("prim_a").is_none());
        assert!(cache.registry.get_primitive("prim_b").is_some());
    }

    #[test]
    fn test_refresh_handles_renamed_def() {
        let tmp = TempDir::new().unwrap();
        let file_path = create_stdlib_file(tmp.path(), "rename.st", "foo");

        let mut cache = IncrementalStdlibCache::from_dirs(vec![tmp.path().to_path_buf()]);
        assert!(cache.registry.get_primitive("foo").is_some());

        std::thread::sleep(std::time::Duration::from_millis(50));

        // Rename the primitive inside the file
        let content = r#"%primitive bar {
    %emit js {
        console.log("bar");
    }
}"#;
        fs::write(&file_path, content).unwrap();

        cache.refresh_if_needed();

        assert!(cache.registry.get_primitive("foo").is_none());
        assert!(cache.registry.get_primitive("bar").is_some());
    }

    #[test]
    fn test_refresh_ignores_test_files() {
        let tmp = TempDir::new().unwrap();
        create_stdlib_file(tmp.path(), "real.st", "real_prim");

        let mut cache = IncrementalStdlibCache::from_dirs(vec![tmp.path().to_path_buf()]);
        assert_eq!(cache.registry.primitive_count(), 1);

        // Add a test file — should be ignored
        create_stdlib_file(tmp.path(), "something.test.st", "test_prim");

        cache.refresh_if_needed();

        assert_eq!(cache.registry.primitive_count(), 1);
        assert!(cache.registry.get_primitive("test_prim").is_none());
    }

    #[test]
    fn test_refresh_ignores_examples_dir() {
        let tmp = TempDir::new().unwrap();
        create_stdlib_file(tmp.path(), "real.st", "real_prim2");

        let mut cache = IncrementalStdlibCache::from_dirs(vec![tmp.path().to_path_buf()]);
        assert_eq!(cache.registry.primitive_count(), 1);

        // Add a file in examples/ subdir — should be ignored
        let examples_dir = tmp.path().join("examples");
        fs::create_dir(&examples_dir).unwrap();
        create_stdlib_file(&examples_dir, "example.st", "example_prim");

        cache.refresh_if_needed();

        assert_eq!(cache.registry.primitive_count(), 1);
        assert!(cache.registry.get_primitive("example_prim").is_none());
    }

    #[test]
    fn test_refresh_parse_error_preserves_registry() {
        let tmp = TempDir::new().unwrap();
        let file_a = create_stdlib_file(tmp.path(), "a.st", "good_prim");
        create_stdlib_file(tmp.path(), "b.st", "other_prim");

        let mut cache = IncrementalStdlibCache::from_dirs(vec![tmp.path().to_path_buf()]);
        assert_eq!(cache.registry.primitive_count(), 2);

        std::thread::sleep(std::time::Duration::from_millis(50));

        // Write invalid syntax to file a
        fs::write(&file_a, "%primitive bad { INVALID SYNTAX !!!").unwrap();

        cache.refresh_if_needed();

        // Old defs from bad file removed, other file's defs preserved
        assert!(cache.registry.get_primitive("good_prim").is_none());
        assert!(cache.registry.get_primitive("other_prim").is_some());
        // Error should be collected
        assert!(!cache.errors.is_empty());
    }

    #[test]
    fn test_refresh_multiple_changes_at_once() {
        let tmp = TempDir::new().unwrap();
        let file_a = create_stdlib_file(tmp.path(), "a.st", "prim_a2");
        let file_b = create_stdlib_file(tmp.path(), "b.st", "prim_b2");

        let mut cache = IncrementalStdlibCache::from_dirs(vec![tmp.path().to_path_buf()]);
        assert_eq!(cache.registry.primitive_count(), 2);

        std::thread::sleep(std::time::Duration::from_millis(50));

        // Modify file_a (rename primitive)
        let content = r#"%primitive prim_a_new {
    %emit js {
        console.log("a_new");
    }
}"#;
        fs::write(&file_a, content).unwrap();

        // Delete file_b
        fs::remove_file(&file_b).unwrap();

        // Add file_c
        create_stdlib_file(tmp.path(), "c.st", "prim_c2");

        cache.refresh_if_needed();

        assert!(cache.registry.get_primitive("prim_a2").is_none());
        assert!(cache.registry.get_primitive("prim_a_new").is_some());
        assert!(cache.registry.get_primitive("prim_b2").is_none());
        assert!(cache.registry.get_primitive("prim_c2").is_some());
        assert_eq!(cache.registry.primitive_count(), 2);
    }

    #[test]
    fn test_embedded_stdlib_skips_mtime_check() {
        // Construct a cache with file_mtimes: None (simulates embedded mode)
        let cache_inner = IncrementalStdlibCache {
            registry: MetaRegistry::new(),
            errors: Vec::new(),
            file_mtimes: None,
            stdlib_dirs: Vec::new(),
            cache_path: None,
        };

        let mut cache = cache_inner;
        // Should be a no-op and not panic
        cache.refresh_if_needed();
        assert_eq!(cache.registry.primitive_count(), 0);
    }

    #[test]
    fn test_get_returns_clone() {
        let tmp = TempDir::new().unwrap();
        create_stdlib_file(tmp.path(), "clone.st", "clone_prim");

        let cache = IncrementalStdlibCache::from_dirs(vec![tmp.path().to_path_buf()]);

        let (reg1, _) = cache.get();
        let (reg2, _) = cache.get();

        // Both should have the primitive
        assert!(reg1.get_primitive("clone_prim").is_some());
        assert!(reg2.get_primitive("clone_prim").is_some());
    }

    // ===== D. Binary cache tests =====

    #[test]
    fn test_binary_cache_round_trip() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("stdlib_rt");
        fs::create_dir(&dir).unwrap();
        create_stdlib_file(&dir, "rt_a.st", "rt_prim_a");
        create_stdlib_file(&dir, "rt_b.st", "rt_prim_b");

        // First call: full parse + writes binary cache
        let cache1 = IncrementalStdlibCache::from_dirs(vec![dir.clone()]);
        assert!(cache1.registry.get_primitive("rt_prim_a").is_some());
        assert!(cache1.registry.get_primitive("rt_prim_b").is_some());

        // Verify cache file was written
        let cache_path = dir.parent().unwrap().join(".cache").join("registry.bin");
        assert!(
            cache_path.exists(),
            "binary cache file should exist after first call"
        );

        // Second call: should deserialize from binary cache (same mtimes)
        let cache2 = IncrementalStdlibCache::from_dirs(vec![dir.clone()]);
        assert!(cache2.registry.get_primitive("rt_prim_a").is_some());
        assert!(cache2.registry.get_primitive("rt_prim_b").is_some());
        assert_eq!(
            cache2.file_mtimes.as_ref().unwrap().len(),
            cache1.file_mtimes.as_ref().unwrap().len()
        );
    }

    #[test]
    fn test_binary_cache_version_rejection() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("stdlib_vr");
        fs::create_dir(&dir).unwrap();
        create_stdlib_file(&dir, "vr.st", "vr_prim");

        // Write a cache with a wrong version
        let cache_dir = tmp.path().join(".cache");
        fs::create_dir_all(&cache_dir).unwrap();
        let cache_file = cache_dir.join("registry.bin");

        let mut file_mtimes = HashMap::new();
        collect_file_mtimes(&dir, &mut file_mtimes);

        let bad_cache = BinaryStdlibCache {
            version: "0.0.0-invalid".to_string(),
            struct_canary: CACHE_STRUCT_HASH.to_string(),
            file_mtimes: file_mtimes.clone(),
            registry: MetaRegistry::new(),
        };
        write_binary_cache(&cache_file, &bad_cache);
        assert!(cache_file.exists());

        // load_binary_cache should reject due to version mismatch
        let loaded = load_binary_cache(&cache_file);
        assert!(
            loaded.is_none(),
            "cache with wrong version should be rejected"
        );
    }

    #[test]
    fn test_binary_cache_corrupt_rejection() {
        let tmp = TempDir::new().unwrap();
        let cache_dir = tmp.path().join(".cache");
        fs::create_dir_all(&cache_dir).unwrap();
        let cache_file = cache_dir.join("registry.bin");

        // Write garbage bytes
        fs::write(&cache_file, b"this is not valid bincode data at all!!!").unwrap();
        assert!(cache_file.exists());

        // load_binary_cache should gracefully return None
        let loaded = load_binary_cache(&cache_file);
        assert!(
            loaded.is_none(),
            "corrupt cache file should be handled gracefully"
        );
    }

    // ===== E. I4 / gh-18: capture-consumption lint runs on the cache path =====

    /// A macro whose %form binds a capture it never forwards must surface as an
    /// E0964 load error on the from_dirs (cached) path — a lint the fast path
    /// skips is a lint that dies the day builds get fast.
    fn create_unconsuming_macro_file(dir: &Path) -> PathBuf {
        let file_path = dir.join("half-impl.st");
        let content = r#"%macro half-impl {
  %form {
    @half-impl(x: $x:number = 1) { $body:keyframes }
  }
  %binds {
    intersection(&self, threshold: 0.1) -> { $visible }
  }
}
"#;
        fs::write(&file_path, content).unwrap();
        file_path
    }

    #[test]
    fn lint_runs_on_from_dirs_path() {
        let tmp = TempDir::new().unwrap();
        // stdlib dir nested under tmp so the binary cache lands inside tmp
        // (`binary_cache_path` = stdlib_dir.parent()/.cache/registry.bin).
        let stdlib = tmp.path().join("stdlib");
        fs::create_dir_all(&stdlib).unwrap();
        create_stdlib_file(&stdlib, "a.st", "cache_prim_lint");
        create_unconsuming_macro_file(&stdlib);

        let cache = IncrementalStdlibCache::from_dirs(vec![stdlib]);
        let has_lint = cache
            .errors
            .iter()
            .any(|e| e.message.contains("never consumes"));
        assert!(
            has_lint,
            "the capture-consumption lint must fire on from_dirs (cached) load"
        );
        // A poisoned registry must NEVER be persisted: the binary cache write
        // is gated on a clean load, so a lint error must leave the cache dir
        // with no serialized registry.bin.
        let cache_file = tmp.path().join(".cache").join("registry.bin");
        assert!(
            !cache_file.exists(),
            "a registry carrying a lint error must not be serialized"
        );
    }

    #[test]
    fn lint_refreshes_after_incremental_change() {
        let tmp = TempDir::new().unwrap();
        create_stdlib_file(tmp.path(), "a.st", "cache_prim_lint2");
        let clean_file = create_unconsuming_macro_file(tmp.path());

        let mut cache = IncrementalStdlibCache::from_dirs(vec![tmp.path().to_path_buf()]);
        assert!(cache.errors.iter().any(|e| e.message.contains("never consumes")));

        // Fix the macro (forward both captures) — the lint must clear on refresh.
        let fixed = r#"%macro half-impl {
  %form {
    @half-impl(x: $x:number = 1) { $body:keyframes }
  }
  %binds {
    intersection(&self, threshold: $x) -> { $visible }
    apply-animations(&self, driver: $visible, animations: $body)
  }
}
"#;
        fs::write(&clean_file, fixed).unwrap();
        // Touch mtime (mtime granularity can be coarse) — rewrite with a delay
        // is unreliable, so force a distinct mtime by waiting.
        std::thread::sleep(std::time::Duration::from_millis(20));
        fs::write(&clean_file, fixed).unwrap();

        cache.refresh_if_needed();
        let still_lint = cache
            .errors
            .iter()
            .any(|e| e.message.contains("never consumes"));
        assert!(
            !still_lint,
            "after fixing the macro, the capture-consumption lint must clear on refresh"
        );
    }
}

