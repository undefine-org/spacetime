//! Grammar functions for the event-based parser.
//!
//! These functions define the structural syntax of Spacetime — they parse
//! directives as `@name args (params) { body }` without knowing specific
//! directive families. Semantic interpretation is done by MatchSink + FormCompiler
//! via FormClause patterns from the SyntaxRegistry.
//!
//! # Architecture
//!
//! Grammar functions receive a `&mut Parser` and use its API (bump, expect, at,
//! start/complete) to emit events. The parser has no knowledge of %form patterns.

pub mod directives;
pub mod expressions;
pub mod meta;
pub mod properties;
pub mod selectors;

use super::parser::Parser;
use crate::syntax::cst::SyntaxKind;

/// Parse the root of a Spacetime source file.
///
/// The `source` parameter is needed for meta-clause keyword dispatch
/// (e.g., `%form` vs `%emit` need different body parsing strategies).
///
/// ```text
/// root = statement*
/// ```
pub fn root(p: &mut Parser, source: &str) {
    let m = p.start();

    while !p.at_end() {
        if p.at(SyntaxKind::AT_SIGN) {
            // PLAN-039: thread the real source (not "") so a directive body's HTML regions
            // can be skipped via scan_html_end and nested constructs surface structurally.
            directives::directive_with_source(p, source);
        } else if p.at(SyntaxKind::DOLLAR) {
            directives::variable_ref(p);
        } else if p.at(SyntaxKind::AMPERSAND) {
            directives::element_ref(p);
        } else if p.at(SyntaxKind::PERCENT) {
            meta::meta_def(p, source);
        } else if p.at(SyntaxKind::LT) {
            // Top-level HTML element literal (PLAN-023 W1). The events/match pass has no
            // HTML grammar; without this it would bump token-by-token and treat `$`-refs /
            // hole inners INSIDE the markup as variable_ref matches, desyncing offsets and
            // dropping later FormMatches (BUG-043). Skip the whole HTML region opaquely
            // (its reactive holes are lowered separately by the W1 treesink).
            let start = p.current_offset() as usize;
            let end = crate::syntax::cst::scan_html_end(source, start);
            // Advance past every token that began inside the region.
            while !p.at_end() && (p.current_offset() as usize) < end {
                p.bump_any();
            }
            // Safety: guarantee progress even if the scan did not advance.
            if (p.current_offset() as usize) <= start && !p.at_end() {
                p.bump_any();
            }
        } else if p.at(SyntaxKind::DOT)
            || p.at(SyntaxKind::HASH)
            || p.at(SyntaxKind::IDENT)
            || p.at(SyntaxKind::L_BRACKET)
        {
            // Potential CSS selector or scope block. L_BRACKET covers attribute
            // selectors (`[data-st-instance] { ... }`) — without it the opener is
            // bumped token-by-token and the body's directives never become a
            // SCOPE_BLOCK, so selector-scope macros inside (`@mcp-host`) are
            // mis-read as file-scope and dropped (FEAT-127).
            directives::scope_or_property(p);
        } else {
            // Skip unexpected tokens
            p.bump_any();
        }
    }

    m.complete_p(p, SyntaxKind::ROOT);
}
