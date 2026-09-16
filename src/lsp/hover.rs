//! Hover provider for Spacetime LSP.
//!
//! Provides documentation on hover for directives, parameters, and other elements.

#[cfg(feature = "lsp")]
use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position, Range};

#[cfg(feature = "lsp")]
use super::document::DocumentState;
#[cfg(feature = "lsp")]
use super::form_registry::{DirectiveParam, DirectiveSignature, FormRegistry};
#[cfg(feature = "lsp")]
use super::position::PositionMapper;
#[cfg(feature = "lsp")]
use crate::parser::meta_ast::CaptureType;

// =============================================================================
// Hover Context
// =============================================================================

/// Context at the cursor position for determining what to show in hover.
#[derive(Debug, Clone, PartialEq)]
pub enum HoverContext {
    /// On a directive name (after @)
    DirectiveName { name: String, range: (usize, usize) },
    /// On a parameter name inside parentheses
    ParamName {
        directive_name: String,
        param_name: String,
        range: (usize, usize),
    },
    /// On a variable reference ($name)
    Variable { name: String, range: (usize, usize) },
    /// On a template reference (&name)
    Template { name: String, range: (usize, usize) },
    /// No hover context found
    None,
}

// =============================================================================
// Main Hover Function (LSP only)
// =============================================================================

#[cfg(feature = "lsp")]
/// Provide hover information for the given document position.
pub fn provide_hover(
    doc: &DocumentState,
    position: Position,
    form_registry: &FormRegistry,
) -> Option<Hover> {
    let offset = doc.position_mapper.offset_from_position(position);
    let content = &doc.content;

    let context = extract_hover_context(content, offset);

    match context {
        HoverContext::DirectiveName { name, range } => {
            let sig = form_registry.get_directive(&name)?;
            Some(Hover {
                contents: HoverContents::Markup(format_directive_hover(sig)),
                range: Some(offset_range_to_lsp_range(range, &doc.position_mapper)),
            })
        }
        HoverContext::ParamName {
            directive_name,
            param_name,
            range,
        } => {
            let sig = form_registry.get_directive(&directive_name)?;
            let param = sig.params.iter().find(|p| p.name == param_name)?;
            Some(Hover {
                contents: HoverContents::Markup(format_param_hover(param, sig)),
                range: Some(offset_range_to_lsp_range(range, &doc.position_mapper)),
            })
        }
        HoverContext::Variable { name, range } => {
            let value = format!(
                "```spacetime\n${}\n```\n\n*State variable `${}`*",
                name, name
            );
            Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value,
                }),
                range: Some(offset_range_to_lsp_range(range, &doc.position_mapper)),
            })
        }
        HoverContext::Template { name, range } => {
            let value = format!(
                "```spacetime\n&{}\n```\n\n*Template reference `&{}`*",
                name, name
            );
            Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value,
                }),
                range: Some(offset_range_to_lsp_range(range, &doc.position_mapper)),
            })
        }
        HoverContext::None => None,
    }
}

// =============================================================================
// Context Extraction
// =============================================================================

/// Extract hover context at a given offset.
pub fn extract_hover_context(content: &str, offset: usize) -> HoverContext {
    // First, try to find if we're on a directive
    if let Some(directive_ctx) = find_directive_at_offset(content, offset) {
        return directive_ctx;
    }

    // Check if we're on a parameter name
    if let Some(param_ctx) = find_param_at_offset(content, offset) {
        return param_ctx;
    }

    // Check for sigil-prefixed names
    if let Some(ctx) = find_sigil_at_offset(content, offset, '$', |name, range| {
        HoverContext::Variable { name, range }
    }) {
        return ctx;
    }
    if let Some(ctx) = find_sigil_at_offset(content, offset, '&', |name, range| {
        HoverContext::Template { name, range }
    }) {
        return ctx;
    }

    HoverContext::None
}

/// Find a sigil-prefixed name at offset ($variable, &template).
fn find_sigil_at_offset<F>(
    content: &str,
    offset: usize,
    sigil: char,
    make_ctx: F,
) -> Option<HoverContext>
where
    F: FnOnce(String, (usize, usize)) -> HoverContext,
{
    let bytes = content.as_bytes();
    let sigil_byte = sigil as u8;

    let sigil_pos = if offset < bytes.len() && bytes[offset] == sigil_byte {
        Some(offset)
    } else {
        let mut pos = offset;
        while pos > 0 {
            pos -= 1;
            if bytes[pos] == sigil_byte {
                break;
            }
            if !bytes[pos].is_ascii_alphanumeric() && bytes[pos] != b'-' && bytes[pos] != b'_' {
                return None;
            }
        }
        if pos < bytes.len() && bytes[pos] == sigil_byte {
            Some(pos)
        } else {
            None
        }
    };

    let sigil_pos = sigil_pos?;
    let name_start = sigil_pos + 1;
    if name_start >= bytes.len() {
        return None;
    }

    let mut end = name_start;
    while end < bytes.len()
        && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'-' || bytes[end] == b'_')
    {
        end += 1;
    }

    if end == name_start {
        return None;
    }

    if offset > end {
        return None;
    }

    let name = content[name_start..end].to_string();
    Some(make_ctx(name, (sigil_pos, end)))
}

/// Find directive name at offset if cursor is on one.
fn find_directive_at_offset(content: &str, offset: usize) -> Option<HoverContext> {
    // Look backwards for @
    let before = &content[..offset.min(content.len())];
    let at_pos = before.rfind('@')?;

    // Get the text between @ and cursor
    let between = &before[at_pos + 1..];

    // Check for whitespace or special chars that would break the directive name
    if between.contains(|c: char| !c.is_alphanumeric() && c != '-' && c != '_' && c != ' ') {
        return None;
    }

    // Find the end of the directive name (look forward from cursor)
    let after = &content[offset..];
    let end_offset = after
        .find(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
        .unwrap_or(after.len());

    let directive_name = format!("{}{}", between, &after[..end_offset]);

    // Make sure we're not past the @ context (check for parens or braces between)
    if between.contains('(') || between.contains('{') {
        return None;
    }

    if directive_name.is_empty() {
        return None;
    }

    let start = at_pos + 1;
    let end = offset + end_offset;

    Some(HoverContext::DirectiveName {
        name: directive_name,
        range: (start, end),
    })
}

/// Find parameter name at offset if cursor is on one.
fn find_param_at_offset(content: &str, offset: usize) -> Option<HoverContext> {
    let before = &content[..offset.min(content.len())];

    // Find the last @ and check if we're inside parens
    let at_pos = before.rfind('@')?;
    let after_at = &before[at_pos + 1..];

    // Find opening paren
    let paren_pos = after_at.find('(')?;

    // Make sure we're inside parens (no closing paren)
    let after_paren = &after_at[paren_pos + 1..];
    if after_paren.contains(')') {
        return None;
    }

    // Extract directive name
    let directive_name = after_at[..paren_pos].trim();
    if directive_name.is_empty() {
        return None;
    }

    // Now find the parameter at cursor
    // Look backwards from cursor to find param start (after comma or open paren)
    let inside_parens_start = at_pos + 1 + paren_pos + 1;
    let cursor_in_parens = offset - inside_parens_start;

    if cursor_in_parens > after_paren.len() {
        return None;
    }

    let inside_parens = &after_paren[..cursor_in_parens];

    // Find the start of current parameter (after last comma)
    let param_start = inside_parens.rfind(',').map(|p| p + 1).unwrap_or(0);
    let param_content = inside_parens[param_start..].trim_start();

    // Find colon - if cursor is before colon, we're on param name
    if let Some(colon_pos) = param_content.find(':') {
        // Calculate if cursor is before colon
        let cursor_rel = cursor_in_parens - param_start;
        let trimmed_start =
            inside_parens[param_start..].len() - inside_parens[param_start..].trim_start().len();

        if cursor_rel <= trimmed_start + colon_pos {
            let param_name = param_content[..colon_pos].trim();
            if !param_name.is_empty() {
                let abs_start = inside_parens_start + param_start + trimmed_start;
                let abs_end = abs_start + colon_pos;

                return Some(HoverContext::ParamName {
                    directive_name: directive_name.to_string(),
                    param_name: param_name.to_string(),
                    range: (abs_start, abs_end),
                });
            }
        }
    } else {
        // No colon yet, we might be typing param name
        let param_name = param_content.trim();
        if !param_name.is_empty() {
            let trimmed_start = inside_parens[param_start..].len()
                - inside_parens[param_start..].trim_start().len();
            let abs_start = inside_parens_start + param_start + trimmed_start;
            let _abs_end = offset;

            // Check if cursor is still on the param name
            let after = &content[offset..];
            let end_ext = after
                .find(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
                .unwrap_or(0);
            let full_param_name = format!("{}{}", param_name, &after[..end_ext]);

            if !full_param_name.is_empty() {
                return Some(HoverContext::ParamName {
                    directive_name: directive_name.to_string(),
                    param_name: full_param_name,
                    range: (abs_start, offset + end_ext),
                });
            }
        }
    }

    None
}

#[cfg(feature = "lsp")]
/// Convert byte offset range to LSP Range.
fn offset_range_to_lsp_range(range: (usize, usize), mapper: &PositionMapper) -> Range {
    Range {
        start: mapper.position_from_offset(range.0),
        end: mapper.position_from_offset(range.1),
    }
}

// =============================================================================
// Hover Formatting (LSP only)
// =============================================================================

#[cfg(feature = "lsp")]
/// Format hover content for a directive.
pub fn format_directive_hover(sig: &DirectiveSignature) -> MarkupContent {
    let mut value = String::new();

    // Signature in code block
    value.push_str(&format!(
        "```spacetime\n{}\n```\n\n",
        sig.format_signature()
    ));

    // Brief description based on macro name
    value.push_str(&format!("Directive created by `{}`\n\n", sig.macro_name));

    // Parameters section
    if !sig.params.is_empty() {
        value.push_str("### Parameters\n\n");
        for param in &sig.params {
            let modifier_str = match param.modifier {
                crate::parser::meta_ast::CaptureModifier::Optional => " (optional)",
                crate::parser::meta_ast::CaptureModifier::ZeroOrMore => " (multiple)",
                crate::parser::meta_ast::CaptureModifier::OneOrMore => " (one or more)",
                crate::parser::meta_ast::CaptureModifier::Counted(_) => " (counted)",
                crate::parser::meta_ast::CaptureModifier::Required => "",
            };
            value.push_str(&format!(
                "- **{}**: `{}`{}\n",
                param.name,
                format_capture_type(&param.capture_type),
                modifier_str
            ));
            if let Some(ref default) = param.default_value {
                value.push_str(&format!("  - Default: `{}`\n", default));
            }
        }
        value.push('\n');
    }

    // Exported signals section
    if !sig.exports.is_empty() {
        value.push_str("### Exports\n\n");
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

    // Body type
    if let Some(ref body_type) = sig.body_type {
        value.push_str(&format!(
            "### Body\n\nExpects: `{}`\n\n",
            format_capture_type(body_type)
        ));
    }

    // Bound primitives
    if !sig.bound_primitives.is_empty() {
        value.push_str(&format!(
            "### Binds\n\n{}\n\n",
            sig.bound_primitives
                .iter()
                .map(|p| format!("`{}`", p))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    // Documentation
    if let Some(ref doc) = sig.documentation {
        value.push_str("---\n\n");
        value.push_str(doc);
        value.push_str("\n\n");
    }

    // Source location
    value.push_str(&format!(
        "*Defined in [`{}`]({})*",
        sig.source_file.display(),
        sig.source_file.display()
    ));

    MarkupContent {
        kind: MarkupKind::Markdown,
        value,
    }
}

#[cfg(feature = "lsp")]
/// Format hover content for a primitive.
pub fn format_primitive_hover(prim: &super::form_registry::PrimitiveInfo) -> MarkupContent {
    let mut value = String::new();

    // Signature in code block
    value.push_str(&format!(
        "```spacetime\n%primitive {}\n```\n\n",
        prim.format_signature()
    ));

    // Parameters section
    if !prim.params.is_empty() {
        value.push_str("### Parameters\n\n");
        for param in &prim.params {
            if param.is_element {
                value.push_str(&format!("- **{}** - element reference\n", param.name));
            } else if let Some(ref default) = param.default_value {
                value.push_str(&format!(
                    "- **{}**: `{}` = `{}`\n",
                    param.name, param.param_type, default
                ));
            } else {
                value.push_str(&format!("- **{}**: `{}`\n", param.name, param.param_type));
            }
        }
        value.push('\n');
    }

    // Exports section
    if !prim.exports.is_empty() {
        value.push_str("### Exports\n\n");
        for export in &prim.exports {
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

    // Source location
    value.push_str(&format!(
        "*Defined in [`{}`]({})*",
        prim.source_file.display(),
        prim.source_file.display()
    ));

    MarkupContent {
        kind: MarkupKind::Markdown,
        value,
    }
}

#[cfg(feature = "lsp")]
/// Format hover content for a parameter.
pub fn format_param_hover(param: &DirectiveParam, sig: &DirectiveSignature) -> MarkupContent {
    let mut value = String::new();

    // Parameter name and type
    value.push_str(&format!("```spacetime\n{}\n```\n\n", param.format()));

    // Description
    value.push_str(&format!("Parameter of `@{}`\n\n", sig.name));

    // Type description
    value.push_str(&format!(
        "**Type:** {}\n\n",
        format_capture_type_description(&param.capture_type)
    ));

    // Default value
    if let Some(ref default) = param.default_value {
        value.push_str(&format!("**Default:** `{}`\n\n", default));
    }

    // Required/optional
    if param.is_required() {
        value.push_str("*This parameter is required.*\n");
    } else {
        value.push_str("*This parameter is optional.*\n");
    }

    MarkupContent {
        kind: MarkupKind::Markdown,
        value,
    }
}

#[cfg(feature = "lsp")]
/// Format a CaptureType as a type name.
fn format_capture_type(ct: &CaptureType) -> String {
    match ct {
        CaptureType::Ident => "ident".to_string(),
        CaptureType::DashedIdent => "dashed_ident".to_string(),
        CaptureType::EventName => "event_name".to_string(),
        CaptureType::String => "string".to_string(),
        CaptureType::Number => "number".to_string(),
        CaptureType::Bool => "bool".to_string(),
        CaptureType::Time => "time".to_string(),
        CaptureType::Length => "length".to_string(),
        CaptureType::Duration => "duration".to_string(),
        CaptureType::Easing => "easing".to_string(),
        CaptureType::Typeref => "typeref".to_string(),
        CaptureType::Binding => "binding".to_string(),
        CaptureType::Event => "event".to_string(),
        CaptureType::Expr => "expr".to_string(),
        CaptureType::Properties => "properties".to_string(),
        CaptureType::Fields => "fields".to_string(),
        CaptureType::Params => "params".to_string(),
        CaptureType::States => "states".to_string(),
        CaptureType::Transitions => "transitions".to_string(),
        CaptureType::Keyframes => "keyframes".to_string(),
        CaptureType::Selector => "selector".to_string(),
        CaptureType::Element => "element".to_string(),
        CaptureType::Preset => "preset".to_string(),
        CaptureType::MutationActions => "mutation_actions".to_string(),
        CaptureType::Union(variants) => {
            let opts: Vec<String> = variants.iter().map(|v| format!("\"{}\"", v)).collect();
            format!("({})", opts.join(" | "))
        }
        CaptureType::Template => "template".to_string(),
        CaptureType::ParamList => "param_list".to_string(),
        CaptureType::HtmlBlock => "html_block".to_string(),
        CaptureType::JsBlock => "jsblock".to_string(),
        CaptureType::ComponentBody => "component_body".to_string(),
        CaptureType::TemplateInvocation => "template_invocation".to_string(),
        CaptureType::PatternMatch { variant, .. } => format!("pattern_{}", variant.to_lowercase()),
        CaptureType::Custom(name) => name.clone(),
        CaptureType::Balanced(d) => format!("balanced('{}')", d),
        CaptureType::SkipBlock => "skip_block".to_string(),
        CaptureType::Color => "color".to_string(),
    }
}

#[cfg(feature = "lsp")]
/// Format a CaptureType with a human-readable description.
fn format_capture_type_description(ct: &CaptureType) -> String {
    match ct {
        CaptureType::Ident => "An identifier (e.g., `myName`, `some-id`)".to_string(),
        CaptureType::DashedIdent => {
            "A form reference — an identifier beginning with `--` (e.g., `--rise`)".to_string()
        }
        CaptureType::EventName => "A bare event name (never a `$`-sigiled binding)".to_string(),
        CaptureType::String => "A quoted string (e.g., `\"hello\"`)".to_string(),
        CaptureType::Number => "A numeric value (e.g., `42`, `3.14`)".to_string(),
        CaptureType::Bool => "A boolean (`true` or `false`)".to_string(),
        CaptureType::Time => "A time value (e.g., `0`, `0.5`, `1`)".to_string(),
        CaptureType::Length => "A CSS length (e.g., `10px`, `2rem`, `50%`)".to_string(),
        CaptureType::Duration => "A time duration (e.g., `0.3s`, `300ms`)".to_string(),
        CaptureType::Easing => {
            "An easing function (e.g., `ease-out`, `cubic-bezier(0.4, 0, 0.2, 1)`)".to_string()
        }
        CaptureType::Typeref => "A type reference (e.g., `User`, `string`)".to_string(),
        CaptureType::Binding => "A binding expression (e.g., `$.user.name`)".to_string(),
        CaptureType::Event => "An event name (e.g., `click`, `hover`, `scroll`)".to_string(),
        CaptureType::Expr => "A Spacetime expression".to_string(),
        CaptureType::Properties => "A block of CSS property declarations".to_string(),
        CaptureType::Fields => "Field definitions for a type or data structure".to_string(),
        CaptureType::Params => "Parameter definitions".to_string(),
        CaptureType::States => "State machine state definitions".to_string(),
        CaptureType::Transitions => "State transition definitions".to_string(),
        CaptureType::Keyframes => "Keyframe animation definitions".to_string(),
        CaptureType::Selector => "A CSS selector (e.g., `.class`, `#id`, `element`)".to_string(),
        CaptureType::Element => "An element reference (e.g., `&self`, `&container`)".to_string(),
        CaptureType::Preset => {
            "A RETIRED capture type — `~` preset refs are gone; named easings are `@form easing --name`".to_string()
        }
        CaptureType::MutationActions => "Mutation action definitions".to_string(),
        CaptureType::Union(variants) => {
            let opts: Vec<String> = variants.iter().map(|v| format!("`\"{}\"`", v)).collect();
            format!("One of: {}", opts.join(", "))
        }
        CaptureType::Template => "A template definition".to_string(),
        CaptureType::ParamList => "A parameter list".to_string(),
        CaptureType::HtmlBlock => "An HTML block with interpolation".to_string(),
        CaptureType::JsBlock => "A JavaScript block with multi-statement code".to_string(),
        CaptureType::ComponentBody => {
            "A structured component body with HTML, CSS rules, state declarations, and directives"
                .to_string()
        }
        CaptureType::TemplateInvocation => "A template invocation".to_string(),
        CaptureType::PatternMatch { variant, bindings } => {
            if bindings.is_empty() {
                format!(
                    "A pattern match expression (e.g., `$signal is {}`)",
                    variant
                )
            } else {
                format!(
                    "A pattern match expression with bindings (e.g., `$signal is {} {{ {} }}`)",
                    variant,
                    bindings
                        .iter()
                        .map(|b| format!("${}", b))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
        CaptureType::Custom(name) => format!("A custom capture type '{}'", name),
        CaptureType::Balanced(d) => {
            format!(
                "A balanced token run collected until '{}' at nesting depth 0",
                d
            )
        }
        CaptureType::SkipBlock => "A skipped balanced { ... } block".to_string(),
        CaptureType::Color => {
            "A CSS color value (e.g., `#E85D4A`, `rgb(232, 93, 74)`, `cyan`, `oklch(0.7 0.15 180)`)"
                .to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_directive_at_offset_simple() {
        let content = "@scroll";
        let ctx = find_directive_at_offset(content, 3);
        assert!(matches!(ctx, Some(HoverContext::DirectiveName { name, .. }) if name == "scroll"));
    }

    #[test]
    fn test_find_directive_at_offset_with_context() {
        let content = ".hero { @fade-in }";
        let ctx = find_directive_at_offset(content, 12);
        assert!(matches!(ctx, Some(HoverContext::DirectiveName { name, .. }) if name == "fade-in"));
    }

    #[test]
    fn test_find_directive_at_offset_not_on_directive() {
        let content = "some text";
        let ctx = find_directive_at_offset(content, 3);
        assert!(ctx.is_none());
    }

    #[test]
    fn test_find_directive_inside_parens_returns_none() {
        let content = "@scroll(duration: 0.3s)";
        let ctx = find_directive_at_offset(content, 15);
        assert!(ctx.is_none());
    }

    #[test]
    fn test_extract_hover_context_directive() {
        let content = "@scroll";
        let ctx = extract_hover_context(content, 4);
        assert!(matches!(ctx, HoverContext::DirectiveName { name, .. } if name == "scroll"));
    }

    #[test]
    fn test_extract_hover_context_none() {
        let content = "some text";
        let ctx = extract_hover_context(content, 3);
        assert!(matches!(ctx, HoverContext::None));
    }
}
