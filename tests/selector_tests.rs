//! Integration tests for CSS selector parsing and CSS passthrough emission.
//!
//! Verifies that descendant combinator selectors (.parent a {}) parse
//! correctly through the full parse → compile pipeline, and that plain
//! CSS declarations in scope blocks are emitted in the compiled CSS output.

use spacetime::{compile, compiler::CompileOptions, parse};

#[test]
fn test_descendant_selector_compiles_without_errors() {
    let input = r#"
.nav-links a {
    color: blue;
    text-decoration: none;
}
"#;
    let ast = parse(input).expect("should parse descendant selector");
    assert_eq!(ast.scopes.len(), 1);
    assert_eq!(ast.scopes[0].selector, ".nav-links a");
    assert_eq!(ast.scopes[0].css_declarations.len(), 2);

    let compiled = compile(&ast, CompileOptions::default());
    assert!(
        compiled.pipeline_errors.is_empty(),
        "No pipeline errors: {:?}",
        compiled.pipeline_errors
    );
}

#[test]
fn test_multi_level_descendant_compiles() {
    let input = r#"
nav ul li {
    list-style: none;
}
"#;
    let ast = parse(input).expect("should parse multi-level descendant");
    assert_eq!(ast.scopes.len(), 1);
    assert_eq!(ast.scopes[0].selector, "nav ul li");
    assert_eq!(ast.scopes[0].css_declarations.len(), 1);
    assert_eq!(ast.scopes[0].css_declarations[0].property, "list-style");

    let compiled = compile(&ast, CompileOptions::default());
    assert!(
        compiled.pipeline_errors.is_empty(),
        "No pipeline errors: {:?}",
        compiled.pipeline_errors
    );
}

#[test]
fn test_descendant_with_compound_compiles() {
    let input = r#"
.wrapper div.inner {
    display: flex;
}
"#;
    let ast = parse(input).expect("should parse descendant + compound");
    assert_eq!(ast.scopes.len(), 1);
    assert_eq!(ast.scopes[0].selector, ".wrapper div.inner");
    assert_eq!(ast.scopes[0].css_declarations.len(), 1);

    let compiled = compile(&ast, CompileOptions::default());
    assert!(
        compiled.pipeline_errors.is_empty(),
        "No pipeline errors: {:?}",
        compiled.pipeline_errors
    );
}

// =============================================================================
// CSS Passthrough Emission Tests (PROJ-054)
// =============================================================================

#[test]
fn test_basic_css_passthrough() {
    let input = ".test {\n    color: blue;\n    font-size: 16px;\n}\n";
    let ast = parse(input).unwrap();
    let compiled = compile(&ast, CompileOptions::default());
    assert!(
        compiled.css.contains("color: blue"),
        "got: {}",
        compiled.css
    );
    assert!(
        compiled.css.contains("font-size: 16px"),
        "got: {}",
        compiled.css
    );
    assert!(compiled.css.contains(".test"), "got: {}", compiled.css);
}

#[test]
fn test_nested_scope_css_passthrough() {
    let input = ".parent {\n    .child {\n        display: flex;\n    }\n}\n";
    let ast = parse(input).unwrap();
    let compiled = compile(&ast, CompileOptions::default());
    assert!(
        compiled.css.contains(".parent .child"),
        "got: {}",
        compiled.css
    );
    assert!(
        compiled.css.contains("display: flex"),
        "got: {}",
        compiled.css
    );
}

#[test]
fn test_descendant_selector_css_passthrough() {
    let input = ".nav a {\n    color: blue;\n    text-decoration: none;\n}\n";
    let ast = parse(input).unwrap();
    let compiled = compile(&ast, CompileOptions::default());
    assert!(compiled.css.contains(".nav a"), "got: {}", compiled.css);
    assert!(
        compiled.css.contains("color: blue"),
        "got: {}",
        compiled.css
    );
}

#[test]
fn test_child_combinator_css_passthrough() {
    // Parser now preserves `>` in nested selector, compose_selector produces ".parent > .child"
    let input = ".parent {\n    > .child {\n        margin: 0;\n    }\n}\n";
    let ast = parse(input).unwrap();
    let compiled = compile(&ast, CompileOptions::default());
    assert!(
        compiled.css.contains(".parent > .child"),
        "got: {}",
        compiled.css
    );
    assert!(compiled.css.contains("margin: 0"), "got: {}", compiled.css);
}

#[test]
fn test_multiple_scopes_css_passthrough() {
    let input = ".a {\n    color: red;\n}\n.b {\n    color: blue;\n}\n";
    let ast = parse(input).unwrap();
    let compiled = compile(&ast, CompileOptions::default());
    assert!(compiled.css.contains("color: red"), "got: {}", compiled.css);
    assert!(
        compiled.css.contains("color: blue"),
        "got: {}",
        compiled.css
    );
}

#[test]
fn test_empty_scope_no_css() {
    // Scope with only directives, no css_declarations → no empty block in CSS
    let input = ".test {\n    @on &.click {\n        &visible <- true;\n    }\n}\n";
    let ast = parse(input).unwrap();
    let compiled = compile(&ast, CompileOptions::default());
    // Should NOT contain an empty ".test { }" block from passthrough
    let passthrough_empty = compiled.css.contains(".test {\n}");
    assert!(
        !passthrough_empty,
        "should not emit empty scope, got: {}",
        compiled.css
    );
}

// =============================================================================
// Ampersand Nested Scope CSS Passthrough (PROJ-054)
// =============================================================================

#[test]
fn test_ampersand_css_passthrough() {
    // &:hover inside .btn should emit ".btn:hover { ... }" in CSS
    let input = ".btn {\n    &:hover {\n        color: red;\n    }\n}\n";
    let ast = parse(input).unwrap();
    let compiled = compile(&ast, CompileOptions::default());
    assert!(compiled.css.contains(".btn:hover"), "got: {}", compiled.css);
    assert!(compiled.css.contains("color: red"), "got: {}", compiled.css);
}

#[test]
fn test_ampersand_class_css_passthrough() {
    // &.active inside .card should emit ".card.active { ... }" in CSS
    let input = ".card {\n    &.active {\n        opacity: 1;\n    }\n}\n";
    let ast = parse(input).unwrap();
    let compiled = compile(&ast, CompileOptions::default());
    assert!(
        compiled.css.contains(".card.active"),
        "got: {}",
        compiled.css
    );
    assert!(compiled.css.contains("opacity: 1"), "got: {}", compiled.css);
}
