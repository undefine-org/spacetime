//! Meta-clause parsing for the event-based parser.
//!
//! Handles `%primitive`, `%macro`, `%form`, `%capture_type`, `%bind`,
//! `%emit`, `%yield`, `%cleanup`, and other meta-clauses.
//!
//! # Body parsing strategies
//!
//! Different meta-clause types need different body parsing:
//!
//! - **Raw body** (`%form`, `%capture_type`): Body contains pattern DSL, not
//!   Spacetime syntax. Parsed as opaque text with balanced brace tracking.
//!
//! - **Emit body** (`%emit`): Body contains foreign code (JS/CSS/HTML).
//!   Parsed as opaque text BUT interprets `%` tokens for nested `%yield`
//!   and `%cleanup` clauses.
//!
//! - **Standard body** (everything else): Body contains normal Spacetime
//!   syntax, parsed via `directives::body()`.

use crate::syntax::cst::SyntaxKind;
use crate::syntax::events::parser::Parser;

/// Parse a meta definition or meta clause: `%name args { ... }`
///
/// Uses the source text to determine the keyword and dispatch to the
/// appropriate body parser.
pub fn meta_def(p: &mut Parser, source: &str) {
    let m = p.start();

    p.expect(SyntaxKind::PERCENT);

    // Capture the keyword text for body dispatch
    let keyword = if p.at(SyntaxKind::IDENT) || p.current().is_keyword() {
        let text = p.nth_text(0, source).to_string();
        p.bump_any();
        text
    } else {
        String::new()
    };

    // Inline arguments
    super::directives::inline_args(p);

    // Optional parenthesized params
    if p.at(SyntaxKind::L_PAREN) {
        super::directives::arg_list(p);
    }

    // Body dispatch based on keyword
    if p.at(SyntaxKind::L_BRACE) {
        match keyword.as_str() {
            "form" | "capture_type" | "captureType" => {
                // Pattern DSL — treat body as opaque text
                raw_body(p);
            }
            "vendor" => {
                // Vendor manifest — field DSL, treat body as opaque text
                raw_body(p);
            }
            "emit" => {
                // Foreign code island — opaque but interpret % tokens
                emit_body(p);
            }
            _ => {
                // Standard Spacetime body
                super::directives::body_with_source(p, source);
            }
        }
    }

    // Optional semicolon (for clause-style: %yield animation;)
    p.eat(SyntaxKind::SEMICOLON);

    m.complete_p(p, SyntaxKind::META_DEF);
}

/// Parse a raw body: `{ ... }` where contents are opaque text.
///
/// Used for `%form` and `%capture_type` bodies that contain pattern DSL
/// syntax, not Spacetime syntax. Just tracks balanced braces.
fn raw_body(p: &mut Parser) {
    let m = p.start();

    p.expect(SyntaxKind::L_BRACE);

    let mut depth = 1i32;
    while !p.at_end() && depth > 0 {
        if p.current() == SyntaxKind::L_BRACE {
            depth += 1;
        } else if p.current() == SyntaxKind::R_BRACE {
            depth -= 1;
            if depth == 0 {
                break;
            }
        }
        p.bump_any();
    }

    p.expect(SyntaxKind::R_BRACE);

    m.complete_p(p, SyntaxKind::BODY);
}

/// Parse an emit body: `{ ... }` where contents are foreign code.
///
/// Used for `%emit js { ... }`, `%emit css { ... }`, etc. The body is
/// mostly opaque but `%` tokens are interpreted as nested meta-clauses
/// (`%yield`, `%cleanup`, etc.).
fn emit_body(p: &mut Parser) {
    let m = p.start();

    p.expect(SyntaxKind::L_BRACE);

    let mut depth = 1i32;
    while !p.at_end() && depth > 0 {
        match p.current() {
            SyntaxKind::L_BRACE => {
                depth += 1;
                p.bump_any();
            }
            SyntaxKind::R_BRACE => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                p.bump_any();
            }
            SyntaxKind::PERCENT => {
                // Nested meta-clause: %yield, %cleanup, etc.
                meta_clause_in_emit(p);
            }
            _ => {
                p.bump_any();
            }
        }
    }

    p.expect(SyntaxKind::R_BRACE);

    m.complete_p(p, SyntaxKind::BODY);
}

/// Parse a meta-clause nested within an %emit body.
///
/// Handles `%yield`, `%cleanup`, and other meta tokens found inside
/// foreign code blocks. These are minimal: `%name args;` or `%name { ... }`.
fn meta_clause_in_emit(p: &mut Parser) {
    let m = p.start();

    p.expect(SyntaxKind::PERCENT);

    // Clause name
    if p.at(SyntaxKind::IDENT) || p.current().is_keyword() {
        p.bump_any();
    }

    // Inline args until semicolon, brace, or next %
    while !p.at_end()
        && !p.at(SyntaxKind::SEMICOLON)
        && !p.at(SyntaxKind::L_BRACE)
        && !p.at(SyntaxKind::R_BRACE)
        && !p.at(SyntaxKind::PERCENT)
    {
        p.bump_any();
    }

    // Optional body
    if p.at(SyntaxKind::L_BRACE) {
        raw_body(p);
    }

    p.eat(SyntaxKind::SEMICOLON);

    m.complete_p(p, SyntaxKind::META_DEF);
}

#[cfg(test)]
mod tests {
    use crate::syntax::cst::SyntaxKind;
    use crate::syntax::cst::lexer::Lexer;
    use crate::syntax::events::input::Input;
    use crate::syntax::events::parser::Parser;
    use crate::syntax::events::process::process;
    use crate::syntax::events::strategy::ErrorStrategy;
    use crate::syntax::events::tree_sink::TreeSink;

    /// Parse source through event-based parser using meta-aware root.
    fn parse_meta_source(
        source: &str,
    ) -> (
        crate::syntax::cst::SyntaxNode,
        Vec<crate::syntax::events::tree_sink::ParseError>,
    ) {
        let lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        let input = Input::from_tokens(&tokens);
        let mut parser = Parser::new(&input, ErrorStrategy::Verbose);

        // Use a root that dispatches % to meta_def with source
        let m = parser.start();
        while !parser.at_end() {
            if parser.at(SyntaxKind::PERCENT) {
                super::meta_def(&mut parser, source);
            } else if parser.at(SyntaxKind::AT_SIGN) {
                super::super::directives::directive(&mut parser);
            } else {
                parser.bump_any();
            }
        }
        m.complete_p(&mut parser, SyntaxKind::ROOT);

        let mut events = parser.finish();
        let mut tree_sink = TreeSink::new();
        process(&mut events, source, &input, &mut tree_sink);
        tree_sink.finish()
    }

    fn assert_meta_parses_losslessly(source: &str) {
        let (tree, errors) = parse_meta_source(source);
        assert_eq!(tree.kind(), SyntaxKind::ROOT);
        assert!(errors.is_empty(), "Errors for {:?}: {:?}", source, errors);

        let reconstructed: String = tree
            .descendants_with_tokens()
            .filter_map(|el| el.into_token())
            .map(|tok| tok.text().to_string())
            .collect();
        assert_eq!(
            reconstructed, source,
            "CST should losslessly preserve source text"
        );
    }

    #[test]
    fn meta_primitive() {
        assert_meta_parses_losslessly("%primitive button { }");
    }

    #[test]
    fn meta_macro() {
        assert_meta_parses_losslessly("%macro fade-in { }");
    }

    #[test]
    fn meta_form_raw_body() {
        // %form body is raw — pattern DSL shouldn't be parsed as CSS properties
        assert_meta_parses_losslessly("%form { @on $event:ident }");
        assert_meta_parses_losslessly("%form { @load ($duration:time?) }");
    }

    #[test]
    fn meta_capture_type_raw_body() {
        // %capture_type body is raw — pattern DSL
        assert_meta_parses_losslessly("%capture_type easing { }");
        assert_meta_parses_losslessly("%capture_type param_list { ( $name:binding )* }");
    }

    #[test]
    fn meta_bind() {
        assert_meta_parses_losslessly("%bind { event <- $event; }");
    }

    #[test]
    fn meta_emit_with_js() {
        // %emit body is an island — JS code preserved as opaque tokens
        assert_meta_parses_losslessly("%emit js { console.log('hello'); }");
    }

    #[test]
    fn meta_emit_with_nested_yield() {
        // %yield inside %emit body should be parsed as nested meta-clause
        let source = "%emit js { var x = %yield animation; return x; }";
        let (tree, errors) = parse_meta_source(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);

        // Find the nested META_DEF for %yield
        let has_nested_meta = tree
            .descendants()
            .filter(|n| n.kind() == SyntaxKind::META_DEF)
            .count();
        // Outer %emit + inner %yield = 2 META_DEFs
        assert!(
            has_nested_meta >= 2,
            "Should have outer %emit and inner %yield META_DEF nodes, found {}",
            has_nested_meta
        );
    }

    #[test]
    fn meta_yield() {
        assert_meta_parses_losslessly("%yield animation;");
    }

    #[test]
    fn meta_cleanup() {
        assert_meta_parses_losslessly("%cleanup { removeEventListener(); }");
    }

    #[test]
    fn meta_emit_balanced_braces() {
        // JS code with nested braces should be handled
        let source = "%emit js { if (true) { console.log('nested'); } }";
        let (tree, errors) = parse_meta_source(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);

        let reconstructed: String = tree
            .descendants_with_tokens()
            .filter_map(|el| el.into_token())
            .map(|tok| tok.text().to_string())
            .collect();
        assert_eq!(reconstructed, source);
    }

    // --- Foreign-code bodies: a delimiter's meaning belongs to the grammar that
    // owns the text (BUG-040, BUG-041). `%emit js` is an ISLAND of JavaScript, so
    // Spacetime's STRING tokenization must not decide where its braces are: a quote
    // or backtick inside a JS regex CHARACTER CLASS is an ordinary character to JS,
    // and treating it as a string opener swallows the braces that close the block.

    #[test]
    fn meta_emit_js_regex_char_class_with_quotes() {
        // BUG-040: `/[&<>"']/g` — the `"` opened a Spacetime STRING token that ran
        // past the block's `}`, so emit_body's depth counter never reached 0 and the
        // parse died with "expected R_BRACE, found EOF" nowhere near the regex.
        assert_meta_parses_losslessly(
            "%emit js { function esc(s){ return String(s).replace(/[&<>\"']/g, function(c){ return c; }); } }",
        );
    }

    #[test]
    fn meta_emit_js_regex_char_class_with_backtick() {
        // BUG-041: a backtick inside a regex char class was read as a template-literal
        // opener, desyncing the block — the primitive was then SILENTLY dropped from
        // the registry (downstream emitted `// Primitive not found`), which is why
        // this must be a parse-level gate rather than a compile-succeeds check.
        assert_meta_parses_losslessly(
            "%emit js { const masked = \"x\".replace(/(['\"`])(?:\\\\.|(?!\\1).)*\\1/g, (s) => ''); }",
        );
    }

    #[test]
    fn meta_emit_js_regex_containing_braces() {
        // A regex quantifier `{2,4}` and a char class containing braces must not be
        // counted as block structure — the brace tracker is scanning JS, not Spacetime.
        assert_meta_parses_losslessly("%emit js { const re = /a{2,4}[{}]/g; }");
    }

    #[test]
    fn meta_emit_js_apostrophe_in_comment() {
        // BUG-040 acceptance: an apostrophe inside a JS comment must not open a string.
        assert_meta_parses_losslessly("%emit js { // the parser's own comment\n const x = 1; }");
    }

    #[test]
    fn meta_emit_js_division_is_not_a_regex() {
        // The regex/divide ambiguity: `a / b` is division, and the `/` must NOT open a
        // regex literal that then swallows the closing brace. Disambiguated by the
        // previous significant token, exactly as a JS lexer does.
        assert_meta_parses_losslessly("%emit js { const r = (a + b) / 2; const s = x / y; }");
    }

    #[test]
    fn meta_form_pattern_not_parsed_as_css() {
        // The pattern `@on $event:ident ($duration:time?)` should NOT trigger
        // CSS property parsing for "duration:time" inside the %form body.
        let source = "%form { @on $event:ident ($duration:time?) }";
        let (tree, errors) = parse_meta_source(source);
        assert!(errors.is_empty(), "Errors: {:?}", errors);

        // The body should not contain CSS_PROPERTY nodes
        let has_css_prop = tree
            .descendants()
            .any(|n| n.kind() == SyntaxKind::CSS_PROPERTY);
        assert!(
            !has_css_prop,
            "Raw body should not produce CSS_PROPERTY nodes"
        );
    }
}
