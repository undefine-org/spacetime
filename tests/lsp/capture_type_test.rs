//! Tests for LSP integration with %capture_type definitions.
//!
//! This module tests the user experience for custom capture types like `param_list`,
//! including hover, completions, form registry integration, and diagnostics.

use std::path::PathBuf;

use spacetime::lsp::form_registry::{DirectiveParam, DirectiveSignature, FormRegistry};
use spacetime::lsp::{DocumentState, HoverContext, extract_hover_context, provide_hover};
use spacetime::parser::SourceSpan;
use spacetime::parser::meta_ast::{CaptureModifier, CaptureType};

use tower_lsp::lsp_types::{HoverContents, Position};

// =============================================================================
// Helper Functions
// =============================================================================

/// Create a test FormRegistry with directives using various capture types.
fn create_test_registry_with_capture_types() -> FormRegistry {
    let mut registry = FormRegistry::new();

    // @template directive with param_list type
    registry.register(DirectiveSignature {
        name: "template".to_string(),
        macro_name: "TemplateMacro".to_string(),
        params: vec![
            DirectiveParam {
                name: "name".to_string(),
                capture_var: "name".to_string(),
                capture_type: CaptureType::Ident,
                modifier: CaptureModifier::Required,
                default_value: None,
            },
            DirectiveParam {
                name: "params".to_string(),
                capture_var: "params".to_string(),
                capture_type: CaptureType::ParamList,
                modifier: CaptureModifier::Required,
                default_value: None,
            },
        ],
        body_type: Some(CaptureType::HtmlBlock),
        definition_span: SourceSpan::default(),
        source_file: PathBuf::from("/stdlib/template.st"),
        documentation: Some("Define a reusable template with parameters.".to_string()),
        exports: vec![],
        bound_primitives: vec![],
        form: None,
    });

    // @each directive with template_invocation type
    registry.register(DirectiveSignature {
        name: "each".to_string(),
        macro_name: "EachMacro".to_string(),
        params: vec![
            DirectiveParam {
                name: "source".to_string(),
                capture_var: "source".to_string(),
                capture_type: CaptureType::Binding,
                modifier: CaptureModifier::Required,
                default_value: None,
            },
            DirectiveParam {
                name: "item".to_string(),
                capture_var: "item".to_string(),
                capture_type: CaptureType::Ident,
                modifier: CaptureModifier::Required,
                default_value: None,
            },
            DirectiveParam {
                name: "invocations".to_string(),
                capture_var: "invocations".to_string(),
                capture_type: CaptureType::TemplateInvocation,
                modifier: CaptureModifier::ZeroOrMore,
                default_value: None,
            },
        ],
        body_type: None,
        definition_span: SourceSpan::default(),
        source_file: PathBuf::from("/stdlib/each.st"),
        documentation: Some("Iterate over a data source.".to_string()),
        exports: vec![],
        bound_primitives: vec![],
        form: None,
    });

    // @slot directive with template capture type
    registry.register(DirectiveSignature {
        name: "slot".to_string(),
        macro_name: "SlotMacro".to_string(),
        params: vec![DirectiveParam {
            name: "content".to_string(),
            capture_var: "content".to_string(),
            capture_type: CaptureType::Template,
            modifier: CaptureModifier::Required,
            default_value: None,
        }],
        body_type: None,
        definition_span: SourceSpan::default(),
        source_file: PathBuf::from("/stdlib/slot.st"),
        documentation: Some("Define a slot for template content.".to_string()),
        exports: vec![],
        bound_primitives: vec![],
        form: None,
    });

    // @custom-type directive with Custom capture type
    registry.register(DirectiveSignature {
        name: "custom-handler".to_string(),
        macro_name: "CustomHandlerMacro".to_string(),
        params: vec![DirectiveParam {
            name: "handler".to_string(),
            capture_var: "handler".to_string(),
            capture_type: CaptureType::Custom("event_handler".to_string()),
            modifier: CaptureModifier::Required,
            default_value: None,
        }],
        body_type: None,
        definition_span: SourceSpan::default(),
        source_file: PathBuf::from("/stdlib/custom.st"),
        documentation: Some("Register a custom event handler.".to_string()),
        exports: vec![],
        bound_primitives: vec![],
        form: None,
    });

    registry
}

/// Create a DocumentState for testing.
fn document_from(content: &str) -> DocumentState {
    DocumentState::new(content.to_string(), 1)
}

// =============================================================================
// Test: Hover on Directive with ParamList Capture Type
// =============================================================================

#[test]
fn hover_on_template_directive_shows_param_list_type() {
    let doc = document_from("@template");
    let registry = create_test_registry_with_capture_types();
    let hover = provide_hover(
        &doc,
        Position {
            line: 0,
            character: 5,
        },
        &registry,
    );

    assert!(hover.is_some());
    let hover = hover.unwrap();

    if let HoverContents::Markup(markup) = hover.contents {
        // Should show param_list in the signature
        assert!(
            markup.value.contains("param_list"),
            "Hover should show 'param_list' type in signature. Got: {}",
            markup.value
        );
        // Should show the directive name
        assert!(
            markup.value.contains("@template"),
            "Hover should show '@template'. Got: {}",
            markup.value
        );
    } else {
        panic!("Expected markup content");
    }
}

#[test]
fn hover_on_template_directive_shows_documentation() {
    let doc = document_from("@template");
    let registry = create_test_registry_with_capture_types();
    let hover = provide_hover(
        &doc,
        Position {
            line: 0,
            character: 5,
        },
        &registry,
    );

    assert!(hover.is_some());
    let hover = hover.unwrap();

    if let HoverContents::Markup(markup) = hover.contents {
        assert!(
            markup.value.contains("Define a reusable template"),
            "Hover should show documentation. Got: {}",
            markup.value
        );
    } else {
        panic!("Expected markup content");
    }
}

#[test]
fn hover_on_each_directive_shows_template_invocation_type() {
    let doc = document_from("@each");
    let registry = create_test_registry_with_capture_types();
    let hover = provide_hover(
        &doc,
        Position {
            line: 0,
            character: 3,
        },
        &registry,
    );

    assert!(hover.is_some());
    let hover = hover.unwrap();

    if let HoverContents::Markup(markup) = hover.contents {
        // Should show template_invocation in the signature
        assert!(
            markup.value.contains("template_invocation"),
            "Hover should show 'template_invocation' type. Got: {}",
            markup.value
        );
    } else {
        panic!("Expected markup content");
    }
}

#[test]
fn hover_on_slot_directive_shows_template_type() {
    let doc = document_from("@slot");
    let registry = create_test_registry_with_capture_types();
    let hover = provide_hover(
        &doc,
        Position {
            line: 0,
            character: 3,
        },
        &registry,
    );

    assert!(hover.is_some());
    let hover = hover.unwrap();

    if let HoverContents::Markup(markup) = hover.contents {
        assert!(
            markup.value.contains("template"),
            "Hover should show 'template' type. Got: {}",
            markup.value
        );
    } else {
        panic!("Expected markup content");
    }
}

#[test]
fn hover_on_custom_handler_shows_custom_type() {
    let doc = document_from("@custom-handler");
    let registry = create_test_registry_with_capture_types();
    let hover = provide_hover(
        &doc,
        Position {
            line: 0,
            character: 8,
        },
        &registry,
    );

    assert!(hover.is_some());
    let hover = hover.unwrap();

    if let HoverContents::Markup(markup) = hover.contents {
        // Custom types should display their name directly
        assert!(
            markup.value.contains("event_handler"),
            "Hover should show custom type name 'event_handler'. Got: {}",
            markup.value
        );
    } else {
        panic!("Expected markup content");
    }
}

// =============================================================================
// Test: FormRegistry Correctly Extracts ParamList from Macro Definitions
// =============================================================================

mod form_registry_integration {
    use spacetime::lsp::FormRegistry;
    use spacetime::metasystem::MetaRegistry;
    use spacetime::parser::SourceSpan;
    use spacetime::parser::meta_ast::{
        CaptureModifier, CaptureType, FormCapture, FormClause, FormInlineElement, FormParam,
        MacroDefAst, ParamDefault,
    };
    use std::path::Path;

    fn make_form_param(
        name: &str,
        var_name: &str,
        capture_type: CaptureType,
        modifier: CaptureModifier,
        default: Option<ParamDefault>,
    ) -> FormParam {
        FormParam {
            name: name.to_string(),
            elements: vec![FormInlineElement::Capture(
                FormCapture {
                    var_name: var_name.to_string(),
                    capture_type,
                    modifier,
                    alias_capture: None,
                },
                default,
            )],
            default: None,
        }
    }

    fn make_inline_capture(
        var_name: &str,
        capture_type: CaptureType,
        modifier: CaptureModifier,
    ) -> FormInlineElement {
        FormInlineElement::Capture(
            FormCapture {
                var_name: var_name.to_string(),
                capture_type,
                modifier,
                alias_capture: None,
            },
            None,
        )
    }

    fn make_form_clause(
        directive_name: &str,
        inline_elements: Vec<FormInlineElement>,
        params: Vec<FormParam>,
        has_body: bool,
    ) -> FormClause {
        FormClause {
            directive_name: directive_name.to_string(),
            inline_elements,
            params,
            post_arg_inline: vec![],
            body_capture: if has_body {
                Some("$body".to_string())
            } else {
                None
            },
            body_params: Vec::new(),
            body_groups: Vec::new(),
            span: SourceSpan::default(),
        }
    }

    fn make_macro(name: &str, _creates: Option<&str>, form: Option<FormClause>) -> MacroDefAst {
        MacroDefAst {
            name: name.to_string(),
            form,
            binds: vec![],
            derives: vec![],
            states: None,
            registers: None,
            imports: None,
            resolves: None,
            scopes: vec![],
            scope_within: vec![],
            order: None,
            body: vec![],
            requires: vec![],
            span: SourceSpan::default(),
            source_file: None,
            module: None,
            doc: None,
            retired: None,
            ..Default::default()
        }
    }

    #[test]
    fn form_registry_extracts_param_list_type() {
        // Test @template &name($params:param_list) pattern
        let inline_elements = vec![
            FormInlineElement::Literal("&".to_string()),
            make_inline_capture("name", CaptureType::Ident, CaptureModifier::Required),
        ];
        let params = vec![make_form_param(
            "params",
            "params",
            CaptureType::ParamList,
            CaptureModifier::Required,
            None,
        )];
        let form = make_form_clause("template", inline_elements, params, true);
        let macro_def = make_macro("TemplateMacro", Some("template"), Some(form));

        let mut registry = MetaRegistry::new();
        registry.register_macro(macro_def).unwrap();

        let form_registry =
            FormRegistry::from_meta_registry(&registry, Path::new("/stdlib/template.st"));

        let directive = form_registry.get_directive("template").unwrap();
        assert_eq!(directive.params.len(), 2);
        assert_eq!(directive.params[0].name, "name");
        assert_eq!(directive.params[0].capture_type, CaptureType::Ident);
        assert_eq!(directive.params[1].name, "params");
        assert_eq!(directive.params[1].capture_type, CaptureType::ParamList);
    }

    #[test]
    fn form_registry_extracts_html_block_type() {
        let params = vec![make_form_param(
            "content",
            "content",
            CaptureType::HtmlBlock,
            CaptureModifier::Required,
            None,
        )];
        let form = make_form_clause("fragment", vec![], params, false);
        let macro_def = make_macro("FragmentMacro", Some("fragment"), Some(form));

        let mut registry = MetaRegistry::new();
        registry.register_macro(macro_def).unwrap();

        let form_registry =
            FormRegistry::from_meta_registry(&registry, Path::new("/stdlib/fragment.st"));

        let directive = form_registry.get_directive("fragment").unwrap();
        assert_eq!(directive.params.len(), 1);
        assert_eq!(directive.params[0].capture_type, CaptureType::HtmlBlock);
    }

    #[test]
    fn form_registry_extracts_template_invocation_type() {
        let params = vec![make_form_param(
            "templates",
            "templates",
            CaptureType::TemplateInvocation,
            CaptureModifier::ZeroOrMore,
            None,
        )];
        let form = make_form_clause("render", vec![], params, false);
        let macro_def = make_macro("RenderMacro", Some("render"), Some(form));

        let mut registry = MetaRegistry::new();
        registry.register_macro(macro_def).unwrap();

        let form_registry =
            FormRegistry::from_meta_registry(&registry, Path::new("/stdlib/render.st"));

        let directive = form_registry.get_directive("render").unwrap();
        assert_eq!(directive.params.len(), 1);
        assert_eq!(
            directive.params[0].capture_type,
            CaptureType::TemplateInvocation
        );
        assert_eq!(directive.params[0].modifier, CaptureModifier::ZeroOrMore);
    }

    #[test]
    fn form_registry_extracts_custom_capture_type() {
        let params = vec![make_form_param(
            "handler",
            "handler",
            CaptureType::Custom("event_handler".to_string()),
            CaptureModifier::Required,
            None,
        )];
        let form = make_form_clause("on-event", vec![], params, false);
        let macro_def = make_macro("OnEventMacro", Some("on-event"), Some(form));

        let mut registry = MetaRegistry::new();
        registry.register_macro(macro_def).unwrap();

        let form_registry =
            FormRegistry::from_meta_registry(&registry, Path::new("/stdlib/on-event.st"));

        let directive = form_registry.get_directive("on-event").unwrap();
        assert_eq!(directive.params.len(), 1);
        match &directive.params[0].capture_type {
            CaptureType::Custom(name) => {
                assert_eq!(name, "event_handler");
            }
            _ => panic!("Expected Custom capture type"),
        }
    }

    #[test]
    fn form_registry_handles_multiple_param_list_params() {
        // Test a directive with multiple special capture types
        let params = vec![
            make_form_param(
                "params",
                "params",
                CaptureType::ParamList,
                CaptureModifier::Required,
                None,
            ),
            make_form_param(
                "slots",
                "slots",
                CaptureType::Template,
                CaptureModifier::Optional,
                None,
            ),
        ];
        let form = make_form_clause("component", vec![], params, true);
        let macro_def = make_macro("ComponentMacro", Some("component"), Some(form));

        let mut registry = MetaRegistry::new();
        registry.register_macro(macro_def).unwrap();

        let form_registry =
            FormRegistry::from_meta_registry(&registry, Path::new("/stdlib/component.st"));

        let directive = form_registry.get_directive("component").unwrap();
        assert_eq!(directive.params.len(), 2);
        assert_eq!(directive.params[0].capture_type, CaptureType::ParamList);
        assert_eq!(directive.params[1].capture_type, CaptureType::Template);
    }
}

// =============================================================================
// Test: Signature Formatting Includes ParamList Correctly
// =============================================================================

#[test]
fn signature_format_includes_param_list() {
    let sig = DirectiveSignature {
        name: "template".to_string(),
        macro_name: "TemplateMacro".to_string(),
        params: vec![
            DirectiveParam {
                name: "name".to_string(),
                capture_var: "name".to_string(),
                capture_type: CaptureType::Ident,
                modifier: CaptureModifier::Required,
                default_value: None,
            },
            DirectiveParam {
                name: "params".to_string(),
                capture_var: "params".to_string(),
                capture_type: CaptureType::ParamList,
                modifier: CaptureModifier::Required,
                default_value: None,
            },
        ],
        body_type: Some(CaptureType::HtmlBlock),
        definition_span: SourceSpan::default(),
        source_file: PathBuf::from("/test.st"),
        documentation: None,
        exports: vec![],
        bound_primitives: vec![],
        form: None,
    };

    let formatted = sig.format_signature();
    assert!(
        formatted.contains("@template"),
        "Signature should include @template. Got: {}",
        formatted
    );
    assert!(
        formatted.contains("param_list"),
        "Signature should include param_list type. Got: {}",
        formatted
    );
    assert!(
        formatted.contains("{ ... }"),
        "Signature should include body indicator. Got: {}",
        formatted
    );
}

#[test]
fn signature_format_includes_html_block() {
    let sig = DirectiveSignature {
        name: "fragment".to_string(),
        macro_name: "FragmentMacro".to_string(),
        params: vec![DirectiveParam {
            name: "content".to_string(),
            capture_var: "content".to_string(),
            capture_type: CaptureType::HtmlBlock,
            modifier: CaptureModifier::Required,
            default_value: None,
        }],
        body_type: None,
        definition_span: SourceSpan::default(),
        source_file: PathBuf::from("/test.st"),
        documentation: None,
        exports: vec![],
        bound_primitives: vec![],
        form: None,
    };

    let formatted = sig.format_signature();
    assert!(
        formatted.contains("html_block"),
        "Signature should include html_block type. Got: {}",
        formatted
    );
}

#[test]
fn signature_format_includes_template_invocation() {
    let sig = DirectiveSignature {
        name: "render".to_string(),
        macro_name: "RenderMacro".to_string(),
        params: vec![DirectiveParam {
            name: "templates".to_string(),
            capture_var: "templates".to_string(),
            capture_type: CaptureType::TemplateInvocation,
            modifier: CaptureModifier::ZeroOrMore,
            default_value: None,
        }],
        body_type: None,
        definition_span: SourceSpan::default(),
        source_file: PathBuf::from("/test.st"),
        documentation: None,
        exports: vec![],
        bound_primitives: vec![],
        form: None,
    };

    let formatted = sig.format_signature();
    assert!(
        formatted.contains("template_invocation"),
        "Signature should include template_invocation type. Got: {}",
        formatted
    );
    // ZeroOrMore modifier should be shown
    assert!(
        formatted.contains("*"),
        "Signature should include * modifier for ZeroOrMore. Got: {}",
        formatted
    );
}

#[test]
fn signature_format_includes_custom_type_name() {
    let sig = DirectiveSignature {
        name: "handler".to_string(),
        macro_name: "HandlerMacro".to_string(),
        params: vec![DirectiveParam {
            name: "callback".to_string(),
            capture_var: "callback".to_string(),
            capture_type: CaptureType::Custom("event_handler".to_string()),
            modifier: CaptureModifier::Required,
            default_value: None,
        }],
        body_type: None,
        definition_span: SourceSpan::default(),
        source_file: PathBuf::from("/test.st"),
        documentation: None,
        exports: vec![],
        bound_primitives: vec![],
        form: None,
    };

    let formatted = sig.format_signature();
    assert!(
        formatted.contains("event_handler"),
        "Signature should include custom type name. Got: {}",
        formatted
    );
}

// =============================================================================
// Test: DirectiveParam Format Method for Capture Types
// =============================================================================

#[test]
fn directive_param_format_param_list() {
    let param = DirectiveParam {
        name: "params".to_string(),
        capture_var: "params".to_string(),
        capture_type: CaptureType::ParamList,
        modifier: CaptureModifier::Required,
        default_value: None,
    };

    let formatted = param.format();
    assert!(
        formatted.contains("param_list"),
        "Format should include 'param_list'. Got: {}",
        formatted
    );
    assert!(
        formatted.contains("params"),
        "Format should include param name. Got: {}",
        formatted
    );
}

#[test]
fn directive_param_format_html_block() {
    let param = DirectiveParam {
        name: "content".to_string(),
        capture_var: "content".to_string(),
        capture_type: CaptureType::HtmlBlock,
        modifier: CaptureModifier::Optional,
        default_value: None,
    };

    let formatted = param.format();
    assert!(
        formatted.contains("html_block"),
        "Format should include 'html_block'. Got: {}",
        formatted
    );
    assert!(
        formatted.contains("?"),
        "Format should include '?' for optional. Got: {}",
        formatted
    );
}

#[test]
fn directive_param_format_template_invocation_with_modifier() {
    let param = DirectiveParam {
        name: "invocations".to_string(),
        capture_var: "invocations".to_string(),
        capture_type: CaptureType::TemplateInvocation,
        modifier: CaptureModifier::OneOrMore,
        default_value: None,
    };

    let formatted = param.format();
    assert!(
        formatted.contains("template_invocation"),
        "Format should include 'template_invocation'. Got: {}",
        formatted
    );
    assert!(
        formatted.contains("+"),
        "Format should include '+' for OneOrMore. Got: {}",
        formatted
    );
}

#[test]
fn directive_param_format_custom_type() {
    let param = DirectiveParam {
        name: "handler".to_string(),
        capture_var: "handler".to_string(),
        capture_type: CaptureType::Custom("my_custom_type".to_string()),
        modifier: CaptureModifier::Required,
        default_value: None,
    };

    let formatted = param.format();
    assert!(
        formatted.contains("my_custom_type"),
        "Format should include custom type name. Got: {}",
        formatted
    );
}

// =============================================================================
// Test: Hover Context Extraction for Directives with Capture Types
// =============================================================================

#[test]
fn hover_context_on_template_directive() {
    let content = "@template";
    let ctx = extract_hover_context(content, 5);
    assert!(matches!(
        ctx,
        HoverContext::DirectiveName { name, .. } if name == "template"
    ));
}

#[test]
fn hover_context_on_each_directive() {
    let content = "@each";
    let ctx = extract_hover_context(content, 3);
    assert!(matches!(
        ctx,
        HoverContext::DirectiveName { name, .. } if name == "each"
    ));
}

#[test]
fn hover_context_in_multiline_template() {
    let content = r#"
.container {
    @template &card($title, $body) {
        <div class="card">`$title`</div>
    }
}
"#;
    // Hovering over "template" on line 2 (0-indexed)
    let lines: Vec<&str> = content.lines().collect();
    let template_line = lines[2];
    // Find the offset of "template" in the content
    let offset_to_line2: usize = content.lines().take(2).map(|l| l.len() + 1).sum();
    let template_offset_in_line = template_line.find("template").unwrap();
    let total_offset = offset_to_line2 + template_offset_in_line + 3; // +3 to be inside "template"

    let ctx = extract_hover_context(content, total_offset);
    assert!(matches!(
        ctx,
        HoverContext::DirectiveName { name, .. } if name == "template"
    ));
}

// =============================================================================
// Test: Edge Cases for Capture Type Handling
// =============================================================================

#[test]
fn param_list_with_default_value() {
    let param = DirectiveParam {
        name: "params".to_string(),
        capture_var: "params".to_string(),
        capture_type: CaptureType::ParamList,
        modifier: CaptureModifier::Optional,
        default_value: Some("()".to_string()),
    };

    let formatted = param.format();
    assert!(formatted.contains("param_list"));
    assert!(formatted.contains("?"));
    assert!(formatted.contains("= ()"));
}

#[test]
fn template_type_in_signature() {
    let sig = DirectiveSignature {
        name: "slot".to_string(),
        macro_name: "SlotMacro".to_string(),
        params: vec![DirectiveParam {
            name: "content".to_string(),
            capture_var: "content".to_string(),
            capture_type: CaptureType::Template,
            modifier: CaptureModifier::Required,
            default_value: None,
        }],
        body_type: None,
        definition_span: SourceSpan::default(),
        source_file: PathBuf::from("/test.st"),
        documentation: None,
        exports: vec![],
        bound_primitives: vec![],
        form: None,
    };

    let formatted = sig.format_signature();
    assert!(formatted.contains("template"));
    assert!(formatted.contains("@slot"));
}

#[test]
fn multiple_capture_types_in_signature() {
    let sig = DirectiveSignature {
        name: "widget".to_string(),
        macro_name: "WidgetMacro".to_string(),
        params: vec![
            DirectiveParam {
                name: "name".to_string(),
                capture_var: "name".to_string(),
                capture_type: CaptureType::Ident,
                modifier: CaptureModifier::Required,
                default_value: None,
            },
            DirectiveParam {
                name: "params".to_string(),
                capture_var: "params".to_string(),
                capture_type: CaptureType::ParamList,
                modifier: CaptureModifier::Optional,
                default_value: None,
            },
            DirectiveParam {
                name: "slots".to_string(),
                capture_var: "slots".to_string(),
                capture_type: CaptureType::Template,
                modifier: CaptureModifier::ZeroOrMore,
                default_value: None,
            },
            DirectiveParam {
                name: "handler".to_string(),
                capture_var: "handler".to_string(),
                capture_type: CaptureType::Custom("click_handler".to_string()),
                modifier: CaptureModifier::Optional,
                default_value: None,
            },
        ],
        body_type: Some(CaptureType::HtmlBlock),
        definition_span: SourceSpan::default(),
        source_file: PathBuf::from("/test.st"),
        documentation: None,
        exports: vec![],
        bound_primitives: vec![],
        form: None,
    };

    let formatted = sig.format_signature();

    // Verify all capture types are present
    assert!(formatted.contains("ident"), "Should contain ident type");
    assert!(
        formatted.contains("param_list"),
        "Should contain param_list type"
    );
    assert!(
        formatted.contains("template"),
        "Should contain template type"
    );
    assert!(
        formatted.contains("click_handler"),
        "Should contain custom type"
    );
    assert!(
        formatted.contains("{ ... }"),
        "Should contain body indicator"
    );
}

// =============================================================================
// Test: Capture Type Display in Completions (via signature format)
// =============================================================================

#[test]
fn capture_types_display_correctly_in_registry() {
    let registry = create_test_registry_with_capture_types();

    // Verify template directive
    let template = registry.get_directive("template").unwrap();
    assert_eq!(template.params.len(), 2);
    assert_eq!(template.params[1].capture_type, CaptureType::ParamList);

    // Verify each directive
    let each = registry.get_directive("each").unwrap();
    assert_eq!(each.params.len(), 3);
    assert_eq!(each.params[2].capture_type, CaptureType::TemplateInvocation);

    // Verify slot directive
    let slot = registry.get_directive("slot").unwrap();
    assert_eq!(slot.params[0].capture_type, CaptureType::Template);

    // Verify custom-handler directive
    let custom = registry.get_directive("custom-handler").unwrap();
    match &custom.params[0].capture_type {
        CaptureType::Custom(name) => assert_eq!(name, "event_handler"),
        _ => panic!("Expected Custom capture type"),
    }
}

#[test]
fn directive_completions_include_capture_type_directives() {
    let registry = create_test_registry_with_capture_types();

    // All directives should be available for completion
    let completions = registry.directive_completions("");
    let names: Vec<&str> = completions.iter().map(|(n, _)| n.as_str()).collect();

    assert!(names.contains(&"template"));
    assert!(names.contains(&"each"));
    assert!(names.contains(&"slot"));
    assert!(names.contains(&"custom-handler"));
}

#[test]
fn directive_completions_filter_by_prefix() {
    let registry = create_test_registry_with_capture_types();

    // "tem" should match template
    let completions = registry.directive_completions("tem");
    let names: Vec<&str> = completions.iter().map(|(n, _)| n.as_str()).collect();
    assert!(names.contains(&"template"));
    assert!(!names.contains(&"each"));

    // "sl" should match slot
    let completions = registry.directive_completions("sl");
    let names: Vec<&str> = completions.iter().map(|(n, _)| n.as_str()).collect();
    assert!(names.contains(&"slot"));
    assert!(!names.contains(&"template"));
}

// =============================================================================
// Test: CaptureType Equality for New Types
// =============================================================================

#[test]
fn capture_type_equality_param_list() {
    assert_eq!(CaptureType::ParamList, CaptureType::ParamList);
    assert_ne!(CaptureType::ParamList, CaptureType::Template);
}

#[test]
fn capture_type_equality_html_block() {
    assert_eq!(CaptureType::HtmlBlock, CaptureType::HtmlBlock);
    assert_ne!(CaptureType::HtmlBlock, CaptureType::ParamList);
}

#[test]
fn capture_type_equality_template_invocation() {
    assert_eq!(
        CaptureType::TemplateInvocation,
        CaptureType::TemplateInvocation
    );
    assert_ne!(CaptureType::TemplateInvocation, CaptureType::Template);
}

#[test]
fn capture_type_equality_custom() {
    assert_eq!(
        CaptureType::Custom("foo".to_string()),
        CaptureType::Custom("foo".to_string())
    );
    assert_ne!(
        CaptureType::Custom("foo".to_string()),
        CaptureType::Custom("bar".to_string())
    );
    assert_ne!(
        CaptureType::Custom("foo".to_string()),
        CaptureType::ParamList
    );
}

// =============================================================================
// Test: Source File Preserved for Capture Type Directives
// =============================================================================

#[test]
fn source_file_preserved_for_template_directive() {
    let registry = create_test_registry_with_capture_types();
    let template = registry.get_directive("template").unwrap();
    assert_eq!(template.source_file, PathBuf::from("/stdlib/template.st"));
}

#[test]
fn source_file_preserved_for_each_directive() {
    let registry = create_test_registry_with_capture_types();
    let each = registry.get_directive("each").unwrap();
    assert_eq!(each.source_file, PathBuf::from("/stdlib/each.st"));
}
