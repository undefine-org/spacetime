//! Completion provider for Spacetime LSP.
//!
//! Provides intelligent completions based on FormRegistry directives and context.

#[cfg(feature = "lsp")]
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, Documentation, InsertTextFormat, MarkupContent, MarkupKind,
    Position,
};

#[cfg(feature = "lsp")]
use super::document::DocumentState;
#[cfg(feature = "lsp")]
use super::form_registry::{DirectiveParam, DirectiveSignature, FormRegistry};
#[cfg(feature = "lsp")]
use super::position::PositionMapper;
#[cfg(feature = "lsp")]
use crate::parser::meta_ast::{
    CaptureType, FormCapture, FormClause, FormInlineElement, FormParam, ParamDefault,
};

// =============================================================================
// Completion Context
// =============================================================================

/// Context at the cursor position for determining what completions to offer.
#[derive(Debug, Clone, PartialEq)]
pub enum CompletionContext {
    /// After `@` - suggest all directives
    AfterAt { prefix: String },
    /// Inside `@directive(|)` - suggest parameters
    InsideParams {
        directive_name: String,
        provided_params: Vec<String>,
    },
    /// After `@directive(param: |)` - suggest values based on CaptureType
    ParamValue {
        directive_name: String,
        param_name: String,
    },
    /// Inside a directive body `@directive { | }`
    InsideBody { directive_name: String },
    /// After `$` - suggest variables
    AfterDollar { prefix: String },
    /// After `&` - suggest templates
    AfterAmpersand { prefix: String },
    /// No specific completion context
    None,
}

// =============================================================================
// Main Completion Function (LSP only)
// =============================================================================

/// Provide completions for the given document position.
#[cfg(feature = "lsp")]
pub fn provide_completions(
    doc: &DocumentState,
    position: Position,
    form_registry: &FormRegistry,
) -> Vec<CompletionItem> {
    // Get cursor offset
    let offset = doc.position_mapper.offset_from_position(position);
    let content = &doc.content;

    // Extract context at cursor position
    let context = extract_context_at_position(content, offset);

    match context {
        CompletionContext::AfterAt { prefix } => {
            directive_completions(&prefix, form_registry, &doc.position_mapper, offset)
        }
        CompletionContext::InsideParams {
            directive_name,
            provided_params,
        } => param_completions(&directive_name, &provided_params, form_registry),
        CompletionContext::ParamValue {
            directive_name,
            param_name,
        } => value_completions(&directive_name, &param_name, form_registry),
        CompletionContext::InsideBody { directive_name } => {
            body_completions(&directive_name, form_registry)
        }
        CompletionContext::AfterDollar { .. }
        | CompletionContext::AfterAmpersand { .. } => {
            // Sigil completions would scan the document for definitions
            // For now, return empty - the context detection is the key feature
            vec![]
        }
        CompletionContext::None => vec![],
    }
}

// =============================================================================
// Context Extraction
// =============================================================================

/// Extract the completion context at a given offset in the content.
pub fn extract_context_at_position(content: &str, offset: usize) -> CompletionContext {
    let before = &content[..offset.min(content.len())];

    // Check for sigil prefixes: $, &
    if let Some(ctx) = extract_sigil_context(before, '$', |prefix| CompletionContext::AfterDollar {
        prefix,
    }) {
        return ctx;
    }
    if let Some(ctx) = extract_sigil_context(before, '&', |prefix| {
        CompletionContext::AfterAmpersand { prefix }
    }) {
        return ctx;
    }

    // Check for @directive pattern
    if let Some(at_pos) = before.rfind('@') {
        let after_at = &before[at_pos + 1..];

        // Check if we're inside parentheses
        if let Some(paren_pos) = after_at.rfind('(') {
            // We might be inside params
            if !after_at[paren_pos..].contains(')') {
                // Extract directive name (before paren)
                let directive_part = &after_at[..paren_pos];
                if is_valid_identifier(directive_part) {
                    // Check if we're after a colon (param value context)
                    let inside_parens = &after_at[paren_pos + 1..];
                    if let Some(colon_pos) = inside_parens.rfind(':') {
                        // Check if there's a comma after the colon (meaning we moved on)
                        let after_colon = &inside_parens[colon_pos + 1..];
                        if !after_colon.contains(',') {
                            // We're in param value context
                            // Extract param name (go back from colon to find param name)
                            let before_colon = &inside_parens[..colon_pos];
                            if let Some(param_name) = extract_last_param_name(before_colon) {
                                return CompletionContext::ParamValue {
                                    directive_name: directive_part.to_string(),
                                    param_name,
                                };
                            }
                        }
                    }

                    // We're in param name context
                    let provided = extract_provided_params(inside_parens);
                    return CompletionContext::InsideParams {
                        directive_name: directive_part.to_string(),
                        provided_params: provided,
                    };
                }
            }
        }

        // Check if we're inside braces (body context)
        if let Some(brace_pos) = after_at.rfind('{') {
            // Check if there's a matching closing brace after
            let after_brace = &after_at[brace_pos..];
            if !has_matching_close_brace(after_brace) {
                // Extract directive name
                let directive_part = after_at[..brace_pos].trim();
                // Remove any parameters in parens
                let directive_name = directive_part.split('(').next().unwrap_or("").trim();
                if is_valid_identifier(directive_name) {
                    return CompletionContext::InsideBody {
                        directive_name: directive_name.to_string(),
                    };
                }
            }
        }

        // Simple after @ context
        // Check for whitespace between @ and cursor - if there is, we're not in @ context
        if after_at
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return CompletionContext::AfterAt {
                prefix: after_at.to_string(),
            };
        }
    }

    CompletionContext::None
}

/// Check if a string is a valid identifier (alphanumeric, dash, underscore).
fn is_valid_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ' ')
}

/// Extract the last parameter name before a colon.
fn extract_last_param_name(s: &str) -> Option<String> {
    let trimmed = s.trim();
    // Find the last comma or start of string
    let last_segment = trimmed.rsplit(',').next()?;
    let name = last_segment.trim();
    if is_valid_identifier(name) && !name.is_empty() {
        Some(name.to_string())
    } else {
        None
    }
}

/// Extract already-provided parameter names from inside parens.
fn extract_provided_params(inside_parens: &str) -> Vec<String> {
    let mut params = Vec::new();
    for segment in inside_parens.split(',') {
        if let Some(colon_pos) = segment.find(':') {
            let name = segment[..colon_pos].trim();
            if !name.is_empty() {
                params.push(name.to_string());
            }
        }
    }
    params
}

/// Extract a sigil-prefixed completion context ($, &).
fn extract_sigil_context<F>(before: &str, sigil: char, make_ctx: F) -> Option<CompletionContext>
where
    F: FnOnce(String) -> CompletionContext,
{
    let sigil_pos = before.rfind(sigil)?;

    let after_sigil = &before[sigil_pos + sigil.len_utf8()..];
    if !after_sigil
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return None;
    }

    if sigil_pos > 0 {
        let before_sigil = before.as_bytes()[sigil_pos - 1];
        if before_sigil.is_ascii_alphanumeric() || before_sigil == b'_' || before_sigil == b'-' {
            return None;
        }
    }

    Some(make_ctx(after_sigil.to_string()))
}

/// Check if a string has a matching close brace.
fn has_matching_close_brace(s: &str) -> bool {
    let mut depth = 0;
    for c in s.chars() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

// =============================================================================
// Completion Generators (LSP only)
// =============================================================================

#[cfg(feature = "lsp")]
/// Generate directive completions for after @ context.
fn directive_completions(
    prefix: &str,
    form_registry: &FormRegistry,
    _position_mapper: &PositionMapper,
    _offset: usize,
) -> Vec<CompletionItem> {
    form_registry
        .directive_completions(prefix)
        .into_iter()
        .map(|(name, sig)| directive_completion_item(&name, sig))
        .collect()
}

#[cfg(feature = "lsp")]
/// Create a completion item for a directive.
fn directive_completion_item(name: &str, sig: &DirectiveSignature) -> CompletionItem {
    // Build snippet with parameter placeholders
    let (insert_text, insert_format) = build_directive_snippet(name, sig);

    CompletionItem {
        label: name.to_string(),
        kind: Some(CompletionItemKind::FUNCTION),
        detail: Some(sig.format_signature()),
        documentation: Some(Documentation::MarkupContent(
            format_directive_documentation(sig),
        )),
        insert_text: Some(insert_text),
        insert_text_format: Some(insert_format),
        ..Default::default()
    }
}

#[cfg(feature = "lsp")]
#[cfg(feature = "lsp")]
/// Build a snippet string for a directive, from the directive's REAL %form
/// grammar when available (the compiler-registry case — which is every
/// directive). Literal words, sigils and positional captures come straight
/// from the grammar so the produced shape is exactly what the compiler
/// matches; the old flat `name: ${n}` shape generated structurally-wrong
/// directives that became E0946 build errors after W3 made malformed shapes
/// fail. A snippet built from the grammar cannot be structurally wrong.
pub fn build_directive_snippet(name: &str, sig: &DirectiveSignature) -> (String, InsertTextFormat) {
    if let Some(form) = &sig.form {
        return (render_form_snippet(name, form), InsertTextFormat::SNIPPET);
    }

    // No form clause (defensive — compiler-registry macros always carry one).
    let required_params = sig.required_params();

    if required_params.is_empty() && sig.body_type.is_none() {
        return (name.to_string(), InsertTextFormat::PLAIN_TEXT);
    }

    let mut snippet = name.to_string();
    let mut placeholder_idx = 1;

    if !required_params.is_empty() {
        snippet.push('(');
        for (i, param) in required_params.iter().enumerate() {
            if i > 0 {
                snippet.push_str(", ");
            }
            snippet.push_str(&param.name);
            snippet.push_str(": ");
            snippet.push_str(&format!("${{{}}}", placeholder_idx));
            placeholder_idx += 1;
        }
        snippet.push(')');
    }

    if sig.body_type.is_some() {
        snippet.push_str(&format!(" {{\n\t${placeholder_idx}\n}}"));
    }

    snippet.push_str("$0");

    (snippet, InsertTextFormat::SNIPPET)
}

/// Tabstop counter for building a snippet.
struct SnippetBuilder {
    next_tab: usize,
}

impl SnippetBuilder {
    fn new() -> Self {
        Self { next_tab: 1 }
    }
    fn tab(&mut self) -> usize {
        let t = self.next_tab;
        self.next_tab += 1;
        t
    }
}

/// Render a directive's full post-`@` form from its `%form` grammar.
///
/// Literal words and sigils come straight from the grammar (structural
/// validity is guaranteed); captures become snippet tabstops whose label is a
/// shape-correct placeholder for the capture type (or its default), so a user
/// who tabs through accepting defaults still writes a compiling directive.
fn render_form_snippet(name: &str, form: &FormClause) -> String {
    let mut b = SnippetBuilder::new();
    let mut s = name.to_string();

    for el in &form.inline_elements {
        s.push(' ');
        s.push_str(&render_inline_element(el, &mut b));
    }

    if !form.params.is_empty() {
        s.push('(');
        for (i, p) in form.params.iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            s.push_str(&render_form_param(p, &mut b));
        }
        s.push(')');
    }

    for el in &form.post_arg_inline {
        s.push(' ');
        s.push_str(&render_inline_element(el, &mut b));
    }

    if form.body_capture.is_some() || !form.body_params.is_empty() || !form.body_groups.is_empty() {
        s.push_str(&format!(" {{\n\t${}\n}}", b.tab()));
    }

    s.push_str("$0");
    s
}

fn render_form_param(p: &FormParam, b: &mut SnippetBuilder) -> String {
    // A bare pattern (`$source as $item`) has an empty name and is rendered
    // with no `name:` prefix; a named param is `name: <elements>`.
    let body = render_elements(&p.elements, b);
    if p.name.is_empty() {
        body
    } else {
        format!("{}: {}", p.name, body)
    }
}

fn render_inline_element(el: &FormInlineElement, b: &mut SnippetBuilder) -> String {
    match el {
        FormInlineElement::Capture(cap, default) => render_capture(cap, default.as_ref(), b),
        FormInlineElement::Literal(s) => s.clone(),
        FormInlineElement::Comparison { operator, capture } => {
            format!("{} {}", operator, render_capture(capture, None, b))
        }
        FormInlineElement::Group { elements, .. } => {
            let inner = render_elements(elements, b);
            format!("({})", inner)
        }
        FormInlineElement::KeywordBlock { keyword, body_params, .. } => {
            let inner = render_params(body_params, b);
            format!("{} {{ {} }}", keyword, inner)
        }
        FormInlineElement::PseudoSelector { name, body_params, .. } => {
            let inner = render_params(body_params, b);
            format!(":{} {{ {} }}", name, inner)
        }
        FormInlineElement::PseudoClass { name, body_params } => {
            let inner = render_params(body_params, b);
            format!(":{} {{ {} }}", name, inner)
        }
    }
}

fn render_elements(elements: &[FormInlineElement], b: &mut SnippetBuilder) -> String {
    let mut parts = Vec::with_capacity(elements.len());
    for el in elements {
        parts.push(render_inline_element(el, b));
    }
    parts.join(" ")
}

fn render_params(params: &[FormParam], b: &mut SnippetBuilder) -> String {
    let mut parts = Vec::with_capacity(params.len());
    for p in params {
        parts.push(render_form_param(p, b));
    }
    parts.join(", ")
}

fn render_capture(cap: &FormCapture, default: Option<&ParamDefault>, b: &mut SnippetBuilder) -> String {
    let tab = b.tab();
    let label = default
        .filter(|d| !matches!(d, ParamDefault::None))
        .map(render_default_label)
        .unwrap_or_else(|| capture_placeholder(&cap.capture_type));
    let mut out = format!("${{{tab}:{label}}}");
    // `$source as $item` — a capture with an ALIAS (`alias_capture`). The
    // alias is part of the authored syntax (`@each $items as $item`), so it
    // must appear in the snippet or the shape is structurally incomplete.
    if let Some(alias) = &cap.alias_capture {
        out.push_str(" as ");
        out.push_str(&render_capture(alias, None, b));
    }
    out
}

/// A label for a capture's default value, rendered as the author would type it.
fn render_default_label(d: &ParamDefault) -> String {
    match d {
        ParamDefault::String(s) => format!("\"{s}\""),
        ParamDefault::Number(n) => n.to_string(),
        ParamDefault::Bool(b) => b.to_string(),
        ParamDefault::Length(val, unit) => format!("{val}{unit}"),
        ParamDefault::Array(items) => items
            .iter()
            .map(render_default_label)
            .collect::<Vec<_>>()
            .join(", "),
        ParamDefault::EmptyArray => "[]".to_string(),
        ParamDefault::EmptyObject => "{}".to_string(),
        ParamDefault::None => "value".to_string(),
    }
}

/// A shape-correct placeholder for a capture type — the text a user is likely
/// to write for that kind of value.
fn capture_placeholder(ct: &CaptureType) -> String {
    match ct {
        CaptureType::Ident => "name".to_string(),
        CaptureType::DashedIdent => "--name".to_string(),
        CaptureType::EventName => "--event".to_string(),
        CaptureType::Event => "click".to_string(),
        CaptureType::String => "\"value\"".to_string(),
        CaptureType::Number => "0".to_string(),
        CaptureType::Bool => "false".to_string(),
        CaptureType::Time => "0.3s".to_string(),
        CaptureType::Length => "16px".to_string(),
        CaptureType::Duration => "300ms".to_string(),
        CaptureType::Easing => "ease".to_string(),
        CaptureType::Typeref => "Type".to_string(),
        CaptureType::Binding => "$items".to_string(),
        CaptureType::Expr => "expr".to_string(),
        CaptureType::Properties => "prop: value".to_string(),
        CaptureType::Fields => "fields".to_string(),
        CaptureType::Params => "params".to_string(),
        CaptureType::States => "states".to_string(),
        CaptureType::Transitions => "transitions".to_string(),
        CaptureType::Keyframes => "keyframes".to_string(),
        CaptureType::Selector => ".element".to_string(),
        CaptureType::Element => "el".to_string(),
        CaptureType::Preset => "preset".to_string(),
        CaptureType::MutationActions => "actions".to_string(),
        CaptureType::Template => "template".to_string(),
        CaptureType::ParamList => "args".to_string(),
        CaptureType::HtmlBlock => "<div></div>".to_string(),
        CaptureType::JsBlock => "js".to_string(),
        CaptureType::ComponentBody => "body".to_string(),
        CaptureType::TemplateInvocation => "&template".to_string(),
        CaptureType::Union(variants) => {
            if let Some(first) = variants.first() {
                first.clone()
            } else {
                "variant".to_string()
            }
        }
        CaptureType::PatternMatch { variant, .. } => variant.clone(),
        CaptureType::Custom(_) => "value".to_string(),
        CaptureType::Balanced(_) => "value".to_string(),
        CaptureType::SkipBlock => "".to_string(),
        CaptureType::Color => "#fff".to_string(),
    }
}


#[cfg(feature = "lsp")]
/// Generate parameter completions for inside parentheses context.
fn param_completions(
    directive_name: &str,
    provided_params: &[String],
    form_registry: &FormRegistry,
) -> Vec<CompletionItem> {
    let Some(sig) = form_registry.get_directive(directive_name) else {
        return vec![];
    };

    sig.params
        .iter()
        .filter(|p| !provided_params.contains(&p.name))
        .map(param_completion_item)
        .collect()
}

#[cfg(feature = "lsp")]
/// Create a completion item for a parameter.
fn param_completion_item(param: &DirectiveParam) -> CompletionItem {
    let snippet = format!("{}: ${{1}}", param.name);

    CompletionItem {
        label: param.name.clone(),
        kind: Some(CompletionItemKind::PROPERTY),
        detail: Some(param.format()),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format_param_documentation(param),
        })),
        insert_text: Some(snippet),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        ..Default::default()
    }
}

#[cfg(feature = "lsp")]
/// Format documentation for a parameter.
fn format_param_documentation(param: &DirectiveParam) -> String {
    let mut doc = format!("**{}**\n\n", param.name);
    doc.push_str(&format!(
        "Type: `{}`\n",
        format_capture_type_display(&param.capture_type)
    ));

    if let Some(ref default) = param.default_value {
        doc.push_str(&format!("\nDefault: `{}`", default));
    }

    if param.is_required() {
        doc.push_str("\n\n*Required*");
    }

    doc
}

#[cfg(feature = "lsp")]
/// Generate value completions for param value context.
fn value_completions(
    directive_name: &str,
    param_name: &str,
    form_registry: &FormRegistry,
) -> Vec<CompletionItem> {
    let Some(sig) = form_registry.get_directive(directive_name) else {
        return vec![];
    };

    let Some(param) = sig.params.iter().find(|p| p.name == param_name) else {
        return vec![];
    };

    value_completions_for_type(&param.capture_type)
}

#[cfg(feature = "lsp")]
/// Generate value completions based on capture type.
fn value_completions_for_type(
    capture_type: &crate::parser::meta_ast::CaptureType,
) -> Vec<CompletionItem> {
    use crate::parser::meta_ast::CaptureType;

    match capture_type {
        CaptureType::Bool => vec![
            value_completion_item("true", "Boolean true"),
            value_completion_item("false", "Boolean false"),
        ],
        CaptureType::Easing => vec![
            value_completion_item("linear", "Linear easing"),
            value_completion_item("ease", "Default ease curve"),
            value_completion_item("ease-in", "Accelerating from zero velocity"),
            value_completion_item("ease-out", "Decelerating to zero velocity"),
            value_completion_item("ease-in-out", "Accelerate then decelerate"),
            value_completion_item(
                "cubic-bezier(${1:0.4}, ${2:0}, ${3:0.2}, ${4:1})",
                "Custom cubic bezier",
            ),
        ],
        CaptureType::Duration => vec![
            value_completion_item("0.3s", "300ms (fast)"),
            value_completion_item("0.5s", "500ms (medium)"),
            value_completion_item("1s", "1 second"),
            value_completion_item("${1:0.3}s", "Custom duration"),
        ],
        CaptureType::Union(variants) => variants
            .iter()
            .map(|v| value_completion_item(v, &format!("\"{}\"", v)))
            .collect(),
        _ => vec![],
    }
}

#[cfg(feature = "lsp")]
/// Create a value completion item.
fn value_completion_item(value: &str, description: &str) -> CompletionItem {
    let has_placeholders = value.contains("${");

    CompletionItem {
        label: if has_placeholders {
            // Strip placeholders for display
            value.split("${").next().unwrap_or(value).to_string() + "..."
        } else {
            value.to_string()
        },
        kind: Some(CompletionItemKind::VALUE),
        detail: Some(description.to_string()),
        insert_text: Some(value.to_string()),
        insert_text_format: Some(if has_placeholders {
            InsertTextFormat::SNIPPET
        } else {
            InsertTextFormat::PLAIN_TEXT
        }),
        ..Default::default()
    }
}

#[cfg(feature = "lsp")]
/// Generate completions for inside directive body.
fn body_completions(directive_name: &str, form_registry: &FormRegistry) -> Vec<CompletionItem> {
    let Some(sig) = form_registry.get_directive(directive_name) else {
        return vec![];
    };

    let mut completions = Vec::new();

    // Add exported signals from bound primitives
    completions.extend(exported_signal_completions(sig));

    // Based on body_type, suggest appropriate content
    match &sig.body_type {
        Some(crate::parser::meta_ast::CaptureType::Properties) => {
            // Suggest common CSS properties
            completions.extend(common_css_property_completions());
        }
        Some(crate::parser::meta_ast::CaptureType::Keyframes) => {
            // Suggest keyframe structure
            completions.extend(keyframe_completions());
        }
        Some(crate::parser::meta_ast::CaptureType::States) => {
            // Suggest state definitions
            completions.extend(state_completions());
        }
        _ => {}
    }

    completions
}

#[cfg(feature = "lsp")]
/// Generate completions for exported signals.
fn exported_signal_completions(sig: &DirectiveSignature) -> Vec<CompletionItem> {
    sig.exports
        .iter()
        .map(|export| {
            let label = export.name.clone();
            let type_hint = if export.optional {
                format!("{}?", export.signal_type)
            } else {
                export.signal_type.clone()
            };

            // Build detail showing type and source primitive
            let detail = format!("{}: {}", label, type_hint);

            // Build documentation with description if available
            let mut doc = format!("**{}**\n\nType: `{}`", label, type_hint);
            if let Some(ref desc) = export.description {
                doc.push_str(&format!("\n\n{}", desc));
            }
            if !sig.bound_primitives.is_empty() {
                doc.push_str(&format!(
                    "\n\n*From primitive: {}*",
                    sig.bound_primitives.join(", ")
                ));
            }

            CompletionItem {
                label,
                kind: Some(CompletionItemKind::VARIABLE),
                detail: Some(detail),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: doc,
                })),
                insert_text: Some(export.name.clone()),
                insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                // Sort exports first (with low sort text)
                sort_text: Some(format!("0{}", export.name)),
                ..Default::default()
            }
        })
        .collect()
}

#[cfg(feature = "lsp")]
/// Generate common CSS property completions.
fn common_css_property_completions() -> Vec<CompletionItem> {
    let properties = [
        ("opacity", "0 -> 1", "Fade animation"),
        (
            "transform",
            "translateY(20px) -> translateY(0)",
            "Transform animation",
        ),
        ("background-color", "#000 -> #fff", "Color transition"),
        ("scale", "0.9 -> 1", "Scale animation"),
    ];

    properties
        .iter()
        .map(|(name, value, desc)| CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::PROPERTY),
            detail: Some(desc.to_string()),
            insert_text: Some(format!("{}: {};", name, value)),
            insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
            ..Default::default()
        })
        .collect()
}

#[cfg(feature = "lsp")]
/// Generate keyframe completions.
fn keyframe_completions() -> Vec<CompletionItem> {
    vec![
        CompletionItem {
            label: "0%".to_string(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some("Start keyframe".to_string()),
            insert_text: Some("0% {\n\t$1\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "100%".to_string(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some("End keyframe".to_string()),
            insert_text: Some("100% {\n\t$1\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
    ]
}

#[cfg(feature = "lsp")]
/// Generate state completions.
fn state_completions() -> Vec<CompletionItem> {
    vec![CompletionItem {
        label: "state".to_string(),
        kind: Some(CompletionItemKind::SNIPPET),
        detail: Some("Define a state".to_string()),
        insert_text: Some("${1:name} {\n\t$2\n}".to_string()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        ..Default::default()
    }]
}

// =============================================================================
// Formatting Helpers (LSP only)
// =============================================================================

#[cfg(feature = "lsp")]
/// Format directive documentation as MarkupContent.
fn format_directive_documentation(sig: &DirectiveSignature) -> MarkupContent {
    let mut value = String::new();

    // Signature
    value.push_str(&format!(
        "```spacetime\n{}\n```\n\n",
        sig.format_signature()
    ));

    // Parameters
    if !sig.params.is_empty() {
        value.push_str("**Parameters:**\n\n");
        for param in &sig.params {
            let required = if param.is_required() {
                " *(required)*"
            } else {
                ""
            };
            value.push_str(&format!(
                "- `{}`: {}{}\n",
                param.name,
                format_capture_type_display(&param.capture_type),
                required
            ));
            if let Some(ref default) = param.default_value {
                value.push_str(&format!("  - Default: `{}`\n", default));
            }
        }
        value.push('\n');
    }

    // Exported signals (from bound primitives)
    if !sig.exports.is_empty() {
        value.push_str("**Exports:**\n\n");
        for export in &sig.exports {
            let opt = if export.optional { "?" } else { "" };
            value.push_str(&format!(
                "- `{}`: `{}{}`",
                export.name, export.signal_type, opt
            ));
            if let Some(ref desc) = export.description {
                value.push_str(&format!(" - {}", desc));
            }
            value.push('\n');
        }
        value.push('\n');
    }

    // Custom documentation
    if let Some(ref doc) = sig.documentation {
        value.push_str("---\n\n");
        value.push_str(doc);
        value.push_str("\n\n");
    }

    // Source location
    value.push_str(&format!("*Defined in `{}`*", sig.source_file.display()));

    MarkupContent {
        kind: MarkupKind::Markdown,
        value,
    }
}

#[cfg(feature = "lsp")]
/// Format a CaptureType for display.
fn format_capture_type_display(ct: &crate::parser::meta_ast::CaptureType) -> String {
    use crate::parser::meta_ast::CaptureType;

    match ct {
        CaptureType::Ident => "identifier".to_string(),
        CaptureType::DashedIdent => "form reference (--name)".to_string(),
        CaptureType::EventName => "form reference (--name)".to_string(),
        CaptureType::String => "string".to_string(),
        CaptureType::Number => "number".to_string(),
        CaptureType::Bool => "boolean".to_string(),
        CaptureType::Time => "time".to_string(),
        CaptureType::Length => "length".to_string(),
        CaptureType::Duration => "duration (e.g., 0.3s, 300ms)".to_string(),
        CaptureType::Easing => "easing function".to_string(),
        CaptureType::Typeref => "type reference".to_string(),
        CaptureType::Binding => "binding expression".to_string(),
        CaptureType::Event => "event name".to_string(),
        CaptureType::Expr => "expression".to_string(),
        CaptureType::Properties => "property block".to_string(),
        CaptureType::Fields => "field definitions".to_string(),
        CaptureType::Params => "parameters".to_string(),
        CaptureType::States => "state definitions".to_string(),
        CaptureType::Transitions => "transitions".to_string(),
        CaptureType::Keyframes => "keyframes".to_string(),
        CaptureType::Selector => "CSS selector".to_string(),
        CaptureType::Element => "element reference".to_string(),
        CaptureType::Preset => "RETIRED capture type — `~` preset refs are gone".to_string(),
        CaptureType::MutationActions => "mutation actions".to_string(),
        CaptureType::Union(variants) => {
            let opts: Vec<String> = variants.iter().map(|v| format!("\"{}\"", v)).collect();
            opts.join(" | ")
        }
        CaptureType::Template => "template".to_string(),
        CaptureType::ParamList => "parameter list".to_string(),
        CaptureType::HtmlBlock => "HTML block".to_string(),
        CaptureType::JsBlock => "JavaScript block".to_string(),
        CaptureType::ComponentBody => "component body (HTML + CSS + state)".to_string(),
        CaptureType::TemplateInvocation => "template invocation".to_string(),
        CaptureType::PatternMatch { variant, bindings } => {
            if bindings.is_empty() {
                format!("pattern match (is {})", variant)
            } else {
                format!(
                    "pattern match (is {} {{ {} }})",
                    variant,
                    bindings.join(", ")
                )
            }
        }
        CaptureType::Custom(name) => format!("custom type '{}'", name),
        CaptureType::Balanced(d) => format!("balanced run until '{}'", d),
        CaptureType::SkipBlock => "skipped block".to_string(),
        CaptureType::Color => "CSS color value".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_context_after_at() {
        let content = "@scr";
        let ctx = extract_context_at_position(content, 4);
        assert_eq!(
            ctx,
            CompletionContext::AfterAt {
                prefix: "scr".to_string()
            }
        );
    }

    #[test]
    fn test_extract_context_at_sign_only() {
        let content = "@";
        let ctx = extract_context_at_position(content, 1);
        assert_eq!(
            ctx,
            CompletionContext::AfterAt {
                prefix: "".to_string()
            }
        );
    }

    #[test]
    fn test_extract_context_inside_params() {
        let content = "@scroll(";
        let ctx = extract_context_at_position(content, 8);
        assert_eq!(
            ctx,
            CompletionContext::InsideParams {
                directive_name: "scroll".to_string(),
                provided_params: vec![]
            }
        );
    }

    #[test]
    fn test_extract_context_inside_params_with_existing() {
        let content = "@scroll(duration: 0.3s, ";
        let ctx = extract_context_at_position(content, 24);
        assert_eq!(
            ctx,
            CompletionContext::InsideParams {
                directive_name: "scroll".to_string(),
                provided_params: vec!["duration".to_string()]
            }
        );
    }

    #[test]
    fn test_extract_context_param_value() {
        let content = "@scroll(duration: ";
        let ctx = extract_context_at_position(content, 18);
        assert_eq!(
            ctx,
            CompletionContext::ParamValue {
                directive_name: "scroll".to_string(),
                param_name: "duration".to_string()
            }
        );
    }

    #[test]
    fn test_extract_context_no_context() {
        let content = "some text without directive";
        let ctx = extract_context_at_position(content, 10);
        assert_eq!(ctx, CompletionContext::None);
    }

    #[test]
    fn test_is_valid_identifier() {
        assert!(is_valid_identifier("scroll"));
        assert!(is_valid_identifier("fade-in"));
        assert!(is_valid_identifier("on hover"));
        assert!(is_valid_identifier("my_directive"));
        assert!(!is_valid_identifier(""));
    }

    #[test]
    fn test_extract_provided_params() {
        let inside = "duration: 0.3s, easing: ease-out";
        let params = extract_provided_params(inside);
        assert_eq!(params, vec!["duration".to_string(), "easing".to_string()]);
    }

    #[test]
    fn test_extract_last_param_name() {
        assert_eq!(
            extract_last_param_name("duration"),
            Some("duration".to_string())
        );
        assert_eq!(
            extract_last_param_name("foo: bar, duration"),
            Some("duration".to_string())
        );
        assert_eq!(extract_last_param_name(""), None);
    }
}
