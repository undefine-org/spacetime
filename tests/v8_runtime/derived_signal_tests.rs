//! Derived-signal (`@data derive` / `@data fold`) compiled-behavior tests.
//!
//! BUG-273: a derive whose expression carries an arrow lambda with $-prefixed
//! params (`.filter(($r) => $r.status == "open")`) had EVERY `$name` token —
//! including the lambda's formal parameter — rewritten to
//! `SpacetimeLocal['name']`, producing `(SpacetimeLocal['r']) => …`, an invalid
//! formal-parameter list → `SyntaxError: missing ) after formal parameters` on
//! every recompute (surfaced by the comments pill's `$comOpenCount` derive).
//!
//! These run the REAL compiled bundle in V8 (BUG-252: behavior, not source
//! strings): the derived value must land in the global store and recompute when
//! its source signal updates.

use super::context::V8TestContext;

const ST_JS: &str = include_str!("../../public/runtime/st.js");

fn compile_page(source: &str) -> spacetime::CompiledSpacetime {
    let ast = spacetime::parse(source).expect("test source should parse");
    spacetime::Compiler::from_ast(&ast).compile()
}

fn boot(compiled: &spacetime::CompiledSpacetime) -> V8TestContext {
    let mut c = V8TestContext::new();
    c.eval(ST_JS).expect("load st.js");
    c.set_body_html(&compiled.html).expect("install page html");
    c.eval(&compiled.js).expect("eval page bundle");
    c.trigger_dom_ready().expect("DOMContentLoaded");
    c
}

fn num(c: &mut V8TestContext, expr: &str) -> Option<f64> {
    c.eval(expr).ok().and_then(|v| v.as_f64())
}

/// The pill's exact shape: `.filter(($r) => …).length` over an inline source.
/// The lambda param `$r` is a LOCAL — it must survive the $sig rewrite verbatim
/// and must not become a watched "signal".
#[test]
fn derive_with_arrow_lambda_param_computes() {
    let compiled = compile_page(
        r#"
@data inline $recs : [{"status":"open"},{"status":"closed"},{"status":"in-progress"},{"status":"open"}];
@data derive $openCount number : $recs.filter(($r) => $r.status == "open" || $r.status == "in-progress").length;
"#,
    );
    let mut c = boot(&compiled);

    assert_eq!(
        num(&mut c, "SpacetimeLocal.openCount"),
        Some(3.0),
        "lambda-param derive should compute the filtered count, not die on a SyntaxError"
    );
    let errors = c.get_errors();
    assert!(
        errors.is_empty(),
        "no console errors expected, got: {:?}",
        errors
    );
}

/// Multi-param lambdas and nested member access stay intact.
#[test]
fn derive_with_multi_param_lambda_computes() {
    let compiled = compile_page(
        r#"
@data inline $pairs : [[1, 2], [3, 4], [5, 6]];
@data derive $sum number : $pairs.reduce(($acc, $p) => $acc + $p[0] * $p[1], 0);
"#,
    );
    let mut c = boot(&compiled);

    assert_eq!(
        num(&mut c, "SpacetimeLocal.sum"),
        Some(44.0),
        "2 + 12 + 30 = 44 via a two-param arrow"
    );
    let errors = c.get_errors();
    assert!(errors.is_empty(), "no errors expected, got: {:?}", errors);
}

/// Reactivity survives the fix: updating the SOURCE signal recomputes the
/// derive (the watcher set must contain `recs`, not the phantom `r`).
#[test]
fn derive_with_lambda_recomputes_on_source_update() {
    let compiled = compile_page(
        r#"
@data inline $recs : [{"status":"open"},{"status":"closed"}];
@data derive $openCount number : $recs.filter(($r) => $r.status == "open").length;
"#,
    );
    let mut c = boot(&compiled);
    assert_eq!(num(&mut c, "SpacetimeLocal.openCount"), Some(1.0));

    c.eval(
        r#"
        SpacetimeLocal.recs = [{"status":"open"},{"status":"open"},{"status":"open"}];
        document.dispatchEvent(new CustomEvent('local:recs:updated', { detail: SpacetimeLocal.recs }));
    "#,
    )
    .expect("publish source update");

    assert_eq!(
        num(&mut c, "SpacetimeLocal.openCount"),
        Some(3.0),
        "derive should recompute when its real source updates"
    );
}
