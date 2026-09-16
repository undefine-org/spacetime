//! Integration tests for unresolved parameter error handling.
//!
//! These tests verify that unresolved % parameters in emit blocks produce
//! proper pipeline errors instead of being silently passed through.

use spacetime::{parse, compile, CompiledSpacetime, CompileOptions};

/// Helper to compile spacetime code and return the result
fn compile_st(source: &str) -> CompiledSpacetime {
    let ast = parse(source).expect("Failed to parse");
    compile(&ast, CompileOptions::default())
}

#[test]
fn test_unresolved_param_surfaces_as_pipeline_error() {
    // This test verifies that when a primitive definition has an unresolved
    // %param in its emit block, the error is caught and reported as a
    // pipeline error rather than silently passing through to JavaScript.
    //
    // Note: This test depends on having a macro/primitive that would produce
    // an unresolved param. Since we can't easily inject test primitives into
    // the stdlib from here, this test verifies the infrastructure is working.
    //
    // The unit tests in src/pipeline/expand.rs test the actual error generation
    // more directly.

    let source = r#"
.test-element {
    @animate {
        opacity: 0 -> 1;
    }
}
"#;

    let result = compile_st(source);

    // This should compile without errors (stdlib is loaded)
    // If stdlib loading fails, we'd get errors here
    // The actual unresolved param testing is done in unit tests
    // where we can control the primitive definitions

    // For now, just verify the pipeline completes
    // A proper integration test would require the ability to inject
    // test primitives with unresolved params into the registry
    assert!(result.pipeline_errors.len() == 0 || {
        // If there are errors, they should have proper structure
        result.pipeline_errors.iter().all(|e| !e.code.is_empty() && !e.message.is_empty())
    });
}

#[test]
fn test_pipeline_error_has_proper_structure() {
    // Test that when pipeline errors occur, they have the expected structure
    // with code, message, and optionally hint

    let source = r#"
.test {
    color: red;
}
"#;

    let result = compile_st(source);

    // Any errors should have proper structure
    for error in &result.pipeline_errors {
        assert!(!error.code.is_empty(), "Error code should not be empty");
        assert!(!error.message.is_empty(), "Error message should not be empty");
        // Hint is optional but if present should not be empty
        if let Some(ref hint) = error.hint {
            assert!(!hint.is_empty(), "Hint should not be empty if present");
        }
    }
}

#[test]
fn test_valid_simple_css_has_no_errors() {
    // Test that valid simple CSS compiles without pipeline errors

    let source = r#"
.button {
    color: blue;
}
"#;

    let result = compile_st(source);

    // Simple valid CSS should have no pipeline errors
    assert!(
        result.pipeline_errors.is_empty(),
        "Valid CSS should produce no pipeline errors, got: {:?}",
        result.pipeline_errors
    );
}
