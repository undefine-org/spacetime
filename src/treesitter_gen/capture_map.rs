//! Mapping from Spacetime CaptureType to tree-sitter rule references
//!
//! These functions are used to map Spacetime capture types to their tree-sitter
//! equivalents. Currently used primarily for testing and future enhancements.

#![allow(dead_code)]

use crate::parser::meta_ast::{CaptureModifier, CaptureType};

/// Map CaptureType to tree-sitter rule reference
pub fn capture_to_ts(ct: &CaptureType) -> &'static str {
    match ct {
        CaptureType::Ident => "$.identifier",
        // Maps to `$.identifier`, not an invented `$.dashed_identifier`: the
        // lexer merges `--rise` into ONE identifier token, so that IS the
        // tree-sitter shape. The `--` requirement is a CAPTURE-TYPE constraint
        // enforced by DashedIdentExtractor, not a distinct terminal — emitting a
        // rule name the base grammar does not define would produce an invalid
        // grammar.
        CaptureType::DashedIdent => "$.identifier",
        CaptureType::EventName => "$.identifier",
        CaptureType::String => "$.string",
        CaptureType::Number => "$.number",
        CaptureType::Bool => "choice('true', 'false')",
        CaptureType::Time | CaptureType::Duration => "$.duration",
        CaptureType::Length => "$.dimension",
        CaptureType::Easing => "$.easing_value",
        CaptureType::Typeref => "$.type_ref",
        CaptureType::Binding => "$.variable_ref",
        CaptureType::Event => "$.identifier",
        CaptureType::Expr => "$._expression",
        CaptureType::Properties => "repeat($.property_decl)",
        CaptureType::Fields => "repeat($.field_decl)",
        CaptureType::Params => "$.param_list",
        CaptureType::States => "repeat($.state_decl)",
        CaptureType::Transitions => "repeat($.transition_decl)",
        CaptureType::Keyframes => "$.keyframe_block",
        CaptureType::Selector => "$.selector",
        CaptureType::Element => "$.element_ref",
        CaptureType::Preset => "$.preset_ref",
        CaptureType::Color => "$.color_value",
        CaptureType::MutationActions => "repeat($.mutation_action)",
        CaptureType::Template => "$.template_content",
        CaptureType::ParamList => "$.param_list",
        CaptureType::HtmlBlock => "$.html_block",
        CaptureType::JsBlock => "$.js_block",
        CaptureType::TemplateInvocation => "$.template_invocation",
        CaptureType::ComponentBody => "$.component_body",
        CaptureType::Union(_) => "$._value", // Union types match any value
        CaptureType::PatternMatch { .. } => "$.pattern_match",
        CaptureType::Balanced(_) => "$._balanced", // depth-aware raw run
        CaptureType::SkipBlock => "$._skip_block",
        CaptureType::Custom(name) => {
            // Return a placeholder - we'll handle custom types specially
            // This will be replaced by the emitter with the actual custom type rule
            Box::leak(format!("$.{}_type", name.replace("-", "_")).into_boxed_str())
        }
    }
}

/// Map CaptureType to a simpler tree-sitter rule for value contexts
/// (used when we need a value that can appear in argument position)
pub fn capture_to_ts_value(ct: &CaptureType) -> &'static str {
    match ct {
        CaptureType::Ident => "$.identifier",
        CaptureType::DashedIdent => "$.identifier",
        CaptureType::EventName => "$.identifier",
        CaptureType::String => "$.string",
        CaptureType::Number => "$.number",
        CaptureType::Bool => "choice('true', 'false')",
        CaptureType::Time | CaptureType::Duration => "$.duration",
        CaptureType::Length => "$.dimension",
        CaptureType::Easing => "$.easing_value",
        CaptureType::Typeref => "$.type_ref",
        CaptureType::Binding => "$.variable_ref",
        CaptureType::Event => "$.identifier",
        CaptureType::Expr => "$._value",
        CaptureType::Selector => "$.selector",
        CaptureType::Element => "$.element_ref",
        CaptureType::Preset => "$.preset_ref",
        CaptureType::Template => "$.template_string",
        CaptureType::TemplateInvocation => "$.template_invocation",
        CaptureType::Union(_) => "$._value",
        _ => "$._value",
    }
}

/// Wrap rule with modifier (optional, repeat)
pub fn apply_modifier(rule: &str, modifier: CaptureModifier) -> String {
    match modifier {
        CaptureModifier::Required => rule.to_string(),
        CaptureModifier::Optional => format!("optional({})", rule),
        CaptureModifier::ZeroOrMore => format!("repeat({})", rule),
        CaptureModifier::OneOrMore => format!("repeat1({})", rule),
        // Tree-sitter has no counted-repetition combinator. The generated grammar
        // is for EDITOR highlighting, where a hex run and an identifier-ish run
        // want the same colour anyway, so an unbounded repeat is the honest
        // approximation — length enforcement stays with the compiler, which is the
        // authority. (Same reasoning as DashedIdent mapping to $.identifier.)
        CaptureModifier::Counted(_) => format!("repeat1({})", rule),
    }
}

/// Check if this capture type represents a block-level construct
pub fn is_block_type(ct: &CaptureType) -> bool {
    matches!(
        ct,
        CaptureType::Properties
            | CaptureType::Fields
            | CaptureType::States
            | CaptureType::Transitions
            | CaptureType::Keyframes
            | CaptureType::HtmlBlock
            | CaptureType::JsBlock
            | CaptureType::ComponentBody
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_capture_types() {
        assert_eq!(capture_to_ts(&CaptureType::Ident), "$.identifier");
        assert_eq!(capture_to_ts(&CaptureType::String), "$.string");
        assert_eq!(capture_to_ts(&CaptureType::Number), "$.number");
        assert_eq!(capture_to_ts(&CaptureType::Time), "$.duration");
        assert_eq!(capture_to_ts(&CaptureType::Binding), "$.variable_ref");
        assert_eq!(capture_to_ts(&CaptureType::Element), "$.element_ref");
        assert_eq!(capture_to_ts(&CaptureType::Selector), "$.selector");
    }

    #[test]
    fn test_optional_modifier() {
        let result = apply_modifier("$.duration", CaptureModifier::Optional);
        assert_eq!(result, "optional($.duration)");
    }

    #[test]
    fn test_zero_or_more_modifier() {
        let result = apply_modifier("$.identifier", CaptureModifier::ZeroOrMore);
        assert_eq!(result, "repeat($.identifier)");
    }

    #[test]
    fn test_one_or_more_modifier() {
        let result = apply_modifier("$.identifier", CaptureModifier::OneOrMore);
        assert_eq!(result, "repeat1($.identifier)");
    }

    #[test]
    fn test_required_modifier() {
        let result = apply_modifier("$.identifier", CaptureModifier::Required);
        assert_eq!(result, "$.identifier");
    }
}
