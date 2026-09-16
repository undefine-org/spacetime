//! Extract %form patterns from Spacetime .st files

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormPattern {
    /// The directive name (e.g., "fade-in", "each", "type")
    pub directive: String,
    /// Whether it's a @ directive or % directive
    pub prefix: char,
    /// Inline identifiers before params (e.g., "$name:ident" in "@scroll $name:ident")
    pub inline_captures: Vec<FormCapture>,
    /// Parameters in parentheses
    pub params: Vec<FormParam>,
    /// Optional body block
    pub body: Option<FormBody>,
    /// Source file for debugging
    pub source_file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormParam {
    /// Parameter name (without $)
    pub name: String,
    /// Optional named key (for "key: $value" syntax)
    pub key: Option<String>,
    /// Type annotation
    pub capture_type: String,
    /// Optional marker (? for optional)
    pub optional: bool,
    /// Default value if any
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormCapture {
    /// Capture name (without $ or &)
    pub name: String,
    /// Prefix ($ or &)
    pub prefix: char,
    /// Type annotation
    pub capture_type: String,
    /// Optional marker
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormBody {
    /// Body captures like $body:keyframes
    pub captures: Vec<FormCapture>,
}

/// Extract all %form patterns from .st files in a directory
pub fn extract_all_forms(dir: &Path) -> Result<Vec<FormPattern>, Box<dyn std::error::Error>> {
    let mut forms = Vec::new();

    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "st") {
            let content = std::fs::read_to_string(path)?;
            let file_forms = extract_forms_from_content(&content, path.to_string_lossy().as_ref());
            forms.extend(file_forms);
        }
    }

    Ok(forms)
}

/// Extract %form patterns from file content
fn extract_forms_from_content(content: &str, source_file: &str) -> Vec<FormPattern> {
    let mut forms = Vec::new();

    // Match %form { ... } blocks with brace balancing
    let form_start = Regex::new(r"%form\s*\{").unwrap();

    for mat in form_start.find_iter(content) {
        let start = mat.end();
        if let Some(form_content) = extract_braced_content(content, start) {
            if let Some(pattern) = parse_form_content(&form_content, source_file) {
                forms.push(pattern);
            }
        }
    }

    forms
}

/// Extract content within braces, handling nesting
fn extract_braced_content(content: &str, start: usize) -> Option<String> {
    let bytes = content.as_bytes();
    let mut depth = 1;
    let mut i = start;

    while i < bytes.len() && depth > 0 {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {}
        }
        if depth > 0 {
            i += 1;
        }
    }

    if depth == 0 {
        Some(content[start..i].to_string())
    } else {
        None
    }
}

/// Parse the content of a %form block
fn parse_form_content(content: &str, source_file: &str) -> Option<FormPattern> {
    let content = content.trim();

    // Pattern: @directive or %directive followed by optional parts
    let directive_re = Regex::new(r"^([@%])([a-zA-Z][a-zA-Z0-9_-]*)").unwrap();
    let caps = directive_re.captures(content)?;

    let prefix = caps.get(1)?.as_str().chars().next()?;
    let directive = caps.get(2)?.as_str().to_string();
    let rest = &content[caps.get(0)?.end()..].trim_start();

    let (inline_captures, rest) = parse_inline_captures(rest);
    let (params, rest) = parse_params(rest);
    let body = parse_body(rest);

    Some(FormPattern {
        directive,
        prefix,
        inline_captures,
        params,
        body,
        source_file: source_file.to_string(),
    })
}

/// Parse inline captures like "$name:ident" before params
fn parse_inline_captures(content: &str) -> (Vec<FormCapture>, &str) {
    let mut captures = Vec::new();
    let mut rest = content;

    // Match $name:type or &name:type patterns
    let capture_re = Regex::new(r"^\s*([$&])([a-zA-Z_][a-zA-Z0-9_]*):([a-zA-Z_][a-zA-Z0-9_]*)(\?)?").unwrap();

    while let Some(caps) = capture_re.captures(rest) {
        let prefix = caps.get(1).unwrap().as_str().chars().next().unwrap();
        let name = caps.get(2).unwrap().as_str().to_string();
        let capture_type = caps.get(3).unwrap().as_str().to_string();
        let optional = caps.get(4).is_some();

        captures.push(FormCapture {
            name,
            prefix,
            capture_type,
            optional,
        });

        rest = &rest[caps.get(0).unwrap().end()..];
    }

    (captures, rest)
}

/// Parse parameters in parentheses
fn parse_params(content: &str) -> (Vec<FormParam>, &str) {
    let content = content.trim_start();
    if !content.starts_with('(') {
        return (Vec::new(), content);
    }

    // Find matching paren
    let mut depth = 1;
    let mut i = 1;
    let bytes = content.as_bytes();

    while i < bytes.len() && depth > 0 {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => depth -= 1,
            _ => {}
        }
        if depth > 0 {
            i += 1;
        }
    }

    let params_str = &content[1..i];
    let rest = &content[i + 1..];

    let params = parse_param_list(params_str);
    (params, rest)
}

/// Parse comma-separated parameter list
fn parse_param_list(content: &str) -> Vec<FormParam> {
    let mut params = Vec::new();

    // Split by comma (simple approach - doesn't handle nested parens in defaults)
    for part in content.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        if let Some(param) = parse_single_param(part) {
            params.push(param);
        }
    }

    params
}

/// Parse a single parameter like "duration: $dur:time = 600ms" or "$duration:time"
fn parse_single_param(content: &str) -> Option<FormParam> {
    let content = content.trim();

    // Try "key: $name:type = default" first
    // Type can include union syntax like ("x" | "y") so we allow various chars
    let named_re = Regex::new(r#"^([a-zA-Z_][a-zA-Z0-9_-]*):\s*\$([a-zA-Z_][a-zA-Z0-9_]*):([a-zA-Z_][a-zA-Z0-9_|()"' -]*)(\?)?\s*(?:=\s*(.+))?$"#).unwrap();
    if let Some(caps) = named_re.captures(content) {
        return Some(FormParam {
            key: Some(caps.get(1)?.as_str().to_string()),
            name: caps.get(2)?.as_str().to_string(),
            capture_type: caps.get(3)?.as_str().to_string(),
            optional: caps.get(4).is_some(),
            default: caps.get(5).map(|m| m.as_str().trim().to_string()),
        });
    }

    // Try "$name:type = default"
    let positional_re = Regex::new(r#"^\$([a-zA-Z_][a-zA-Z0-9_]*):([a-zA-Z_][a-zA-Z0-9_|()"' -]*)(\?)?\s*(?:=\s*(.+))?$"#).unwrap();
    if let Some(caps) = positional_re.captures(content) {
        return Some(FormParam {
            key: None,
            name: caps.get(1)?.as_str().to_string(),
            capture_type: caps.get(2)?.as_str().to_string(),
            optional: caps.get(3).is_some(),
            default: caps.get(4).map(|m| m.as_str().trim().to_string()),
        });
    }

    None
}

/// Parse body block
fn parse_body(content: &str) -> Option<FormBody> {
    let content = content.trim();
    if !content.starts_with('{') {
        return None;
    }

    // Extract body content
    let body_content = extract_braced_content(content, 1)?;

    // Find captures in body
    let capture_re = Regex::new(r"\$([a-zA-Z_][a-zA-Z0-9_]*):([a-zA-Z_][a-zA-Z0-9_]*)(\?)?").unwrap();
    let captures: Vec<FormCapture> = capture_re
        .captures_iter(&body_content)
        .map(|caps| FormCapture {
            name: caps.get(1).unwrap().as_str().to_string(),
            prefix: '$',
            capture_type: caps.get(2).unwrap().as_str().to_string(),
            optional: caps.get(3).is_some(),
        })
        .collect();

    if captures.is_empty() {
        None
    } else {
        Some(FormBody { captures })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_form() {
        let content = r#"%form { @fade-in($duration:time) }"#;
        let forms = extract_forms_from_content(content, "test.st");
        assert_eq!(forms.len(), 1);
        assert_eq!(forms[0].directive, "fade-in");
        assert_eq!(forms[0].params.len(), 1);
        assert_eq!(forms[0].params[0].name, "duration");
        assert_eq!(forms[0].params[0].capture_type, "time");
    }

    #[test]
    fn test_form_with_defaults() {
        let content = r#"%form { @fade-in($duration:time = 600ms) }"#;
        let forms = extract_forms_from_content(content, "test.st");
        assert_eq!(forms.len(), 1);
        assert_eq!(forms[0].params[0].default, Some("600ms".to_string()));
    }

    #[test]
    fn test_form_with_body() {
        let content = r#"%form { @scroll $name:ident { $body:keyframes } }"#;
        let forms = extract_forms_from_content(content, "test.st");
        assert_eq!(forms.len(), 1);
        assert_eq!(forms[0].inline_captures.len(), 1);
        assert_eq!(forms[0].inline_captures[0].name, "name");
        assert!(forms[0].body.is_some());
    }

    #[test]
    fn test_named_params() {
        let content = r#"%form { @fade-in(duration: $dur:time, threshold: $th:number) }"#;
        let forms = extract_forms_from_content(content, "test.st");
        assert_eq!(forms.len(), 1);
        assert_eq!(forms[0].params.len(), 2);
        assert_eq!(forms[0].params[0].key, Some("duration".to_string()));
        assert_eq!(forms[0].params[0].name, "dur");
    }
}
