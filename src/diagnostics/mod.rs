//! Diagnostics module for Spacetime compiler errors and warnings.
//!
//! Provides source span tracking, error accumulation, and pretty-printed error messages.

mod codes;
mod collector;
mod span;
pub mod suggestions;

#[cfg(test)]
mod tests;

pub use codes::DiagnosticCode;
pub use collector::{
    Diagnostic, DiagnosticCollector, Severity, find_similar, find_similar_multiple,
    format_suggestion, levenshtein_distance,
};
pub use span::SourceSpan;
