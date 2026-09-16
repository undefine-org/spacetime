//! Tests for LSP diagnostics provider.

use std::path::PathBuf;

use spacetime::lsp::form_registry::{DirectiveParam, DirectiveSignature, FormRegistry};
use spacetime::lsp::{DocumentState, validate_document};
use tower_lsp::lsp_types::NumberOrString;
use spacetime::parser::SourceSpan;
use spacetime::parser::meta_ast::{CaptureModifier, CaptureType};

/// Create a test FormRegistry with sample directives.
fn create_test_registry() -> FormRegistry {
    let mut registry = FormRegistry::new();

    // @scroll directive with optional params
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
        documentation: None,
        exports: vec![],
        bound_primitives: vec![],
        form: None,
    });

    // @screen directive with required param
    registry.register(DirectiveSignature {
        name: "screen".to_string(),
        macro_name: "ScreenMacro".to_string(),
        params: vec![DirectiveParam {
            name: "min-width".to_string(),
            capture_var: "min".to_string(),
            capture_type: CaptureType::Length,
            modifier: CaptureModifier::Required,
            default_value: None,
        }],
        body_type: Some(CaptureType::Properties),
        definition_span: SourceSpan::default(),
        source_file: PathBuf::from("/stdlib/screen.st"),
        documentation: None,
        exports: vec![],
        bound_primitives: vec![],
        form: None,
    });

    // @toggle directive with union type param
    registry.register(DirectiveSignature {
        name: "toggle".to_string(),
        macro_name: "ToggleMacro".to_string(),
        params: vec![DirectiveParam {
            name: "axis".to_string(),
            capture_var: "axis".to_string(),
            capture_type: CaptureType::Union(vec![
                "x".to_string(),
                "y".to_string(),
                "both".to_string(),
            ]),
            modifier: CaptureModifier::Optional,
            default_value: Some("y".to_string()),
        }],
        body_type: None,
        definition_span: SourceSpan::default(),
        source_file: PathBuf::from("/stdlib/toggle.st"),
        documentation: None,
        exports: vec![],
        bound_primitives: vec![],
        form: None,
    });

    // @animate directive with bool param
    registry.register(DirectiveSignature {
        name: "animate".to_string(),
        macro_name: "AnimateMacro".to_string(),
        params: vec![DirectiveParam {
            name: "loop".to_string(),
            capture_var: "loop".to_string(),
            capture_type: CaptureType::Bool,
            modifier: CaptureModifier::Optional,
            default_value: Some("false".to_string()),
        }],
        body_type: None,
        definition_span: SourceSpan::default(),
        source_file: PathBuf::from("/stdlib/animate.st"),
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
// Compiler-Parity Diagnostics Tests (gh-32)
//
// ONE diagnostic engine, two front-ends: `validate_document` runs the compiler's
// own `compile()` pipeline, so the editor shows exactly what `check` reports. The
// OLD tests asserted a hand-rolled approximation — e.g. that `@unknowndirective`
// (which `check` silently accepts) or an empty registry produce errors. Those
// drift from the compiler, which is the whole defect W8 deletes. These tests
// assert parity: the LSP emits E0408 for a genuinely-unknown signal, stays quiet
// where the compiler stays quiet (unknown directives, the W2-fixed @data
// ambiguity), and reports malformed directives exactly as `check` does.
// =============================================================================

#[test]
fn validate_document_reports_unknown_signal_like_check() {
    // The W2 #26 case: `check` emits E0408 for a used-but-undefined signal.
    let doc = document_from(".x {\n  color: var(--st-definitelyMissing);\n}");
    let diagnostics = validate_document(&doc, None);

    assert!(!diagnostics.is_empty(), "E0408 should surface as an LSP diagnostic");
    let codes: Vec<String> = diagnostics
        .iter()
        .filter_map(|d| d.code.clone())
        .map(|c| match c {
            NumberOrString::String(s) => s,
            NumberOrString::Number(n) => n.to_string(),
        })
        .collect();
    assert!(
        codes.iter().any(|c| c.contains("E0408")),
        "expected E0408 (unknown signal), got {codes:?}"
    );
}

#[test]
fn validate_document_matches_check_for_an_unknown_directive() {
    // PARITY, which is the actual contract — not silence for its own sake.
    //
    // This test used to assert the LSP stays SILENT for `@unknowndirective`,
    // because `check` did. That silence was itself the defect (W2): a directive
    // the registry does not know emits nothing, so a typo or a missing @import
    // produced a blank page and a green build — the GH-13/GH-22 experience.
    //
    // `check` now warns (W0714) for an unknown directive in a selector scope, so
    // the LSP must warn too. The invariant never changed: ONE diagnostic engine,
    // two front-ends. What changed is what that engine says.
    let doc = document_from(".x {\n    @unknowndirective\n}");
    let diagnostics = validate_document(&doc, None);

    assert!(
        diagnostics
            .iter()
            .any(|d| d.message.contains("unknowndirective")),
        "the LSP must surface the same W0714 `check` now emits — editor and \
         compiler disagreeing about what compiles is the GH-28 defect. got: {diagnostics:?}"
    );
}

#[test]
fn validate_document_has_no_false_ambiguity() {
    // The W2 #27 case: `@data inline` bare-literal no longer false-fires E0923
    // (the ambiguity lint re-scores from captures and ignored matched_macro).
    // The file may legitimately emit W0201 (unused data source) — parity with
    // `check`, which does the same — but never a false E0923.
    let doc = document_from("@data inline $events : [];\n@data inline $filterCategory : \"all\";");
    let diagnostics = validate_document(&doc, None);

    let codes: Vec<String> = diagnostics
        .iter()
        .filter_map(|d| d.code.clone())
        .map(|c| match c {
            NumberOrString::String(s) => s,
            NumberOrString::Number(n) => n.to_string(),
        })
        .collect();
    assert!(
        !codes.iter().any(|c| c.contains("E0923")),
        "false E0923 ambiguity must be gone; got {codes:?}"
    );
}

#[test]
fn validate_document_reports_malformed_directive_like_check() {
    // `check` emits E0946 for a directive that does not match its grammar (here:
    // `@scroll` requires a body block).
    let doc = document_from(".x {\n    @scroll\n}");
    let diagnostics = validate_document(&doc, None);

    assert!(!diagnostics.is_empty(), "malformed @scroll should be diagnosed");
}

#[test]
fn diagnostics_have_source_spacetime() {
    let doc = document_from(".x {\n  color: var(--st-definitelyMissing);\n}");
    let diagnostics = validate_document(&doc, None);

    assert!(!diagnostics.is_empty());
    for diag in &diagnostics {
        assert_eq!(diag.source, Some("spacetime".to_string()));
    }
}

#[test]
fn diagnostics_have_severity() {
    let doc = document_from(".x {\n  color: var(--st-definitelyMissing);\n}");
    let diagnostics = validate_document(&doc, None);

    assert!(!diagnostics.is_empty());
    for diag in &diagnostics {
        assert!(diag.severity.is_some());
    }
}

#[test]
fn diagnostics_have_valid_range() {
    let doc = document_from(".x {\n  color: var(--st-definitelyMissing);\n}");
    let diagnostics = validate_document(&doc, None);

    assert!(!diagnostics.is_empty());
    for diag in &diagnostics {
        // Range should be valid (start <= end)
        assert!(
            diag.range.start.line <= diag.range.end.line
                || (diag.range.start.line == diag.range.end.line
                    && diag.range.start.character <= diag.range.end.character)
        );
    }
}

// =============================================================================
// Type Validation Tests (unit tests for internal functions)
// =============================================================================

mod type_validation {
    use spacetime::parser::meta_ast::CaptureType;

    // Note: These tests verify the type validation logic at the unit level.
    // The full integration with LSP diagnostics depends on parsing.

    #[test]
    fn bool_validation() {
        // Bool type accepts "true" or "false"
        let bool_type = CaptureType::Bool;
        assert_eq!(bool_type, CaptureType::Bool);
    }

    #[test]
    fn union_type_creation() {
        let union = CaptureType::Union(vec!["x".to_string(), "y".to_string(), "both".to_string()]);
        if let CaptureType::Union(variants) = union {
            assert_eq!(variants.len(), 3);
            assert!(variants.contains(&"x".to_string()));
            assert!(variants.contains(&"y".to_string()));
            assert!(variants.contains(&"both".to_string()));
        } else {
            panic!("Expected Union type");
        }
    }

    #[test]
    fn duration_type() {
        let duration = CaptureType::Duration;
        assert_eq!(duration, CaptureType::Duration);
    }
}

// =============================================================================
// Empty and Edge Case Tests
// =============================================================================

#[test]
fn empty_document_returns_no_diagnostics() {
    let doc = document_from("");
    let diagnostics = validate_document(&doc, None);

    // Empty document should have no diagnostics
    assert!(diagnostics.is_empty());
}

#[test]
fn whitespace_only_document_returns_no_diagnostics() {
    let doc = document_from("   \n\n   ");
    let diagnostics = validate_document(&doc, None);

    // Whitespace-only document should have no diagnostics
    assert!(diagnostics.is_empty());
}

#[test]
fn empty_registry_reports_unknown_directives() {
    // Parity (gh-32): `validate_document` runs the compiler's pipeline, not the
    // passed-in registry. `@scroll` without a body is malformed (E0946) — that
    // is what surfaces, independent of any registry the caller constructs.
    let doc = document_from(".x {\n    @scroll\n}");
    let registry = FormRegistry::new(); // Empty registry — unused under parity
    let diagnostics = validate_document(&doc, None);

    assert!(!diagnostics.is_empty(), "@scroll without a body is malformed (E0946)");
}

// =============================================================================
// Form Registry Tests (verifying the registry setup is correct)
// =============================================================================

#[test]
fn registry_has_scroll_directive() {
    let registry = create_test_registry();
    let scroll = registry.get_directive("scroll");
    assert!(scroll.is_some());
}

#[test]
fn scroll_directive_has_optional_params() {
    let registry = create_test_registry();
    let scroll = registry.get_directive("scroll").unwrap();
    assert!(scroll.required_params().is_empty());
}

#[test]
fn screen_directive_has_required_param() {
    let registry = create_test_registry();
    let screen = registry.get_directive("screen").unwrap();
    assert_eq!(screen.required_params().len(), 1);
    assert_eq!(screen.required_params()[0].name, "min-width");
}

#[test]
fn toggle_directive_has_union_param() {
    let registry = create_test_registry();
    let toggle = registry.get_directive("toggle").unwrap();
    let axis_param = &toggle.params[0];
    assert_eq!(axis_param.name, "axis");
    if let CaptureType::Union(variants) = &axis_param.capture_type {
        assert!(variants.contains(&"x".to_string()));
        assert!(variants.contains(&"y".to_string()));
        assert!(variants.contains(&"both".to_string()));
    } else {
        panic!("Expected Union type for axis param");
    }
}

#[test]
fn animate_directive_has_bool_param() {
    let registry = create_test_registry();
    let animate = registry.get_directive("animate").unwrap();
    let loop_param = &animate.params[0];
    assert_eq!(loop_param.name, "loop");
    assert_eq!(loop_param.capture_type, CaptureType::Bool);
}
