//! WASM module tests.
//!
//! These tests verify the WASM bindings compile correctly and the data structures
//! serialize properly. The actual WASM functionality is tested in the browser
//! via `wasm_browser_test.html`.

#![cfg(feature = "wasm")]

use spacetime::wasm::*;

#[test]
fn test_compile_result_serialization() {
    // Test that CompileResult can be created and has expected fields
    let result = CompileResult {
        css: ".test { opacity: 1; }".to_string(),
        js: "console.log('test');".to_string(),
        errors: vec![],
    };

    assert!(!result.css.is_empty());
    assert!(!result.js.is_empty());
    assert!(result.errors.is_empty());
}

#[test]
fn test_compile_error_serialization() {
    // Test that CompileError can be created with all fields
    let error = CompileError {
        message: "unexpected token".to_string(),
        line: 5,
        column: 10,
        code: Some("E003".to_string()),
    };

    assert_eq!(error.message, "unexpected token");
    assert_eq!(error.line, 5);
    assert_eq!(error.column, 10);
    assert_eq!(error.code, Some("E003".to_string()));
}

#[test]
fn test_completion_item_serialization() {
    // Test that WasmCompletionItem can be created
    let item = WasmCompletionItem {
        label: "fade-in".to_string(),
        kind: "Function".to_string(),
        detail: Some("Fade-in animation macro".to_string()),
        insert_text: Some("fade-in".to_string()),
        documentation: Some("Creates a visibility-triggered fade-in animation.".to_string()),
    };

    assert_eq!(item.label, "fade-in");
    assert_eq!(item.kind, "Function");
    assert!(item.detail.is_some());
    assert!(item.insert_text.is_some());
    assert!(item.documentation.is_some());
}

#[test]
fn test_hover_result_serialization() {
    // Test that WasmHover can be created with range
    let hover = WasmHover {
        contents: "# @fade-in\n\nCreates fade-in animation.".to_string(),
        range: Some(WasmRange {
            start_line: 0,
            start_column: 1,
            end_line: 0,
            end_column: 8,
        }),
    };

    assert!(!hover.contents.is_empty());
    assert!(hover.range.is_some());

    let range = hover.range.unwrap();
    assert_eq!(range.start_line, 0);
    assert_eq!(range.start_column, 1);
    assert_eq!(range.end_line, 0);
    assert_eq!(range.end_column, 8);
}

#[test]
fn test_hover_result_without_range() {
    // Test that WasmHover can be created without range
    let hover = WasmHover {
        contents: "Documentation text".to_string(),
        range: None,
    };

    assert!(!hover.contents.is_empty());
    assert!(hover.range.is_none());
}

#[test]
fn test_wasm_range_serialization() {
    // Test WasmRange creation
    let range = WasmRange {
        start_line: 10,
        start_column: 5,
        end_line: 12,
        end_column: 20,
    };

    assert_eq!(range.start_line, 10);
    assert_eq!(range.start_column, 5);
    assert_eq!(range.end_line, 12);
    assert_eq!(range.end_column, 20);
}

#[test]
fn test_compile_result_with_errors() {
    // Test CompileResult with errors
    let result = CompileResult {
        css: String::new(),
        js: String::new(),
        errors: vec![
            CompileError {
                message: "Parse error".to_string(),
                line: 1,
                column: 0,
                code: Some("E003".to_string()),
            },
            CompileError {
                message: "Unknown directive".to_string(),
                line: 5,
                column: 4,
                code: Some("E171".to_string()),
            },
        ],
    };

    assert!(result.css.is_empty());
    assert!(result.js.is_empty());
    assert_eq!(result.errors.len(), 2);
    assert_eq!(result.errors[0].code, Some("E003".to_string()));
    assert_eq!(result.errors[1].code, Some("E171".to_string()));
}

#[test]
fn test_completion_item_minimal() {
    // Test WasmCompletionItem with minimal fields
    let item = WasmCompletionItem {
        label: "scroll".to_string(),
        kind: "Function".to_string(),
        detail: None,
        insert_text: None,
        documentation: None,
    };

    assert_eq!(item.label, "scroll");
    assert_eq!(item.kind, "Function");
    assert!(item.detail.is_none());
    assert!(item.insert_text.is_none());
    assert!(item.documentation.is_none());
}
