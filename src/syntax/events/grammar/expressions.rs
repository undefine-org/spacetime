//! Expression parsing for the event-based parser.
//!
//! Handles parenthesized expressions, array expressions, and
//! inline expression parsing within CSS values and directive arguments.

use crate::syntax::cst::SyntaxKind;
use crate::syntax::events::parser::Parser;

/// Parse a parenthesized group: `(...)`.
///
/// Used within CSS values for function call arguments and grouped expressions.
pub fn paren_group(p: &mut Parser) {
    let m = p.start();

    p.expect(SyntaxKind::L_PAREN);

    let mut depth = 1i32;
    while !p.at_end() && depth > 0 {
        if p.at(SyntaxKind::L_PAREN) {
            depth += 1;
            p.bump_any();
        } else if p.at(SyntaxKind::R_PAREN) {
            depth -= 1;
            if depth > 0 {
                p.bump_any();
            }
        } else {
            p.bump_any();
        }
    }

    p.expect(SyntaxKind::R_PAREN);

    m.complete_p(p, SyntaxKind::PAREN_EXPR);
}

/// Parse an array expression: `[a, b, c]`.
pub fn array_expr(p: &mut Parser) {
    let m = p.start();

    p.expect(SyntaxKind::L_BRACKET);

    let mut depth = 1i32;
    while !p.at_end() && depth > 0 {
        if p.at(SyntaxKind::L_BRACKET) {
            depth += 1;
            p.bump_any();
        } else if p.at(SyntaxKind::R_BRACKET) {
            depth -= 1;
            if depth > 0 {
                p.bump_any();
            }
        } else {
            p.bump_any();
        }
    }

    p.expect(SyntaxKind::R_BRACKET);

    m.complete_p(p, SyntaxKind::ARRAY_EXPR);
}
