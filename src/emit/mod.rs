//! Emit Layer - String Generation from IR Types
//!
//! This module is the ONLY place where IR types get converted to final
//! output strings. All stringification is centralized here for:
//! - Consistent formatting
//! - Easy minification toggle
//! - Source map generation
//!
//! Each emitter converts its IR type to a string:
//! - `js::emit()` - JsExpr/JsStmt -> JavaScript
//! - `css::emit()` - CssExpr -> CSS
//! - `glsl::emit()` - GlslExpr -> GLSL
//! - `html::emit()` - HtmlExpr -> HTML
//! - `sourcemap::SourceMapBuilder` -> Source Map V3 JSON

pub mod css;
pub mod css_parser;
pub mod emit_tokenizer;
pub mod glsl;
pub mod html;
pub mod html_hydrate;
pub mod html_reactive;
pub mod js;
pub mod js_parser;
pub mod metasystem_codegen;
pub mod st_expr;
pub mod sourcemap;
pub mod writer;

#[cfg(test)]
mod tests;

pub use sourcemap::{Mapping, SourceMapBuilder, SourceSpan, decode_vlq, encode_vlq};
pub use writer::{EmitOutput, SourceMapWriter, inline_css_source_map, inline_js_source_map};

use crate::ir::{CodeFragment, FragmentKind};

/// Emit options for controlling output format
#[derive(Debug, Clone, Default)]
pub struct EmitOptions {
    /// Minify output (remove whitespace, shorten names where safe)
    pub minify: bool,
    /// Indentation string (ignored if minify=true)
    pub indent: String,
    /// Line ending (ignored if minify=true)
    pub newline: String,
    /// Generate source maps for JS/CSS output
    pub source_maps: bool,
    /// Include source content in source maps (sourcesContent field)
    pub source_maps_include_content: bool,
}

impl EmitOptions {
    /// Default pretty-print options
    pub fn pretty() -> Self {
        Self {
            minify: false,
            indent: "  ".to_string(),
            newline: "\n".to_string(),
            source_maps: false,
            source_maps_include_content: false,
        }
    }

    /// Minified output
    pub fn minified() -> Self {
        Self {
            minify: true,
            indent: String::new(),
            newline: String::new(),
            source_maps: false,
            source_maps_include_content: false,
        }
    }

    /// Pretty-print with source maps enabled
    pub fn pretty_with_source_maps() -> Self {
        Self {
            minify: false,
            indent: "  ".to_string(),
            newline: "\n".to_string(),
            source_maps: true,
            source_maps_include_content: true,
        }
    }

    /// Minified with source maps enabled
    pub fn minified_with_source_maps() -> Self {
        Self {
            minify: true,
            indent: String::new(),
            newline: String::new(),
            source_maps: true,
            source_maps_include_content: true,
        }
    }

    /// Enable source maps on existing options
    pub fn with_source_maps(mut self, include_content: bool) -> Self {
        self.source_maps = true;
        self.source_maps_include_content = include_content;
        self
    }
}

/// Emit a code fragment to its target language
pub fn emit_fragment(fragment: &CodeFragment, opts: &EmitOptions) -> String {
    match &fragment.kind {
        FragmentKind::Js(stmts) => {
            js::emit_stmts(stmts, opts).expect("JS emit should not fail after resolution")
        }
        FragmentKind::Css(exprs) => css::emit_all(exprs, opts),
        FragmentKind::Glsl(expr) => glsl::emit(expr, opts),
        FragmentKind::Html(exprs) => html::emit_all(exprs, opts),
    }
}

/// Compiled output containing all generated code
#[derive(Debug, Clone, Default)]
pub struct CompiledOutput {
    pub js: String,
    pub css: String,
    pub glsl: Option<String>,
    pub html: Option<String>,
    /// Source map for JavaScript output (V3 JSON format)
    pub js_source_map: Option<String>,
    /// Source map for CSS output (V3 JSON format)
    pub css_source_map: Option<String>,
}

impl CompiledOutput {
    /// Combine multiple outputs
    pub fn merge(&mut self, other: CompiledOutput) {
        if !other.js.is_empty() {
            if !self.js.is_empty() {
                self.js.push('\n');
            }
            self.js.push_str(&other.js);
        }
        if !other.css.is_empty() {
            if !self.css.is_empty() {
                self.css.push('\n');
            }
            self.css.push_str(&other.css);
        }
        if let Some(glsl) = other.glsl {
            self.glsl = Some(glsl);
        }
        if let Some(html) = other.html {
            self.html = Some(html);
        }
        // Note: Source maps are not merged - this would require remapping offsets.
        // When merging, the caller should rebuild source maps if needed.
        if other.js_source_map.is_some() {
            self.js_source_map = other.js_source_map;
        }
        if other.css_source_map.is_some() {
            self.css_source_map = other.css_source_map;
        }
    }
}
