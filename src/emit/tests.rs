//! Tests for the emit layer

use super::*;
use crate::ir::*;

// ============================================================================
// JavaScript Emitter Tests
// ============================================================================

// 3.14 below is an arbitrary decimal test fixture (verifying number
// literal emission), not an intended pi approximation --
// clippy::approx_constant is a false positive on this test.
#[allow(clippy::approx_constant)]
#[test]
fn test_js_literals() {
    let opts = EmitOptions::pretty();

    assert_eq!(
        js::emit_expr(&JsExpr::Lit(JsLit::String("hello".into())), &opts).unwrap(),
        "\"hello\""
    );
    assert_eq!(
        js::emit_expr(&JsExpr::Lit(JsLit::Number(42.0)), &opts).unwrap(),
        "42"
    );
    assert_eq!(
        js::emit_expr(&JsExpr::Lit(JsLit::Number(3.14)), &opts).unwrap(),
        "3.14"
    );
    assert_eq!(
        js::emit_expr(&JsExpr::Lit(JsLit::Bool(true)), &opts).unwrap(),
        "true"
    );
    assert_eq!(
        js::emit_expr(&JsExpr::Lit(JsLit::Null), &opts).unwrap(),
        "null"
    );
    assert_eq!(
        js::emit_expr(&JsExpr::Lit(JsLit::Undefined), &opts).unwrap(),
        "undefined"
    );
}

#[test]
fn test_js_string_escaping() {
    let opts = EmitOptions::pretty();

    let expr = JsExpr::Lit(JsLit::String("hello\nworld".into()));
    assert_eq!(js::emit_expr(&expr, &opts).unwrap(), "\"hello\\nworld\"");

    let expr = JsExpr::Lit(JsLit::String("say \"hi\"".into()));
    assert_eq!(js::emit_expr(&expr, &opts).unwrap(), "\"say \\\"hi\\\"\"");
}

#[test]
fn test_js_variable() {
    let opts = EmitOptions::pretty();
    assert_eq!(
        js::emit_expr(&JsExpr::Var("foo".into()), &opts).unwrap(),
        "foo"
    );
}

#[test]
fn test_js_property_access() {
    let opts = EmitOptions::pretty();

    let expr = JsExpr::Prop {
        obj: Box::new(JsExpr::Var("obj".into())),
        prop: "name".into(),
    };
    assert_eq!(js::emit_expr(&expr, &opts).unwrap(), "obj.name");

    // Bracket notation for special properties
    let expr = JsExpr::Prop {
        obj: Box::new(JsExpr::Var("obj".into())),
        prop: "data-value".into(),
    };
    assert_eq!(js::emit_expr(&expr, &opts).unwrap(), "obj[\"data-value\"]");
}

#[test]
fn test_js_function_call() {
    let opts = EmitOptions::pretty();

    let expr = JsExpr::Call {
        callee: Box::new(JsExpr::Var("fn".into())),
        args: vec![
            JsExpr::Lit(JsLit::Number(1.0)),
            JsExpr::Lit(JsLit::Number(2.0)),
        ],
    };
    assert_eq!(js::emit_expr(&expr, &opts).unwrap(), "fn(1, 2)");
}

#[test]
fn test_js_method_call() {
    let opts = EmitOptions::pretty();

    let expr = JsExpr::Method {
        obj: Box::new(JsExpr::Var("arr".into())),
        method: "map".into(),
        args: vec![JsExpr::Var("fn".into())],
    };
    assert_eq!(js::emit_expr(&expr, &opts).unwrap(), "arr.map(fn)");
}

#[test]
fn test_js_arrow_function() {
    let opts = EmitOptions::pretty();

    // Single param
    let expr = JsExpr::Arrow {
        params: vec!["x".into()],
        body: Box::new(JsExpr::Binary {
            left: Box::new(JsExpr::Var("x".into())),
            op: BinOp::Mul,
            right: Box::new(JsExpr::Lit(JsLit::Number(2.0))),
        }),
    };
    assert_eq!(js::emit_expr(&expr, &opts).unwrap(), "x => x * 2");

    // Multiple params
    let expr = JsExpr::Arrow {
        params: vec!["a".into(), "b".into()],
        body: Box::new(JsExpr::Binary {
            left: Box::new(JsExpr::Var("a".into())),
            op: BinOp::Add,
            right: Box::new(JsExpr::Var("b".into())),
        }),
    };
    assert_eq!(js::emit_expr(&expr, &opts).unwrap(), "(a, b) => a + b");
}

#[test]
fn test_js_template_literal() {
    let opts = EmitOptions::pretty();

    let expr = JsExpr::Template {
        parts: vec![
            TemplatePart::Text("Hello ".into()),
            TemplatePart::Expr(JsExpr::Var("name".into())),
            TemplatePart::Text("!".into()),
        ],
    };
    assert_eq!(js::emit_expr(&expr, &opts).unwrap(), "`Hello ${name}!`");
}

#[test]
fn test_js_binary_ops() {
    let opts = EmitOptions::pretty();

    let expr = JsExpr::Binary {
        left: Box::new(JsExpr::Var("a".into())),
        op: BinOp::Add,
        right: Box::new(JsExpr::Var("b".into())),
    };
    assert_eq!(js::emit_expr(&expr, &opts).unwrap(), "a + b");

    let expr = JsExpr::Binary {
        left: Box::new(JsExpr::Var("x".into())),
        op: BinOp::EqStrict,
        right: Box::new(JsExpr::Lit(JsLit::Number(0.0))),
    };
    assert_eq!(js::emit_expr(&expr, &opts).unwrap(), "x === 0");
}

#[test]
fn test_js_ternary() {
    let opts = EmitOptions::pretty();

    let expr = JsExpr::Ternary {
        cond: Box::new(JsExpr::Var("cond".into())),
        then_: Box::new(JsExpr::Lit(JsLit::String("yes".into()))),
        else_: Box::new(JsExpr::Lit(JsLit::String("no".into()))),
    };
    assert_eq!(
        js::emit_expr(&expr, &opts).unwrap(),
        "cond ? \"yes\" : \"no\""
    );
}

#[test]
fn test_js_object_literal() {
    let opts = EmitOptions::minified();

    let expr = JsExpr::Object(vec![
        ("name".into(), JsExpr::Lit(JsLit::String("test".into()))),
        ("value".into(), JsExpr::Lit(JsLit::Number(42.0))),
    ]);
    assert_eq!(
        js::emit_expr(&expr, &opts).unwrap(),
        "{ name: \"test\", value: 42 }"
    );
}

#[test]
fn test_js_array_literal() {
    let opts = EmitOptions::pretty();

    let expr = JsExpr::Array(vec![
        JsExpr::Lit(JsLit::Number(1.0)),
        JsExpr::Lit(JsLit::Number(2.0)),
        JsExpr::Lit(JsLit::Number(3.0)),
    ]);
    assert_eq!(js::emit_expr(&expr, &opts).unwrap(), "[1, 2, 3]");
}

#[test]
fn test_js_statements() {
    let opts = EmitOptions::pretty();

    let stmt = JsStmt::Decl {
        kind: DeclKind::Const,
        name: "x".into(),
        init: Some(JsExpr::Lit(JsLit::Number(42.0))),
    };
    assert_eq!(js::emit_stmt(&stmt, &opts, 0).unwrap(), "const x = 42;");

    let stmt = JsStmt::Return(Some(JsExpr::Var("result".into())));
    assert_eq!(js::emit_stmt(&stmt, &opts, 0).unwrap(), "return result;");
}

#[test]
fn test_js_if_statement() {
    let opts = EmitOptions::minified();

    let stmt = JsStmt::If {
        cond: JsExpr::Var("cond".into()),
        then_: vec![JsStmt::Return(Some(JsExpr::Lit(JsLit::Bool(true))))],
        else_: Some(vec![JsStmt::Return(Some(JsExpr::Lit(JsLit::Bool(false))))]),
    };
    assert_eq!(
        js::emit_stmt(&stmt, &opts, 0).unwrap(),
        "if (cond) { return true; } else { return false; }"
    );
}

#[test]
fn test_js_for_of() {
    let opts = EmitOptions::minified();

    let stmt = JsStmt::ForOf {
        var: "item".into(),
        iter: JsExpr::Var("items".into()),
        body: vec![JsStmt::Expr(JsExpr::Call {
            callee: Box::new(JsExpr::Var("process".into())),
            args: vec![JsExpr::Var("item".into())],
        })],
    };
    assert_eq!(
        js::emit_stmt(&stmt, &opts, 0).unwrap(),
        "for (const item of items) { process(item); }"
    );
}

// ============================================================================
// CSS Emitter Tests
// ============================================================================

#[test]
fn test_css_rule() {
    let opts = EmitOptions::pretty();

    let expr = CssExpr::Rule {
        selector: ".button".into(),
        declarations: vec![
            CssDecl {
                property: "color".into(),
                value: "red".into(),
                important: false,
            },
            CssDecl {
                property: "font-size".into(),
                value: "16px".into(),
                important: false,
            },
        ],
    };
    let result = css::emit(&expr, &opts);
    assert!(result.contains(".button"));
    assert!(result.contains("color: red"));
    assert!(result.contains("font-size: 16px"));
}

#[test]
fn test_css_rule_minified() {
    let opts = EmitOptions::minified();

    let expr = CssExpr::Rule {
        selector: ".button".into(),
        declarations: vec![CssDecl {
            property: "color".into(),
            value: "red".into(),
            important: false,
        }],
    };
    assert_eq!(css::emit(&expr, &opts), ".button{color: red}");
}

#[test]
fn test_css_important() {
    let opts = EmitOptions::minified();

    let expr = CssExpr::Rule {
        selector: ".override".into(),
        declarations: vec![CssDecl {
            property: "display".into(),
            value: "none".into(),
            important: true,
        }],
    };
    assert_eq!(
        css::emit(&expr, &opts),
        ".override{display: none !important}"
    );
}

#[test]
fn test_css_keyframes() {
    let opts = EmitOptions::minified();

    let expr = CssExpr::Keyframes {
        name: "fadeIn".into(),
        frames: vec![
            KeyframeBlock {
                selector: "from".into(),
                declarations: vec![CssDecl {
                    property: "opacity".into(),
                    value: "0".into(),
                    important: false,
                }],
            },
            KeyframeBlock {
                selector: "to".into(),
                declarations: vec![CssDecl {
                    property: "opacity".into(),
                    value: "1".into(),
                    important: false,
                }],
            },
        ],
    };
    let result = css::emit(&expr, &opts);
    assert!(result.contains("@keyframes fadeIn"));
    assert!(result.contains("from{opacity: 0}"));
    assert!(result.contains("to{opacity: 1}"));
}

// ============================================================================
// GLSL Emitter Tests
// ============================================================================

#[test]
fn test_glsl_raw() {
    let opts = EmitOptions::pretty();

    let expr = GlslExpr::Raw("void main() { gl_FragColor = vec4(1.0); }".into());
    assert_eq!(
        glsl::emit(&expr, &opts),
        "void main() { gl_FragColor = vec4(1.0); }"
    );
}

#[test]
fn test_glsl_program() {
    let opts = EmitOptions::pretty();

    let expr = GlslExpr::Program {
        vertex: "void main() { gl_Position = vec4(0.0); }".into(),
        fragment: "void main() { gl_FragColor = vec4(1.0); }".into(),
    };
    let result = glsl::emit(&expr, &opts);
    assert!(result.contains("VERTEX SHADER"));
    assert!(result.contains("FRAGMENT SHADER"));
    assert!(result.contains("gl_Position"));
    assert!(result.contains("gl_FragColor"));
}

// ============================================================================
// HTML Emitter Tests
// ============================================================================

#[test]
fn test_html_element() {
    let opts = EmitOptions::minified();

    let expr = HtmlExpr::Element {
        tag: "div".into(),
        attrs: vec![("class".into(), crate::ir::AttrPart::lit("container"))],
        children: vec![HtmlExpr::Text("Hello".into())],
        line: None,
    };
    assert_eq!(
        html::emit(&expr, &opts),
        "<div class=\"container\">Hello</div>"
    );
}

#[test]
fn test_html_void_element() {
    let opts = EmitOptions::minified();

    let expr = HtmlExpr::Element {
        tag: "br".into(),
        attrs: vec![],
        children: vec![],
        line: None,
    };
    assert_eq!(html::emit(&expr, &opts), "<br>");

    let expr = HtmlExpr::Element {
        tag: "img".into(),
        attrs: vec![
            ("src".into(), crate::ir::AttrPart::lit("test.png")),
            ("alt".into(), crate::ir::AttrPart::lit("Test")),
        ],
        children: vec![],
        line: None,
    };
    assert_eq!(
        html::emit(&expr, &opts),
        "<img src=\"test.png\" alt=\"Test\">"
    );
}

#[test]
fn test_html_escaping() {
    let opts = EmitOptions::minified();

    let expr = HtmlExpr::Text("<script>alert('xss')</script>".into());
    assert_eq!(
        html::emit(&expr, &opts),
        "&lt;script&gt;alert('xss')&lt;/script&gt;"
    );
}

#[test]
fn test_html_nested() {
    let opts = EmitOptions::minified();

    let expr = HtmlExpr::Element {
        tag: "ul".into(),
        attrs: vec![],
        children: vec![
            HtmlExpr::Element {
                tag: "li".into(),
                attrs: vec![],
                children: vec![HtmlExpr::Text("Item 1".into())],
                line: None,
            },
            HtmlExpr::Element {
                tag: "li".into(),
                attrs: vec![],
                children: vec![HtmlExpr::Text("Item 2".into())],
                line: None,
            },
        ],
        line: None,
    };
    assert_eq!(
        html::emit(&expr, &opts),
        "<ul><li>Item 1</li><li>Item 2</li></ul>"
    );
}

// ============================================================================
// Fragment Emitter Tests
// ============================================================================

#[test]
fn test_emit_fragment_js() {
    let opts = EmitOptions::pretty();

    let fragment = CodeFragment {
        kind: FragmentKind::Js(vec![JsStmt::Decl {
            kind: DeclKind::Const,
            name: "x".into(),
            init: Some(JsExpr::Lit(JsLit::Number(42.0))),
        }]),
        deps: vec!["$signal".into()],
        source_span: None,
    };
    assert_eq!(emit_fragment(&fragment, &opts), "const x = 42;");
}

#[test]
fn test_emit_fragment_css() {
    let opts = EmitOptions::minified();

    let fragment = CodeFragment {
        kind: FragmentKind::Css(vec![CssExpr::Rule {
            selector: ".test".into(),
            declarations: vec![CssDecl {
                property: "color".into(),
                value: "blue".into(),
                important: false,
            }],
        }]),
        deps: vec![],
        source_span: None,
    };
    assert_eq!(emit_fragment(&fragment, &opts), ".test{color: blue}");
}
