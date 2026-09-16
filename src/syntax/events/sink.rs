//! Sink trait — the consumer interface for parser events.
//!
//! After `process()` resolves forward_parent chains and inserts trivia,
//! it calls methods on a `Sink` in correct nested tree order.

use crate::syntax::cst::SyntaxKind;

/// Consumer of resolved parser events. Implementations produce different
/// outputs from the same parse: CST (TreeSink), FormMatch (MatchSink),
/// diagnostics (DiagnosticSink).
pub trait Sink {
    /// Begin a new child node of the given kind.
    fn start_node(&mut self, kind: SyntaxKind);

    /// Emit a token with its text.
    fn token(&mut self, kind: SyntaxKind, text: &str);

    /// Finish the current node (matching the most recent unfinished `start_node`).
    fn finish_node(&mut self);

    /// Record a parse error at the given byte offset.
    fn error(&mut self, msg: String, offset: usize);
}
