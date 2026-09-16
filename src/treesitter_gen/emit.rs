//! Grammar emitter - generates tree-sitter grammar.js from directive rules
//!
//! This module provides functions to generate tree-sitter grammar rules from
//! Spacetime directive definitions. Some helper functions are kept for future
//! enhancements to support more specific directive parsing.

// Allow unused functions - kept for future enhancements and testing
#![allow(dead_code)]

use super::capture_map::{apply_modifier, capture_to_ts, capture_to_ts_value};
use super::walker::extract_primary_capture;
use super::{DirectiveRule, InlineRule, ParamRule};

const BASE_GRAMMAR: &str = include_str!("base_grammar.js");

/// Generate complete grammar.js from directive rules
pub fn generate_grammar(directives: &[DirectiveRule]) -> String {
    use std::collections::HashSet;

    // Filter out directives with problematic names
    let valid_directives: Vec<_> = directives
        .iter()
        .filter(|d| {
            let name = d.name.trim_start_matches('@');
            // Skip directives with special prefixes or invalid characters
            !name.starts_with('$')
                && !name.starts_with('&')
                && !name.contains(':')
                && !name.is_empty()
        })
        .collect();

    // Deduplicate directives by sanitized name (keep first occurrence)
    let mut seen_names = HashSet::new();
    let unique_directives: Vec<_> = valid_directives
        .into_iter()
        .filter(|d| {
            let name = sanitize_name(&d.name);
            seen_names.insert(name)
        })
        .collect();

    // Generate directive name choices (simple string literals)
    let directive_names: Vec<String> = unique_directives
        .iter()
        .map(|d| {
            let name = d.name.trim_start_matches('@');
            format!("'{}'", name)
        })
        .collect();

    let directive_choice = if directive_names.is_empty() {
        "$.identifier".to_string()
    } else {
        directive_names.join(",\n      ")
    };

    // No longer generating individual directive rules - using generic parsing
    BASE_GRAMMAR
        .replace(
            "// {{DIRECTIVE_RULES}}",
            "// Directive rules omitted - using generic directive parsing",
        )
        .replace("// {{DIRECTIVE_CHOICE}}", &directive_choice)
}

fn sanitize_name(name: &str) -> String {
    // Remove leading @ and $ for directive names, replace invalid chars with _
    name.trim_start_matches('@')
        .trim_start_matches('$')
        .trim_start_matches('&')
        .replace("-", "_")
        .replace(".", "_")
}

/// Emit a single directive rule
fn emit_directive_rule(rule: &DirectiveRule) -> String {
    let name = sanitize_name(&rule.name);
    // Strip @ prefix from directive name for the literal
    let directive_literal = rule.name.trim_start_matches('@');
    let mut parts = vec!["'@'".to_string(), format!("'{}'", directive_literal)];

    // Inline elements (before parens): @on $event:event
    for inline in &rule.inline_elements {
        parts.push(emit_inline_element(inline));
    }

    // Params in parentheses
    if !rule.params.is_empty() {
        let params_js = emit_params(&rule.params);
        parts.push("'('".to_string());
        parts.push(params_js);
        parts.push("')'".to_string());
    }

    // Optional body
    if rule.has_body {
        if rule.body_params.is_empty() {
            parts.push("optional($.block)".to_string());
        } else {
            // Structured body with specific params
            let body_content = emit_body_params(&rule.body_params);
            parts.push(format!("optional(seq('{{', {}, '}}'))", body_content));
        }
    }

    // Optional trailing semicolon
    parts.push("optional(';')".to_string());

    format!(
        "{}_directive: $ => seq(\n      {}\n    )",
        name,
        parts.join(",\n      ")
    )
}

/// Emit inline element rule
fn emit_inline_element(inline: &InlineRule) -> String {
    match inline {
        InlineRule::Capture {
            capture_type,
            modifier,
            alias,
            ..
        } => {
            let ts_rule = capture_to_ts_value(capture_type);
            let base = apply_modifier(ts_rule, *modifier);
            if let Some(alias_capture) = alias {
                // Handle "as $alias:type" pattern
                format!(
                    "seq({}, 'as', {})",
                    base,
                    emit_inline_element(alias_capture)
                )
            } else {
                base
            }
        }
        InlineRule::Literal(lit) => {
            if is_keyword(lit) {
                format!("'{}'", lit)
            } else {
                format!("'{}'", escape_js_string(lit))
            }
        }
        InlineRule::Comparison {
            operator,
            capture_type,
            modifier,
        } => {
            let ts_rule = capture_to_ts_value(capture_type);
            let value = apply_modifier(ts_rule, *modifier);
            format!("seq('{}', {})", operator, value)
        }
        InlineRule::KeywordBlock {
            keyword,
            body_params,
            modifier,
        } => {
            let body = if body_params.is_empty() {
                "repeat($._block_item)".to_string()
            } else {
                emit_body_params(body_params)
            };
            let block = format!("seq('{}', '{{', {}, '}}')", keyword, body);
            apply_modifier(&block, *modifier)
        }
        InlineRule::PseudoSelector {
            name,
            body_params,
            modifier,
        } => {
            let body = if body_params.is_empty() {
                "repeat($._block_item)".to_string()
            } else {
                emit_body_params(body_params)
            };
            let block = format!("seq('(', ':', '{}', '{{', {}, '}}', ')')", name, body);
            apply_modifier(&block, *modifier)
        }
        InlineRule::PseudoClass { name, body_params } => {
            let body = if body_params.is_empty() {
                "repeat($._block_item)".to_string()
            } else {
                emit_body_params(body_params)
            };
            format!("seq(':', '{}', '{{', {}, '}}')", name, body)
        }
    }
}

/// Emit parameters as tree-sitter rules
fn emit_params(params: &[ParamRule]) -> String {
    if params.is_empty() {
        return "optional($._args_inner)".to_string();
    }

    let param_rules: Vec<String> = params
        .iter()
        .enumerate()
        .filter(|(_, p)| !p.name.is_empty()) // Skip params with empty names
        .map(|(i, p)| {
            let param_value = emit_param_value(p);

            // Named param: name: value
            let param_js = format!("seq('{}', ':', {})", p.name, param_value);

            // First param can be positional or named, rest are optional with comma
            if i == 0 {
                // First param: can be just the value or named
                format!("choice({}, {})", param_value, param_js)
            } else {
                // Subsequent params: optional, comma-separated
                format!("optional(seq(',', choice({}, {})))", param_value, param_js)
            }
        })
        .collect();

    if param_rules.is_empty() {
        return "optional($._args_inner)".to_string();
    }

    param_rules.join(",\n        ")
}

/// Emit a single parameter's value pattern
fn emit_param_value(param: &ParamRule) -> String {
    if param.elements.is_empty() {
        // No elements - default to any value
        return "$._value".to_string();
    }

    if param.elements.len() == 1 {
        // Single element
        let el = &param.elements[0];
        match el {
            InlineRule::Capture {
                capture_type,
                modifier,
                alias,
                ..
            } => {
                let ts_rule = capture_to_ts_value(capture_type);
                let base = apply_modifier(ts_rule, *modifier);

                // Handle alias pattern
                if let Some(alias_el) = alias {
                    return format!(
                        "seq({}, optional(seq('as', {})))",
                        base,
                        emit_inline_element(alias_el)
                    );
                }

                // Handle default value - params with defaults are optional
                if param.default.is_some() {
                    return format!("optional({})", base);
                }

                base
            }
            _ => emit_inline_element(el),
        }
    } else {
        // Multiple elements - emit as sequence
        let elements: Vec<String> = param.elements.iter().map(emit_inline_element).collect();
        format!("seq({})", elements.join(", "))
    }
}

/// Emit body params (structured body content)
fn emit_body_params(params: &[ParamRule]) -> String {
    if params.is_empty() {
        return "repeat($._block_item)".to_string();
    }

    let param_rules: Vec<String> = params
        .iter()
        .map(|p| {
            let (capture_type, modifier) = extract_primary_capture(p);
            let ts_rule = capture_to_ts(&capture_type);

            // Body params are typically property-like: name: value;
            let decl = format!(
                "seq('{}', ':', {}, optional(';'))",
                p.name,
                apply_modifier(ts_rule, modifier)
            );

            // Make body params optional
            format!("optional({})", decl)
        })
        .collect();

    format!("seq({})", param_rules.join(", "))
}

/// Check if a literal is a keyword
fn is_keyword(s: &str) -> bool {
    matches!(
        s,
        "is" | "in"
            | "as"
            | "if"
            | "else"
            | "for"
            | "while"
            | "true"
            | "false"
            | "null"
            | "from"
            | "to"
    )
}

/// Escape string for JavaScript
fn escape_js_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

/// Generate custom type rules from CaptureTypeDefAst
pub fn generate_custom_type_rules(custom_types: &[super::CustomTypeRule]) -> Vec<(String, String)> {
    custom_types
        .iter()
        .map(|ct| {
            let name = format!("{}_type", sanitize_name(&ct.name));
            (name, ct.rule.clone())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::meta_ast::{CaptureModifier, CaptureType, ParamDefault};

    #[test]
    fn test_emit_simple_directive() {
        let rule = DirectiveRule {
            name: "fade-in".to_string(),
            params: vec![ParamRule {
                name: "duration".to_string(),
                elements: vec![InlineRule::Capture {
                    var_name: "duration".to_string(),
                    capture_type: CaptureType::Time,
                    modifier: CaptureModifier::Required,
                    alias: None,
                }],
                default: Some(ParamDefault::String("300ms".to_string())),
            }],
            inline_elements: vec![],
            has_body: false,
            body_params: vec![],
        };

        let js = emit_directive_rule(&rule);

        assert!(js.contains("fade_in_directive"));
        assert!(js.contains("'@'"));
        assert!(js.contains("'fade-in'"));
        assert!(js.contains("$.duration"));
    }

    #[test]
    fn test_emit_directive_with_body() {
        let rule = DirectiveRule {
            name: "data".to_string(),
            params: vec![ParamRule {
                name: "src".to_string(),
                elements: vec![InlineRule::Capture {
                    var_name: "src".to_string(),
                    capture_type: CaptureType::String,
                    modifier: CaptureModifier::Required,
                    alias: None,
                }],
                default: None,
            }],
            inline_elements: vec![],
            has_body: true,
            body_params: vec![],
        };

        let js = emit_directive_rule(&rule);

        assert!(js.contains("data_directive"));
        assert!(js.contains("optional($.block)"));
    }

    #[test]
    fn test_emit_inline_elements() {
        let rule = DirectiveRule {
            name: "on".to_string(),
            params: vec![],
            inline_elements: vec![InlineRule::Capture {
                var_name: "event".to_string(),
                capture_type: CaptureType::Event,
                modifier: CaptureModifier::Required,
                alias: None,
            }],
            has_body: true,
            body_params: vec![],
        };

        let js = emit_directive_rule(&rule);

        assert!(js.contains("on_directive"));
        assert!(js.contains("$.identifier")); // Event is mapped to identifier
    }

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("fade-in"), "fade_in");
        assert_eq!(sanitize_name("fade-in-up"), "fade_in_up");
        assert_eq!(sanitize_name("data.fetch"), "data_fetch");
    }

    #[test]
    fn test_generate_grammar_empty() {
        let grammar = generate_grammar(&[]);
        assert!(grammar.contains("spacetime"));
        assert!(grammar.contains("$.generic_directive"));
    }

    #[test]
    fn test_generate_grammar_with_directives() {
        let directives = vec![DirectiveRule {
            name: "test".to_string(),
            params: vec![],
            inline_elements: vec![],
            has_body: false,
            body_params: vec![],
        }];

        let grammar = generate_grammar(&directives);
        // Now we generate directive names as string choices
        assert!(grammar.contains("'test'"));
        assert!(grammar.contains("_known_directive_name"));
    }
}
