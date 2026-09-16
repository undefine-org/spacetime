//! IR Builder Functions
//!
//! Helper functions to construct structured IR types more ergonomically.
//! These builders make it easier to generate CSS IR without verbose struct construction.

use super::{CssDecl, CssExpr, KeyframeBlock};

/// Build a CSS rule with selector and declarations
pub fn css_rule(selector: &str, declarations: Vec<(&str, &str)>) -> CssExpr {
    CssExpr::Rule {
        selector: selector.to_string(),
        declarations: declarations
            .into_iter()
            .map(|(p, v)| CssDecl {
                property: p.to_string(),
                value: v.to_string(),
                important: false,
            })
            .collect(),
    }
}

/// Build a CSS rule with CssDecl structs directly
pub fn css_rule_with_decls(selector: &str, declarations: Vec<CssDecl>) -> CssExpr {
    CssExpr::Rule {
        selector: selector.to_string(),
        declarations,
    }
}

/// Build a single CSS declaration
pub fn css_decl(property: &str, value: &str) -> CssDecl {
    CssDecl {
        property: property.to_string(),
        value: value.to_string(),
        important: false,
    }
}

/// Build @keyframes with frame tuples
pub fn css_keyframes(name: &str, frames: Vec<(&str, Vec<(&str, &str)>)>) -> CssExpr {
    CssExpr::Keyframes {
        name: name.to_string(),
        frames: frames
            .into_iter()
            .map(|(sel, decls)| KeyframeBlock {
                selector: sel.to_string(),
                declarations: decls
                    .into_iter()
                    .map(|(p, v)| CssDecl {
                        property: p.to_string(),
                        value: v.to_string(),
                        important: false,
                    })
                    .collect(),
            })
            .collect(),
    }
}

/// Build @keyframes with KeyframeBlock structs directly
pub fn css_keyframes_with_blocks(name: &str, frames: Vec<KeyframeBlock>) -> CssExpr {
    CssExpr::Keyframes {
        name: name.to_string(),
        frames,
    }
}

/// Build a single keyframe block
pub fn keyframe_block(selector: &str, declarations: Vec<(&str, &str)>) -> KeyframeBlock {
    KeyframeBlock {
        selector: selector.to_string(),
        declarations: declarations
            .into_iter()
            .map(|(p, v)| CssDecl {
                property: p.to_string(),
                value: v.to_string(),
                important: false,
            })
            .collect(),
    }
}

// ============================================================================
// JS IR Builder Functions
// ============================================================================

use super::{DeclKind, JsExpr, JsLit, JsStmt};

/// Build a const declaration
pub fn js_const(name: &str, init: JsExpr) -> JsStmt {
    JsStmt::Decl {
        kind: DeclKind::Const,
        name: name.to_string(),
        init: Some(init),
    }
}

/// Build a let declaration
pub fn js_let(name: &str, init: Option<JsExpr>) -> JsStmt {
    JsStmt::Decl {
        kind: DeclKind::Let,
        name: name.to_string(),
        init,
    }
}

/// Build a variable reference
pub fn js_var(name: &str) -> JsExpr {
    JsExpr::Var(name.to_string())
}

/// Build a function call
pub fn js_call(callee: JsExpr, args: Vec<JsExpr>) -> JsExpr {
    JsExpr::Call {
        callee: Box::new(callee),
        args,
    }
}

/// Build a method call
pub fn js_method(obj: JsExpr, method: &str, args: Vec<JsExpr>) -> JsExpr {
    JsExpr::Method {
        obj: Box::new(obj),
        method: method.to_string(),
        args,
    }
}

/// Build property access
pub fn js_prop(obj: JsExpr, prop: &str) -> JsExpr {
    JsExpr::Prop {
        obj: Box::new(obj),
        prop: prop.to_string(),
    }
}

/// Build an arrow function
pub fn js_arrow(params: Vec<&str>, body: JsExpr) -> JsExpr {
    JsExpr::Arrow {
        params: params.into_iter().map(|s| s.to_string()).collect(),
        body: Box::new(body),
    }
}

/// Build an arrow function with block body
pub fn js_arrow_block(params: Vec<&str>, stmts: Vec<JsStmt>) -> JsExpr {
    JsExpr::Arrow {
        params: params.into_iter().map(|s| s.to_string()).collect(),
        body: Box::new(JsExpr::Block { stmts, expr: None }),
    }
}

/// Build an object literal
pub fn js_object(fields: Vec<(&str, JsExpr)>) -> JsExpr {
    JsExpr::Object(
        fields
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    )
}

/// Build an array literal
pub fn js_array(items: Vec<JsExpr>) -> JsExpr {
    JsExpr::Array(items)
}

/// Build a string literal
pub fn js_string(s: &str) -> JsExpr {
    JsExpr::Lit(JsLit::String(s.to_string()))
}

/// Build a number literal
pub fn js_number(n: f64) -> JsExpr {
    JsExpr::Lit(JsLit::Number(n))
}

/// Build a boolean literal
pub fn js_bool(b: bool) -> JsExpr {
    JsExpr::Lit(JsLit::Bool(b))
}

/// Build null literal
pub fn js_null() -> JsExpr {
    JsExpr::Lit(JsLit::Null)
}

/// Build an expression statement
pub fn js_expr_stmt(expr: JsExpr) -> JsStmt {
    JsStmt::Expr(expr)
}

/// Build a return statement
pub fn js_return(expr: Option<JsExpr>) -> JsStmt {
    JsStmt::Return(expr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_css_rule_builder() {
        let rule = css_rule(".test", vec![("color", "red")]);
        match rule {
            CssExpr::Rule {
                selector,
                declarations,
            } => {
                assert_eq!(selector, ".test");
                assert_eq!(declarations.len(), 1);
                assert_eq!(declarations[0].property, "color");
                assert_eq!(declarations[0].value, "red");
            }
            _ => panic!("Expected Rule variant"),
        }
    }

    #[test]
    fn test_css_keyframes_builder() {
        let kf = css_keyframes(
            "fade-in",
            vec![
                ("0%", vec![("opacity", "0")]),
                ("100%", vec![("opacity", "1")]),
            ],
        );
        match kf {
            CssExpr::Keyframes { name, frames } => {
                assert_eq!(name, "fade-in");
                assert_eq!(frames.len(), 2);
            }
            _ => panic!("Expected Keyframes variant"),
        }
    }

    // ========================================================================
    // JS Builder Tests
    // ========================================================================

    #[test]
    fn test_js_const_builder() {
        let stmt = js_const("x", js_number(42.0));
        match stmt {
            JsStmt::Decl { kind, name, init } => {
                assert_eq!(kind, DeclKind::Const);
                assert_eq!(name, "x");
                assert!(init.is_some());
                assert_eq!(init.unwrap(), JsExpr::Lit(JsLit::Number(42.0)));
            }
            _ => panic!("Expected Decl variant"),
        }
    }

    #[test]
    fn test_js_let_builder() {
        let stmt = js_let("y", None);
        match stmt {
            JsStmt::Decl { kind, name, init } => {
                assert_eq!(kind, DeclKind::Let);
                assert_eq!(name, "y");
                assert!(init.is_none());
            }
            _ => panic!("Expected Decl variant"),
        }
    }

    #[test]
    fn test_js_var_builder() {
        let expr = js_var("myVar");
        assert_eq!(expr, JsExpr::Var("myVar".to_string()));
    }

    #[test]
    fn test_js_call_builder() {
        let expr = js_call(js_var("fn"), vec![js_string("arg1"), js_number(2.0)]);
        match expr {
            JsExpr::Call { callee, args } => {
                assert_eq!(*callee, JsExpr::Var("fn".to_string()));
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Expected Call variant"),
        }
    }

    #[test]
    fn test_js_method_builder() {
        let expr = js_method(js_var("obj"), "doSomething", vec![js_bool(true)]);
        match expr {
            JsExpr::Method { obj, method, args } => {
                assert_eq!(*obj, JsExpr::Var("obj".to_string()));
                assert_eq!(method, "doSomething");
                assert_eq!(args.len(), 1);
            }
            _ => panic!("Expected Method variant"),
        }
    }

    #[test]
    fn test_js_prop_builder() {
        let expr = js_prop(js_var("window"), "location");
        match expr {
            JsExpr::Prop { obj, prop } => {
                assert_eq!(*obj, JsExpr::Var("window".to_string()));
                assert_eq!(prop, "location");
            }
            _ => panic!("Expected Prop variant"),
        }
    }

    #[test]
    fn test_js_arrow_builder() {
        let expr = js_arrow(vec!["x", "y"], js_var("x"));
        match expr {
            JsExpr::Arrow { params, body } => {
                assert_eq!(params, vec!["x".to_string(), "y".to_string()]);
                assert_eq!(*body, JsExpr::Var("x".to_string()));
            }
            _ => panic!("Expected Arrow variant"),
        }
    }

    #[test]
    fn test_js_arrow_block_builder() {
        let expr = js_arrow_block(vec!["a"], vec![js_return(Some(js_var("a")))]);
        match expr {
            JsExpr::Arrow { params, body } => {
                assert_eq!(params, vec!["a".to_string()]);
                match *body {
                    JsExpr::Block { stmts, expr } => {
                        assert_eq!(stmts.len(), 1);
                        assert!(expr.is_none());
                    }
                    _ => panic!("Expected Block body"),
                }
            }
            _ => panic!("Expected Arrow variant"),
        }
    }

    #[test]
    fn test_js_object_builder() {
        let expr = js_object(vec![("name", js_string("test")), ("count", js_number(5.0))]);
        match expr {
            JsExpr::Object(fields) => {
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].0, "name");
                assert_eq!(fields[1].0, "count");
            }
            _ => panic!("Expected Object variant"),
        }
    }

    #[test]
    fn test_js_array_builder() {
        let expr = js_array(vec![js_number(1.0), js_number(2.0), js_number(3.0)]);
        match expr {
            JsExpr::Array(items) => {
                assert_eq!(items.len(), 3);
            }
            _ => panic!("Expected Array variant"),
        }
    }

    // 3.14 below is an arbitrary decimal test fixture (verifying number
    // literal construction), not an intended pi approximation --
    // clippy::approx_constant is a false positive on this test.
    #[allow(clippy::approx_constant)]
    #[test]
    fn test_js_literals() {
        assert_eq!(
            js_string("hello"),
            JsExpr::Lit(JsLit::String("hello".to_string()))
        );
        assert_eq!(js_number(3.14), JsExpr::Lit(JsLit::Number(3.14)));
        assert_eq!(js_bool(true), JsExpr::Lit(JsLit::Bool(true)));
        assert_eq!(js_null(), JsExpr::Lit(JsLit::Null));
    }

    #[test]
    fn test_js_expr_stmt_builder() {
        let stmt = js_expr_stmt(js_call(js_var("console"), vec![]));
        match stmt {
            JsStmt::Expr(_) => {}
            _ => panic!("Expected Expr variant"),
        }
    }

    #[test]
    fn test_js_return_builder() {
        let stmt = js_return(Some(js_number(42.0)));
        match stmt {
            JsStmt::Return(Some(expr)) => {
                assert_eq!(expr, JsExpr::Lit(JsLit::Number(42.0)));
            }
            _ => panic!("Expected Return variant with value"),
        }

        let stmt_none = js_return(None);
        match stmt_none {
            JsStmt::Return(None) => {}
            _ => panic!("Expected Return variant without value"),
        }
    }
}
