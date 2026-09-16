//! Import path resolution
//!
//! Resolves `@import "path"` statements to actual file locations.

use dashmap::DashMap;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::config::WorkspaceConfig;

/// Result of resolving an import path
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResolvedImport {
    /// Resolved to a file on disk
    File(PathBuf),
    /// Resolved to an embedded stdlib module
    Stdlib { module_name: String },
    /// Could not resolve the import
    Unresolved {
        original_path: String,
        searched_paths: Vec<PathBuf>,
    },
}

impl ResolvedImport {
    /// Get the file path if this is a File variant
    pub fn as_file(&self) -> Option<&PathBuf> {
        match self {
            ResolvedImport::File(p) => Some(p),
            _ => None,
        }
    }

    /// Check if this import resolved successfully
    pub fn is_resolved(&self) -> bool {
        !matches!(self, ResolvedImport::Unresolved { .. })
    }
}

/// Resolves import paths to file locations
pub struct ImportResolver {
    config: WorkspaceConfig,
    /// Cache of resolved paths: (importing_file, import_path) -> resolved
    cache: DashMap<(PathBuf, String), ResolvedImport>,
    /// Known stdlib module names
    stdlib_modules: HashSet<String>,
}

impl ImportResolver {
    /// Every directory to search for `stdlib/`, most specific first.
    ///
    /// BUG-326 — resolution is anchored to the FILE that wrote the import, not
    /// to `$PWD`. The same file compiled by the same binary used to pass or fail
    /// depending on the shell's location, because the only roots were a
    /// configured path and `<cwd>/stdlib`. That makes the compiler's answer a
    /// function of the invoking environment rather than of the source, and
    /// breaks every caller that does not chdir first: editors and language
    /// servers (arbitrary cwd), pre-commit hooks, CI building several projects,
    /// any script passing absolute paths.
    ///
    /// Order is most-specific-first so a project vendoring its own stdlib still
    /// wins over the toolchain copy (preserving BUG-340's filesystem-first
    /// intent):
    ///   1. explicitly configured `stdlib_path`
    ///   2. every ancestor of the importing file that has a `stdlib/`
    ///   3. the toolchain's own stdlib, beside the running binary
    ///
    /// `$PWD` is deliberately absent: a file's meaning belongs to the file.
    fn stdlib_roots(&self, from_file: &Path) -> Vec<PathBuf> {
        let mut roots: Vec<PathBuf> = Vec::new();
        let mut push = |p: PathBuf, roots: &mut Vec<PathBuf>| {
            if !roots.contains(&p) {
                roots.push(p);
            }
        };

        if let Some(ref stdlib_path) = self.config.stdlib_path {
            push(stdlib_path.clone(), &mut roots);
        }

        // `workspace_root_for` is THE existing answer to "which checkout owns
        // this file" — written for BUG-227, hardened by BUG-350. Reuse it rather
        // than walking ancestors here: it already prefers the checkout enclosing
        // the FILE over any cwd-derived guess, falls back to the binary's own
        // checkout for a site outside any checkout, and requires
        // `stdlib/primitives` rather than a bare directory named `stdlib`
        // (BUG-350: `docs/stdlib/` holds only prose and satisfied a naive
        // `is_dir()`, yielding a zero-byte build).
        //
        // A second, parallel notion of "where the stdlib is" is precisely how
        // these two layers drifted apart to begin with.
        if let Some(root) = crate::toolchain::workspace_root_for(from_file) {
            push(root.join("stdlib"), &mut roots);
        }

        // Last resort: the historical `<cwd>/stdlib`. Kept BELOW the
        // file-anchored answer so it can no longer decide which stdlib wins — it
        // only rescues a case the walk could not resolve at all.
        if let Ok(cwd) = std::env::current_dir() {
            push(cwd.join("stdlib"), &mut roots);
        }

        roots
    }

    /// Create a new import resolver with the given config
    pub fn new(config: WorkspaceConfig) -> Self {
        let stdlib_modules = Self::build_stdlib_index();
        Self {
            config,
            cache: DashMap::new(),
            stdlib_modules,
        }
    }

    /// Update config (e.g., when workspace root changes)
    pub fn update_config(&mut self, config: WorkspaceConfig) {
        self.config = config;
        self.cache.clear();
    }

    /// Resolve an import path relative to the importing file
    pub fn resolve(&self, import_path: &str, from_file: &Path) -> ResolvedImport {
        let cache_key = (from_file.to_path_buf(), import_path.to_string());

        if let Some(cached) = self.cache.get(&cache_key) {
            return cached.clone();
        }

        let resolved = self.resolve_inner(import_path, from_file);
        self.cache.insert(cache_key, resolved.clone());
        resolved
    }

    fn resolve_inner(&self, import_path: &str, from_file: &Path) -> ResolvedImport {
        // 1. Check for alias (e.g., "@lib/foo")
        for (alias, target) in &self.config.import_aliases {
            if import_path.starts_with(alias) {
                let rest = &import_path[alias.len()..];
                let rest = rest.strip_prefix('/').unwrap_or(rest);
                return self.resolve_path(&target.join(rest));
            }
        }

        // 2. Relative imports: "./sibling.st", "../parent.st"
        if import_path.starts_with("./") || import_path.starts_with("../") {
            let base_dir = from_file.parent().unwrap_or(Path::new("."));
            return self.resolve_path(&base_dir.join(import_path));
        }

        // 3. Absolute imports: "/full/path.st" (relative to workspace root)
        if import_path.starts_with('/') {
            let path = self.config.root.join(&import_path[1..]);
            return self.resolve_path(&path);
        }

        // 4. Stdlib imports: "stdlib" (bare aggregate) or "stdlib/macros/fade-in"
        if import_path == "stdlib" {
            // Bare stdlib import resolves to stdlib/index.st on disk if available.
            // Try the configured `stdlib_path` first (workspace-root derived), then
            // fall back to `<cwd>/stdlib` — mirrors `resolve_stdlib`'s fallback below
            // (module-path imports like `stdlib/text`), which already needs this
            // because `workspace_root` is frequently a SITE directory (e.g. `spacetime
            // serve projects/<name>/` passes the site dir, not the repo root), so
            // `workspace_root.join("stdlib")` doesn't exist there — only `<cwd>/stdlib`
            // does. Without this fallback, a bare `@import "stdlib"` silently drops to
            // the embedded-stdlib dead end (`ResolvedImport::Stdlib`, which `resolve_
            // imports` never recurses into — see its `Stdlib { .. } => continue` arm),
            // so ANY stdlib file reached only through the bare aggregate (e.g.
            // `stdlib/macros/timeline.st`'s own `@import "stdlib/text"`) never gets its
            // transitive imports followed, even though the exact same import string
            // resolves fine from a file that reaches `stdlib/text` directly.
            let roots = self.stdlib_roots(from_file);
            for root in &roots {
                let index_path = root.join("index.st");
                if index_path.exists() {
                    return ResolvedImport::File(index_path);
                }
            }
            return ResolvedImport::Stdlib {
                module_name: "index".to_string(),
            };
        }
        if let Some(module) = import_path.strip_prefix("stdlib/") {
            // Strip "stdlib/"
            return self.resolve_stdlib(module, from_file);
        }

        // 5. Bare imports: check stdlib first, then include paths
        if self.is_stdlib_module(import_path) {
            return self.resolve_stdlib(import_path, from_file);
        }

        // 6. Try BESIDE THE IMPORTING FILE (BUG-363).
        //
        // `@import "_helpers.st"` is how a project shares declarations across
        // its pages, and it means "the file next to me" — that is what a
        // relative path means everywhere else in computing. Only `./_helpers.st`
        // was honoured (branch 2 above); the bare spelling, which is what
        // authors actually write, fell through to the workspace root and failed:
        //
        //     Could not resolve import '_helpers.st' from projects/animations/index.st.
        //     Searched: ["<toolchain root>/_helpers.st"]
        //
        // The importing file sits in `projects/animations/`; the file it wants
        // is right beside it. Anchoring to the workspace root only works when
        // the project IS the toolchain checkout, which for every real site it
        // is not — the same wrong-base defect as BUG-227 (stdlib registry
        // trusting CWD) and BUG-326.
        //
        // This step is placed AFTER the stdlib branches deliberately: a project
        // that happens to contain a `stdlib/` directory must not capture
        // `@import "stdlib/md"`, so sibling lookup may only answer what the
        // stdlib has already declined. It is placed BEFORE the workspace root
        // so the NEARER file wins — an import inside `pages/about.st` means
        // `pages/_local.st`, not a same-named file at the project top level.
        let from_sibling = from_file
            .parent()
            .unwrap_or(Path::new("."))
            .join(import_path);
        if let ResolvedImport::File(p) = self.resolve_path(&from_sibling) {
            return ResolvedImport::File(p);
        }

        // 7. Try as relative to workspace root
        let from_root = self.config.root.join(import_path);
        if let ResolvedImport::File(p) = self.resolve_path(&from_root) {
            return ResolvedImport::File(p);
        }

        // 8. Not found. Report BOTH bases that were tried: the old message named
        // only the workspace root, which is precisely why the failure read as
        // nonsense — it pointed at a directory the author had never mentioned.
        ResolvedImport::Unresolved {
            original_path: import_path.to_string(),
            searched_paths: vec![from_sibling, from_root],
        }
    }

    fn resolve_path(&self, path: &Path) -> ResolvedImport {
        // Try with .st extension if not present
        let paths_to_try = if path.extension().is_some() {
            vec![path.to_path_buf()]
        } else {
            vec![path.with_extension("st"), path.to_path_buf()]
        };

        for p in &paths_to_try {
            if let Ok(canonical) = p.canonicalize()
                && canonical.exists()
                && canonical.extension().is_some_and(|e| e == "st")
            {
                return ResolvedImport::File(canonical);
            }
            // Also try without canonicalize for non-existent but valid paths
            if p.exists() {
                return ResolvedImport::File(p.clone());
            }
        }

        ResolvedImport::Unresolved {
            original_path: path.to_string_lossy().to_string(),
            searched_paths: paths_to_try,
        }
    }

    /// `from_file` anchors the stdlib search to the importing file's project
    /// rather than to `$PWD` (BUG-326) — see `stdlib_roots`.
    fn resolve_stdlib(&self, module_path: &str, from_file: &Path) -> ResolvedImport {
        // Normalize: "macros/fade-in" or "fade-in"
        let normalized = module_path.strip_suffix(".st").unwrap_or(module_path);

        // Check filesystem stdlib FIRST. Try the configured `stdlib_path`, then
        // fall back to `<cwd>/stdlib` — so `stdlib/*` resolves even when the
        // workspace root was derived from a site file's parent dir (e.g.
        // `build path/to/site.st`) rather than the repo root.
        //
        // ORDER MATTERS (BUG-340). The embedded-stdlib check used to run first,
        // and `build_stdlib_index` indexes every `.st` under `stdlib/` by BARE
        // NAME — so `stdlib/md/index.st` registers the module `"md"`, and
        // `@import "stdlib/md"` matched it and returned `ResolvedImport::Stdlib`.
        // That variant is a DEAD END for the opt-in modules: `resolve_imports`
        // skips it ("handled by MetaRegistry"), but `MetaRegistry` only loads
        // `STDLIB_DIRS`, which deliberately excludes them — they are meant to
        // load on demand, from disk, when a page imports them. The result was
        // that EVERY opt-in module went dark: `@doc` (so every literate `.st.md`
        // page), `@balance`, `@stage`, each failing with "unknown primitive".
        //
        // A file on disk is the more specific answer and can always be followed;
        // the embedded name is the fallback for when there is no checkout.
        let roots = self.stdlib_roots(from_file);

        let mut searched = Vec::new();
        for root in &roots {
            // Flat `<module>.st`, then directory module `<module>/index.st`
            // (the modular layout — PLAN-024).
            let flat = root.join(format!("{}.st", normalized));
            if flat.exists() {
                return ResolvedImport::File(flat);
            }
            searched.push(flat);
            let dir_index = root.join(normalized).join("index.st");
            if dir_index.exists() {
                return ResolvedImport::File(dir_index);
            }
            searched.push(dir_index);
        }

        // No file on disk — fall back to the embedded stdlib name, which is how
        // an installed binary with no checkout resolves the always-on modules.
        if self.is_stdlib_module(normalized) {
            return ResolvedImport::Stdlib {
                module_name: normalized.to_string(),
            };
        }

        ResolvedImport::Unresolved {
            original_path: module_path.to_string(),
            searched_paths: searched,
        }
    }

    fn is_stdlib_module(&self, name: &str) -> bool {
        // Check various forms of the name
        self.stdlib_modules.contains(name)
            || self.stdlib_modules.contains(&format!("macros/{}", name))
            || self
                .stdlib_modules
                .contains(&format!("primitives/{}", name))
    }

    /// Build the index of known stdlib modules by WALKING stdlib.
    ///
    /// This used to be a hand-written list whose own comment said "Mirrors
    /// STDLIB_MACROS from form_registry.rs" — a list GH-28 deleted, leaving this
    /// one mirroring nothing. It had already drifted: `@import "handle"`
    /// resolved to nothing while `stdlib/macros/handle.st` existed and declared
    /// a live `%macro handle`, so adding a stdlib module still required editing
    /// a second list that nobody would think to look for.
    ///
    /// Two sources of truth for "what modules exist" do not drift only while
    /// someone maintains them — and nothing fails when they diverge, which is
    /// what makes the drift invisible until a user hits it. So this derives the
    /// set from the filesystem: every `.st` under `stdlib/` is a module, indexed
    /// both by its category path (`macros/handle`) and its bare name (`handle`),
    /// which are the two spellings `@import` accepts.
    fn build_stdlib_index() -> HashSet<String> {
        let mut out = HashSet::new();
        let Some(root) = Self::stdlib_root() else {
            return out;
        };
        Self::walk_stdlib(&root, &root, &mut out);
        out
    }

    /// Locate the stdlib directory, searching upward from the executable and the
    /// working directory so the LSP finds it whether it runs from a checkout or
    /// an installed binary.
    fn stdlib_root() -> Option<PathBuf> {
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Ok(cwd) = std::env::current_dir() {
            candidates.push(cwd);
        }
        if let Ok(exe) = std::env::current_exe()
            && let Some(dir) = exe.parent()
        {
            candidates.push(dir.to_path_buf());
        }
        for start in candidates {
            let mut cur: Option<&Path> = Some(start.as_path());
            while let Some(dir) = cur {
                let cand = dir.join("stdlib");
                if cand.is_dir() {
                    return Some(cand);
                }
                cur = dir.parent();
            }
        }
        None
    }

    /// Index every `.st` under `dir` by category path and bare name.
    fn walk_stdlib(root: &Path, dir: &Path, out: &mut HashSet<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // `vendor/` holds third-party bundles, not importable modules.
                if path.file_name().is_some_and(|n| n == "vendor") {
                    continue;
                }
                // `__dev__` and `__host__` are internal trees, not author-facing
                // imports; indexing them would advertise modules that are not a
                // public surface.
                if path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("__"))
                {
                    continue;
                }
                Self::walk_stdlib(root, &path, out);
            } else if path.extension().is_some_and(|e| e == "st")
                && let Ok(rel) = path.strip_prefix(root)
            {
                let rel = rel.with_extension("");
                let rel_str = rel.to_string_lossy().replace('\\', "/");
                if let Some(stem) = rel.file_name().and_then(|s| s.to_str()) {
                    // `index` is spelled by its directory (`@import "stdlib/dnd"`).
                    if stem == "index" {
                        if let Some(parent) = rel.parent().and_then(|p| p.to_str())
                            && !parent.is_empty()
                        {
                            out.insert(parent.to_string());
                            if let Some(base) = parent.rsplit('/').next() {
                                out.insert(base.to_string());
                            }
                        }
                    } else {
                        out.insert(stem.to_string());
                    }
                }
                // The category-path spelling, EXCEPT for an index file: that
                // module is spelled by its directory, and leaking `dnd/index`
                // would advertise an import nobody should write.
                if !rel_str.is_empty() && !rel_str.ends_with("/index") && rel_str != "index" {
                    out.insert(rel_str);
                }
            }
        }
    }

    /// Invalidate cache entries that depend on a file
    pub fn invalidate_file(&self, path: &Path) {
        self.cache.retain(|_, v| match v {
            ResolvedImport::File(p) => p != path,
            _ => true,
        });
    }

    /// Clear the entire cache
    pub fn clear_cache(&self) {
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> WorkspaceConfig {
        WorkspaceConfig {
            root: PathBuf::from("/project"),
            exclude_patterns: vec![],
            indexing_strategy: super::super::config::IndexingStrategy::Hybrid,
            import_aliases: [("@lib".to_string(), PathBuf::from("/project/src/lib"))]
                .into_iter()
                .collect(),
            stdlib_path: None,
        }
    }

    #[test]
    fn test_stdlib_resolution() {
        let resolver = ImportResolver::new(test_config());

        // Test bare module name that should match via "macros/{name}" pattern
        let result = resolver.resolve("fade-in", Path::new("/project/src/main.st"));
        assert!(
            matches!(result, ResolvedImport::Stdlib { .. }),
            "Expected Stdlib for 'fade-in', got {:?}",
            result
        );

        // Test bare primitive name that should match via "primitives/{name}" pattern
        let result = resolver.resolve("scroll", Path::new("/project/src/main.st"));
        assert!(
            matches!(result, ResolvedImport::Stdlib { .. }),
            "Expected Stdlib for 'scroll', got {:?}",
            result
        );

        // Test full stdlib path for a macro.
        //
        // BUG-340 (1b24e320) made `resolve_stdlib` try the FILESYSTEM before the
        // embedded module index, so with a checkout present this answers
        // `File(<repo>/stdlib/macros/fade-in.st)` rather than `Stdlib`. That is
        // the point of the fix, not a regression: for the opt-in modules
        // (`stdlib/md`, `stdlib/text`, `stdlib/3d`) `Stdlib` is a DEAD END —
        // `resolve_imports` skips it as "handled by MetaRegistry", and
        // MetaRegistry loads only STDLIB_DIRS, which deliberately excludes them.
        // A path on disk can always be followed; the embedded name is the
        // fallback for an installed binary with no checkout.
        //
        // This test runs from a checkout, so filesystem-first is DETERMINISTIC
        // here and the assertion pins `File` exactly — accepting `Stdlib` too
        // would re-admit the dead end this is meant to keep out. (Until BUG-347
        // it demanded `Stdlib` and so failed on every developer machine.)
        let result = resolver.resolve("stdlib/macros/fade-in", Path::new("/project/src/main.st"));
        match &result {
            ResolvedImport::File(p) => assert!(
                p.ends_with("stdlib/macros/fade-in.st"),
                "filesystem-first resolution must point AT the module, got {p:?}"
            ),
            other => panic!(
                "'stdlib/macros/fade-in' must resolve to the file on disk \
                 (BUG-340: `Stdlib` is a dead end — `resolve_imports` skips it \
                 and MetaRegistry's STDLIB_DIRS excludes the opt-in modules), \
                 got {other:?}"
            ),
        }
    }

    #[test]
    fn test_bare_stdlib_import_falls_back_to_cwd() {
        // Regression: `resolve_stdlib` (used for `stdlib/X` module-path
        // imports like `stdlib/text`) already fell back to `<cwd>/stdlib`
        // when the configured `stdlib_path` (workspace-root derived) doesn't
        // exist there — needed because `workspace_root` is frequently a SITE
        // directory (e.g. `spacetime serve projects/<name>/` passes the site
        // dir, not the repo root). The bare `"stdlib"` aggregate-import
        // branch had NO such fallback: it only checked the configured
        // `stdlib_path`, and on failure dropped straight to the embedded-
        // stdlib dead end (`ResolvedImport::Stdlib`, which `resolve_imports`
        // never recurses into). That meant ANY stdlib file reached ONLY
        // through the bare aggregate (e.g. `stdlib/macros/timeline.st`'s own
        // `@import "stdlib/text"`) never got its transitive imports
        // followed — even though the exact same `@import "stdlib/text"`
        // string resolved fine from a file that reached it directly. Found
        // via direct reproduction while wiring value-change-driver's line-
        // aware rewrite (PLAN-072) to the shared stdlib/text pretext module:
        // `pretext` never got demand-emitted under `spacetime serve
        // projects/<name>/` with a bare `@import "stdlib"`, despite working
        // fine under `cargo run -- check` (whose workspace_root happens to
        // BE the repo root, masking the gap).
        let config = WorkspaceConfig {
            root: PathBuf::from("/definitely/does/not/exist/anywhere"),
            exclude_patterns: vec![],
            indexing_strategy: super::super::config::IndexingStrategy::Hybrid,
            import_aliases: std::collections::HashMap::new(),
            stdlib_path: Some(PathBuf::from("/definitely/does/not/exist/anywhere/stdlib")),
        };
        let resolver = ImportResolver::new(config);

        // The real stdlib/ lives at the repo root, which is this test
        // process's cwd (cargo test's default cwd is the crate root).
        let result = resolver.resolve(
            "stdlib",
            Path::new("/definitely/does/not/exist/anywhere/site/index.st"),
        );
        match result {
            ResolvedImport::File(path) => {
                assert!(
                    path.ends_with("stdlib/index.st"),
                    "expected bare 'stdlib' import to resolve to a real stdlib/index.st via the \
                     cwd fallback when stdlib_path doesn't exist, got: {:?}",
                    path
                );
            }
            other => panic!(
                "expected ResolvedImport::File via cwd fallback, got {:?} — the bare 'stdlib' \
                 import must not drop to the embedded-stdlib dead end when a real stdlib/ \
                 exists at cwd",
                other
            ),
        }
    }

    #[test]
    fn test_alias_resolution() {
        let resolver = ImportResolver::new(test_config());

        // This won't actually resolve to a file (file doesn't exist),
        // but we can verify the path construction
        let result = resolver.resolve("@lib/utils", Path::new("/project/src/main.st"));
        match result {
            ResolvedImport::Unresolved { searched_paths, .. } => {
                assert!(
                    searched_paths
                        .iter()
                        .any(|p| p.to_string_lossy().contains("src/lib/utils"))
                );
            }
            _ => {}
        }
    }

    #[test]
    fn test_bare_stdlib_resolves_to_index() {
        // With no stdlib_path on disk, bare "stdlib" must still resolve to the embedded index module
        // (rather than being treated as a relative file path that points at a directory).
        let resolver = ImportResolver::new(test_config());
        let result = resolver.resolve("stdlib", Path::new("/project/src/main.st"));
        match result {
            ResolvedImport::Stdlib { ref module_name } => {
                assert_eq!(
                    module_name, "index",
                    "bare stdlib should map to the index module"
                );
            }
            ResolvedImport::File(_) => {
                // Acceptable when stdlib_path is configured and stdlib/index.st exists on disk.
            }
            other => panic!(
                "Expected Stdlib(index) or File for bare 'stdlib', got {:?}",
                other
            ),
        }
    }
}

#[cfg(test)]
mod w5_stdlib_index_tests {
    //! W5 (PLAN-141) — the module index is DERIVED, not hand-maintained.
    //!
    //! GH-28 deleted the LSP's parallel macro registry because two sources of
    //! truth for the same fact drift, and nothing fails when they do. This index
    //! was the surviving fragment of that defect: a hand-written list whose own
    //! comment said it mirrored `STDLIB_MACROS` — the very list GH-28 removed.
    //!
    //! It had already drifted. `stdlib/macros/handle.st` declares a live
    //! `%macro handle`, and `@import "handle"` did not resolve, because nobody
    //! knew to add it to a second list in a different file.

    use super::*;

    /// The case that proves the drift is gone: a module the old hardcoded list
    /// omitted must resolve, by both spellings `@import` accepts.
    #[test]
    fn a_module_the_hardcoded_list_omitted_now_resolves() {
        let idx = ImportResolver::build_stdlib_index();
        assert!(
            idx.contains("handle"),
            "`@import \"handle\"` must resolve — stdlib/macros/handle.st declares \
             a live %macro. The hardcoded list omitted it, which is exactly the \
             GH-28 drift this index was rebuilt to end."
        );
        assert!(
            idx.contains("macros/handle"),
            "the category-path spelling must resolve too"
        );
    }

    /// Derivation must not LOSE what the hand-written list covered. These are
    /// spellings real pages use; a regression here is worse than the drift.
    #[test]
    fn the_previously_listed_modules_all_still_resolve() {
        let idx = ImportResolver::build_stdlib_index();
        for name in [
            "macros/form",
            "macros/on",
            "macros/each",
            "macros/websocket",
            "enum/state",
            "primitives/intersection",
            "primitives/fetch",
            "fade-in",
            "each",
            "scroll",
            "tick",
            "dom",
        ] {
            assert!(
                idx.contains(name),
                "`{name}` resolved under the hand-written list and must still resolve"
            );
        }
    }

    /// A directory module is spelled by its directory (`@import "stdlib/dnd"`),
    /// so `dnd/index.st` indexes as `dnd` — not as `dnd/index`.
    #[test]
    fn a_directory_module_is_spelled_by_its_directory() {
        let idx = ImportResolver::build_stdlib_index();
        assert!(idx.contains("dnd"), "`@import \"stdlib/dnd\"` must resolve");
        assert!(
            !idx.contains("dnd/index"),
            "the index file must not leak its filename as an importable spelling"
        );
    }

    /// Vendored third-party bundle TREES are not importable modules.
    ///
    /// NB the exclusion is on the `vendor/` DIRECTORY, not on the name: several
    /// modules are legitimately called `vendor.st` (`stdlib/md/vendor.st` wraps
    /// the vendored markdown engine behind a Spacetime surface, which is the
    /// project's vendoring rail). My first spelling of this test asserted no
    /// indexed name may CONTAIN "vendor" and was simply wrong — it failed on a
    /// real module. Assert what the rule actually is.
    #[test]
    fn vendor_trees_are_not_indexed() {
        let idx = ImportResolver::build_stdlib_index();
        assert!(
            !idx.iter().any(|m| m.contains("vendor/")),
            "no module may live UNDER a vendor/ directory: {:?}",
            idx.iter().filter(|m| m.contains("vendor/")).collect::<Vec<_>>()
        );
        assert!(
            idx.contains("md/vendor"),
            "a module legitimately NAMED vendor.st is still a module"
        );
    }
}
