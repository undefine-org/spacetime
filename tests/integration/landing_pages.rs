//! Landing page compilation tests.
//!
//! Verifies that the three landing pages (tests/fixtures/landing/1-3/) parse,
//! compile without JS syntax errors, and produce animation timeline output.

use spacetime::parser::parse;
use spacetime::{compile, compiler::CompileOptions};
use std::fs;

/// Helper: compile a landing page and return (js, css, errors)
fn compile_landing_page(num: u8) -> (String, String, Vec<String>) {
    let path = format!("tests/fixtures/landing/{}/index.st", num);
    let content =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to read {}: {}", path, e));

    let ast = parse(&content).unwrap_or_else(|e| panic!("Failed to parse {}: {:?}", path, e));

    let compiled = compile(&ast, CompileOptions::default());
    let errors: Vec<String> = compiled
        .pipeline_errors
        .iter()
        .map(|e| e.message.clone())
        .collect();

    (compiled.js, compiled.css, errors)
}

/// Check that a JS string has no duplicate `const` declarations in the
/// same onCleanup scope (the specific bug from PROJ-042).
fn has_duplicate_const_in_cleanup(js: &str) -> Vec<String> {
    let mut duplicates = Vec::new();

    // Find each ST.onCleanup callback and check for duplicate const declarations
    for (i, segment) in js.split("ST.onCleanup").enumerate() {
        if i == 0 {
            continue; // skip text before first onCleanup
        }

        // Extract the callback body (find matching braces)
        let mut brace_depth = 0;
        let mut in_body = false;
        let mut body = String::new();

        for ch in segment.chars() {
            if ch == '{' {
                brace_depth += 1;
                in_body = true;
            }
            if in_body {
                body.push(ch);
            }
            if ch == '}' {
                brace_depth -= 1;
                if brace_depth == 0 && in_body {
                    break;
                }
            }
        }

        // Count const declarations in this cleanup body
        let mut const_decls: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for line in body.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("const ") {
                // Extract the variable name
                if let Some(name) = trimmed
                    .strip_prefix("const ")
                    .and_then(|s| s.split(|c: char| !c.is_alphanumeric() && c != '_').next())
                {
                    *const_decls.entry(name.to_string()).or_insert(0) += 1;
                }
            }
        }

        for (name, count) in &const_decls {
            if *count > 1 {
                duplicates.push(format!(
                    "Duplicate const '{}' declared {} times in onCleanup block",
                    name, count
                ));
            }
        }
    }

    duplicates
}

// =============================================================================
// LANDING PAGE 1
// =============================================================================

#[test]
fn test_landing_page_1_parses() {
    let content = fs::read_to_string("tests/fixtures/landing/1/index.st").unwrap();
    let result = parse(&content);
    assert!(
        result.is_ok(),
        "Landing page 1 should parse: {:?}",
        result.err()
    );
}

#[test]
fn test_landing_page_1_compiles_clean() {
    let (js, _css, errors) = compile_landing_page(1);
    assert!(
        errors.is_empty(),
        "Landing page 1 should compile without pipeline errors: {:?}",
        errors
    );
    assert!(!js.is_empty(), "Landing page 1 should produce JS output");
}

#[test]
fn test_landing_page_1_no_duplicate_const_in_cleanup() {
    let (js, _, _) = compile_landing_page(1);
    let duplicates = has_duplicate_const_in_cleanup(&js);
    assert!(
        duplicates.is_empty(),
        "Landing page 1 has duplicate const declarations in cleanup: {:?}",
        duplicates
    );
}

// =============================================================================
// LANDING PAGE 2
// =============================================================================

#[test]
fn test_landing_page_2_parses() {
    let content = fs::read_to_string("tests/fixtures/landing/2/index.st").unwrap();
    let result = parse(&content);
    assert!(
        result.is_ok(),
        "Landing page 2 should parse: {:?}",
        result.err()
    );
}

#[test]
fn test_landing_page_2_compiles_clean() {
    let (js, _css, errors) = compile_landing_page(2);
    assert!(
        errors.is_empty(),
        "Landing page 2 should compile without pipeline errors: {:?}",
        errors
    );
    assert!(!js.is_empty(), "Landing page 2 should produce JS output");
}

#[test]
fn test_landing_page_2_no_duplicate_const_in_cleanup() {
    let (js, _, _) = compile_landing_page(2);
    let duplicates = has_duplicate_const_in_cleanup(&js);
    assert!(
        duplicates.is_empty(),
        "Landing page 2 has duplicate const declarations in cleanup: {:?}",
        duplicates
    );
}

// =============================================================================
// LANDING PAGE 3
// =============================================================================

#[test]
fn test_landing_page_3_parses() {
    let content = fs::read_to_string("tests/fixtures/landing/3/index.st").unwrap();
    let result = parse(&content);
    assert!(
        result.is_ok(),
        "Landing page 3 should parse: {:?}",
        result.err()
    );
}

#[test]
fn test_landing_page_3_compiles_clean() {
    let (js, _css, errors) = compile_landing_page(3);
    assert!(
        errors.is_empty(),
        "Landing page 3 should compile without pipeline errors: {:?}",
        errors
    );
    assert!(!js.is_empty(), "Landing page 3 should produce JS output");
}

#[test]
fn test_landing_page_3_no_duplicate_const_in_cleanup() {
    let (js, _, _) = compile_landing_page(3);
    let duplicates = has_duplicate_const_in_cleanup(&js);
    assert!(
        duplicates.is_empty(),
        "Landing page 3 has duplicate const declarations in cleanup: {:?}",
        duplicates
    );
}

// =============================================================================
// CROSS-PAGE TESTS
// =============================================================================

#[test]
fn test_landing_pages_produce_animation_timelines() {
    for num in 1..=3 {
        let (js, _, _) = compile_landing_page(num);

        // All landing pages should produce ST runtime initialization
        assert!(
            js.contains("ST.") || js.contains("spacetime"),
            "Landing page {} should reference the ST runtime",
            num
        );
    }
}
// Tests for private user sites (ikarchitecte, ora-ventures.com) were removed
// when those sites moved out with the projects/ reorganization.
