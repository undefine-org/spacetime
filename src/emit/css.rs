//! CSS Emitter
//!
//! Converts CssExpr IR types to CSS strings.

use std::collections::HashMap;

use crate::ir::{CssDecl, CssExpr, CssPart, KeyframeBlock, Marker};

use super::EmitOptions;

/// Context for CSS placeholder resolution
pub struct CssContext {
    /// Parameter values (from %param)
    pub params: HashMap<String, String>,
    /// Element references (from %&element)
    pub elements: HashMap<String, String>,
}

impl CssContext {
    pub fn new() -> Self {
        Self {
            params: HashMap::new(),
            elements: HashMap::new(),
        }
    }

    /// Add a parameter value
    pub fn with_param(mut self, name: &str, value: &str) -> Self {
        self.params.insert(name.to_string(), value.to_string());
        self
    }

    /// Add an element reference
    pub fn with_element(mut self, name: &str, value: &str) -> Self {
        self.elements.insert(name.to_string(), value.to_string());
        self
    }
}

impl Default for CssContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolve all placeholders in CSS parts
pub fn resolve_parts(parts: &[CssPart], ctx: &CssContext) -> Vec<CssPart> {
    parts.iter().map(|p| resolve_part(p, ctx)).collect()
}

/// Resolve placeholders in a single CSS part
fn resolve_part(part: &CssPart, ctx: &CssContext) -> CssPart {
    match part {
        CssPart::Raw(s) => CssPart::Raw(s.clone()),
        CssPart::Expr(e) => CssPart::Expr(resolve_expr(e, ctx)),
        CssPart::Marker(m) => resolve_marker(m, ctx),
    }
}

/// Resolve a CSS expression (recursively for Composite)
pub fn resolve_expr(expr: &CssExpr, ctx: &CssContext) -> CssExpr {
    match expr {
        CssExpr::Raw(s) => CssExpr::Raw(s.clone()),
        CssExpr::Rule {
            selector,
            declarations,
        } => CssExpr::Rule {
            selector: selector.clone(),
            declarations: declarations.clone(),
        },
        CssExpr::Keyframes { name, frames } => CssExpr::Keyframes {
            name: name.clone(),
            frames: frames.clone(),
        },
        CssExpr::Composite(parts) => CssExpr::Composite(resolve_parts(parts, ctx)),
    }
}

/// Resolve a marker to its value
fn resolve_marker(marker: &Marker<CssExpr>, ctx: &CssContext) -> CssPart {
    match marker {
        Marker::Param { name, .. } => {
            if let Some(value) = ctx.params.get(name) {
                CssPart::Raw(value.clone())
            } else if name == "self" {
                // `self` is a runtime-only reference (e.g. @when &self.scrollSpy.isActive)
                // that legitimately appears in marker IR but has no CSS meaning. Skip silently.
                CssPart::Raw(String::new())
            } else {
                log::error!(
                    "Unknown param '{}' during CSS marker resolution. \
                     Available params: {:?}.",
                    name,
                    ctx.params.keys().collect::<Vec<_>>()
                );
                CssPart::Raw(format!("/* spacetime: unknown param '{}' */", name))
            }
        }
        Marker::Element(name) => {
            if let Some(value) = ctx.elements.get(name) {
                CssPart::Raw(value.clone())
            } else {
                log::error!(
                    "Unknown element '{}' during CSS marker resolution. \
                     Available elements: {:?}.",
                    name,
                    ctx.elements.keys().collect::<Vec<_>>()
                );
                CssPart::Raw(format!("/* spacetime: unknown element '{}' */", name))
            }
        }
        Marker::Signal(name) => {
            log::error!(
                "Signal '${}' appeared in CSS marker resolution. \
                 Signals are not valid in CSS.",
                name
            );
            CssPart::Raw(format!("/* spacetime: signal '{}' invalid in CSS */", name))
        }
        Marker::VarRef {
            var,
            field,
            in_string: _,
        } => {
            let field_str = field
                .as_ref()
                .map(|f| format!(".{}", f))
                .unwrap_or_default();
            log::error!(
                "VarRef '%${}{}' appeared in CSS marker resolution. \
                 VarRefs are not valid in CSS.",
                var,
                field_str
            );
            CssPart::Raw(format!(
                "/* spacetime: varref '%${}{}' invalid in CSS */",
                var, field_str
            ))
        }
        Marker::Directive { keyword, .. } => {
            log::error!(
                "Directive '%{}' appeared in CSS marker resolution. \
                 Directives are not valid in CSS.",
                keyword
            );
            CssPart::Raw(format!(
                "/* spacetime: directive '%{}' invalid in CSS */",
                keyword
            ))
        }
    }
}

/// Emit a CSS expression
pub fn emit(expr: &CssExpr, opts: &EmitOptions) -> String {
    match expr {
        CssExpr::Rule {
            selector,
            declarations,
        } => emit_rule(selector, declarations, opts),
        CssExpr::Keyframes { name, frames } => emit_keyframes(name, frames, opts),
        CssExpr::Raw(s) => s.clone(),
        CssExpr::Composite(parts) => parts
            .iter()
            .map(|p| emit_part(p, opts))
            .collect::<Vec<_>>()
            .join(""),
    }
}

/// Emit a CssPart
fn emit_part(part: &CssPart, opts: &EmitOptions) -> String {
    match part {
        CssPart::Raw(s) => s.clone(),
        CssPart::Expr(e) => emit(e, opts),
        CssPart::Marker(m) => emit_marker(m),
    }
}

/// Emit a CSS marker - markers MUST be resolved before emission
fn emit_marker(marker: &Marker<CssExpr>) -> String {
    // Markers MUST be resolved before emission.
    // If we reach here, it's a compiler bug.
    panic!(
        "BUG: Unresolved marker {:?} reached CSS emission. \
         This is a compiler bug - please file an issue at \
         https://github.com/anthropics/spacetime/issues",
        marker
    );
}

/// Emit multiple CSS expressions
pub fn emit_all(exprs: &[CssExpr], opts: &EmitOptions) -> String {
    let nl = if opts.minify { "" } else { "\n\n" };
    exprs
        .iter()
        .map(|e| emit(e, opts))
        .collect::<Vec<_>>()
        .join(nl)
}

/// Emit a CSS rule
fn emit_rule(selector: &str, declarations: &[CssDecl], opts: &EmitOptions) -> String {
    if opts.minify {
        let decls = declarations
            .iter()
            .map(emit_decl)
            .collect::<Vec<_>>()
            .join(";");
        format!("{}{{{}}}", selector, decls)
    } else {
        let nl = &opts.newline;
        let indent = &opts.indent;
        let decls = declarations
            .iter()
            .map(|d| format!("{}{};", indent, emit_decl(d)))
            .collect::<Vec<_>>()
            .join(nl);
        format!("{} {{{}{}{}}}", selector, nl, decls, nl)
    }
}

/// Emit @keyframes block
fn emit_keyframes(name: &str, frames: &[KeyframeBlock], opts: &EmitOptions) -> String {
    if opts.minify {
        let frames_str = frames
            .iter()
            .map(|f| emit_keyframe_block(f, opts))
            .collect::<Vec<_>>()
            .join("");
        format!("@keyframes {}{{{}}}", name, frames_str)
    } else {
        let nl = &opts.newline;
        let indent = &opts.indent;
        let frames_str = frames
            .iter()
            .map(|f| {
                let block = emit_keyframe_block(f, opts);
                // Indent each line of the block
                block
                    .lines()
                    .map(|l| format!("{}{}", indent, l))
                    .collect::<Vec<_>>()
                    .join(nl)
            })
            .collect::<Vec<_>>()
            .join(nl);
        format!("@keyframes {} {{{}{}{}}}", name, nl, frames_str, nl)
    }
}

/// Emit a single keyframe block
fn emit_keyframe_block(block: &KeyframeBlock, opts: &EmitOptions) -> String {
    if opts.minify {
        let decls = block
            .declarations
            .iter()
            .map(emit_decl)
            .collect::<Vec<_>>()
            .join(";");
        format!("{}{{{}}}", block.selector, decls)
    } else {
        let nl = &opts.newline;
        let indent = &opts.indent;
        let decls = block
            .declarations
            .iter()
            .map(|d| format!("{}{};", indent, emit_decl(d)))
            .collect::<Vec<_>>()
            .join(nl);
        format!("{} {{{}{}{}}}", block.selector, nl, decls, nl)
    }
}

/// Lower a bare token reference to the `var()` a browser understands.
///
/// Spacetime spells a token reference `color: --ink`; `var()` is a CSS construct
/// the language does not need, because `--name` already names a value
/// unambiguously. But a BROWSER only resolves `var(--ink)`, so the bare spelling
/// is an authoring surface that must be lowered on the way out.
///
/// Only a VALUE position is rewritten. A custom property's own declaration
/// (`--ink: #222`) keeps its literal text: there `--ink` is the PROPERTY, not a
/// reference to one, and wrapping it would produce `--ink: var(#222)`.
fn lower_token_value(property: &str, value: &str) -> String {
    let trimmed = value.trim();
    if property.starts_with("--") {
        return value.to_string();
    }
    if crate::syntax::events::is_token_reference(trimmed) && !trimmed.starts_with("var(") {
        return format!("var({trimmed})");
    }
    value.to_string()
}

/// Emit a single CSS declaration
fn emit_decl(decl: &CssDecl) -> String {
    let value = lower_token_value(&decl.property, &decl.value);
    if decl.important {
        format!("{}: {} !important", decl.property, value)
    } else {
        format!("{}: {}", decl.property, value)
    }
}

// ============================================================================
// Writer-Based Emission (for Source Maps)
// ============================================================================

use super::sourcemap::SourceSpan;
use super::writer::SourceMapWriter;

/// Emit a CSS expression to a writer with optional source mapping.
pub fn emit_to_writer(
    expr: &CssExpr,
    opts: &EmitOptions,
    writer: &mut SourceMapWriter,
    span: Option<&SourceSpan>,
) {
    // Mark the start of this expression if span provided
    if let Some(s) = span {
        writer.mark(s);
    }

    match expr {
        CssExpr::Rule {
            selector,
            declarations,
        } => {
            emit_rule_to_writer(selector, declarations, opts, writer);
        }
        CssExpr::Keyframes { name, frames } => {
            emit_keyframes_to_writer(name, frames, opts, writer);
        }
        CssExpr::Raw(s) => {
            writer.write(s);
        }
        CssExpr::Composite(parts) => {
            for part in parts {
                emit_part_to_writer(part, opts, writer);
            }
        }
    }
}

/// Emit a CssPart to a writer
fn emit_part_to_writer(part: &CssPart, opts: &EmitOptions, writer: &mut SourceMapWriter) {
    match part {
        CssPart::Raw(s) => writer.write(s),
        CssPart::Expr(e) => emit_to_writer(e, opts, writer, None),
        CssPart::Marker(_) => {
            // Markers should be resolved before emission
            panic!("BUG: Unresolved marker reached CSS writer emission");
        }
    }
}

/// Emit multiple CSS expressions to a writer.
pub fn emit_all_to_writer(exprs: &[CssExpr], opts: &EmitOptions, writer: &mut SourceMapWriter) {
    let nl = if opts.minify { "" } else { "\n\n" };
    for (i, expr) in exprs.iter().enumerate() {
        if i > 0 {
            writer.write(nl);
        }
        emit_to_writer(expr, opts, writer, None);
    }
}

/// Emit a CSS rule to a writer.
fn emit_rule_to_writer(
    selector: &str,
    declarations: &[CssDecl],
    opts: &EmitOptions,
    writer: &mut SourceMapWriter,
) {
    writer.write(selector);
    if opts.minify {
        writer.write("{");
        for (i, decl) in declarations.iter().enumerate() {
            if i > 0 {
                writer.write(";");
            }
            emit_decl_to_writer(decl, writer);
        }
        writer.write("}");
    } else {
        writer.write(" {\n");
        for decl in declarations {
            writer.write(&opts.indent);
            emit_decl_to_writer(decl, writer);
            writer.write(";\n");
        }
        writer.write("}");
    }
}

/// Emit @keyframes block to a writer.
fn emit_keyframes_to_writer(
    name: &str,
    frames: &[KeyframeBlock],
    opts: &EmitOptions,
    writer: &mut SourceMapWriter,
) {
    writer.write("@keyframes ");
    writer.write(name);
    if opts.minify {
        writer.write("{");
        for frame in frames {
            emit_keyframe_block_to_writer(frame, opts, writer);
        }
        writer.write("}");
    } else {
        writer.write(" {\n");
        for frame in frames {
            writer.write(&opts.indent);
            emit_keyframe_block_to_writer(frame, opts, writer);
            writer.write("\n");
        }
        writer.write("}");
    }
}

/// Emit a single keyframe block to a writer.
fn emit_keyframe_block_to_writer(
    block: &KeyframeBlock,
    opts: &EmitOptions,
    writer: &mut SourceMapWriter,
) {
    writer.write(&block.selector);
    if opts.minify {
        writer.write("{");
        for (i, decl) in block.declarations.iter().enumerate() {
            if i > 0 {
                writer.write(";");
            }
            emit_decl_to_writer(decl, writer);
        }
        writer.write("}");
    } else {
        writer.write(" {\n");
        for decl in &block.declarations {
            writer.write(&opts.indent);
            writer.write(&opts.indent);
            emit_decl_to_writer(decl, writer);
            writer.write(";\n");
        }
        writer.write(&opts.indent);
        writer.write("}");
    }
}

/// Emit a CSS declaration to a writer.
fn emit_decl_to_writer(decl: &CssDecl, writer: &mut SourceMapWriter) {
    writer.write(&decl.property);
    writer.write(": ");
    // Same lowering as `emit_decl` — the source-map path must not emit a
    // different stylesheet from the plain one, or a bare token would work in a
    // normal build and break under `--sourcemap`.
    writer.write(&lower_token_value(&decl.property, &decl.value));
    if decl.important {
        writer.write(" !important");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ctx() -> CssContext {
        CssContext::new()
            .with_param("color", "blue")
            .with_element("el", ".test-element")
    }

    #[test]
    fn resolve_unknown_param_returns_comment() {
        let ctx = test_ctx();
        let marker = Marker::Param {
            name: "unknown".into(),
            field: None,
            in_string: None,
        };
        let result = resolve_marker(&marker, &ctx);
        match result {
            CssPart::Raw(s) => assert!(
                s.contains("spacetime: unknown param"),
                "Expected error comment, got: {}",
                s
            ),
            other => panic!("Expected CssPart::Raw, got {:?}", other),
        }
    }

    #[test]
    fn resolve_unknown_element_returns_comment() {
        let ctx = test_ctx();
        let marker = Marker::Element("unknown".into());
        let result = resolve_marker(&marker, &ctx);
        match result {
            CssPart::Raw(s) => assert!(
                s.contains("spacetime: unknown element"),
                "Expected error comment, got: {}",
                s
            ),
            other => panic!("Expected CssPart::Raw, got {:?}", other),
        }
    }

    #[test]
    fn resolve_signal_in_css_returns_comment() {
        let ctx = test_ctx();
        let marker = Marker::Signal("test".into());
        let result = resolve_marker(&marker, &ctx);
        match result {
            CssPart::Raw(s) => assert!(
                s.contains("spacetime: signal"),
                "Expected error comment, got: {}",
                s
            ),
            other => panic!("Expected CssPart::Raw, got {:?}", other),
        }
    }

    #[test]
    #[should_panic(expected = "BUG: Unresolved marker")]
    fn emit_unresolved_marker_panics() {
        let marker: Marker<CssExpr> = Marker::Param {
            name: "test".into(),
            field: None,
            in_string: None,
        };
        emit_marker(&marker);
    }
}
