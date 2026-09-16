//! Workspace-aware LSP infrastructure
//!
//! This module provides project-wide indexing, import resolution, and cross-file
//! navigation for Spacetime files.
//!
//! ## Components
//!
//! - [`WorkspaceConfig`]: Configuration from `.local/spacetime.yaml`
//! - [`ImportResolver`]: Resolves `@import` paths to file locations
//! - [`DependencyGraph`]: Tracks import relationships and detects cycles
//! - [`UsageIndex`]: Bidirectional symbol-to-location mapping
//! - [`WorkspaceManager`]: Central coordinator for all workspace features

mod config;
mod dependency_graph;
mod import_resolver;
mod manager;
mod usage_index;
mod usage_visitor;

pub use config::{IndexingStrategy, WorkspaceConfig};
pub use dependency_graph::DependencyGraph;
pub use import_resolver::{ImportResolver, ResolvedImport};
pub use manager::WorkspaceManager;
pub use usage_index::{ReferenceKind, ReferenceLocation, SymbolId, SymbolKind, UsageIndex};
pub use usage_visitor::UsageVisitor;
