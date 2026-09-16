//! Integration tests for PROJ-106: Export Validation Diagnostics
//!
//! Tests that @exports declarations are validated against template state,
//! emitting E0917 when an exported $var is not defined in the template.

use spacetime::compiler::CompileOptions;
use spacetime::{compile, parse};

/// Compile a .st snippet and return pipeline error codes.
fn compile_st_error_codes(input: &str) -> Vec<String> {
    let ast = parse(input).expect("Failed to parse");
    let compiled = compile(&ast, CompileOptions::default());
    compiled
        .pipeline_errors
        .iter()
        .map(|e| e.code.clone())
        .collect()
}

/// Compile a .st snippet and return pipeline error count.
fn compile_st_errors(input: &str) -> usize {
    let ast = parse(input).expect("Failed to parse");
    let compiled = compile(&ast, CompileOptions::default());
    compiled.pipeline_errors.len()
}

// =============================================================================
// E0917: @exports declares $var not defined in template state
// =============================================================================

/// @exports { $missing } where $missing is not declared as state → E0917
#[test]
fn test_e0917_export_undefined_state_var() {
    let input = r#"
@template &widget() {
    <div class="widget"></div>
    $count number: 0;
    @exports { $missing }
}
"#;

    let codes = compile_st_error_codes(input);
    assert!(
        codes.iter().any(|c| c == "E0917"),
        "Expected E0917 for exporting undefined $missing. Got codes: {:?}",
        codes
    );
}

/// @exports { $count: mut } where $count IS declared → no E0917
#[test]
fn test_e0917_export_defined_state_var_clean() {
    let input = r#"
@template &counter() {
    <div class="counter"></div>
    $count number: 0;
    @exports { $count: mut }
}
"#;

    let errors = compile_st_errors(input);
    assert_eq!(
        errors, 0,
        "Expected no errors when exporting a defined state variable"
    );
}

/// Multiple exports (semicolon-separated), one undefined → exactly one E0917
#[test]
fn test_e0917_mixed_valid_and_invalid_exports() {
    let input = r#"
@template &panel() {
    <div class="panel"></div>
    $open bool: false;
    @exports { $open; $ghost }
}
"#;

    let codes = compile_st_error_codes(input);
    let e0917_count = codes.iter().filter(|c| c.as_str() == "E0917").count();
    assert_eq!(
        e0917_count, 1,
        "Expected exactly 1 E0917 for $ghost. Got codes: {:?}",
        codes
    );
}

/// Template with no @exports block → no E0917
#[test]
fn test_e0917_no_exports_no_diagnostic() {
    let input = r#"
@template &simple() {
    <div class="simple"></div>
    $count number: 0;
}
"#;

    let errors = compile_st_errors(input);
    assert_eq!(errors, 0, "Template without @exports should have no errors");
}

/// All exports undefined → one E0917 per export
#[test]
fn test_e0917_all_exports_undefined() {
    let input = r#"
@template &broken() {
    <div class="broken"></div>
    @exports { $alpha; $beta }
}
"#;

    let codes = compile_st_error_codes(input);
    let e0917_count = codes.iter().filter(|c| c.as_str() == "E0917").count();
    assert_eq!(
        e0917_count, 2,
        "Expected 2 E0917 diagnostics for $alpha and $beta. Got codes: {:?}",
        codes
    );
}

// =============================================================================
// Additional @exports diagnostic scenarios (ITEM-106-005)
// =============================================================================

/// @exports { $count } (read-only, no mut keyword) compiles without errors
#[test]
fn test_exports_readonly_compiles_clean() {
    let input = r#"
@template &readonly-counter() {
    <div class="ro-counter"></div>
    $count number: 0;
    @exports { $count }
}
"#;

    let errors = compile_st_errors(input);
    assert_eq!(
        errors, 0,
        "Read-only @exports {{$count}} with defined $count should produce no errors"
    );
}

/// Multiple valid exports: @exports { $a; $b } both defined → no errors
#[test]
fn test_exports_multiple_valid_no_errors() {
    let input = r#"
@template &multi-export() {
    <div class="multi"></div>
    $x number: 0;
    $y number: 0;
    @exports { $x; $y }
}
"#;

    let errors = compile_st_errors(input);
    assert_eq!(
        errors, 0,
        "Multiple valid exports (both defined in state) should produce no errors"
    );
}

/// Empty @exports {} block → no errors, no crash
#[test]
fn test_exports_empty_block_no_errors() {
    let input = r#"
@template &empty-exports() {
    <div class="empty"></div>
    $count number: 0;
    @exports {}
}
"#;

    let errors = compile_st_errors(input);
    assert_eq!(
        errors, 0,
        "Empty @exports {{}} block should produce no errors"
    );
}

/// Mixed mutability exports: @exports { $a: mut; $b } all defined → no errors
#[test]
fn test_exports_mixed_mutability_valid() {
    let input = r#"
@template &mixed-mut() {
    <div class="mixed"></div>
    $count number: 0;
    $label string: "hello";
    @exports { $count: mut; $label }
}
"#;

    let errors = compile_st_errors(input);
    assert_eq!(
        errors, 0,
        "Mixed mut/readonly exports with all vars defined should produce no errors"
    );
}

/// @exports emits metadata in JS output (contains __stExports or export-related code)
#[test]
fn test_exports_js_output_contains_metadata() {
    let input = r#"
@template &js-check() {
    <div class="check"></div>
    $value number: 10;
    @exports { $value: mut }
}
"#;

    let ast = parse(input).expect("Failed to parse");
    let compiled = compile(&ast, CompileOptions::default());
    let js = compiled.js;

    assert!(
        js.contains("__stExports") || js.contains("exports"),
        "JS output should contain export metadata wiring.\nJS output (first 2000 chars):\n{}",
        &js[..js.len().min(2000)]
    );
}

/// Three exports, two undefined → exactly 2 E0917 diagnostics
#[test]
fn test_e0917_partial_undefined_three_exports() {
    let input = r#"
@template &three-exports() {
    <div class="three"></div>
    $valid number: 0;
    @exports { $valid; $phantom; $ghost }
}
"#;

    let codes = compile_st_error_codes(input);
    let e0917_count = codes.iter().filter(|c| c.as_str() == "E0917").count();
    assert_eq!(
        e0917_count, 2,
        "Expected 2 E0917 for $phantom and $ghost (only $valid is defined). Got codes: {:?}",
        codes
    );
}
