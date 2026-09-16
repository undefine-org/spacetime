//! CSS property and value parsing for the event-based parser.
//!
//! Handles CSS property declarations (`name: value;`), CSS values
//! (including function calls, transitions, keyframe blocks), and
//! property/statement disambiguation.

use crate::syntax::cst::SyntaxKind;
use crate::syntax::events::parser::Parser;

/// Parse a CSS property: `name: value;`
///
/// ```text
/// css_property = IDENT '?'? ':' css_value ';'?
/// ```
/// Parse a reactive class-toggle property: `.active: $v;` (BUG-068). Identical to
/// `css_property` except the leading `.` is consumed INTO the CSS_PROPERTY node so the
/// name (`CssProperty::name_text`) reconstructs as `.active` — the class-toggle marker the
/// emitter routes on. The dot must be present (caller checked `is_class_property`).
pub fn class_property(p: &mut Parser) {
    let m = p.start();
    p.expect(SyntaxKind::DOT);
    // Property name (the class, sans dot)
    if p.at(SyntaxKind::IDENT) || p.current().is_keyword() {
        p.bump_any();
    }
    p.expect(SyntaxKind::COLON);
    css_value(p);
    p.eat(SyntaxKind::SEMICOLON);
    m.complete_p(p, SyntaxKind::CSS_PROPERTY);
}
pub fn css_property(p: &mut Parser) {
    let m = p.start();

    // Property name
    if p.at(SyntaxKind::IDENT) || p.current().is_keyword() {
        p.bump_any();
    }

    // Optional question mark (for optional fields)
    p.eat(SyntaxKind::QUESTION);

    // Colon separator
    p.expect(SyntaxKind::COLON);

    // Property value
    css_value(p);

    // Optional semicolon
    p.eat(SyntaxKind::SEMICOLON);

    m.complete_p(p, SyntaxKind::CSS_PROPERTY);
}

/// Parse a CSS value — everything after the colon until semicolon or closing brace.
///
/// ```text
/// css_value = (value_part | transition_arrow | keyframe_block)*
/// ```
pub fn css_value(p: &mut Parser) {
    let m = p.start();

    while !p.at_end()
        && !p.at(SyntaxKind::SEMICOLON)
        && !p.at(SyntaxKind::R_BRACE)
        && !p.at(SyntaxKind::R_PAREN)
        && !p.at(SyntaxKind::R_BRACKET)
    {
        css_value_part(p);
    }

    m.complete_p(p, SyntaxKind::CSS_VALUE);
}

/// Parse a single component of a CSS value.
fn css_value_part(p: &mut Parser) {
    match p.current() {
        // Variable reference in value: $var
        SyntaxKind::DOLLAR => {
            super::directives::variable_ref_inline(p);
        }
        // Element reference in value: &elem
        SyntaxKind::AMPERSAND => {
            super::directives::element_ref_inline(p);
        }
        // Parenthesized expression or function call args
        SyntaxKind::L_PAREN => {
            super::expressions::paren_group(p);
        }
        // Function call: ident(...)
        SyntaxKind::IDENT
        | SyntaxKind::NUMBER
        | SyntaxKind::NUMBER_WITH_UNIT
        | SyntaxKind::COLOR
        | SyntaxKind::STRING => {
            p.bump_any();
            // Check for function call: ident(...)
            if p.at(SyntaxKind::L_PAREN) {
                super::directives::arg_list(p);
            }
        }
        // Keywords allowed in values
        kind if kind.is_keyword() => {
            p.bump_any();
        }
        // Transition arrow: ->
        SyntaxKind::ARROW => {
            p.bump_any();
        }
        // Operators and misc tokens allowed in CSS values
        SyntaxKind::PLUS
        | SyntaxKind::MINUS
        | SyntaxKind::STAR
        | SyntaxKind::SLASH
        | SyntaxKind::DOT
        | SyntaxKind::COMMA
        | SyntaxKind::PERCENT
        | SyntaxKind::EXCLAIM
        | SyntaxKind::PIPE
        | SyntaxKind::QUESTION
        | SyntaxKind::CARET => {
            p.bump_any();
        }
        // Comparison operators
        SyntaxKind::EQUALS
        | SyntaxKind::LT
        | SyntaxKind::GT
        | SyntaxKind::EQ_EQ
        | SyntaxKind::EQ_EQ_EQ
        | SyntaxKind::NOT_EQ
        | SyntaxKind::NOT_EQ_EQ
        | SyntaxKind::LT_EQ
        | SyntaxKind::GT_EQ
        | SyntaxKind::AND_AND
        | SyntaxKind::OR_OR
        | SyntaxKind::QUESTION_QUESTION => {
            p.bump_any();
        }
        // Keyframe block inside value: { 0%: ...; 50%: ...; 100%: ...; }
        SyntaxKind::L_BRACE => {
            keyframe_block(p);
        }
        // Stop at boundaries — don't consume
        SyntaxKind::SEMICOLON
        | SyntaxKind::R_BRACE
        | SyntaxKind::R_PAREN
        | SyntaxKind::R_BRACKET
        | SyntaxKind::EOF => {}
        // Unknown token in value — consume to make progress
        _ => {
            p.bump_any();
        }
    }
}

/// Parse a keyframe block: `{ 0%: val; 50%: val; 100%: val; }`
fn keyframe_block(p: &mut Parser) {
    let m = p.start();

    p.expect(SyntaxKind::L_BRACE);

    while !p.at_end() && !p.at(SyntaxKind::R_BRACE) {
        keyframe(p);
    }

    p.expect(SyntaxKind::R_BRACE);

    m.complete_p(p, SyntaxKind::KEYFRAME_BLOCK);
}

/// Parse a single keyframe: `50%: value;`
fn keyframe(p: &mut Parser) {
    let m = p.start();

    // Percentage or number
    if p.at(SyntaxKind::NUMBER_WITH_UNIT) || p.at(SyntaxKind::NUMBER) {
        p.bump_any();
    }

    // Colon
    if p.at(SyntaxKind::COLON) {
        p.bump_any();
    }

    // Value (consume until semicolon or closing brace)
    while !p.at_end()
        && !p.at(SyntaxKind::SEMICOLON)
        && !p.at(SyntaxKind::R_BRACE)
        && !p.at(SyntaxKind::R_PAREN)
        && !p.at(SyntaxKind::R_BRACKET)
    {
        css_value_part(p);
    }

    // Optional semicolon
    p.eat(SyntaxKind::SEMICOLON);

    m.complete_p(p, SyntaxKind::KEYFRAME);
}

/// Check if the parser is at an identifier followed by a colon (property pattern).
///
/// Looks ahead: `IDENT '?'? ':'`
pub fn at_property(p: &Parser) -> bool {
    if !p.at(SyntaxKind::IDENT) && !p.current().is_keyword() {
        return false;
    }
    // Look ahead past identifier and optional question mark
    let mut offset = 1;
    if p.nth(offset) == SyntaxKind::QUESTION {
        offset += 1;
    }
    p.nth(offset) == SyntaxKind::COLON
}

/// Check if at a newline-separated property start (for multiline value parsing).
pub fn at_newline_property_start(p: &Parser) -> bool {
    // IDENT followed by COLON (with optional QUESTION between)
    at_property(p)
}
