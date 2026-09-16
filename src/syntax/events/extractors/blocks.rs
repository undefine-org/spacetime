//! Block extractors — extract block-shaped captures from token sequences.
//!
//! Handles JsBlock, HtmlBlock, MutationActions, Template, and ComponentBody captures.
//! These operate on balanced-brace token regions.
//!
//! Handles JsBlock, HtmlBlock, MutationActions, and Template captures.
//! These operate on balanced-brace token regions.

use crate::syntax::cst::SyntaxKind;
use crate::syntax::form_match::CapturedValue;

use super::{CaptureExtractor, ExtractResult, TokenData};

/// Extract a JavaScript block: `{ multi-statement JS code }`.
///
/// Returns the raw content between braces as `CapturedValue::Expr`.
/// Used by `%emit js { ... }` and similar constructs.
pub struct JsBlockExtractor;

impl CaptureExtractor for JsBlockExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        if tokens.is_empty() {
            return None;
        }

        // Must start with L_BRACE
        if tokens[0].kind != SyntaxKind::L_BRACE {
            return None;
        }

        let mut depth = 0i32;
        let mut pos = 0;

        for (i, tok) in tokens.iter().enumerate() {
            match tok.kind {
                SyntaxKind::L_BRACE => {
                    depth += 1;
                    pos = i;
                }
                SyntaxKind::R_BRACE => {
                    depth -= 1;
                    pos = i;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {
                    pos = i;
                }
            }
        }

        if depth != 0 {
            return None; // Unbalanced braces
        }

        // Extract content between the braces (excluding braces themselves)
        let content = if pos > 1 {
            let start = tokens[1].text_range.0;
            let end = tokens[pos].text_range.0; // up to closing brace
            source[start..end].trim().to_string()
        } else {
            String::new()
        };

        Some((CapturedValue::Expr(content), pos + 1))
    }
}

/// Extract an HTML block with interpolation: `<div>$value.prop</div>`.
///
/// Returns content as `CapturedValue::Expr` since HTML blocks are
/// processed by the emit/codegen layer, not the parser.
pub struct HtmlBlockExtractor;

impl CaptureExtractor for HtmlBlockExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        // HTML blocks are treated as opaque expression content
        // The actual HTML parsing happens in the emit/codegen layer
        super::simple::ExprExtractor.extract(tokens, source)
    }
}

/// Extract mutation actions: `property: from -> to;` sequences.
///
/// Falls back to Expr extraction — mutation actions are semantically
/// similar to keyframes and are handled by the expand layer.
pub struct MutationActionsExtractor;

impl CaptureExtractor for MutationActionsExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        super::simple::ExprExtractor.extract(tokens, source)
    }
}

/// Extract a template — child element templates.
///
/// Falls back to Expr extraction — templates are processed
/// by the codegen layer.
pub struct TemplateExtractor;

impl CaptureExtractor for TemplateExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        super::simple::ExprExtractor.extract(tokens, source)
    }
}

/// Extract a component body: structured parse of HTML + CSS rules + state
/// declarations + behavioral directives + content injections.
///
/// When used as a token-based extractor (fallback path), delegates to
/// ExprExtractor since the primary parsing happens in form_compiler.rs
/// via `parse_component_body()` on the raw body string.
pub struct ComponentBodyExtractor;

impl CaptureExtractor for ComponentBodyExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        // The primary ComponentBody parsing happens in form_compiler.rs
        // via parse_component_body() on the raw body string.
        // This extractor is the token-based fallback — treat as expression.
        super::simple::ExprExtractor.extract(tokens, source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tok(kind: SyntaxKind, start: usize, end: usize) -> TokenData {
        TokenData {
            kind,
            text_range: (start, end),
        }
    }

    #[test]
    fn jsblock_simple() {
        let source = "{ return 42; }";
        let tokens = [
            tok(SyntaxKind::L_BRACE, 0, 1),
            tok(SyntaxKind::WHITESPACE, 1, 2),
            tok(SyntaxKind::IDENT, 2, 8), // return
            tok(SyntaxKind::WHITESPACE, 8, 9),
            tok(SyntaxKind::NUMBER, 9, 11), // 42
            tok(SyntaxKind::SEMICOLON, 11, 12),
            tok(SyntaxKind::WHITESPACE, 12, 13),
            tok(SyntaxKind::R_BRACE, 13, 14),
        ];
        let result = JsBlockExtractor.extract(&tokens, source);
        let (val, consumed) = result.unwrap();
        assert_eq!(consumed, 8);
        assert_eq!(val, CapturedValue::Expr("return 42;".to_string()));
    }

    #[test]
    fn jsblock_nested_braces() {
        let source = "{ if (true) { x(); } }";
        let tokens = [
            tok(SyntaxKind::L_BRACE, 0, 1),
            tok(SyntaxKind::WHITESPACE, 1, 2),
            tok(SyntaxKind::IDENT, 2, 4), // if
            tok(SyntaxKind::WHITESPACE, 4, 5),
            tok(SyntaxKind::L_PAREN, 5, 6),
            tok(SyntaxKind::IDENT, 6, 10), // true
            tok(SyntaxKind::R_PAREN, 10, 11),
            tok(SyntaxKind::WHITESPACE, 11, 12),
            tok(SyntaxKind::L_BRACE, 12, 13),
            tok(SyntaxKind::WHITESPACE, 13, 14),
            tok(SyntaxKind::IDENT, 14, 15), // x
            tok(SyntaxKind::L_PAREN, 15, 16),
            tok(SyntaxKind::R_PAREN, 16, 17),
            tok(SyntaxKind::SEMICOLON, 17, 18),
            tok(SyntaxKind::WHITESPACE, 18, 19),
            tok(SyntaxKind::R_BRACE, 19, 20),
            tok(SyntaxKind::WHITESPACE, 20, 21),
            tok(SyntaxKind::R_BRACE, 21, 22),
        ];
        let result = JsBlockExtractor.extract(&tokens, source);
        let (val, consumed) = result.unwrap();
        assert_eq!(consumed, 18);
        assert_eq!(val, CapturedValue::Expr("if (true) { x(); }".to_string()));
    }

    #[test]
    fn jsblock_empty() {
        let source = "{ }";
        let tokens = [
            tok(SyntaxKind::L_BRACE, 0, 1),
            tok(SyntaxKind::WHITESPACE, 1, 2),
            tok(SyntaxKind::R_BRACE, 2, 3),
        ];
        let result = JsBlockExtractor.extract(&tokens, source);
        let (val, consumed) = result.unwrap();
        assert_eq!(consumed, 3);
        assert_eq!(val, CapturedValue::Expr(String::new()));
    }

    #[test]
    fn jsblock_requires_brace() {
        let source = "hello";
        let tokens = [tok(SyntaxKind::IDENT, 0, 5)];
        let result = JsBlockExtractor.extract(&tokens, source);
        assert!(result.is_none());
    }
}
