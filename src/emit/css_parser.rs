//! CSS emit block tokenizer.
//!
//! Parses CSS emit blocks to identify Spacetime placeholders.
//! CSS supports only:
//! - `%param` - primitive parameter reference
//! - `%&element` - element reference (for selectors)

use crate::ir::{CssExpr, CssPart, Marker};

/// Reserved keywords that start with % but are not param placeholders
const RESERVED_PERCENT_KEYWORDS: &[&str] = &[
    "yield", "if", "else", "elif", "for", "emit", "cleanup", "exports",
];

/// Parse CSS emit content into a CssExpr.
///
/// If the content contains placeholders, returns a Composite.
/// Otherwise returns Raw.
pub fn parse_emit_css(content: &str) -> CssExpr {
    let parts = tokenize_css(content);

    // If no markers, just return Raw
    if parts.len() == 1
        && let CssPart::Raw(s) = &parts[0]
    {
        return CssExpr::Raw(s.clone());
    }

    CssExpr::Composite(parts)
}

/// Tokenize CSS content into parts
fn tokenize_css(content: &str) -> Vec<CssPart> {
    let mut parts = Vec::new();
    let chars: Vec<char> = content.chars().collect();
    let mut pos = 0;
    let mut current_raw = String::new();

    while pos < chars.len() {
        if chars[pos] == '%' {
            // Check for %&element or %param
            if let Some((part, end)) = try_parse_element(&chars, pos) {
                if !current_raw.is_empty() {
                    parts.push(CssPart::Raw(std::mem::take(&mut current_raw)));
                }
                parts.push(part);
                pos = end;
            } else if let Some((part, end)) = try_parse_param(&chars, pos) {
                if !current_raw.is_empty() {
                    parts.push(CssPart::Raw(std::mem::take(&mut current_raw)));
                }
                parts.push(part);
                pos = end;
            } else {
                current_raw.push(chars[pos]);
                pos += 1;
            }
        } else {
            current_raw.push(chars[pos]);
            pos += 1;
        }
    }

    if !current_raw.is_empty() {
        parts.push(CssPart::Raw(current_raw));
    }

    parts
}

/// Try to parse %&name element reference
fn try_parse_element(chars: &[char], pos: usize) -> Option<(CssPart, usize)> {
    if pos + 2 >= chars.len() || chars[pos] != '%' || chars[pos + 1] != '&' {
        return None;
    }

    let mut end = pos + 2;
    while end < chars.len() && is_ident_char(chars[end]) {
        end += 1;
    }

    if end == pos + 2 {
        return None; // No identifier after %&
    }

    let name: String = chars[pos + 2..end].iter().collect();
    Some((CssPart::Marker(Marker::Element(name)), end))
}

/// Try to parse %name param reference
fn try_parse_param(chars: &[char], pos: usize) -> Option<(CssPart, usize)> {
    if pos >= chars.len() || chars[pos] != '%' {
        return None;
    }

    let mut end = pos + 1;
    while end < chars.len() && is_ident_char(chars[end]) {
        end += 1;
    }

    if end == pos + 1 {
        return None; // No identifier after %
    }

    let name: String = chars[pos + 1..end].iter().collect();

    // Skip reserved keywords
    if RESERVED_PERCENT_KEYWORDS.contains(&name.as_str()) {
        return None;
    }

    Some((
        CssPart::Marker(Marker::Param {
            name,
            field: None,
            in_string: None,
        }),
        end,
    ))
}

/// Check if character is valid in an identifier
fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_css() {
        let css = ".button { color: red; }";
        let result = parse_emit_css(css);
        assert_eq!(result, CssExpr::Raw(css.to_string()));
    }

    #[test]
    fn test_param_reference() {
        let css = ".button { color: %color; }";
        let result = parse_emit_css(css);
        match result {
            CssExpr::Composite(parts) => {
                assert_eq!(parts.len(), 3);
                assert_eq!(parts[0], CssPart::Raw(".button { color: ".to_string()));
                assert_eq!(
                    parts[1],
                    CssPart::Marker(Marker::Param {
                        name: "color".to_string(),
                        field: None,
                        in_string: None
                    })
                );
                assert_eq!(parts[2], CssPart::Raw("; }".to_string()));
            }
            _ => panic!("Expected Composite"),
        }
    }

    #[test]
    fn test_element_reference() {
        let css = "[data-el=%&box] { display: block; }";
        let result = parse_emit_css(css);
        match result {
            CssExpr::Composite(parts) => {
                assert_eq!(parts.len(), 3);
                assert_eq!(parts[0], CssPart::Raw("[data-el=".to_string()));
                assert_eq!(
                    parts[1],
                    CssPart::Marker(Marker::Element("box".to_string()))
                );
                assert_eq!(parts[2], CssPart::Raw("] { display: block; }".to_string()));
            }
            _ => panic!("Expected Composite"),
        }
    }

    #[test]
    fn test_multiple_params() {
        // .%name { width: %width; height: %height; }
        // Parts: "." + %name + " { width: " + %width + "; height: " + %height + "; }"
        let css = ".%name { width: %width; height: %height; }";
        let result = parse_emit_css(css);
        match result {
            CssExpr::Composite(parts) => {
                assert_eq!(parts.len(), 7);
            }
            _ => panic!("Expected Composite"),
        }
    }

    #[test]
    fn test_param_followed_by_hyphen() {
        // %name-width should be %name + "-width", not a single param "name-width"
        let css = "--st-%name-width: 0px;";
        let result = parse_emit_css(css);
        match result {
            CssExpr::Composite(parts) => {
                assert_eq!(parts[0], CssPart::Raw("--st-".to_string()));
                assert_eq!(
                    parts[1],
                    CssPart::Marker(Marker::Param {
                        name: "name".to_string(),
                        field: None,
                        in_string: None
                    })
                );
                assert_eq!(parts[2], CssPart::Raw("-width: 0px;".to_string()));
            }
            _ => panic!("Expected Composite, got {:?}", result),
        }
    }

    #[test]
    fn test_reserved_keywords_skipped() {
        // %yield should not be treated as a param
        let css = "/* %yield not a param */";
        let result = parse_emit_css(css);
        assert_eq!(result, CssExpr::Raw(css.to_string()));
    }
}
