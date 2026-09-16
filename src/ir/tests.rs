//! Tests for IR types

use super::*;

#[test]
fn test_js_lit_construction() {
    let string_lit = JsLit::String("hello".to_string());
    assert_eq!(string_lit, JsLit::String("hello".to_string()));

    let num_lit = JsLit::Number(42.0);
    assert_eq!(num_lit, JsLit::Number(42.0));

    let bool_lit = JsLit::Bool(true);
    assert_eq!(bool_lit, JsLit::Bool(true));

    let null_lit = JsLit::Null;
    assert_eq!(null_lit, JsLit::Null);

    let undef_lit = JsLit::Undefined;
    assert_eq!(undef_lit, JsLit::Undefined);
}

#[test]
fn test_js_expr_var() {
    let var_expr = JsExpr::Var("myVar".to_string());
    assert_eq!(var_expr, JsExpr::Var("myVar".to_string()));
}

#[test]
fn test_js_expr_prop_access() {
    let prop_expr = JsExpr::Prop {
        obj: Box::new(JsExpr::Var("obj".to_string())),
        prop: "field".to_string(),
    };

    if let JsExpr::Prop { obj, prop } = prop_expr {
        assert_eq!(*obj, JsExpr::Var("obj".to_string()));
        assert_eq!(prop, "field");
    } else {
        panic!("Expected Prop variant");
    }
}

#[test]
fn test_js_expr_index_access() {
    let index_expr = JsExpr::Index {
        arr: Box::new(JsExpr::Var("arr".to_string())),
        idx: Box::new(JsExpr::Lit(JsLit::Number(0.0))),
    };

    if let JsExpr::Index { arr, idx } = index_expr {
        assert_eq!(*arr, JsExpr::Var("arr".to_string()));
        assert_eq!(*idx, JsExpr::Lit(JsLit::Number(0.0)));
    } else {
        panic!("Expected Index variant");
    }
}

#[test]
fn test_js_expr_call() {
    let call_expr = JsExpr::Call {
        callee: Box::new(JsExpr::Var("console".to_string())),
        args: vec![JsExpr::Lit(JsLit::String("hello".to_string()))],
    };

    if let JsExpr::Call { callee, args } = call_expr {
        assert_eq!(*callee, JsExpr::Var("console".to_string()));
        assert_eq!(args.len(), 1);
    } else {
        panic!("Expected Call variant");
    }
}

#[test]
fn test_js_expr_method_call() {
    let method_expr = JsExpr::Method {
        obj: Box::new(JsExpr::Var("console".to_string())),
        method: "log".to_string(),
        args: vec![JsExpr::Lit(JsLit::String("hello".to_string()))],
    };

    if let JsExpr::Method { obj, method, args } = method_expr {
        assert_eq!(*obj, JsExpr::Var("console".to_string()));
        assert_eq!(method, "log");
        assert_eq!(args.len(), 1);
    } else {
        panic!("Expected Method variant");
    }
}

#[test]
fn test_js_expr_arrow() {
    let arrow_expr = JsExpr::Arrow {
        params: vec!["x".to_string(), "y".to_string()],
        body: Box::new(JsExpr::Binary {
            left: Box::new(JsExpr::Var("x".to_string())),
            op: BinOp::Add,
            right: Box::new(JsExpr::Var("y".to_string())),
        }),
    };

    if let JsExpr::Arrow { params, body } = arrow_expr {
        assert_eq!(params, vec!["x", "y"]);
        assert!(matches!(*body, JsExpr::Binary { .. }));
    } else {
        panic!("Expected Arrow variant");
    }
}

#[test]
fn test_js_expr_binary() {
    let binary_expr = JsExpr::Binary {
        left: Box::new(JsExpr::Lit(JsLit::Number(1.0))),
        op: BinOp::Add,
        right: Box::new(JsExpr::Lit(JsLit::Number(2.0))),
    };

    if let JsExpr::Binary { left, op, right } = binary_expr {
        assert_eq!(*left, JsExpr::Lit(JsLit::Number(1.0)));
        assert_eq!(op, BinOp::Add);
        assert_eq!(*right, JsExpr::Lit(JsLit::Number(2.0)));
    } else {
        panic!("Expected Binary variant");
    }
}

#[test]
fn test_js_expr_ternary() {
    let ternary_expr = JsExpr::Ternary {
        cond: Box::new(JsExpr::Lit(JsLit::Bool(true))),
        then_: Box::new(JsExpr::Lit(JsLit::Number(1.0))),
        else_: Box::new(JsExpr::Lit(JsLit::Number(0.0))),
    };

    if let JsExpr::Ternary { cond, then_, else_ } = ternary_expr {
        assert_eq!(*cond, JsExpr::Lit(JsLit::Bool(true)));
        assert_eq!(*then_, JsExpr::Lit(JsLit::Number(1.0)));
        assert_eq!(*else_, JsExpr::Lit(JsLit::Number(0.0)));
    } else {
        panic!("Expected Ternary variant");
    }
}

#[test]
fn test_js_expr_object() {
    let obj_expr = JsExpr::Object(vec![
        (
            "name".to_string(),
            JsExpr::Lit(JsLit::String("test".to_string())),
        ),
        ("value".to_string(), JsExpr::Lit(JsLit::Number(42.0))),
    ]);

    if let JsExpr::Object(fields) = obj_expr {
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].0, "name");
        assert_eq!(fields[1].0, "value");
    } else {
        panic!("Expected Object variant");
    }
}

#[test]
fn test_js_expr_array() {
    let arr_expr = JsExpr::Array(vec![
        JsExpr::Lit(JsLit::Number(1.0)),
        JsExpr::Lit(JsLit::Number(2.0)),
        JsExpr::Lit(JsLit::Number(3.0)),
    ]);

    if let JsExpr::Array(items) = arr_expr {
        assert_eq!(items.len(), 3);
    } else {
        panic!("Expected Array variant");
    }
}

#[test]
fn test_js_expr_template() {
    let template_expr = JsExpr::Template {
        parts: vec![
            TemplatePart::Text("Hello, ".to_string()),
            TemplatePart::Expr(JsExpr::Var("name".to_string())),
            TemplatePart::Text("!".to_string()),
        ],
    };

    if let JsExpr::Template { parts } = template_expr {
        assert_eq!(parts.len(), 3);
        assert!(matches!(&parts[0], TemplatePart::Text(t) if t == "Hello, "));
        assert!(matches!(&parts[1], TemplatePart::Expr(_)));
        assert!(matches!(&parts[2], TemplatePart::Text(t) if t == "!"));
    } else {
        panic!("Expected Template variant");
    }
}

#[test]
fn test_js_stmt_decl() {
    let decl_stmt = JsStmt::Decl {
        kind: DeclKind::Const,
        name: "x".to_string(),
        init: Some(JsExpr::Lit(JsLit::Number(42.0))),
    };

    if let JsStmt::Decl { kind, name, init } = decl_stmt {
        assert_eq!(kind, DeclKind::Const);
        assert_eq!(name, "x");
        assert!(init.is_some());
    } else {
        panic!("Expected Decl variant");
    }
}

#[test]
fn test_js_stmt_if() {
    let if_stmt = JsStmt::If {
        cond: JsExpr::Lit(JsLit::Bool(true)),
        then_: vec![JsStmt::Return(Some(JsExpr::Lit(JsLit::Number(1.0))))],
        else_: Some(vec![JsStmt::Return(Some(JsExpr::Lit(JsLit::Number(0.0))))]),
    };

    if let JsStmt::If { cond, then_, else_ } = if_stmt {
        assert_eq!(cond, JsExpr::Lit(JsLit::Bool(true)));
        assert_eq!(then_.len(), 1);
        assert!(else_.is_some());
    } else {
        panic!("Expected If variant");
    }
}

#[test]
fn test_js_stmt_for_of() {
    let for_of_stmt = JsStmt::ForOf {
        var: "item".to_string(),
        iter: JsExpr::Var("items".to_string()),
        body: vec![JsStmt::Expr(JsExpr::Method {
            obj: Box::new(JsExpr::Var("console".to_string())),
            method: "log".to_string(),
            args: vec![JsExpr::Var("item".to_string())],
        })],
    };

    if let JsStmt::ForOf { var, iter, body } = for_of_stmt {
        assert_eq!(var, "item");
        assert_eq!(iter, JsExpr::Var("items".to_string()));
        assert_eq!(body.len(), 1);
    } else {
        panic!("Expected ForOf variant");
    }
}

#[test]
fn test_css_expr_rule() {
    let rule = CssExpr::Rule {
        selector: ".button".to_string(),
        declarations: vec![
            CssDecl {
                property: "color".to_string(),
                value: "red".to_string(),
                important: false,
            },
            CssDecl {
                property: "background".to_string(),
                value: "blue".to_string(),
                important: true,
            },
        ],
    };

    if let CssExpr::Rule {
        selector,
        declarations,
    } = rule
    {
        assert_eq!(selector, ".button");
        assert_eq!(declarations.len(), 2);
        assert!(!declarations[0].important);
        assert!(declarations[1].important);
    } else {
        panic!("Expected Rule variant");
    }
}

#[test]
fn test_css_expr_keyframes() {
    let keyframes = CssExpr::Keyframes {
        name: "fadeIn".to_string(),
        frames: vec![
            KeyframeBlock {
                selector: "from".to_string(),
                declarations: vec![CssDecl {
                    property: "opacity".to_string(),
                    value: "0".to_string(),
                    important: false,
                }],
            },
            KeyframeBlock {
                selector: "to".to_string(),
                declarations: vec![CssDecl {
                    property: "opacity".to_string(),
                    value: "1".to_string(),
                    important: false,
                }],
            },
        ],
    };

    if let CssExpr::Keyframes { name, frames } = keyframes {
        assert_eq!(name, "fadeIn");
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].selector, "from");
        assert_eq!(frames[1].selector, "to");
    } else {
        panic!("Expected Keyframes variant");
    }
}

#[test]
fn test_glsl_expr_program() {
    let program = GlslExpr::Program {
        vertex: "void main() { gl_Position = vec4(0.0); }".to_string(),
        fragment: "void main() { gl_FragColor = vec4(1.0); }".to_string(),
    };

    if let GlslExpr::Program { vertex, fragment } = program {
        assert!(vertex.contains("gl_Position"));
        assert!(fragment.contains("gl_FragColor"));
    } else {
        panic!("Expected Program variant");
    }
}

#[test]
fn test_html_expr_element() {
    let element = HtmlExpr::Element {
        tag: "div".to_string(),
        attrs: vec![("class".to_string(), crate::ir::AttrPart::lit("container"))],
        children: vec![
            HtmlExpr::Text("Hello".to_string()),
            HtmlExpr::Element {
                tag: "span".to_string(),
                attrs: vec![],
                children: vec![HtmlExpr::Text("World".to_string())],
                line: None,
            },
        ],
        line: None,
    };

    if let HtmlExpr::Element {
        tag,
        attrs,
        children,
        ..
    } = element
    {
        assert_eq!(tag, "div");
        assert_eq!(attrs.len(), 1);
        assert_eq!(children.len(), 2);
    } else {
        panic!("Expected Element variant");
    }
}

#[test]
fn test_code_fragment_construction() {
    let fragment = CodeFragment {
        kind: FragmentKind::Js(vec![JsStmt::Decl {
            kind: DeclKind::Const,
            name: "x".to_string(),
            init: Some(JsExpr::Lit(JsLit::Number(42.0))),
        }]),
        deps: vec!["signal1".to_string(), "signal2".to_string()],
        source_span: None,
    };

    assert_eq!(fragment.deps.len(), 2);
    if let FragmentKind::Js(stmts) = fragment.kind {
        assert_eq!(stmts.len(), 1);
    } else {
        panic!("Expected Js variant");
    }
}

#[test]
fn test_bin_op_variants() {
    // Test all binary operators exist and are distinct
    let ops = [
        BinOp::Add,
        BinOp::Sub,
        BinOp::Mul,
        BinOp::Div,
        BinOp::Mod,
        BinOp::Eq,
        BinOp::Ne,
        BinOp::Lt,
        BinOp::Le,
        BinOp::Gt,
        BinOp::Ge,
        BinOp::EqStrict,
        BinOp::NeStrict,
        BinOp::And,
        BinOp::Or,
        BinOp::BitAnd,
        BinOp::BitOr,
        BinOp::BitXor,
        BinOp::Shl,
        BinOp::Shr,
        BinOp::UShr,
    ];

    // Ensure all ops are distinct
    for (i, op1) in ops.iter().enumerate() {
        for (j, op2) in ops.iter().enumerate() {
            if i != j {
                assert_ne!(op1, op2);
            }
        }
    }
}

#[test]
fn test_unary_op_variants() {
    let ops = [UnaryOp::Not, UnaryOp::Neg, UnaryOp::Typeof, UnaryOp::Void];

    for (i, op1) in ops.iter().enumerate() {
        for (j, op2) in ops.iter().enumerate() {
            if i != j {
                assert_ne!(op1, op2);
            }
        }
    }
}

#[test]
fn test_decl_kind_variants() {
    assert_ne!(DeclKind::Const, DeclKind::Let);
    assert_ne!(DeclKind::Let, DeclKind::Var);
    assert_ne!(DeclKind::Const, DeclKind::Var);
}

#[test]
fn test_nested_expressions() {
    // Test deeply nested expression: obj.method(arr[0].prop + 1)
    let nested = JsExpr::Method {
        obj: Box::new(JsExpr::Var("obj".to_string())),
        method: "method".to_string(),
        args: vec![JsExpr::Binary {
            left: Box::new(JsExpr::Prop {
                obj: Box::new(JsExpr::Index {
                    arr: Box::new(JsExpr::Var("arr".to_string())),
                    idx: Box::new(JsExpr::Lit(JsLit::Number(0.0))),
                }),
                prop: "prop".to_string(),
            }),
            op: BinOp::Add,
            right: Box::new(JsExpr::Lit(JsLit::Number(1.0))),
        }],
    };

    // Just ensure it constructs without panic
    assert!(matches!(nested, JsExpr::Method { .. }));
}

#[test]
fn test_clone_and_eq() {
    let original = JsExpr::Binary {
        left: Box::new(JsExpr::Var("x".to_string())),
        op: BinOp::Mul,
        right: Box::new(JsExpr::Lit(JsLit::Number(2.0))),
    };

    let cloned = original.clone();
    assert_eq!(original, cloned);
}
