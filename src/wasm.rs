//! WASM bindings for Spacetime compiler and LSP features.
//!
//! Provides browser-accessible functions for:
//! - `compile()` - Compile Spacetime source to CSS/JS
//! - `completions()` - Get LSP-style completions at a position
//! - `hover()` - Get hover documentation at a position
//! - `diagnostics()` - Get validation diagnostics for a document
//!
//! ## Usage from JavaScript
//!
//! ```javascript
//! import init, { compile, completions, hover, diagnostics } from './spacetime.js';
//!
//! await init();
//!
//! const result = compile('.hero { @fade-in { opacity: 0 -> 1 } }');
//! console.log(result.css, result.js);
//! ```

#![cfg(feature = "wasm")]

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::compiler::{CompileOptions, compile as compile_ast};
use crate::lsp::form_registry::FormRegistry;
use crate::parser::parse;

// =============================================================================
// Initialization
// =============================================================================

/// Initialize panic hook for better error messages in browser console.
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

// =============================================================================
// Compile API
// =============================================================================

/// Compile result returned to JavaScript.
#[derive(Serialize, Deserialize)]
pub struct CompileResult {
    /// Generated CSS code
    pub css: String,
    /// Generated JavaScript code
    pub js: String,
    /// Compilation errors (empty if successful)
    pub errors: Vec<CompileError>,
}

/// A compilation error with location information.
#[derive(Serialize, Deserialize)]
pub struct CompileError {
    /// Error message
    pub message: String,
    /// Line number (0-indexed)
    pub line: u32,
    /// Column number (0-indexed)
    pub column: u32,
    /// Error code (e.g., "E003")
    pub code: Option<String>,
}

/// Compile Spacetime source code to CSS and JavaScript.
///
/// Returns a JavaScript object with `css`, `js`, and `errors` properties.
///
/// # Arguments
/// * `source` - Spacetime source code
///
/// # Returns
/// A `CompileResult` object serialized as a JavaScript value.
#[wasm_bindgen]
pub fn compile(source: &str) -> JsValue {
    match compile_source(source) {
        Ok((css, js)) => {
            let result = CompileResult {
                css,
                js,
                errors: vec![],
            };
            serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
        }
        Err(errors) => {
            let result = CompileResult {
                css: String::new(),
                js: String::new(),
                errors,
            };
            serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
        }
    }
}

/// Internal compile function that returns Result.
fn compile_source(source: &str) -> Result<(String, String), Vec<CompileError>> {
    // Parse the source
    let file = match parse(source) {
        Ok(file) => file,
        Err(e) => {
            // Extract line/column from parser error if possible
            let (line, column) = extract_error_position(&e.to_string());
            return Err(vec![CompileError {
                message: e.to_string(),
                line,
                column,
                code: Some("E003".to_string()),
            }]);
        }
    };

    // Run the full 5-layer compile pipeline
    let output = compile_ast(&file, CompileOptions::default());

    Ok((output.css, output.js))
}

/// Extract line and column from a parser error message.
fn extract_error_position(error: &str) -> (u32, u32) {
    // Look for pattern like " --> 5:10"
    for line in error.lines() {
        if let Some(arrow_pos) = line.find("-->") {
            let after_arrow = line[arrow_pos + 3..].trim();
            if let Some(colon_pos) = after_arrow.find(':') {
                if let (Ok(line_num), Ok(col_num)) = (
                    after_arrow[..colon_pos].trim().parse::<u32>(),
                    after_arrow[colon_pos + 1..].trim().parse::<u32>(),
                ) {
                    // Convert to 0-indexed
                    return (line_num.saturating_sub(1), col_num.saturating_sub(1));
                }
            }
        }
    }
    (0, 0)
}

// =============================================================================
// Completions API
// =============================================================================

/// Completion item for JavaScript.
#[derive(Serialize, Deserialize)]
pub struct WasmCompletionItem {
    /// Label to display
    pub label: String,
    /// Kind of completion (e.g., "Function", "Property")
    pub kind: String,
    /// Brief description
    pub detail: Option<String>,
    /// Text to insert
    pub insert_text: Option<String>,
    /// Full documentation
    pub documentation: Option<String>,
}

/// Get completions at a cursor position.
///
/// # Arguments
/// * `source` - Spacetime source code
/// * `line` - Line number (0-indexed)
/// * `column` - Column number (0-indexed)
///
/// # Returns
/// Array of completion items as a JavaScript value.
#[wasm_bindgen]
pub fn completions(source: &str, line: u32, column: u32) -> JsValue {
    use crate::lsp::form_registry::FormRegistry;

    // Create document state manually (simplified version for WASM)
    let registry = FormRegistry::from_compiler();

    // Extract context and generate completions
    let offset = calculate_offset(source, line, column);
    let context = crate::lsp::completion::extract_context_at_position(source, offset);

    // Generate completions based on context
    let items = match context {
        crate::lsp::completion::CompletionContext::AfterAt { prefix } => registry
            .directive_completions(&prefix)
            .into_iter()
            .map(|(name, sig)| WasmCompletionItem {
                label: name,
                kind: "Function".to_string(),
                detail: Some(sig.format_signature()),
                insert_text: Some(sig.name.clone()),
                documentation: sig.documentation.clone(),
            })
            .collect::<Vec<_>>(),
        crate::lsp::completion::CompletionContext::InsideParams {
            directive_name,
            provided_params,
        } => {
            if let Some(sig) = registry.get_directive(&directive_name) {
                sig.params
                    .iter()
                    .filter(|p| !provided_params.contains(&p.name))
                    .map(|p| WasmCompletionItem {
                        label: p.name.clone(),
                        kind: "Property".to_string(),
                        detail: Some(p.format()),
                        insert_text: Some(format!("{}: ", p.name)),
                        documentation: None,
                    })
                    .collect()
            } else {
                vec![]
            }
        }
        _ => vec![],
    };

    serde_wasm_bindgen::to_value(&items).unwrap_or(JsValue::NULL)
}

/// Calculate byte offset from line and column.
fn calculate_offset(source: &str, line: u32, column: u32) -> usize {
    let mut current_line = 0u32;
    let mut offset = 0usize;

    for (i, ch) in source.char_indices() {
        if current_line == line {
            // Count characters in this line up to column
            let mut col = 0u32;
            for (j, c) in source[i..].char_indices() {
                if col == column {
                    return i + j;
                }
                if c == '\n' {
                    break;
                }
                col += 1;
            }
            return i + (column as usize).min(source.len() - i);
        }
        if ch == '\n' {
            current_line += 1;
        }
        offset = i + ch.len_utf8();
    }

    offset.min(source.len())
}

// =============================================================================
// Hover API
// =============================================================================

/// Hover result for JavaScript.
#[derive(Serialize, Deserialize)]
pub struct WasmHover {
    /// Hover contents (markdown)
    pub contents: String,
    /// Range where the hover applies
    pub range: Option<WasmRange>,
}

/// A text range.
#[derive(Serialize, Deserialize)]
pub struct WasmRange {
    /// Start line (0-indexed)
    pub start_line: u32,
    /// Start column (0-indexed)
    pub start_column: u32,
    /// End line (0-indexed)
    pub end_line: u32,
    /// End column (0-indexed)
    pub end_column: u32,
}

/// Get hover information at a cursor position.
///
/// # Arguments
/// * `source` - Spacetime source code
/// * `line` - Line number (0-indexed)
/// * `column` - Column number (0-indexed)
///
/// # Returns
/// Hover information as a JavaScript value, or null if no hover is available.
#[wasm_bindgen]
pub fn hover(source: &str, line: u32, column: u32) -> JsValue {
    let registry = FormRegistry::from_compiler();
    let offset = calculate_offset(source, line, column);

    let context = crate::lsp::hover::extract_hover_context(source, offset);

    match context {
        crate::lsp::hover::HoverContext::DirectiveName { name, range } => {
            if let Some(sig) = registry.get_directive(&name) {
                let contents = format_directive_hover_markdown(sig);
                let wasm_range = offset_range_to_wasm_range(source, range);

                let result = WasmHover {
                    contents,
                    range: Some(wasm_range),
                };
                serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
            } else {
                JsValue::NULL
            }
        }
        crate::lsp::hover::HoverContext::ParamName {
            directive_name,
            param_name,
            range,
        } => {
            if let Some(sig) = registry.get_directive(&directive_name) {
                if let Some(param) = sig.params.iter().find(|p| p.name == param_name) {
                    let contents = format_param_hover_markdown(param, sig);
                    let wasm_range = offset_range_to_wasm_range(source, range);

                    let result = WasmHover {
                        contents,
                        range: Some(wasm_range),
                    };
                    return serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL);
                }
            }
            JsValue::NULL
        }
        crate::lsp::hover::HoverContext::Preset { name, range } => {
            let contents = format!(
                "```spacetime\n~{}\n```\n\n*Preset reference `~{}`*",
                name, name
            );
            let wasm_range = offset_range_to_wasm_range(source, range);
            let result = WasmHover {
                contents,
                range: Some(wasm_range),
            };
            serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
        }
        crate::lsp::hover::HoverContext::Variable { name, range } => {
            let contents = format!(
                "```spacetime\n${}\n```\n\n*State variable `${}`*",
                name, name
            );
            let wasm_range = offset_range_to_wasm_range(source, range);
            let result = WasmHover {
                contents,
                range: Some(wasm_range),
            };
            serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
        }
        crate::lsp::hover::HoverContext::Template { name, range } => {
            let contents = format!(
                "```spacetime\n&{}\n```\n\n*Template reference `&{}`*",
                name, name
            );
            let wasm_range = offset_range_to_wasm_range(source, range);
            let result = WasmHover {
                contents,
                range: Some(wasm_range),
            };
            serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
        }
        crate::lsp::hover::HoverContext::None => JsValue::NULL,
    }
}

/// Format directive hover as markdown.
fn format_directive_hover_markdown(sig: &crate::lsp::form_registry::DirectiveSignature) -> String {
    let mut md = String::new();

    md.push_str("```spacetime\n");
    md.push_str(&sig.format_signature());
    md.push_str("\n```\n\n");

    md.push_str(&format!("Directive created by `{}`\n\n", sig.macro_name));

    if !sig.params.is_empty() {
        md.push_str("### Parameters\n\n");
        for param in &sig.params {
            let required = if param.is_required() {
                " (required)"
            } else {
                ""
            };
            md.push_str(&format!(
                "- **{}**: `{}`{}\n",
                param.name,
                param.format(),
                required
            ));
        }
    }

    if let Some(ref doc) = sig.documentation {
        md.push_str("\n---\n\n");
        md.push_str(doc);
    }

    md
}

/// Format parameter hover as markdown.
fn format_param_hover_markdown(
    param: &crate::lsp::form_registry::DirectiveParam,
    sig: &crate::lsp::form_registry::DirectiveSignature,
) -> String {
    let mut md = String::new();

    md.push_str("```spacetime\n");
    md.push_str(&param.format());
    md.push_str("\n```\n\n");

    md.push_str(&format!("Parameter of `@{}`\n\n", sig.name));

    if param.is_required() {
        md.push_str("*This parameter is required.*\n");
    } else {
        md.push_str("*This parameter is optional.*\n");
    }

    md
}

/// Convert byte offset range to WASM range.
fn offset_range_to_wasm_range(source: &str, range: (usize, usize)) -> WasmRange {
    let (start_line, start_column) = offset_to_line_col(source, range.0);
    let (end_line, end_column) = offset_to_line_col(source, range.1);

    WasmRange {
        start_line,
        start_column,
        end_line,
        end_column,
    }
}

/// Convert byte offset to line and column.
fn offset_to_line_col(source: &str, offset: usize) -> (u32, u32) {
    let mut line = 0u32;
    let mut col = 0u32;
    let mut current_offset = 0usize;

    for ch in source.chars() {
        if current_offset >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
        current_offset += ch.len_utf8();
    }

    (line, col)
}

// =============================================================================
// Diagnostics API
// =============================================================================

/// Get diagnostics for a Spacetime document.
///
/// # Arguments
/// * `source` - Spacetime source code
///
/// # Returns
/// Array of diagnostic errors as a JavaScript value.
#[wasm_bindgen]
pub fn diagnostics(source: &str) -> JsValue {
    let mut errors = Vec::new();

    // ONE DIAGNOSTIC ENGINE, THREE FRONT-ENDS (GH-28, PLAN-141 W5).
    //
    // This used to run its OWN rule: a raw text scan for `@name` runs
    // (`find_directive_calls`, now deleted) plus a registry lookup, emitting
    // E171 "Unknown directive" when the lookup missed. That is a second, worse
    // parser over the same bytes — it could not tell a directive from an email
    // address in a string, and it disagreed with the compiler about what
    // compiles: the same source was clean under `check` and erroneous here.
    //
    // GH-28 deleted the LSP's parallel registry for exactly this reason. This
    // was the surviving fragment of the same defect, in the third front-end.
    // It now publishes the compiler's diagnostics, like `check` and the LSP do.
    match parse(source) {
        Ok(file) => {
            let (meta, _errs) = crate::compiler::cached_stdlib_registry();
            for d in crate::analysis::diagnostics::collect_document_diagnostics(
                &file,
                None,
                Some(&meta),
            ) {
                let (line, column) = d
                    .span
                    .map(|s| offset_to_line_col(source, s.start))
                    .unwrap_or((1, 1));
                errors.push(CompileError {
                    message: d.message.clone(),
                    line,
                    column,
                    code: Some(d.code.as_str().to_string()),
                });
            }
        }
        Err(e) => {
            // Parse failed
            let (line, column) = extract_error_position(&e.to_string());
            errors.push(CompileError {
                message: e.to_string(),
                line,
                column,
                code: Some("E003".to_string()),
            });
        }
    }

    serde_wasm_bindgen::to_value(&errors).unwrap_or(JsValue::NULL)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_offset() {
        let source = "line 0\nline 1\nline 2";
        assert_eq!(calculate_offset(source, 0, 0), 0);
        assert_eq!(calculate_offset(source, 0, 4), 4);
        assert_eq!(calculate_offset(source, 1, 0), 7);
        assert_eq!(calculate_offset(source, 1, 4), 11);
        assert_eq!(calculate_offset(source, 2, 0), 14);
    }

    #[test]
    fn test_offset_to_line_col() {
        let source = "line 0\nline 1\nline 2";
        assert_eq!(offset_to_line_col(source, 0), (0, 0));
        assert_eq!(offset_to_line_col(source, 4), (0, 4));
        assert_eq!(offset_to_line_col(source, 7), (1, 0));
        assert_eq!(offset_to_line_col(source, 11), (1, 4));
    }

    #[test]
    fn test_extract_error_position() {
        let error = " --> 5:10\n  |\n5 | bad code";
        let (line, col) = extract_error_position(error);
        assert_eq!(line, 4); // 0-indexed
        assert_eq!(col, 9); // 0-indexed
    }
}
