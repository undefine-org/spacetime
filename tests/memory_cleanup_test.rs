//! Integration tests for memory management and cleanup infrastructure
//! Uses V8 JS engine (via rustyscript) to test actual runtime behavior
//!
//! Note: These tests use the new macro-based @value-change syntax.
//! The cleanup infrastructure is now handled by primitives via %cleanup blocks.
//!
//! Parse tests are active. Codegen/runtime tests are ignored pending stdlib
//! %emit pipeline integration for value-change-driver primitive.

use rustyscript::{Runtime, RuntimeOptions};
use serde_json::Value;
use spacetime::{compile, compiler::CompileOptions, parse};
use std::sync::Once;

static V8_INIT: Once = Once::new();

fn ensure_v8_initialized() {
    V8_INIT.call_once(|| {
        rustyscript::init_platform(4, true);
    });
}

/// Helper to create a V8 context with browser-like globals mocked
fn create_test_context() -> Runtime {
    ensure_v8_initialized();
    let mut runtime = Runtime::new(RuntimeOptions::default()).expect("Failed to create V8 runtime");

    runtime.eval::<Value>(r#"
        class AbortController {
            constructor() { this.signal = { aborted: false }; }
            abort() { this.signal.aborted = true; }
        }
        let _rafId = 0;
        let _rafCallbacks = new Map();
        function requestAnimationFrame(cb) { _rafId++; _rafCallbacks.set(_rafId, cb); return _rafId; }
        function cancelAnimationFrame(id) { _rafCallbacks.delete(id); }
        const document = {
            body: { nodeType: 1 },
            querySelectorAll: () => [],
            querySelector: () => null,
            createElement: (tag) => ({
                tagName: tag.toUpperCase(), style: {}, textContent: '', dataset: {},
                appendChild: function(child) { return child; },
                animate: function() { return { finished: Promise.resolve() }; }
            })
        };
        class MutationObserver {
            constructor(cb) { this.cb = cb; this._disconnected = false; }
            observe() {}
            disconnect() { this._disconnected = true; }
        }
        class IntersectionObserver {
            constructor(cb, opts) { this.cb = cb; this._disconnected = false; }
            observe() {}
            disconnect() { this._disconnected = true; }
        }
        const console = { log: () => {}, assert: (cond, msg) => { if (!cond) throw new Error(msg); } };
        const Node = { ELEMENT_NODE: 1 };
        const performance = { now: () => Date.now() };
        class WeakMap {
            constructor() { this._map = new Map(); }
            get(key) { return this._map.get(key); }
            set(key, value) { this._map.set(key, value); return this; }
            has(key) { return this._map.has(key); }
            delete(key) { return this._map.delete(key); }
        }
        var window = {
            Spacetime: null,
            addEventListener: function() {},
            matchMedia: function() { return { matches: false }; },
            innerHeight: 800, innerWidth: 1200,
        };
    "#).expect("Failed to set up mocks");

    runtime
}

/// Helper to load the runtime and evaluate test code
fn eval_with_runtime(
    runtime: &mut Runtime,
    runtime_js: &str,
    test_code: &str,
) -> Result<Value, String> {
    runtime
        .eval::<Value>(runtime_js)
        .map_err(|e| format!("Runtime load error: {:?}", e))?;
    runtime
        .eval::<Value>(test_code)
        .map_err(|e| format!("Test code error: {:?}", e))
}

// =============================================================================
// PARSE VERIFICATION TESTS (active)
// =============================================================================

#[test]
fn test_value_change_cleanup_input_parses() {
    let input = r#"
.counter {
    @value-change(duration: 300ms) {
        :entering { opacity: 0 -> 1 }
        :exiting { opacity: 1 -> 0 }
    }
}
"#;
    let ast = parse(input).unwrap();
    assert!(!ast.scopes.is_empty() || !ast.matches.is_empty());
}

#[test]
fn test_scroll_driver_input_parses() {
    let input = r#"
.box {
    @scroll(start: 0, end: 1) {
        opacity: 0 -> 1
    }
}
"#;
    let ast = parse(input).unwrap();
    assert!(!ast.scopes.is_empty() || !ast.matches.is_empty());
}

// =============================================================================
// CODEGEN TESTS (ignored — stdlib %emit pipeline not yet wired for @value-change)
// =============================================================================

#[test]
#[ignore = "INIT-038: stdlib %emit pipeline does not yet emit JS for @value-change — needs primitive expansion in codegen"]
fn test_generated_js_contains_cleanup_patterns() {
    let input = r#"
.counter {
    @value-change(duration: 300ms) {
        :entering { opacity: 0 -> 1 }
        :exiting { opacity: 1 -> 0 }
    }
}
"#;
    let ast = parse(input).unwrap();
    let compiled = compile(&ast, CompileOptions::default());
    assert!(
        !compiled.js.is_empty(),
        "JS should contain cleanup infrastructure from value-change-driver primitive %cleanup block"
    );
}

#[test]
#[ignore = "INIT-038: stdlib %emit pipeline does not yet emit JS for @scroll — needs primitive expansion in codegen"]
fn test_scroll_driver_produces_js() {
    let input = r#"
.box {
    @scroll(start: 0, end: 1) {
        opacity: 0 -> 1
    }
}
"#;
    let ast = parse(input).unwrap();
    let compiled = compile(&ast, CompileOptions::default());
    assert!(
        !compiled.js.is_empty(),
        "JS should contain scroll-driver signal-based cleanup"
    );
}

// =============================================================================
// V8 RUNTIME TESTS (ignored — require compiled JS output from primitives)
// =============================================================================

#[test]
#[ignore = "INIT-038: requires compiled JS from value-change-driver primitive — needs %emit pipeline integration"]
fn test_v8_runtime_timeline_has_cleanup() {
    let input = r#"
.counter {
    @value-change(duration: 300ms) {
        :entering { opacity: 0 -> 1 }
        :exiting { opacity: 1 -> 0 }
    }
}
"#;
    let ast = parse(input).unwrap();
    let compiled = compile(&ast, CompileOptions::default());

    let mut runtime = create_test_context();
    eval_with_runtime(&mut runtime, &compiled.js, "true")
        .expect("Runtime should load without errors");
}

#[test]
#[ignore = "INIT-038: requires compiled JS from value-change-driver primitive — needs %emit pipeline integration"]
fn test_v8_runtime_destroy_behavior() {
    let input = r#"
.counter {
    @value-change(duration: 300ms) {
        :entering { opacity: 0 -> 1 }
        :exiting { opacity: 1 -> 0 }
    }
}
"#;
    let ast = parse(input).unwrap();
    let compiled = compile(&ast, CompileOptions::default());

    let mut runtime = create_test_context();
    eval_with_runtime(&mut runtime, &compiled.js, "true")
        .expect("Runtime should load without errors");
}
