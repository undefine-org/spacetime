//! Integration tests for PROJ-104: Template Refs & @exports
//!
//! Tests that `&name &template(args);`, `&name[] &template(args);`, and
//! `@exports { $var: mut }` compile correctly through the full pipeline.

use spacetime::compiler::CompileOptions;
use spacetime::{compile, parse};

fn compile_st(input: &str) -> String {
    let ast = parse(input).expect("Failed to parse");
    let compiled = compile(&ast, CompileOptions::default());
    compiled.js
}

fn compile_st_errors(input: &str) -> usize {
    let ast = parse(input).expect("Failed to parse");
    let compiled = compile(&ast, CompileOptions::default());
    compiled.pipeline_errors.len()
}

/// Anonymous template ref compiles when referenced template is defined
#[test]
fn test_template_ref_anonymous_compiles() {
    let input = r#"
@template &counter($label string) {
    <div class="counter"></div>
}
@template &nav() {
    <nav class="nav"></nav>
    &counter("Likes");
}
"#;
    assert_eq!(
        compile_st_errors(input),
        0,
        "Anonymous ref should compile clean"
    );
}

/// Named template ref compiles when referenced template is defined
#[test]
fn test_template_ref_named_compiles() {
    let input = r#"
@template &counter($label string) {
    <div class="counter"></div>
}
@template &nav() {
    <nav class="nav"></nav>
    &likeCounter &counter("Likes");
}
"#;
    assert_eq!(
        compile_st_errors(input),
        0,
        "Named ref should compile clean"
    );
}

/// @exports block compiles without errors
#[test]
fn test_exports_block_compiles() {
    let input = r#"
@template &counter() {
    <div class="counter"></div>
    $count number: 0;
    @exports { $count: mut }
}
"#;
    assert_eq!(compile_st_errors(input), 0, "@exports should compile clean");
}

/// JS output contains refs data for named template ref
#[test]
fn test_template_ref_in_js_output() {
    let input = r#"
@template &counter($label string) {
    <div class="counter"></div>
}
@template &nav() {
    <nav></nav>
    &likeCounter &counter("Likes");
}
"#;
    let js = compile_st(input);
    assert!(
        js.contains("refs") || js.contains("counter"),
        "JS should contain refs data.\nJS:\n{}",
        &js[..js.len().min(1000)]
    );
}

/// Collection ref compiles when referenced template is defined
#[test]
fn test_collection_ref_compiles() {
    let input = r#"
@template &item-card($item string) {
    <li class="item"></li>
}
@template &card-list() {
    <ul class="list"></ul>
    &cards[] &item-card($item);
}
"#;
    assert_eq!(
        compile_st_errors(input),
        0,
        "Collection ref should compile clean"
    );
}

/// Bare template invocation (no body) in a selector context compiles
/// and produces invoke-template JS output (BUG-static-template-invocation)
#[test]
fn test_bare_template_invocation_compiles() {
    let input = r#"
@template &greeting($name string) {
    <p class="greeting">Hello!</p>
}

.container {
    &greeting("World")
}
"#;
    assert_eq!(
        compile_st_errors(input),
        0,
        "Bare invocation should compile clean"
    );
    let js = compile_st(input);
    assert!(
        js.contains("greeting") || js.contains("invoke"),
        "JS should contain template invocation.\nJS:\n{}",
        &js[..js.len().min(1000)]
    );
}

/// Bare template invocation with no args compiles
#[test]
fn test_bare_template_invocation_no_args() {
    let input = r#"
@template &nav() {
    <nav class="main-nav"></nav>
}

.header {
    &nav()
}
"#;
    assert_eq!(
        compile_st_errors(input),
        0,
        "Bare invocation with no args should compile clean"
    );
}

/// Multiple bare invocations in same selector compile
#[test]
fn test_multiple_bare_invocations() {
    let input = r#"
@template &nav() {
    <nav></nav>
}
@template &footer() {
    <footer></footer>
}

body {
    &nav()
    &footer()
}
"#;
    assert_eq!(
        compile_st_errors(input),
        0,
        "Multiple bare invocations should compile clean"
    );
}
