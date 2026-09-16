//! GLSL Emitter
//!
//! Converts GlslExpr IR types to GLSL shader code strings.
//!
//! GLSL is typically written as raw code in %emit glsl blocks,
//! so this emitter mostly handles the Raw case. The Program
//! variant wraps vertex and fragment shaders together.

use crate::ir::GlslExpr;

use super::EmitOptions;

/// Emit a GLSL expression
pub fn emit(expr: &GlslExpr, opts: &EmitOptions) -> String {
    match expr {
        GlslExpr::Program { vertex, fragment } => emit_program(vertex, fragment, opts),
        GlslExpr::Raw(s) => {
            if opts.minify {
                minify_glsl(s)
            } else {
                s.clone()
            }
        }
    }
}

/// Emit a shader program with both vertex and fragment shaders
fn emit_program(vertex: &str, fragment: &str, opts: &EmitOptions) -> String {
    // For now, just concatenate with a separator
    // In practice, these would be compiled separately
    if opts.minify {
        format!(
            "// VERTEX\n{}\n// FRAGMENT\n{}",
            minify_glsl(vertex),
            minify_glsl(fragment)
        )
    } else {
        format!(
            "// === VERTEX SHADER ===\n{}\n\n// === FRAGMENT SHADER ===\n{}",
            vertex, fragment
        )
    }
}

/// Basic GLSL minification
///
/// Removes unnecessary whitespace while preserving:
/// - Preprocessor directives (must be on their own line)
/// - Comments (optionally removed)
fn minify_glsl(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let mut prev_was_space = true;

    for line in source.lines() {
        let trimmed = line.trim();

        // Preserve preprocessor directives on their own line
        if trimmed.starts_with('#') {
            if !result.is_empty() && !result.ends_with('\n') {
                result.push('\n');
            }
            result.push_str(trimmed);
            result.push('\n');
            prev_was_space = true;
            continue;
        }

        // Skip empty lines and single-line comments
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        // Process each character
        for c in trimmed.chars() {
            if c.is_whitespace() {
                if !prev_was_space {
                    result.push(' ');
                    prev_was_space = true;
                }
            } else {
                // Remove spaces around operators where safe
                if is_glsl_operator(c) && prev_was_space && !result.is_empty() {
                    // Remove trailing space before operator
                    if result.ends_with(' ') {
                        result.pop();
                    }
                }
                result.push(c);
                prev_was_space = false;
            }
        }
    }

    result.trim().to_string()
}

/// Check if a character is a GLSL operator
fn is_glsl_operator(c: char) -> bool {
    matches!(
        c,
        '=' | '+'
            | '-'
            | '*'
            | '/'
            | '<'
            | '>'
            | '!'
            | '&'
            | '|'
            | '^'
            | '%'
            | '('
            | ')'
            | '{'
            | '}'
            | '['
            | ']'
            | ';'
            | ','
            | '.'
    )
}
