//! Error strategy for the event-based parser.

/// Controls how the parser handles errors.
///
/// Set at parser construction time, immutable during parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorStrategy {
    /// First error stops parsing immediately. Used for CLI `cargo run -- check`.
    Fatal,
    /// Accumulate all errors, continue parsing. Used for LSP and batch compilation.
    Verbose,
    /// Discard errors silently, return None on failure. Used for speculative
    /// form matching (try a pattern, backtrack on failure).
    Silent,
}
