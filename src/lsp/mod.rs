//! Spacetime LSP Support
//!
//! This module provides language server protocol features for Spacetime:
//! - Completions for @ directives
//! - Hover documentation
//! - Go-to-definition for macros
//! - Validation and diagnostics
//!
//! The core abstraction is the `FormRegistry`, which indexes all `%form`
//! declarations from macros to power these features.
//!
//! ## Architecture
//!
//! - `FormRegistry`: Indexes directive signatures from stdlib macros
//! - `DocumentStore`: Manages open documents with AST and position info
//! - `PositionMapper`: Converts between byte offsets and LSP positions (UTF-16)
//! - `SpacetimeLsp`: Main LSP server implementing tower-lsp Backend
//!
//! ## Feature Modules
//!
//! - `completion`: Provides intelligent completions for directives and parameters
//! - `hover`: Shows documentation on hover for directives and parameters
//! - `definition`: Navigate to macro definitions from directive usage
//! - `diagnostics`: Real-time validation and error reporting

pub mod form_registry;

// Lightweight context extraction (used by both LSP and WASM)
#[cfg(any(feature = "lsp", feature = "wasm"))]
pub mod completion;
#[cfg(any(feature = "lsp", feature = "wasm"))]
pub mod hover;

#[cfg(feature = "lsp")]
pub mod colors;
#[cfg(feature = "lsp")]
pub mod definition;
#[cfg(feature = "lsp")]
pub mod diagnostics;
#[cfg(feature = "lsp")]
pub mod document;
#[cfg(feature = "lsp")]
pub mod position;
#[cfg(feature = "lsp")]
pub mod references;
#[cfg(feature = "lsp")]
pub mod semantic_tokens;
#[cfg(feature = "lsp")]
pub mod server;
#[cfg(feature = "lsp")]
pub mod workspace;

pub use form_registry::{
    DirectiveParam, DirectiveSignature, ExportedSignal, FormRegistry, PrimitiveInfo,
    PrimitiveParamInfo,
};

#[cfg(any(feature = "lsp", feature = "wasm"))]
pub use completion::{CompletionContext, extract_context_at_position};
#[cfg(any(feature = "lsp", feature = "wasm"))]
pub use hover::{HoverContext, extract_hover_context};

#[cfg(feature = "lsp")]
pub use colors::{provide_color_presentations, provide_document_colors};
#[cfg(feature = "lsp")]
pub use completion::provide_completions;
#[cfg(feature = "lsp")]
pub use definition::{DefinitionContext, extract_definition_context, provide_definition};
#[cfg(feature = "lsp")]
pub use diagnostics::validate_document;
#[cfg(feature = "lsp")]
pub use document::{DocumentState, DocumentStore};
#[cfg(feature = "lsp")]
pub use hover::provide_hover;
#[cfg(feature = "lsp")]
pub use position::PositionMapper;
#[cfg(feature = "lsp")]
pub use references::provide_references;
#[cfg(feature = "lsp")]
pub use semantic_tokens::{provide_semantic_tokens, semantic_tokens_capability};
#[cfg(feature = "lsp")]
pub use server::{SpacetimeLsp, run_lsp};
#[cfg(feature = "lsp")]
pub use workspace::{WorkspaceConfig, WorkspaceManager};
