//! Meta-AST walker for extracting directive rules

use crate::metasystem::MetaRegistry;
use crate::parser::meta_ast::{
    CaptureModifier, CaptureType, FormCapture, FormClause, FormInlineElement, FormParam,
    MacroDefAst,
};

use super::{DirectiveRule, InlineRule, ParamRule};

pub fn extract_directives(registry: &MetaRegistry) -> Vec<DirectiveRule> {
    registry
        .iter_macros()
        .filter_map(|(_, macro_def)| {
            macro_def
                .form
                .as_ref()
                .map(|form| form_to_directive_rule(macro_def, form))
        })
        .collect()
}

/// Convert a FormClause to a DirectiveRule
fn form_to_directive_rule(_macro_def: &MacroDefAst, form: &FormClause) -> DirectiveRule {
    DirectiveRule {
        name: form.directive_name.clone(),
        params: form.params.iter().map(form_param_to_rule).collect(),
        inline_elements: form
            .inline_elements
            .iter()
            .filter_map(inline_element_to_rule)
            .collect(),
        has_body: form.body_capture.is_some() || !form.body_params.is_empty(),
        body_params: form.body_params.iter().map(form_param_to_rule).collect(),
    }
}

/// Convert a FormParam to a ParamRule
fn form_param_to_rule(param: &FormParam) -> ParamRule {
    ParamRule {
        name: param.name.clone(),
        elements: param
            .elements
            .iter()
            .filter_map(inline_element_to_rule)
            .collect(),
        default: param.default.clone(),
    }
}

/// Convert a FormInlineElement to an InlineRule
fn inline_element_to_rule(el: &FormInlineElement) -> Option<InlineRule> {
    match el {
        FormInlineElement::Capture(capture, _default) => Some(capture_to_inline_rule(capture)),
        FormInlineElement::Literal(lit) => Some(InlineRule::Literal(lit.clone())),
        FormInlineElement::Comparison { operator, capture } => Some(InlineRule::Comparison {
            operator: operator.clone(),
            capture_type: capture.capture_type.clone(),
            modifier: capture.modifier,
        }),
        FormInlineElement::KeywordBlock {
            keyword,
            body_params,
            modifier,
        } => Some(InlineRule::KeywordBlock {
            keyword: keyword.clone(),
            body_params: body_params.iter().map(form_param_to_rule).collect(),
            modifier: *modifier,
        }),
        FormInlineElement::PseudoSelector {
            name,
            body_params,
            modifier,
        } => Some(InlineRule::PseudoSelector {
            name: name.clone(),
            body_params: body_params.iter().map(form_param_to_rule).collect(),
            modifier: *modifier,
        }),
        // A GROUP has no single inline rule (its elements match as a unit); the
        // treesitter walker falls back to no rule for it, which is fine — the
        // primary matcher still handles it.
        FormInlineElement::Group { .. } => None,
        FormInlineElement::PseudoClass { name, body_params } => Some(InlineRule::PseudoClass {
            name: name.clone(),
            body_params: body_params.iter().map(form_param_to_rule).collect(),
        }),
    }
}

/// Convert a FormCapture to an InlineRule::Capture
fn capture_to_inline_rule(capture: &FormCapture) -> InlineRule {
    InlineRule::Capture {
        var_name: capture.var_name.clone(),
        capture_type: capture.capture_type.clone(),
        modifier: capture.modifier,
        alias: capture
            .alias_capture
            .as_ref()
            .map(|alias| Box::new(capture_to_inline_rule(alias))),
    }
}

/// Extract the primary capture type from a param's elements
/// Returns the first capture found, or Expr as default
pub fn extract_primary_capture(param: &ParamRule) -> (CaptureType, CaptureModifier) {
    for el in &param.elements {
        if let InlineRule::Capture {
            capture_type,
            modifier,
            ..
        } = el
        {
            return (capture_type.clone(), *modifier);
        }
    }
    // Default to Expr if no capture found
    (CaptureType::Expr, CaptureModifier::Required)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::meta_ast::{FormCapture, FormClause, FormInlineElement, FormParam};

    fn make_capture(var: &str, ct: CaptureType, modifier: CaptureModifier) -> FormCapture {
        FormCapture {
            var_name: var.to_string(),
            capture_type: ct,
            modifier,
            alias_capture: None,
        }
    }

    fn make_param(name: &str, capture: FormCapture) -> FormParam {
        FormParam {
            name: name.to_string(),
            elements: vec![FormInlineElement::Capture(capture, None)],
            default: None,
        }
    }

    #[test]
    fn test_simple_directive() {
        let form = FormClause {
            directive_name: "fade-in".to_string(),
            inline_elements: vec![],
            params: vec![make_param(
                "duration",
                make_capture("duration", CaptureType::Time, CaptureModifier::Required),
            )],
            post_arg_inline: vec![],
            body_capture: None,
            body_params: vec![],
            body_groups: Vec::new(),
            span: Default::default(),
        };

        let macro_def = MacroDefAst {
            name: "fade-in".to_string(),
            form: Some(form.clone()),
            ..Default::default()
        };

        let rule = form_to_directive_rule(&macro_def, &form);

        assert_eq!(rule.name, "fade-in");
        assert_eq!(rule.params.len(), 1);
        assert_eq!(rule.params[0].name, "duration");
        assert!(!rule.has_body);
    }

    #[test]
    fn test_directive_with_body() {
        let form = FormClause {
            directive_name: "data".to_string(),
            inline_elements: vec![],
            params: vec![
                make_param(
                    "src",
                    make_capture("src", CaptureType::String, CaptureModifier::Required),
                ),
                make_param(
                    "as",
                    make_capture("name", CaptureType::Ident, CaptureModifier::Optional),
                ),
            ],
            post_arg_inline: vec![],
            body_capture: Some("content".to_string()),
            body_params: vec![],
            body_groups: Vec::new(),
            span: Default::default(),
        };

        let macro_def = MacroDefAst {
            name: "data-fetch".to_string(),
            form: Some(form.clone()),
            ..Default::default()
        };

        let rule = form_to_directive_rule(&macro_def, &form);

        assert_eq!(rule.name, "data");
        assert_eq!(rule.params.len(), 2);
        assert!(rule.has_body);
    }

    #[test]
    fn test_inline_elements() {
        let form = FormClause {
            directive_name: "on".to_string(),
            inline_elements: vec![FormInlineElement::Capture(
                make_capture("event", CaptureType::Event, CaptureModifier::Required),
                None,
            )],
            params: vec![make_param(
                "selector",
                make_capture("selector", CaptureType::Selector, CaptureModifier::Optional),
            )],
            post_arg_inline: vec![],
            body_capture: Some("body".to_string()),
            body_params: vec![],
            body_groups: Vec::new(),
            span: Default::default(),
        };

        let macro_def = MacroDefAst {
            name: "on-event".to_string(),
            form: Some(form.clone()),
            ..Default::default()
        };

        let rule = form_to_directive_rule(&macro_def, &form);

        assert_eq!(rule.name, "on");
        assert_eq!(rule.inline_elements.len(), 1);
        if let InlineRule::Capture { var_name, .. } = &rule.inline_elements[0] {
            assert_eq!(var_name, "event");
        } else {
            panic!("Expected Capture inline element");
        }
    }

    #[test]
    fn test_extract_primary_capture() {
        let param = ParamRule {
            name: "duration".to_string(),
            elements: vec![InlineRule::Capture {
                var_name: "dur".to_string(),
                capture_type: CaptureType::Time,
                modifier: CaptureModifier::Optional,
                alias: None,
            }],
            default: None,
        };

        let (ct, modifier) = extract_primary_capture(&param);
        assert_eq!(ct, CaptureType::Time);
        assert_eq!(modifier, CaptureModifier::Optional);
    }
}
