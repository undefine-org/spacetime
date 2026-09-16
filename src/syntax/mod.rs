//! Syntax Module - Universal Form-Based Syntax Definitions and Span Utilities
//!
//! This module implements the universal %form-based syntax system.
//! All user-facing syntax (not just @macros, but also $variables, &elements, etc.)
//! is defined via %form patterns in .st files, not hardcoded in Rust.
//!
//! # Architecture
//!
//! ```text
//! stdlib/*.st files → SyntaxRegistry (prefix-indexed %forms)
//!                           ↓
//! source.st + registry → Vec<FormMatch> (unified AST)
//!                           ↓
//! resolve, sort, expand → CompiledOutput
//! ```
//!
//! # Key Principle
//!
//! "%form needs to be the unique reference of the syntax that defines it."
//! (from standards.org)
//!
//! The prefix character (@, $, &, #, etc.) is extracted from the %form pattern
//! itself - no separate %creates declaration needed.

mod bootstrap;
pub mod cst;
pub mod events;
mod form_match;
mod registry;
pub mod span;
mod stdlib_registry;
mod visitor;

pub(crate) mod conversions;
pub mod form_scoring;
/// `(directive, property) -> capture type`, derived from registered forms
/// rather than written down a second time (PLAN-136 W3).
pub mod property_types;

pub use bootstrap::{BootstrapError, bootstrap_stdlib, load_macros_from_dir};
pub use form_match::{
    CapturedValue, ComponentStateDecl, ExportDecl, FormMatch, JsQuoting, KeyframeDef, LengthValue,
    ParamDef, PropertyDef, SignalScope, TemplateParamDef, TemplateParamKind, TemplateRef,
    VariableResolver, bind_value_to_js, collect_dotted_signal_reads, collect_signal_deps,
    collect_st_var_deps, handler_body_signal_deps, on_motion_body_signal_deps,
    component_body_to_js_from_scope, component_payload_to_js, doc_comment_before, file_header_doc, inject_provenance_into_matches,
    inject_template_provenance, normalize_invocation_args, resolve_bind_value_css_override,
    signal_read, split_filter_pipe, state_decls_to_js, transpile_signal_expr,
    transpile_signal_expr_with,
};
pub use registry::{RegisteredForm, SyntaxRegistry, extract_prefix};
pub use stdlib_registry::{
    STDLIB_FORM_DECLARATIONS, STDLIB_FORM_DECLARATION_MATCHES, STDLIB_REGISTRY, is_stdlib_loaded, match_statement_stdlib,
    stdlib_form_count, stdlib_prefixes,
};
pub use visitor::{
    BindingCollector, BindingRenamer, ElementRefCollector, FormMatchCounter, MacroCollector,
    TreePrinter, Visitor, VisitorMut,
};

/// Compose a parent + child selector following CSS nesting conventions.
///
/// The SINGLE owner of selector composition (BUG-206): nested selector scopes
/// store only their authored LEAF text; this composes the full path. Both the
/// scope-tree builder (which precomputes each scope's `composed_selector`) and
/// any ad-hoc consumer resolve through here, so directive bindings and CSS emit
/// can never diverge on what `> .controls > button.inc` means.
///
/// `&` concatenates (`&:hover`); a leading combinator (`>`/`+`/`~`) or any other
/// child is joined as a descendant/compound (`parent child`).
pub(crate) fn compose_selector(parent: &str, child: &str) -> String {
    if child.starts_with('&') {
        // & = concatenation: .btn + &:hover → .btn:hover
        format!("{}{}", parent, &child[1..])
    } else {
        // Combinator (`.parent > .child`) and descendant (`.parent .child`) both
        // join with a single space — the leading `>`/`+`/`~` carries the relation.
        format!("{} {}", parent, child)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SourceSpan;
    use crate::parser::meta_ast::{
        CaptureModifier, CaptureType, FormCapture, FormClause, FormInlineElement, MacroDefAst,
    };

    fn make_test_macro(name: &str, directive_name: &str) -> MacroDefAst {
        MacroDefAst {
            retired: None,
            name: name.to_string(),
            form: Some(FormClause {
                directive_name: directive_name.to_string(),
                inline_elements: vec![],
                params: vec![],
                post_arg_inline: vec![],
                body_capture: None,
                body_params: Vec::new(),
                body_groups: Vec::new(),
                span: SourceSpan::default(),
            }),
            binds: vec![],
            derives: vec![],
            states: None,
            registers: None,
            imports: None,
            order: None,
            resolves: None,
            scopes: vec![],
            scope_within: Vec::new(),
            scope_element: Vec::new(),
            body: vec![],
            span: SourceSpan::default(),
            source_file: None,
            requires: vec![],
            module: None,
            doc: None,
            ..Default::default()
        }
    }
    

    fn make_local_state_macro() -> MacroDefAst {
        // Simulates: %form { $name:ident $type:ident : $value:expr ; }
        MacroDefAst {
            retired: None,
            name: "local-state".to_string(),
            form: Some(FormClause {
                directive_name: "$".to_string(), // Prefix is $
                inline_elements: vec![
                    FormInlineElement::Capture(
                        FormCapture {
                            var_name: "name".to_string(),
                            capture_type: CaptureType::Ident,
                            modifier: CaptureModifier::Required,
                            alias_capture: None,
                        },
                        None,
                    ),
                    FormInlineElement::Capture(
                        FormCapture {
                            var_name: "type".to_string(),
                            capture_type: CaptureType::Ident,
                            modifier: CaptureModifier::Required,
                            alias_capture: None,
                        },
                        None,
                    ),
                    FormInlineElement::Literal(":".to_string()),
                    FormInlineElement::Capture(
                        FormCapture {
                            var_name: "value".to_string(),
                            capture_type: CaptureType::Expr,
                            modifier: CaptureModifier::Required,
                            alias_capture: None,
                        },
                        None,
                    ),
                    FormInlineElement::Literal(";".to_string()),
                ],
                params: vec![],
                post_arg_inline: vec![],
                body_capture: None,
                body_params: Vec::new(),
                body_groups: Vec::new(),
                span: SourceSpan::default(),
            }),
            binds: vec![],
            derives: vec![],
            states: None,
            registers: None,
            imports: None,
            order: None,
            resolves: None,
            scopes: vec![],
            scope_within: Vec::new(),
            scope_element: Vec::new(),
            body: vec![],
            span: SourceSpan::default(),
            source_file: None,
            requires: vec![],
            module: None,
            doc: None,
            ..Default::default()
        }
    }
    

    #[test]
    fn test_extract_prefix_at_directive() {
        let macro_def = make_test_macro("data", "@data");
        assert_eq!(extract_prefix(&macro_def), Some('@'));
    }

    #[test]
    fn test_extract_prefix_dollar_variable() {
        let macro_def = make_local_state_macro();
        assert_eq!(extract_prefix(&macro_def), Some('$'));
    }

    #[test]
    fn test_registry_prefix_indexing() {
        let mut registry = SyntaxRegistry::new();

        let data_macro = make_test_macro("data", "@data");
        let each_macro = make_test_macro("each", "@each");
        let local_macro = make_local_state_macro();

        registry.register(&data_macro);
        registry.register(&each_macro);
        registry.register(&local_macro);

        // '@' prefix should have 2 forms
        assert_eq!(registry.forms_for_prefix('@').len(), 2);

        // '$' prefix should have 1 form
        assert_eq!(registry.forms_for_prefix('$').len(), 1);

        // '&' prefix should have 0 forms
        assert_eq!(registry.forms_for_prefix('&').len(), 0);
    }

    #[test]
    fn test_form_match_captures() {
        let mut captures = std::collections::HashMap::new();
        captures.insert(
            "name".to_string(),
            CapturedValue::Ident("count".to_string()),
        );
        captures.insert(
            "type".to_string(),
            CapturedValue::Ident("number".to_string()),
        );
        captures.insert("value".to_string(), CapturedValue::Number(0.0));

        let form_match = FormMatch {
            macro_name: "local-state".to_string(),
            matched_macro: None,
            captures,
            capture_spans: std::collections::HashMap::new(),
            selector: None,
            span: SourceSpan::default(),
            source_file: None,
            namespace_qualifier: Vec::new(),
            doc: None,
        };

        assert_eq!(form_match.macro_name, "local-state");
        assert!(form_match.get_ident("name").is_some());
        assert_eq!(form_match.get_ident("name").unwrap(), "count");
    }
}

/// Substitute a `$name` param as a TOKEN, never a substring: `$x` must not
/// rewrite `$x2` (prefix corruption, PLAN-124 W3 review P2), and a replacement
/// value that itself names another param must not be re-rewritten by a later
/// pass. THE single owner of form/param token substitution — motion forms
/// (pipeline/drivers.rs) and style-form splices (parser/mod.rs, BUG-298)
/// both substitute through here.
pub(crate) fn replace_param_token(hay: &str, needle: &str, value: &str) -> String {
    if !hay.contains(needle) {
        return hay.to_string();
    }
    let mut out = String::with_capacity(hay.len());
    let mut rest = hay;
    while let Some(pos) = rest.find(needle) {
        let after = &rest[pos + needle.len()..];
        let boundary = after
            .chars()
            .next()
            .map(|c| !(c.is_alphanumeric() || c == '_'))
            .unwrap_or(true);
        if boundary {
            out.push_str(&rest[..pos]);
            out.push_str(value);
        } else {
            out.push_str(&rest[..pos + needle.len()]);
        }
        rest = after;
    }
    out.push_str(rest);
    out
}

