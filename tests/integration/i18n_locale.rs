//! Integration tests for @locale macro and i18n pipeline.
//!
//! Tests verify that the i18n test fixture parses, compiles, and produces
//! the expected build scripts for locale-based static site generation.

use spacetime::parser::parse;
use spacetime::{compile, compiler::CompileOptions};
use std::fs;

#[test]
fn test_locale_fixture_parses() {
    let content = fs::read_to_string("tests/fixtures/i18n-test-site/index.st")
        .expect("Failed to read i18n fixture index.st");

    let result = parse(&content);
    assert!(
        result.is_ok(),
        "@locale fixture should parse: {:?}",
        result.err()
    );
}

#[test]
fn test_locale_fixture_compiles_with_build_scripts() {
    let content = fs::read_to_string("tests/fixtures/i18n-test-site/index.st")
        .expect("Failed to read i18n fixture index.st");

    let ast = parse(&content).expect("@locale fixture should parse");
    let compiled = compile(&ast, CompileOptions::default());

    // @locale should produce build scripts (from %emit build-js)
    assert!(
        !compiled.build_scripts.is_empty(),
        "@locale should produce build scripts, got none"
    );

    // Build script should contain locale iteration logic
    let build_js = compiled.build_scripts.join("\n");
    assert!(
        build_js.contains("parseHTML"),
        "Build script should use parseHTML for DOM manipulation"
    );
}

#[test]
fn test_locale_fixture_compiles_with_runtime_js() {
    let content = fs::read_to_string("tests/fixtures/i18n-test-site/index.st")
        .expect("Failed to read i18n fixture index.st");

    let ast = parse(&content).expect("@locale fixture should parse");
    let compiled = compile(&ast, CompileOptions::default());

    // @locale should produce runtime JS (from %emit js)
    assert!(
        !compiled.js.is_empty(),
        "@locale should produce runtime JS output"
    );

    // Runtime JS should contain locale switching functions
    assert!(
        compiled.js.contains("__st_locale") || compiled.js.contains("__st_setLocale"),
        "Runtime JS should contain locale management functions, got: {}",
        &compiled.js[..compiled.js.len().min(200)]
    );
}

#[test]
fn test_locale_fixture_no_pipeline_errors() {
    let content = fs::read_to_string("tests/fixtures/i18n-test-site/index.st")
        .expect("Failed to read i18n fixture index.st");

    let ast = parse(&content).expect("@locale fixture should parse");
    let compiled = compile(&ast, CompileOptions::default());

    // Filter out any "unknown directive" errors — @locale should be recognized
    let unknown_directive_errors: Vec<_> = compiled
        .pipeline_errors
        .iter()
        .filter(|e| e.message.to_lowercase().contains("unknown"))
        .collect();

    assert!(
        unknown_directive_errors.is_empty(),
        "@locale should not produce unknown-directive errors: {:?}",
        unknown_directive_errors
            .iter()
            .map(|e| &e.message)
            .collect::<Vec<_>>()
    );
}
