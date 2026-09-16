//! Workspace Manager
//!
//! Central coordinator for workspace-wide indexing and cross-file features.

use dashmap::DashMap;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::lsp::form_registry::FormRegistry;
use crate::parser::StFile;

use super::config::{IndexingStrategy, WorkspaceConfig};
use super::dependency_graph::{DependencyGraph, ImportEdge};
use super::import_resolver::{ImportResolver, ResolvedImport};
use super::usage_index::{ReferenceLocation, SymbolId, UsageIndex};
use super::usage_visitor::UsageVisitor;

/// Error type for workspace operations
#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Circular import detected: {0:?}")]
    CircularImport(Vec<PathBuf>),
}

/// Indexed file information
#[derive(Debug, Clone)]
pub struct IndexedFile {
    /// Canonical file path
    pub path: PathBuf,
    /// Content hash for change detection
    pub content_hash: u64,
    /// Parsed AST
    pub ast: Option<StFile>,
    /// Resolved imports (path string, resolved import)
    pub imports: Vec<(String, ResolvedImport)>,
    /// Version (for open documents)
    pub version: i32,
}

/// Central workspace coordinator
pub struct WorkspaceManager {
    /// Workspace roots
    roots: std::sync::Mutex<Vec<PathBuf>>,
    /// Configuration
    config: RwLock<WorkspaceConfig>,
    /// Import resolver
    import_resolver: RwLock<ImportResolver>,
    /// Dependency graph
    dependency_graph: RwLock<DependencyGraph>,
    /// Usage index
    usage_index: Arc<UsageIndex>,
    /// Indexed files
    files: DashMap<PathBuf, IndexedFile>,
    /// Form registry (project + stdlib)
    form_registry: RwLock<FormRegistry>,
    /// Whether the base (compiler stdlib + project overlays) registry is loaded.
    ///
    /// Loaded lazily on the async path so a slow first stdlib load never blocks
    /// `WorkspaceManager::new` (the LSP must not block startup on it).
    base_loaded: AtomicBool,
}

impl WorkspaceManager {
    /// Create a new workspace manager
    pub fn new(roots: Vec<PathBuf>) -> Self {
        let config = if let Some(root) = roots.first() {
            WorkspaceConfig::load(root)
        } else {
            WorkspaceConfig::default_for(Path::new("."))
        };

        let import_resolver = ImportResolver::new(config.clone());

        Self {
            roots: std::sync::Mutex::new(roots),
            config: RwLock::new(config),
            import_resolver: RwLock::new(import_resolver),
            dependency_graph: RwLock::new(DependencyGraph::new()),
            usage_index: Arc::new(UsageIndex::new()),
            files: DashMap::new(),
            // Base registry is built lazily by `ensure_base_registry` (async,
            // off the reactor) so `new` never blocks on a stdlib load.
            form_registry: RwLock::new(FormRegistry::new()),
            base_loaded: AtomicBool::new(false),
        }
    }

    /// Load the base registry (compiler stdlib + this workspace's project
    /// `_prelude.st` overlays) into `self.form_registry`, once.
    ///
    /// Reuses `cached_stdlib_registry()` — the compiler's mtime-based binary
    /// cache — rather than a third load path, so it does not re-parse the full
    /// stdlib on startup. Runs on a blocking thread so the reactor is never
    /// stalled by the first (or a cold-cache) load.
    async fn ensure_base_registry(&self) {
        if self.base_loaded.load(Ordering::Relaxed) {
            return;
        }
        let roots = self.roots.lock().unwrap().clone();
        let base = tokio::task::spawn_blocking(move || {
            let (mut meta, _errors) = crate::compiler::cached_stdlib_registry();
            for root in &roots {
                // The project overlay is how the compiler auto-loads a
                // project's own `%macro`s; the LSP serves the same set so a
                // `_prelude.st` directive completes, hovers and goes-to-
                // definition exactly when `spacetime check` resolves it.
                crate::compiler::load_project_overlay(&mut meta, Some(root));
            }
            FormRegistry::from_compiler_registry(&meta)
        })
        .await
        .unwrap_or_default();
        {
            let mut registry = self.form_registry.write().await;
            // Keep any file-local macros indexed so far (base loads first in
            // practice because `index_file`/`index_content` call
            // `ensure_base_registry` before mutating, but be safe).
            let mut merged = base;
            for sig in registry.all_directives().cloned().collect::<Vec<_>>() {
                merged.register(sig);
            }
            *registry = merged;
        }
        self.base_loaded.store(true, Ordering::Relaxed);
    }

    /// Drop the cached base registry so the next request rebuilds it.
    ///
    /// The whole point of GH-28 was that the editor must serve the SAME macro
    /// set the compiler resolves. A registry loaded once and then latched
    /// forever re-creates that divergence on a slower clock: add a `%macro` to
    /// your project's `_prelude.st` and the editor keeps completing the old set
    /// until you restart it, while `check` already sees the new one.
    ///
    /// So a prelude edit must invalidate. This clears the latch rather than
    /// rebuilding inline, because the rebuild is `spawn_blocking` work that
    /// belongs on the next request's path, not on the notification handler.
    pub fn invalidate_base_registry(&self) {
        self.base_loaded.store(false, Ordering::Relaxed);
    }

    /// Is `path` a file whose change invalidates the project overlay?
    ///
    /// `_prelude.st` is THE project overlay — the compiler auto-loads it into
    /// the registry before any page compiles — so editing it changes what every
    /// other file means.
    pub fn is_overlay_path(path: &std::path::Path) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n == "_prelude.st")
    }

    /// Add a workspace root
    pub async fn add_root(&self, root: PathBuf) {
        // Mutate the roots list (the old code never did — it only reloaded
        // config, so `scan_workspace` had nothing to walk and the registry was
        // never project-aware).
        self.roots.lock().unwrap().push(root.clone());

        // Reload config from new root
        let config = WorkspaceConfig::load(&root);
        *self.config.write().await = config.clone();

        let mut resolver = self.import_resolver.write().await;
        resolver.update_config(config);
        drop(resolver);

        // Build the base registry (stdlib + this root's `_prelude.st` overlay)
        // and index the workspace's own `.st` files for cross-file features.
        self.ensure_base_registry().await;
        self.scan_workspace().await;
    }

    /// Get the form registry for LSP features (stdlib + project overlays +
    /// any file-local macros indexed into the workspace).
    pub async fn form_registry(&self) -> FormRegistry {
        self.ensure_base_registry().await;
        self.form_registry.read().await.clone()
    }
    /// Index a single file
    pub async fn index_file(&self, path: &Path) -> Result<(), WorkspaceError> {
        self.ensure_base_registry().await;
        // Read file content
        let content = tokio::fs::read_to_string(path).await?;
        let content_hash = hash_content(&content);

        // Check if file is already indexed with same content
        if let Some(existing) = self.files.get(path)
            && existing.content_hash == content_hash
        {
            return Ok(()); // No change
        }

        // Parse the file
        let ast = match crate::parser::parse(&content) {
            Ok(ast) => Some(ast),
            Err(e) => {
                // Store file without AST for tracking
                self.files.insert(
                    path.to_path_buf(),
                    IndexedFile {
                        path: path.to_path_buf(),
                        content_hash,
                        ast: None,
                        imports: vec![],
                        version: 0,
                    },
                );
                return Err(WorkspaceError::Parse(e.to_string()));
            }
        };

        // Resolve imports - collect both the import AST (with span) and resolved path
        let resolver = self.import_resolver.read().await;
        let import_asts: Vec<_> = ast.as_ref().map(|a| a.imports.clone()).unwrap_or_default();

        let imports: Vec<_> = import_asts
            .iter()
            .map(|import_ast| {
                let resolved = resolver.resolve(&import_ast.path, path);
                (import_ast.path.clone(), resolved)
            })
            .collect();
        drop(resolver);

        // Update dependency graph - now with proper span tracking
        {
            let mut graph = self.dependency_graph.write().await;
            graph.upsert_file(path.to_path_buf(), content_hash);

            let edges: Vec<_> = import_asts
                .iter()
                .zip(imports.iter())
                .filter_map(|(import_ast, (_, resolved))| {
                    resolved.as_file().map(|target| {
                        (
                            target.clone(),
                            ImportEdge {
                                import_path: import_ast.path.clone(),
                                span: import_ast.span.into(), // Now using actual import span
                            },
                        )
                    })
                })
                .collect();

            graph.update_imports(path, edges);

            // Check for cycles
            if let Some(cycle) = graph.detect_cycle(path) {
                return Err(WorkspaceError::CircularImport(cycle));
            }
        }

        // Update usage index
        if let Some(ref ast) = ast {
            let file_index = UsageVisitor::visit_file(ast, path);
            self.usage_index.update_file(path, file_index);

            // Update form registry with any macro definitions
            let mut registry = self.form_registry.write().await;
            registry.index_file(ast, path);
        }

        // Store indexed file
        self.files.insert(
            path.to_path_buf(),
            IndexedFile {
                path: path.to_path_buf(),
                content_hash,
                ast,
                imports,
                version: 0,
            },
        );

        Ok(())
    }

    /// Index a file from content (for open documents)
    pub async fn index_content(
        &self,
        path: &Path,
        content: &str,
        version: i32,
    ) -> Result<(), WorkspaceError> {
        self.ensure_base_registry().await;
        let content_hash = hash_content(content);

        // Parse the file
        let ast = match crate::parser::parse(content) {
            Ok(ast) => Some(ast),
            Err(e) => {
                self.files.insert(
                    path.to_path_buf(),
                    IndexedFile {
                        path: path.to_path_buf(),
                        content_hash,
                        ast: None,
                        imports: vec![],
                        version,
                    },
                );
                return Err(WorkspaceError::Parse(e.to_string()));
            }
        };

        // Resolve imports - collect both the import AST (with span) and resolved path
        let resolver = self.import_resolver.read().await;
        let import_asts: Vec<_> = ast.as_ref().map(|a| a.imports.clone()).unwrap_or_default();

        let imports: Vec<_> = import_asts
            .iter()
            .map(|import_ast| {
                let resolved = resolver.resolve(&import_ast.path, path);
                (import_ast.path.clone(), resolved)
            })
            .collect();
        drop(resolver);

        // Update dependency graph - now with proper span tracking
        {
            let mut graph = self.dependency_graph.write().await;
            graph.upsert_file(path.to_path_buf(), content_hash);

            let edges: Vec<_> = import_asts
                .iter()
                .zip(imports.iter())
                .filter_map(|(import_ast, (_, resolved))| {
                    resolved.as_file().map(|target| {
                        (
                            target.clone(),
                            ImportEdge {
                                import_path: import_ast.path.clone(),
                                span: import_ast.span.into(), // Now using actual import span
                            },
                        )
                    })
                })
                .collect();

            graph.update_imports(path, edges);
        }

        // Update usage index
        if let Some(ref ast) = ast {
            let file_index = UsageVisitor::visit_file(ast, path);
            self.usage_index.update_file(path, file_index);

            // Update form registry
            let mut registry = self.form_registry.write().await;
            registry.index_file(ast, path);
        }

        // Store indexed file
        self.files.insert(
            path.to_path_buf(),
            IndexedFile {
                path: path.to_path_buf(),
                content_hash,
                ast,
                imports,
                version,
            },
        );

        Ok(())
    }

    /// Handle file change (re-index and invalidate dependents)
    pub async fn on_file_changed(&self, path: &Path) {
        // Re-index the file
        let _ = self.index_file(path).await;

        // Get dependents and re-validate them
        let dependents = {
            let graph = self.dependency_graph.read().await;
            graph.get_dependents(path)
        };

        for dep in dependents {
            // Re-index each dependent (their imports may have changed validity)
            let _ = self.index_file(&dep).await;
        }
    }

    /// Handle file deletion
    pub async fn on_file_deleted(&self, path: &Path) {
        // Get dependents before removal
        let dependents = {
            let graph = self.dependency_graph.read().await;
            graph.get_dependents(path)
        };

        // Remove from all indices
        self.files.remove(path);
        self.usage_index.remove_file(path);

        {
            let mut graph = self.dependency_graph.write().await;
            graph.remove_file(path);
        }

        // Invalidate cache entries
        {
            let resolver = self.import_resolver.read().await;
            resolver.invalidate_file(path);
        }

        // Re-validate dependents (their imports now fail)
        for dep in dependents {
            let _ = self.index_file(&dep).await;
        }
    }

    /// Scan workspace for all .st files
    pub async fn scan_workspace(&self) {
        let config = self.config.read().await;
        let roots = self.roots.lock().unwrap().clone();

        for root in &roots {
            self.scan_directory(root, &config).await;
        }
    }

    async fn scan_directory(&self, dir: &Path, config: &WorkspaceConfig) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if config.should_exclude(&path) {
                continue;
            }

            if path.is_dir() {
                Box::pin(self.scan_directory(&path, config)).await;
            } else if path.extension().is_some_and(|e| e == "st") {
                let _ = self.index_file(&path).await;
            }
        }
    }

    /// Get definition location for a symbol
    pub fn get_definition(&self, symbol: &SymbolId) -> Option<ReferenceLocation> {
        self.usage_index.get_definition(symbol)
    }

    /// Find all references to a symbol
    pub fn find_references(&self, symbol: &SymbolId) -> Vec<ReferenceLocation> {
        self.usage_index.find_references(symbol)
    }

    /// Find symbol at a position in a file
    pub fn symbol_at(&self, path: &Path, offset: usize) -> Option<SymbolId> {
        self.usage_index.symbol_at(path, offset)
    }

    /// Get resolved imports for a file
    pub fn get_imports(&self, path: &Path) -> Vec<(String, ResolvedImport)> {
        self.files
            .get(path)
            .map(|f| f.imports.clone())
            .unwrap_or_default()
    }

    /// Get files that import a given file
    pub async fn get_dependents(&self, path: &Path) -> HashSet<PathBuf> {
        let graph = self.dependency_graph.read().await;
        graph.get_dependents(path)
    }

    /// Get all files transitively imported by a file
    pub async fn get_all_imports(&self, path: &Path) -> HashSet<PathBuf> {
        let graph = self.dependency_graph.read().await;
        graph.get_all_imports(path)
    }

    /// Check if a file would create a circular import
    pub async fn would_create_cycle(&self, from: &Path, to: &Path) -> bool {
        let graph = self.dependency_graph.read().await;
        graph.would_create_cycle(from, to)
    }

    /// Get indexed file info
    pub fn get_file(&self, path: &Path) -> Option<IndexedFile> {
        self.files.get(path).map(|f| f.clone())
    }

    /// Get all indexed files
    pub fn all_files(&self) -> Vec<PathBuf> {
        self.files.iter().map(|e| e.key().clone()).collect()
    }

    /// Get workspace config
    pub async fn config(&self) -> WorkspaceConfig {
        self.config.read().await.clone()
    }

    /// Get indexing strategy
    pub async fn indexing_strategy(&self) -> IndexingStrategy {
        self.config.read().await.indexing_strategy
    }
}

/// Hash file content for change detection
fn hash_content(content: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_new_workspace() {
        let manager = WorkspaceManager::new(vec![PathBuf::from("/project")]);
        assert!(manager.all_files().is_empty());
    }

    #[tokio::test]
    async fn test_index_content() {
        let manager = WorkspaceManager::new(vec![PathBuf::from("/project")]);

        let content = r#"
.box {
    @scroll {
        opacity: 0 -> 1;
    }
}
"#;

        let result = manager
            .index_content(Path::new("/project/test.st"), content, 1)
            .await;

        // Should succeed (content is valid)
        assert!(result.is_ok());

        // Should have indexed the file
        assert!(manager.get_file(Path::new("/project/test.st")).is_some());
    }

    /// gh-28 FOLLOW-UP: a `_prelude.st` EDIT must reach the editor.
    ///
    /// Serving the compiler's live registry closes the drift only if the cache
    /// can be invalidated. `ensure_base_registry` latches on `base_loaded`, so
    /// without this the registry loaded at startup is served forever: add a
    /// macro to your prelude and `check` sees it while the editor keeps
    /// completing the old set until restart. That is the GH-28 divergence
    /// reintroduced on a slower clock, which is why this is a regression gate
    /// and not a nicety.
    #[tokio::test]
    async fn editing_the_prelude_invalidates_the_served_registry() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        let prelude = root.join("_prelude.st");

        std::fs::write(
            &prelude,
            "%macro project-alpha {\n  %form {\n    @project-alpha(a: $a:string)\n  }\n}\n",
        )
        .unwrap();

        let manager = WorkspaceManager::new(vec![]);
        manager.add_root(root.clone()).await;

        assert!(
            manager.form_registry().await.get_directive("project-alpha").is_some(),
            "the first load must serve the prelude's macro"
        );

        // The author adds a second macro.
        std::fs::write(
            &prelude,
            "%macro project-alpha {\n  %form {\n    @project-alpha(a: $a:string)\n  }\n}\n\
             %macro project-beta {\n  %form {\n    @project-beta(b: $b:string)\n  }\n}\n",
        )
        .unwrap();

        // Without invalidation the registry is latched and still serves the old set.
        manager.invalidate_base_registry();

        let registry = manager.form_registry().await;
        assert!(
            registry.get_directive("project-beta").is_some(),
            "a macro ADDED to _prelude.st must be served after invalidation — \
             otherwise the editor and `check` disagree until restart (GH-28)"
        );
        assert!(
            registry.get_directive("project-alpha").is_some(),
            "the pre-existing macro must survive the rebuild"
        );
    }

    /// The invalidation trigger is a property of the PATH, so both the in-editor
    /// edit path and the watched-files path can share one predicate.
    #[test]
    fn only_the_project_overlay_invalidates() {
        use std::path::Path;
        assert!(WorkspaceManager::is_overlay_path(Path::new("/p/_prelude.st")));
        assert!(WorkspaceManager::is_overlay_path(Path::new("_prelude.st")));
        assert!(!WorkspaceManager::is_overlay_path(Path::new("/p/index.st")));
        assert!(
            !WorkspaceManager::is_overlay_path(Path::new("/p/prelude.st")),
            "only the underscore-prefixed overlay is THE project overlay"
        );
    }

    /// gh-28 ACCEPTANCE: a project macro declared in `_prelude.st` is served by
    /// the live registry — it appears in completion, hover, AND go-to-definition,
    /// exactly because the LSP serves the compiler's own registry (stdlib +
    /// project overlay) rather than a hand-written include_str! snapshot.
    #[tokio::test]
    async fn project_macro_in_prelude_serves_completion_hover_and_definition() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();

        // A project `%macro` declaring the directive `@project-card`.
        let prelude = root.join("_prelude.st");
        std::fs::write(
            &prelude,
            r#"/// A project-defined directive.
%macro project-card {
  %form {
    @project-card(title: $title:string) {
      $body:properties
    }
  }
}
"#,
        )
        .unwrap();

        let manager = WorkspaceManager::new(vec![]);
        manager.add_root(root.clone()).await;

        let registry = manager.form_registry().await;

        // Completion: the macro is a known directive after `@`.
        let sig = registry
            .get_directive("project-card")
            .expect("project macro must be in the live registry");
        assert_eq!(sig.source_file, prelude, "go-to-definition must point at the real prelude file");

        // Hover + definition need a document using the directive.
        let doc = crate::lsp::document::DocumentState::new(
            "@project-card(title: \"Hi\")".to_string(),
            1,
        );

        // Completion after `@project-` suggests the directive.
        let items = crate::lsp::completion::provide_completions(
            &doc,
            tower_lsp::lsp_types::Position { line: 0, character: 1 },
            &registry,
        );
        assert!(
            items.iter().any(|i| i.label == "project-card"),
            "completion must include the project macro, got {:?}",
            items.iter().map(|i| &i.label).collect::<Vec<_>>()
        );

        // Hover on the directive name returns the macro's doc.
        let hover = crate::lsp::hover::provide_hover(
            &doc,
            tower_lsp::lsp_types::Position { line: 0, character: 5 },
            &registry,
        );
        assert!(
            hover.is_some(),
            "hover must resolve the project macro"
        );

        // Go-to-definition on the directive name points at the prelude file.
        let doc_uri = tower_lsp::lsp_types::Url::from_file_path(&prelude).unwrap();
        let def = crate::lsp::definition::provide_definition(
            &doc,
            tower_lsp::lsp_types::Position { line: 0, character: 5 },
            &registry,
            &manager,
            &doc_uri,
        );
        let loc = match def {
            Some(tower_lsp::lsp_types::GotoDefinitionResponse::Scalar(loc)) => loc,
            other => panic!("expected a scalar definition location, got {other:?}"),
        };
        assert!(
            loc.uri.to_file_path().unwrap() == prelude,
            "go-to-definition must open the prelude file, got {}",
            loc.uri
        );
    }

    /// gh-30/31 ACCEPTANCE: references to `$items` inside `@each` return the
    /// EXACT capture occurrences (not the whole `@each` block), with NO
    /// duplicates, and `includeDeclaration` actually differing.
    #[tokio::test]
    async fn each_items_references_are_exact_no_dupes_and_declaration_aware() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        let manager = WorkspaceManager::new(vec![]);
        manager.add_root(root.clone()).await;

        let file = root.join("list.st");
        // Valid `@each` grammar: the `%form` declares `template_invocation+`
        // (a `&template(...)` splice) in the body, so a bare-CSS body would not
        // match — which is correct. This shape exercises the capture-span path.
        let content = "@data inline $items : [];\n.rows {\n    @each($items as $item) {\n        &row-item($item);\n    }\n}\n";
        manager
            .index_content(&file, content, 1)
            .await
            .expect("index succeeds");

        // `update_file` canonicalizes the path (gh-30); the interval index is
        // keyed by the canonical path, so look up through it.
        std::fs::write(&file, content).unwrap();
        let canonical = std::fs::canonicalize(&file).unwrap_or_else(|_| file.clone());

        // Position on the `$items` token inside `@each($items as $item)`.
        let offset = content.find("$items as $item").expect("source shape");
        let symbol = manager
            .symbol_at(&canonical, offset)
            .expect("symbol at cursor");
        assert_eq!(symbol.name, "items", "cursor on $items resolves to the items binding");

        // With includeDeclaration=true: definition + usages.
        let with_decl = manager.find_references(&symbol);
        // With includeDeclaration=false: usages only (no Definition kind).
        let decl_kind = crate::lsp::workspace::ReferenceKind::Definition;
        let no_decl: Vec<_> = with_decl
            .iter()
            .filter(|r| r.kind != decl_kind)
            .collect();

        assert!(!with_decl.is_empty(), "references must not be empty");
        assert!(
            with_decl.len() > no_decl.len(),
            "includeDeclaration=true must add the definition; {} vs {}",
            with_decl.len(),
            no_decl.len()
        );

        // No duplicates: every (file, span) appears once.
        let mut seen = std::collections::HashSet::new();
        for r in &with_decl {
            let key = (r.file.clone(), r.span.start, r.span.end);
            assert!(
                seen.insert(key),
                "duplicate reference at {}:{}-{}",
                r.file.display(),
                r.span.start,
                r.span.end
            );
        }

        // gh-31: the `$items` reference span is the CAPTURE (`$items` token),
        // not the whole `@each` block. Assert it is short (token-length).
        let source_capture_len = "$items".len();
        for r in with_decl.iter().filter(|r| r.kind != decl_kind) {
            assert!(
                r.span.end - r.span.start <= source_capture_len + 2,
                "reference span must be the exact $items capture ({}:{}-{}), not the whole block",
                r.file.display(),
                r.span.start,
                r.span.end
            );
        }
    }

}
