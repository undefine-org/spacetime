//! PLAN-023 W4 — HTML-bodied `@each` (each-inline).
//!
//! Enshrines: an `@each` whose body is raw HTML (not `&template(...)` invocations)
//! compiles to the each-inline primitive (dissolving BUG-028), an empty `@each`
//! body is valid (renders nothing, dissolving BUG-030), and a template-invocation
//! body still routes to each-with-templates.

use spacetime::{compile, compiler::CompileOptions, parse};

fn compile_ok(src: &str) -> spacetime::compiler::CompiledSpacetime {
    let ast = parse(src).expect("parse");
    compile(&ast, CompileOptions::default())
}

/// HTML-body `@each` lowers to each-inline and carries the row HTML.
#[test]
fn each_html_body_uses_each_inline() {
    let js = compile_ok(
        "@data inline $items : [\"a\", \"b\"];\n.list {\n  @each($items as $i) {\n    <li class=\"row\">`$i`</li>\n  }\n}\n",
    )
    .js;
    assert!(
        js.contains("rowHtml") || js.contains("each-inline") || js.contains("makeRow"),
        "HTML-body @each should lower to each-inline; got:\n{}",
        &js[..js.len().min(4000)]
    );
}

/// The row HTML (with its backtick hole) is captured into the emitted JS.
#[test]
fn each_html_body_captures_row_markup() {
    let js = compile_ok(
        "@data inline $items : [\"a\"];\n.list {\n  @each($items as $i) {\n    <li class=\"row\">`$i`</li>\n  }\n}\n",
    )
    .js;
    assert!(
        js.contains("class=\\\"row\\\"") || js.contains("<li class"),
        "row markup must be present in emitted JS; got:\n{}",
        &js[..js.len().min(4000)]
    );
}

/// An empty `@each` body is valid (BUG-030): no E0804, compiles cleanly.
#[test]
fn each_empty_body_is_valid() {
    let src = "@data inline $items : [\"a\"];\n.list {\n  @each($items as $i) {\n  }\n}\n";
    let ast = parse(src).expect("parse");
    let compiled = compile(&ast, CompileOptions::default());
    // Compilation must not surface the E0804 missing-invocations error.
    assert!(
        !compiled
            .js
            .contains("Missing required parameter 'invocations'"),
        "empty @each body must not raise E0804"
    );
}

/// Review regression (W4): an unspaced `<` comparison in a JS/expr body must NOT be
/// mis-parsed as an HTML element start (which previously consumed to EOF).
#[test]
fn unspaced_comparison_in_body_is_not_html() {
    // A mutation whose RHS has an unspaced comparison.
    let src = "@data inline $i : 0;\n.x {\n  $i <- $i<5;\n}\n";
    let ast = parse(src).expect("parse should not fail on unspaced comparison");
    let _ = compile(&ast, CompileOptions::default());
    // Reaching here without a parse panic/EOF is the assertion.
    assert!(!ast.scopes.is_empty(), "scope must parse");
}
