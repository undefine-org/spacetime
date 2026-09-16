//! Selector parsing for the event-based parser.
//!
//! Handles CSS-style selectors: `.class`, `#id`, `[attr]`, `:pseudo`,
//! universal `*`, element types, combinators, and compound selectors.

use crate::syntax::cst::SyntaxKind;
use crate::syntax::events::parser::Parser;

/// Parse a CSS selector.
///
/// ```text
/// selector = selector_part (combinator selector_part)*
/// combinator = ',' | '>' | '+' | '~' | WHITESPACE
/// ```
pub fn selector(p: &mut Parser) {
    let m = p.start();

    selector_part(p);

    // Handle combinators and additional selector parts
    loop {
        if p.at(SyntaxKind::COMMA) {
            p.bump_any();
            selector_part(p);
        } else if p.at(SyntaxKind::GT) || p.at(SyntaxKind::PLUS) || p.at(SyntaxKind::TILDE) {
            p.bump_any();
            selector_part(p);
        } else if p.at(SyntaxKind::DOT)
            || p.at(SyntaxKind::HASH)
            || p.at(SyntaxKind::L_BRACKET)
            || p.at(SyntaxKind::COLON)
        {
            // Compound selector continuation (no combinator — e.g. .class#id[attr]:hover)
            // Note: IDENT/STAR/AMPERSAND are NOT included here because this parser is also
            // used for directive inline args where IDENT after a selector is an argument,
            // not a descendant combinator. The CST parser handles descendant combinators
            // in its own parse_selector() method.
            selector_part(p);
        } else {
            break;
        }
    }

    m.complete_p(p, SyntaxKind::SELECTOR);
}

/// Parse a single selector part.
///
/// ```text
/// selector_part = class_sel | id_sel | attr_sel | pseudo_sel | universal | type_sel | parent_ref
/// ```
fn selector_part(p: &mut Parser) {
    let m = p.start();

    match p.current() {
        SyntaxKind::DOT => {
            // Class selector: .class-name
            p.bump_any();
            if p.at(SyntaxKind::IDENT) || p.current().is_keyword() {
                p.bump_any();
            }
        }
        SyntaxKind::HASH => {
            // ID selector: #id-name (HASH already includes the #)
            p.bump_any();
            if p.at(SyntaxKind::IDENT) {
                p.bump_any();
            }
        }
        SyntaxKind::L_BRACKET => {
            // Attribute selector: [attr], [attr=val], etc.
            p.bump_any();
            let mut depth = 1i32;
            while !p.at_end() && depth > 0 {
                if p.at(SyntaxKind::L_BRACKET) {
                    depth += 1;
                } else if p.at(SyntaxKind::R_BRACKET) {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                p.bump_any();
            }
            if p.at(SyntaxKind::R_BRACKET) {
                p.bump_any();
            }
        }
        SyntaxKind::COLON => {
            // Pseudo-class or pseudo-element
            p.bump_any(); // first colon
            if p.at(SyntaxKind::COLON) {
                p.bump_any(); // second colon for ::pseudo-element
            }
            if p.at(SyntaxKind::IDENT) || p.current().is_keyword() {
                p.bump_any();
            }
            // Optional functional args: :nth-child(2n+1)
            if p.at(SyntaxKind::L_PAREN) {
                p.bump_any();
                let mut depth = 1i32;
                while !p.at_end() && depth > 0 {
                    if p.at(SyntaxKind::L_PAREN) {
                        depth += 1;
                    } else if p.at(SyntaxKind::R_PAREN) {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    p.bump_any();
                }
                if p.at(SyntaxKind::R_PAREN) {
                    p.bump_any();
                }
            }
        }
        SyntaxKind::STAR => {
            // Universal selector: *
            p.bump_any();
        }
        SyntaxKind::AMPERSAND => {
            // Parent reference: &
            p.bump_any();
        }
        _ => {
            // Element type selector: div, body, span, etc.
            if p.at(SyntaxKind::IDENT) || p.current().is_keyword() {
                p.bump_any();
            }
        }
    }

    m.complete_p(p, SyntaxKind::SELECTOR_PART);
}

/// Parse a CSS selector in scope-block context (allows descendant IDENT combinators).
///
/// Unlike `selector()`, this variant includes IDENT/STAR/AMPERSAND as descendant
/// combinator continuations. This is correct for scope blocks (`.foo a:hover { }`)
/// but NOT for directive inline args where IDENT after a selector is an argument.
pub fn scope_selector(p: &mut Parser) {
    let m = p.start();

    selector_part(p);

    loop {
        if p.at(SyntaxKind::COMMA) {
            p.bump_any();
            selector_part(p);
        } else if p.at(SyntaxKind::GT) || p.at(SyntaxKind::PLUS) || p.at(SyntaxKind::TILDE) {
            p.bump_any();
            selector_part(p);
        } else if p.at(SyntaxKind::DOT)
            || p.at(SyntaxKind::HASH)
            || p.at(SyntaxKind::L_BRACKET)
            || p.at(SyntaxKind::COLON)
        {
            // Compound selector continuation: .class#id[attr]:hover
            selector_part(p);
        } else if p.at(SyntaxKind::IDENT) || p.at(SyntaxKind::STAR) || p.at(SyntaxKind::AMPERSAND) {
            // Descendant combinator: .foo a, .parent * { }, &:hover { }
            selector_part(p);
        } else {
            break;
        }
    }

    m.complete_p(p, SyntaxKind::SELECTOR);
}
