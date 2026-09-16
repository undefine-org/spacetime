//! Pattern extractors — extract pattern-matching and union types from token sequences.
//!
//! Handles PatternMatch, Union, ParamList, and TemplateInvocation captures.

use crate::syntax::cst::SyntaxKind;
use std::collections::HashMap;

use crate::syntax::form_match::CapturedValue;

use super::{CaptureExtractor, ExtractResult, TokenData};

/// Extract a pattern match expression: `$signal is Variant { $field1, $field2 }`.
///
/// Returns the pattern as a structured string representation in `CapturedValue::String`.
/// The downstream pattern-match expander parses this string representation.
pub struct PatternMatchExtractor;

impl CaptureExtractor for PatternMatchExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        let mut pos = 0;

        // Skip whitespace
        while pos < tokens.len() && tokens[pos].kind == SyntaxKind::WHITESPACE {
            pos += 1;
        }

        // Expect $binding
        if pos >= tokens.len() || tokens[pos].kind != SyntaxKind::DOLLAR {
            return None;
        }
        pos += 1;

        // Binding name
        if pos >= tokens.len()
            || (tokens[pos].kind != SyntaxKind::IDENT && !tokens[pos].kind.is_keyword())
        {
            return None;
        }
        let binding_name = tokens[pos].text(source).to_string();
        pos += 1;

        // Skip whitespace
        while pos < tokens.len() && tokens[pos].kind == SyntaxKind::WHITESPACE {
            pos += 1;
        }

        // Expect "is" keyword
        if pos >= tokens.len() {
            return None;
        }
        let is_text = tokens[pos].text(source);
        if is_text != "is" {
            return None;
        }
        pos += 1;

        // Skip whitespace
        while pos < tokens.len() && tokens[pos].kind == SyntaxKind::WHITESPACE {
            pos += 1;
        }

        // Variant name
        if pos >= tokens.len()
            || (tokens[pos].kind != SyntaxKind::IDENT && !tokens[pos].kind.is_keyword())
        {
            return None;
        }
        let variant_name = tokens[pos].text(source).to_string();
        pos += 1;

        // Skip whitespace
        while pos < tokens.len() && tokens[pos].kind == SyntaxKind::WHITESPACE {
            pos += 1;
        }

        // Optional bindings block: { $field1, $field2 }
        let mut bindings = Vec::new();
        if pos < tokens.len() && tokens[pos].kind == SyntaxKind::L_BRACE {
            pos += 1; // skip {

            while pos < tokens.len() && tokens[pos].kind != SyntaxKind::R_BRACE {
                // Skip whitespace and commas
                if tokens[pos].kind == SyntaxKind::WHITESPACE
                    || tokens[pos].kind == SyntaxKind::COMMA
                {
                    pos += 1;
                    continue;
                }

                // Expect $binding
                if tokens[pos].kind == SyntaxKind::DOLLAR {
                    pos += 1;
                    if pos < tokens.len()
                        && (tokens[pos].kind == SyntaxKind::IDENT || tokens[pos].kind.is_keyword())
                    {
                        bindings.push(tokens[pos].text(source).to_string());
                        pos += 1;
                    }
                } else {
                    break;
                }
            }

            // Consume closing brace
            if pos < tokens.len() && tokens[pos].kind == SyntaxKind::R_BRACE {
                pos += 1;
            }
        }

        // Build structured string representation
        let pattern_str = if bindings.is_empty() {
            format!("${} is {}", binding_name, variant_name)
        } else {
            let bindings_str = bindings
                .iter()
                .map(|b| format!("${}", b))
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "${} is {} {{ {} }}",
                binding_name, variant_name, bindings_str
            )
        };

        Some((CapturedValue::String(pattern_str), pos))
    }
}

/// Extract a union type value: one of the allowed variants.
///
/// Union types are parameterized by their allowed variants, so this
/// extractor takes the variants at construction time.
pub struct UnionExtractor {
    pub variants: Vec<String>,
}

impl CaptureExtractor for UnionExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        let tok = tokens.first()?;

        // Try string token (quoted variant)
        if tok.kind == SyntaxKind::STRING {
            let text = tok.text(source);
            if text.len() >= 2 {
                let inner = &text[1..text.len() - 1];
                if self.variants.iter().any(|v| v == inner) {
                    return Some((CapturedValue::Ident(inner.to_string()), 1));
                }
            }
        }

        // Try ident token (unquoted variant)
        if tok.kind == SyntaxKind::IDENT || tok.kind.is_keyword() {
            let text = tok.text(source);
            if self.variants.iter().any(|v| v == text) {
                return Some((CapturedValue::Ident(text.to_string()), 1));
            }
        }

        None
    }
}

/// Extract a template parameter list: `($title, &content, $footer?)`.
///
// ParamListExtractor (Rust) DELETED in PLAN-023 W2: param_list is now a stdlib
// %capture_type (capture-types/param_list.st) compiled via the PEG and reified by
// reify_param_list into ParamList(Vec<TemplateParamDef>). See custom.rs. The golden test
// param_list_via_stdlib_matches_rust_extractor pins the equivalence that justified deletion.

/// Extract a template invocation: `&template-name($arg1, key: $val)`.
///
pub struct TemplateInvocationExtractor;

impl CaptureExtractor for TemplateInvocationExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        if tokens.is_empty() {
            return None;
        }

        let mut pos = 0;

        while pos < tokens.len() && tokens[pos].kind == SyntaxKind::WHITESPACE {
            pos += 1;
        }

        if pos >= tokens.len() || tokens[pos].kind != SyntaxKind::AMPERSAND {
            return None;
        }
        pos += 1;

        while pos < tokens.len() && tokens[pos].kind == SyntaxKind::WHITESPACE {
            pos += 1;
        }

        let name_start = pos;
        while pos < tokens.len() {
            match tokens[pos].kind {
                SyntaxKind::WHITESPACE
                | SyntaxKind::L_PAREN
                | SyntaxKind::SEMICOLON
                | SyntaxKind::COMMA
                | SyntaxKind::R_BRACE
                | SyntaxKind::EOF => break,
                _ => pos += 1,
            }
        }

        if pos == name_start {
            return None;
        }

        let name = source[tokens[name_start].text_range.0..tokens[pos - 1].text_range.1]
            .trim()
            .trim_start_matches('&')
            .to_string();

        if name.is_empty() {
            return None;
        }

        while pos < tokens.len() && tokens[pos].kind == SyntaxKind::WHITESPACE {
            pos += 1;
        }

        let mut args = Vec::new();
        if pos < tokens.len() && tokens[pos].kind == SyntaxKind::L_PAREN {
            pos += 1;
            let mut arg_start = pos;
            let mut depth = 0i32;
            let mut saw_closing_paren = false;

            while pos < tokens.len() {
                match tokens[pos].kind {
                    SyntaxKind::L_PAREN | SyntaxKind::L_BRACE | SyntaxKind::L_BRACKET => {
                        depth += 1;
                        pos += 1;
                    }
                    SyntaxKind::R_PAREN => {
                        if depth == 0 {
                            if let Some(arg) = extract_segment(source, tokens, arg_start, pos) {
                                args.push(CapturedValue::String(arg));
                            }
                            pos += 1;
                            saw_closing_paren = true;
                            break;
                        }
                        depth -= 1;
                        pos += 1;
                    }
                    SyntaxKind::R_BRACE | SyntaxKind::R_BRACKET => {
                        if depth > 0 {
                            depth -= 1;
                        }
                        pos += 1;
                    }
                    SyntaxKind::COMMA if depth == 0 => {
                        if let Some(arg) = extract_segment(source, tokens, arg_start, pos) {
                            args.push(CapturedValue::String(arg));
                        }
                        pos += 1;
                        arg_start = pos;
                    }
                    SyntaxKind::EOF => break,
                    _ => pos += 1,
                }
            }

            if !saw_closing_paren {
                return None;
            }
        }

        while pos < tokens.len() && tokens[pos].kind == SyntaxKind::WHITESPACE {
            pos += 1;
        }

        if pos < tokens.len() && tokens[pos].kind == SyntaxKind::SEMICOLON {
            pos += 1;
        }

        let mut invocation = HashMap::new();
        invocation.insert("name".to_string(), CapturedValue::String(name));
        invocation.insert("args".to_string(), CapturedValue::Array(args));

        Some((CapturedValue::Named(invocation), pos))
    }
}

fn extract_segment(
    source: &str,
    tokens: &[TokenData],
    start_idx: usize,
    end_idx: usize,
) -> Option<String> {
    if start_idx >= end_idx {
        return None;
    }

    let start = tokens[start_idx].text_range.0;
    let end = tokens[end_idx - 1].text_range.1;
    let value = source[start..end].trim();

    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn tok(kind: SyntaxKind, start: usize, end: usize) -> TokenData {
        TokenData {
            kind,
            text_range: (start, end),
        }
    }

    #[test]
    fn pattern_match_simple() {
        let source = "$signal is Loading";
        let tokens = [
            tok(SyntaxKind::DOLLAR, 0, 1),
            tok(SyntaxKind::IDENT, 1, 7), // signal
            tok(SyntaxKind::WHITESPACE, 7, 8),
            tok(SyntaxKind::IDENT, 8, 10), // is
            tok(SyntaxKind::WHITESPACE, 10, 11),
            tok(SyntaxKind::IDENT, 11, 18), // Loading
        ];
        let result = PatternMatchExtractor.extract(&tokens, source);
        assert_eq!(
            result,
            Some((CapturedValue::String("$signal is Loading".to_string()), 6))
        );
    }

    #[test]
    fn pattern_match_with_bindings() {
        let source = "$signal is Success { $data, $count }";
        let tokens = [
            tok(SyntaxKind::DOLLAR, 0, 1),
            tok(SyntaxKind::IDENT, 1, 7), // signal
            tok(SyntaxKind::WHITESPACE, 7, 8),
            tok(SyntaxKind::IDENT, 8, 10), // is
            tok(SyntaxKind::WHITESPACE, 10, 11),
            tok(SyntaxKind::IDENT, 11, 18), // Success
            tok(SyntaxKind::WHITESPACE, 18, 19),
            tok(SyntaxKind::L_BRACE, 19, 20),
            tok(SyntaxKind::WHITESPACE, 20, 21),
            tok(SyntaxKind::DOLLAR, 21, 22),
            tok(SyntaxKind::IDENT, 22, 26), // data
            tok(SyntaxKind::COMMA, 26, 27),
            tok(SyntaxKind::WHITESPACE, 27, 28),
            tok(SyntaxKind::DOLLAR, 28, 29),
            tok(SyntaxKind::IDENT, 29, 34), // count
            tok(SyntaxKind::WHITESPACE, 34, 35),
            tok(SyntaxKind::R_BRACE, 35, 36),
        ];
        let result = PatternMatchExtractor.extract(&tokens, source);
        assert_eq!(
            result,
            Some((
                CapturedValue::String("$signal is Success { $data, $count }".to_string()),
                17
            ))
        );
    }

    #[test]
    fn union_quoted() {
        let extractor = UnionExtractor {
            variants: vec!["x".to_string(), "y".to_string(), "both".to_string()],
        };
        let source = "\"x\"";
        let tokens = [tok(SyntaxKind::STRING, 0, 3)];
        let result = extractor.extract(&tokens, source);
        assert_eq!(result, Some((CapturedValue::Ident("x".to_string()), 1)));
    }

    #[test]
    fn union_unquoted() {
        let extractor = UnionExtractor {
            variants: vec!["x".to_string(), "y".to_string(), "both".to_string()],
        };
        let source = "both";
        let tokens = [tok(SyntaxKind::IDENT, 0, 4)];
        let result = extractor.extract(&tokens, source);
        assert_eq!(result, Some((CapturedValue::Ident("both".to_string()), 1)));
    }

    #[test]
    fn union_rejects_invalid() {
        let extractor = UnionExtractor {
            variants: vec!["x".to_string(), "y".to_string()],
        };
        let source = "z";
        let tokens = [tok(SyntaxKind::IDENT, 0, 1)];
        let result = extractor.extract(&tokens, source);
        assert!(result.is_none());
    }

    // param_list_binding_and_element + param_list_with_optional DELETED (PLAN-023 W2):
    // param_list is now a stdlib %capture_type; equivalence is pinned by the golden test
    // param_list_via_stdlib_matches_rust_extractor in extractors/custom.rs.

    #[test]
    fn template_invocation_extractor_single_arg() {
        let source = "&ado-project($p);";
        let tokens = [
            tok(SyntaxKind::AMPERSAND, 0, 1),
            tok(SyntaxKind::IDENT, 1, 12),
            tok(SyntaxKind::L_PAREN, 12, 13),
            tok(SyntaxKind::DOLLAR, 13, 14),
            tok(SyntaxKind::IDENT, 14, 15),
            tok(SyntaxKind::R_PAREN, 15, 16),
            tok(SyntaxKind::SEMICOLON, 16, 17),
        ];

        let mut expected = HashMap::new();
        expected.insert(
            "name".to_string(),
            CapturedValue::String("ado-project".to_string()),
        );
        expected.insert(
            "args".to_string(),
            CapturedValue::Array(vec![CapturedValue::String("$p".to_string())]),
        );

        let result = TemplateInvocationExtractor.extract(&tokens, source);
        assert_eq!(result, Some((CapturedValue::Named(expected), 7)));
    }

    #[test]
    fn template_invocation_extractor_empty_args() {
        let source = "&product-card();";
        let tokens = [
            tok(SyntaxKind::AMPERSAND, 0, 1),
            tok(SyntaxKind::IDENT, 1, 13),
            tok(SyntaxKind::L_PAREN, 13, 14),
            tok(SyntaxKind::R_PAREN, 14, 15),
            tok(SyntaxKind::SEMICOLON, 15, 16),
        ];

        let mut expected = HashMap::new();
        expected.insert(
            "name".to_string(),
            CapturedValue::String("product-card".to_string()),
        );
        expected.insert("args".to_string(), CapturedValue::Array(vec![]));

        let result = TemplateInvocationExtractor.extract(&tokens, source);
        assert_eq!(result, Some((CapturedValue::Named(expected), 5)));
    }
}
