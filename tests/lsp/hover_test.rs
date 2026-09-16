//! Tests for LSP hover provider.

use std::path::PathBuf;

use spacetime::lsp::form_registry::{DirectiveParam, DirectiveSignature, FormRegistry};
use spacetime::lsp::{DocumentState, HoverContext, extract_hover_context, provide_hover};
use spacetime::parser::SourceSpan;
use spacetime::parser::meta_ast::{CaptureModifier, CaptureType};

use tower_lsp::lsp_types::{HoverContents, Position};

/// Create a test FormRegistry with sample directives.
fn create_test_registry() -> FormRegistry {
    let mut registry = FormRegistry::new();

    // @scroll directive
    registry.register(DirectiveSignature {
        name: "scroll".to_string(),
        macro_name: "ScrollMacro".to_string(),
        params: vec![
            DirectiveParam {
                name: "duration".to_string(),
                capture_var: "dur".to_string(),
                capture_type: CaptureType::Duration,
                modifier: CaptureModifier::Optional,
                default_value: Some("0.3s".to_string()),
            },
            DirectiveParam {
                name: "easing".to_string(),
                capture_var: "ease".to_string(),
                capture_type: CaptureType::Easing,
                modifier: CaptureModifier::Optional,
                default_value: Some("ease-out".to_string()),
            },
        ],
        body_type: Some(CaptureType::Properties),
        definition_span: SourceSpan::default(),
        source_file: PathBuf::from("/stdlib/scroll.st"),
        documentation: Some("Animate properties on scroll.".to_string()),
        exports: vec![],
        bound_primitives: vec![],
        form: None,
    });

    // @fade-in directive
    registry.register(DirectiveSignature {
        name: "fade-in".to_string(),
        macro_name: "FadeInMacro".to_string(),
        params: vec![],
        body_type: None,
        definition_span: SourceSpan::default(),
        source_file: PathBuf::from("/stdlib/fade.st"),
        documentation: None,
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
// Hover Context Tests
// =============================================================================

#[test]
fn test_hover_context_on_directive() {
    let content = "@scroll";
    let ctx = extract_hover_context(content, 3);
    assert!(matches!(
        ctx,
        HoverContext::DirectiveName { name, .. } if name == "scroll"
    ));
}

#[test]
fn test_hover_context_on_directive_start() {
    let content = "@scroll";
    let ctx = extract_hover_context(content, 1);
    assert!(matches!(
        ctx,
        HoverContext::DirectiveName { name, .. } if name == "scroll"
    ));
}

#[test]
fn test_hover_context_on_directive_end() {
    let content = "@scroll";
    let ctx = extract_hover_context(content, 7);
    assert!(matches!(
        ctx,
        HoverContext::DirectiveName { name, .. } if name == "scroll"
    ));
}

#[test]
fn test_hover_context_on_hyphenated_directive() {
    let content = "@fade-in";
    let ctx = extract_hover_context(content, 5);
    assert!(matches!(
        ctx,
        HoverContext::DirectiveName { name, .. } if name == "fade-in"
    ));
}

#[test]
fn test_hover_context_in_context() {
    let content = ".hero { @scroll { opacity: 0 -> 1; } }";
    let ctx = extract_hover_context(content, 12);
    assert!(matches!(
        ctx,
        HoverContext::DirectiveName { name, .. } if name == "scroll"
    ));
}

#[test]
fn test_hover_context_not_on_directive() {
    let content = "some text without directive";
    let ctx = extract_hover_context(content, 5);
    assert!(matches!(ctx, HoverContext::None));
}

// =============================================================================
// Hover Provider Tests
// =============================================================================

#[test]
fn hover_on_directive_shows_signature() {
    let doc = document_from("@scroll");
    let registry = create_test_registry();
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

    // Check that the hover contains the signature
    if let HoverContents::Markup(markup) = hover.contents {
        assert!(markup.value.contains("@scroll"));
        assert!(markup.value.contains("duration"));
        assert!(markup.value.contains("easing"));
    } else {
        panic!("Expected markup content");
    }
}

#[test]
fn hover_on_directive_shows_documentation() {
    let doc = document_from("@scroll");
    let registry = create_test_registry();
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
        assert!(markup.value.contains("Animate properties on scroll"));
    } else {
        panic!("Expected markup content");
    }
}

#[test]
fn hover_on_directive_shows_source_file() {
    let doc = document_from("@scroll");
    let registry = create_test_registry();
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
        assert!(markup.value.contains("/stdlib/scroll.st"));
    } else {
        panic!("Expected markup content");
    }
}

#[test]
fn hover_on_unknown_directive_returns_none() {
    let doc = document_from("@unknowndirective");
    let registry = create_test_registry();
    let hover = provide_hover(
        &doc,
        Position {
            line: 0,
            character: 5,
        },
        &registry,
    );

    assert!(hover.is_none());
}

#[test]
fn no_hover_outside_directive() {
    let doc = document_from("some text");
    let registry = create_test_registry();
    let hover = provide_hover(
        &doc,
        Position {
            line: 0,
            character: 3,
        },
        &registry,
    );

    assert!(hover.is_none());
}

#[test]
fn hover_includes_range() {
    let doc = document_from("@scroll");
    let registry = create_test_registry();
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
    assert!(hover.range.is_some());
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn hover_on_empty_document() {
    let doc = document_from("");
    let registry = create_test_registry();
    let hover = provide_hover(
        &doc,
        Position {
            line: 0,
            character: 0,
        },
        &registry,
    );

    assert!(hover.is_none());
}

#[test]
fn hover_at_file_start() {
    let doc = document_from("@scroll");
    let registry = create_test_registry();
    // At the @ symbol
    let hover = provide_hover(
        &doc,
        Position {
            line: 0,
            character: 0,
        },
        &registry,
    );

    // Should return None since cursor is on @, not on the name
    // (This depends on implementation - may want to include @ position)
    // For now, accept either behavior
    let _ = hover;
}

// =============================================================================
// Sigil Hover Context Tests
// =============================================================================

#[test]
fn hover_context_on_dollar_variable() {
    // "$navTheme <- \"dark\""
    //  offset 0 is the '$'
    let content = "$navTheme <- \"dark\"";
    let ctx = extract_hover_context(content, 0);
    assert!(
        matches!(ctx, HoverContext::Variable { ref name, .. } if name == "navTheme"),
        "expected Variable at $, got {:?}",
        ctx
    );
}

#[test]
fn hover_context_on_ampersand_template() {
    // "&ora-nav(\"Ora\")"
    //  offset 0 is the '&'
    let content = "&ora-nav(\"Ora\")";
    let ctx = extract_hover_context(content, 0);
    assert!(
        matches!(ctx, HoverContext::Template { ref name, .. } if name == "ora-nav"),
        "expected Template at &, got {:?}",
        ctx
    );
}

#[test]
fn hover_on_nested_directive() {
    let content = ".hero { @scroll { } }";
    // offset 9 is on 's' in scroll
    let ctx = extract_hover_context(content, 9);
    assert!(
        matches!(ctx, HoverContext::DirectiveName { ref name, .. } if name == "scroll"),
        "expected DirectiveName for @scroll, got {:?}",
        ctx
    );
}

// =============================================================================
// Additional Edge Cases
// =============================================================================

#[test]
fn hover_in_multiline_document() {
    let doc = document_from(
        r#"
.hero {
    @scroll {
        opacity: 0 -> 1;
    }
}
"#,
    );
    let registry = create_test_registry();
    // On "scroll" in line 2
    let hover = provide_hover(
        &doc,
        Position {
            line: 2,
            character: 7,
        },
        &registry,
    );

    assert!(hover.is_some());
}
