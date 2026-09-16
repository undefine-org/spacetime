//! PLAN-133 W2.5 — `%uses` cross-primitive prelude dependencies.
//!
//! A primitive declares `%uses a, b` in its header: the named primitives'
//! PRELUDES must reach the page (emitted once, dependency-before-dependent)
//! even when the dependency itself is never invoked. These pin the
//! stdlib-level contract via the locale exemplar (the consumer is a plain
//! `@data fetch`; `locale` is never invoked directly). Unknown targets,
//! cycles, transitive ordering, and the prelude-only pull are covered by
//! `order_with_uses` unit tests in src/pipeline/expand.rs; the runtime
//! behavior is covered by the v8 data-source suites and the BUG-274 gate.

use spacetime::{compile, compiler::CompileOptions, parse};

fn js_for(src: &str) -> String {
    let ast = parse(src).expect("parse");
    compile(&ast, CompileOptions::default()).js
}

/// The exemplar: a page whose only locale need is a data-source consumer
/// still gets the locale prelude — `locale` is never invoked directly.
#[test]
fn uses_pulls_dependency_prelude_without_invocation() {
    let js = js_for("@data fetch $docs : \"/data/{locale}.json\";\n");
    assert!(
        js.contains("__stResolveLocale"),
        "consumer-only page must carry the locale prelude; got:\n{}",
        js
    );
}

/// Two consumers of the same dependency must emit its prelude EXACTLY ONCE
/// (dedup by primitive name, first occurrence wins).
#[test]
fn uses_prelude_emits_once_for_two_consumers() {
    let js = js_for(
        "@data fetch $a : \"/data/{locale}.a.json\";\n@data fetch $b : \"/data/{locale}.b.json\";\n",
    );
    let count = js
        .matches("window.__stResolveLocale = window.__stResolveLocale ||")
        .count();
    assert_eq!(
        count, 1,
        "locale prelude must be emitted exactly once, found {count}; got:\n{}",
        js
    );
}

/// Dependency-before-dependent: the pulled prelude lands BEFORE the
/// dependent's own prelude in the emitted bundle.
#[test]
fn uses_prelude_orders_before_dependent() {
    let js = js_for("@data fetch $docs : \"/data/{locale}.json\";\n");
    let dep = js.find("window.__stResolveLocale =").expect("locale prelude");
    let dependent = js
        .find("window.__spacetimeData = window.__spacetimeData ||")
        .expect("data-source prelude");
    assert!(
        dep < dependent,
        "locale prelude ({dep}) must precede data-source prelude ({dependent})"
    );
}

/// R5: host-register owns the `__stHosts` init; signal-call `%uses` it.
/// A page with BOTH @host and @data signal must emit the init EXACTLY ONCE —
/// before R5 the two primitives each carried a copy and prelude dedup
/// (per primitive name) could not catch the cross-primitive duplication.
#[test]
fn uses_host_init_once_for_host_and_signal() {
    let js = js_for(
        r#"
@host $api : http("https://example.invalid")

@data signal $ping() to $api {
  send GET "/ping"
  receive { 200 => Ok($.body); }
}
"#,
    );
    let count = js.matches("window.__stHosts = window.__stHosts ||").count();
    assert_eq!(
        count, 1,
        "__stHosts init must be emitted exactly once for @host + @data signal, found {count}; got:\n{}",
        js
    );
}

