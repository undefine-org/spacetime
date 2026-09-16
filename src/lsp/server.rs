//! Spacetime LSP Server Implementation
//!
//! Main server using tower-lsp for the Language Server Protocol.

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use super::colors::{provide_color_presentations, provide_document_colors};
use super::completion::provide_completions;
use super::definition::provide_definition;
use super::diagnostics::validate_document;
use super::document::DocumentStore;
use super::hover::provide_hover;
use super::references::find_references_at;
use super::semantic_tokens::{provide_semantic_tokens, semantic_tokens_capability};
use super::workspace::WorkspaceManager;

/// Spacetime Language Server.
///
/// Provides LSP features for `.st` files:
/// - Completions for `@` directives
/// - Hover documentation
/// - Go-to-definition for macros
/// - Find all references
/// - Diagnostics on open/change
pub struct SpacetimeLsp {
    /// LSP client for sending notifications
    client: Client,
    /// Store of open documents
    documents: DocumentStore,
    /// Workspace manager for cross-file features. Owns THE live FormRegistry
    /// (compiler stdlib + project overlays) that every handler serves from.
    workspace: Arc<WorkspaceManager>,
    /// Workspace roots
    workspace_roots: RwLock<Vec<PathBuf>>,
    /// Latest requested diagnostics generation, per document.
    ///
    /// Publishing diagnostics runs the FULL compiler chain — that is the point
    /// (one diagnostic engine, two front-ends, gh-32). It costs ~45ms, which is
    /// fine once and ruinous per keystroke: five rapid edits queued five full
    /// compiles and blocked every other request behind them.
    ///
    /// Each edit bumps a generation counter, waits out a short quiet period, and
    /// then checks whether it is still newest. A superseded run returns without
    /// compiling, so typing coalesces to ONE compile at the pause — which is when
    /// the author actually wants the answer.
    diag_generation: Arc<dashmap::DashMap<Url, u64>>,
}

impl SpacetimeLsp {
    /// Create a new Spacetime LSP server.
    pub fn new(client: Client) -> Self {
        // No roots yet — `initialize` supplies them via `add_root`, which
        // builds the live registry (stdlib + project overlays) off the reactor.
        let workspace = WorkspaceManager::new(vec![]);

        Self {
            client,
            documents: DocumentStore::new(),
            workspace: Arc::new(workspace),
            workspace_roots: RwLock::new(vec![]),
            diag_generation: Arc::new(dashmap::DashMap::new()),
        }
    }

    /// Publish diagnostics for a document.
    /// Debounced entry point: coalesce a burst of edits into one compile.
    ///
    /// The delay is short enough that diagnostics feel immediate on pause, long
    /// enough that a fast typist does not pay for every character.
    async fn publish_diagnostics_debounced(&self, uri: &Url) {
        const QUIET_MS: u64 = 150;

        let generation = {
            let mut entry = self.diag_generation.entry(uri.clone()).or_insert(0);
            *entry += 1;
            *entry
        };

        tokio::time::sleep(std::time::Duration::from_millis(QUIET_MS)).await;

        // Superseded while we waited — the newer run will publish.
        if self
            .diag_generation
            .get(uri)
            .is_some_and(|g| *g != generation)
        {
            return;
        }

        self.publish_diagnostics_for(uri).await;
    }

    async fn publish_diagnostics_for(&self, uri: &Url) {
        let diagnostics = if let Some(doc) = self.documents.get(uri) {
            // Site root drives the compiler's `_prelude.st` overlay so a
            // project macro resolves in the editor exactly when it does in
            // `spacetime check` (gh-32 / gh-28).
            let site_dir = self.workspace_roots.read().await.first().cloned();
            validate_document(&doc, site_dir.as_deref())
        } else {
            vec![]
        };

        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;
    }
}
#[tower_lsp::async_trait]
impl LanguageServer for SpacetimeLsp {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        // Extract workspace roots
        let mut roots = Vec::new();
        if let Some(folders) = params.workspace_folders {
            for folder in folders {
                if let Ok(path) = folder.uri.to_file_path() {
                    roots.push(path);
                }
            }
        } else if let Some(root_uri) = params.root_uri
            && let Ok(path) = root_uri.to_file_path()
        {
            roots.push(path);
        }

        // Store workspace roots
        *self.workspace_roots.write().await = roots.clone();

        // Initialize workspace with roots
        for root in roots {
            self.workspace.add_root(root).await;
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                // Full document sync - client sends entire content on each change
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),

                // Completion with @ and ( triggers
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![
                        "@".to_string(),
                        "(".to_string(),
                        ":".to_string(),
                        ",".to_string(),
                        "~".to_string(),
                        "$".to_string(),
                        "&".to_string(),
                    ]),
                    resolve_provider: Some(false),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                    all_commit_characters: None,
                    completion_item: None,
                }),

                // Hover for documentation
                hover_provider: Some(HoverProviderCapability::Simple(true)),

                // Go-to-definition for macros and directives
                definition_provider: Some(OneOf::Left(true)),

                // Semantic tokens for syntax highlighting
                semantic_tokens_provider: Some(semantic_tokens_capability()),

                // Color picker for hex color literals
                color_provider: Some(ColorProviderCapability::Simple(true)),

                // Find all references
                references_provider: Some(OneOf::Left(true)),

                // Workspace folder support
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    file_operations: None,
                }),

                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "spacetime-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Spacetime LSP initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let content = params.text_document.text.clone();
        let version = params.text_document.version;

        self.documents
            .open(uri.clone(), params.text_document.text, version);

        // Index in workspace for cross-file features
        if let Ok(path) = uri.to_file_path() {
            let _ = self.workspace.index_content(&path, &content, version).await;
        }

        // Validate and publish diagnostics
        self.publish_diagnostics_for(&uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;

        // With TextDocumentSyncKind::FULL, we get the entire content
        if let Some(change) = params.content_changes.into_iter().next() {
            let content = change.text.clone();
            self.documents.change(&uri, change.text, version);

            // Re-index in workspace
            if let Ok(path) = uri.to_file_path() {
                // Editing `_prelude.st` changes what every OTHER file means: it
                // is the project overlay the compiler loads into the registry
                // before any page compiles. Without invalidating, the editor
                // keeps serving the macro set it loaded at startup while
                // `check` already sees the new one — the GH-28 divergence
                // reintroduced on a slower clock.
                if WorkspaceManager::is_overlay_path(&path) {
                    self.workspace.invalidate_base_registry();
                }
                let _ = self.workspace.index_content(&path, &content, version).await;
            }
        }

        // Re-validate and publish. Debounced: this is the ONLY handler that
        // fires per keystroke, so it is the only one that must coalesce.
        self.publish_diagnostics_debounced(&uri).await;
    }

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        // A `_prelude.st` edited OUTSIDE the editor (git checkout, another tool,
        // a sibling window) must invalidate the overlay too — otherwise the
        // registry is stale until restart, which is exactly the drift GH-28
        // deleted the static registry to prevent.
        for change in &params.changes {
            if let Ok(path) = change.uri.to_file_path()
                && WorkspaceManager::is_overlay_path(&path)
            {
                self.workspace.invalidate_base_registry();
                break;
            }
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents.close(&uri);

        // Clear diagnostics for closed document
        self.client.publish_diagnostics(uri, vec![], None).await;
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let Some(doc) = self.documents.get(uri) else {
            return Ok(None);
        };

        let items = {
            let registry = self.workspace.form_registry().await;
            provide_completions(&doc, position, &registry)
        };

        if items.is_empty() {
            Ok(None)
        } else {
            Ok(Some(CompletionResponse::Array(items)))
        }
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let Some(doc) = self.documents.get(uri) else {
            return Ok(None);
        };

        Ok(provide_hover(&doc, position, &self.workspace.form_registry().await))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let Some(doc) = self.documents.get(uri) else {
            return Ok(None);
        };

        Ok(provide_definition(
            &doc,
            position,
            &self.workspace.form_registry().await,
            &self.workspace,
            uri,
        ))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = &params.text_document.uri;

        let Some(doc) = self.documents.get(uri) else {
            return Ok(None);
        };

        Ok(provide_semantic_tokens(&doc))
    }

    async fn document_color(&self, params: DocumentColorParams) -> Result<Vec<ColorInformation>> {
        let uri = &params.text_document.uri;

        let Some(doc) = self.documents.get(uri) else {
            return Ok(vec![]);
        };

        Ok(provide_document_colors(&doc))
    }

    async fn color_presentation(
        &self,
        params: ColorPresentationParams,
    ) -> Result<Vec<ColorPresentation>> {
        Ok(provide_color_presentations(&params))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let include_declaration = params.context.include_declaration;

        let Some(doc) = self.documents.get(uri) else {
            return Ok(None);
        };

        // Get file path for workspace lookup
        let Ok(path) = uri.to_file_path() else {
            return Ok(None);
        };

        // Get byte offset from position
        let offset = doc.position_mapper.offset_from_position(position);

        // Find references using workspace
        let locations =
            find_references_at(&self.workspace, &path, offset, include_declaration).await;

        if locations.is_empty() {
            Ok(None)
        } else {
            Ok(Some(locations))
        }
    }
}

/// Run the LSP server on stdin/stdout.
pub async fn run_lsp() {
    use tower_lsp::{LspService, Server};

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(SpacetimeLsp::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lsp::form_registry::{DirectiveSignature, FormRegistry};
    use crate::parser::meta_ast::CaptureModifier;
    use std::path::PathBuf;

    fn create_test_registry() -> FormRegistry {
        let mut registry = FormRegistry::new();

        // Add test directives
        registry.register(DirectiveSignature {
            name: "scroll".to_string(),
            macro_name: "ScrollMacro".to_string(),
            params: vec![crate::lsp::form_registry::DirectiveParam {
                name: "duration".to_string(),
                capture_var: "dur".to_string(),
                capture_type: crate::parser::meta_ast::CaptureType::Duration,
                modifier: CaptureModifier::Optional,
                default_value: Some("0.3s".to_string()),
            }],
            body_type: Some(crate::parser::meta_ast::CaptureType::Properties),
            definition_span: crate::parser::SourceSpan::default(),
            source_file: PathBuf::from("/stdlib/scroll.st"),
            documentation: Some("Animate properties on scroll.".to_string()),
            exports: vec![],
            bound_primitives: vec![],
            form: None,
        });

        registry.register(DirectiveSignature {
            name: "fade-in".to_string(),
            macro_name: "FadeInMacro".to_string(),
            params: vec![],
            body_type: None,
            definition_span: crate::parser::SourceSpan::default(),
            source_file: PathBuf::from("/stdlib/fade.st"),
            documentation: None,
            exports: vec![],
            bound_primitives: vec![],
            form: None,
        });

        registry
    }

    #[test]
    fn test_form_registry_completions() {
        let registry = create_test_registry();

        let completions = registry.directive_completions("scr");
        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].0, "scroll");

        let completions = registry.directive_completions("");
        assert_eq!(completions.len(), 2);
    }

    #[test]
    fn test_form_registry_get_directive() {
        let registry = create_test_registry();

        let sig = registry.get_directive("scroll");
        assert!(sig.is_some());
        assert_eq!(sig.unwrap().params.len(), 1);

        let sig = registry.get_directive("unknown");
        assert!(sig.is_none());
    }
}
