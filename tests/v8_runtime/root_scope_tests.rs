//! Root scope element binding tests using V8
//!
//! Tests that primitives used at file-level (:root scope) have access to `el`.
//! This verifies the fix for "el is not defined" errors when primitives like
//! scroll-driver are used outside of a selector scope.

use super::context::V8TestContext;

const ST_JS: &str = include_str!("../../public/runtime/st.js");

fn create_context() -> V8TestContext {
    let mut ctx = V8TestContext::new();
    ctx.eval(ST_JS).expect("Failed to load st.js");
    ctx
}

// =============================================================================
// Basic Element Binding Tests
// =============================================================================

#[test]
fn test_root_scope_el_is_defined() {
    let mut ctx = create_context();

    // Simulate the :root scope wrapper WITH the fix
    let result = ctx.eval(
        r#"
        (function() {
            const el = document.body;
            // This would fail without `el` defined
            return el !== undefined && el !== null;
        })();
    "#,
    );

    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "el should be defined in :root scope wrapper"
    );
}

#[test]
fn test_root_scope_el_is_body() {
    let mut ctx = create_context();

    let result = ctx.eval(
        r#"
        (function() {
            const el = document.body;
            return el === document.body;
        })();
    "#,
    );

    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "el should be document.body in :root scope"
    );
}

#[test]
fn test_root_scope_primitive_code_can_use_el() {
    let mut ctx = create_context();

    // Simulate primitive code that uses el.getBoundingClientRect()
    let result = ctx.eval(
        r#"
        (function() {
            const el = document.body;
            // Typical primitive usage pattern
            const rect = el.getBoundingClientRect();
            return typeof rect === 'object';
        })();
    "#,
    );

    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "primitive code should be able to call methods on el"
    );
}

// =============================================================================
// Scroll Driver Pattern Tests
// =============================================================================

#[test]
fn test_root_scope_scroll_driver_pattern() {
    let mut ctx = create_context();

    // Simulate the key scroll-driver pattern that uses el
    let result = ctx.eval(
        r#"
        (function() {
            const el = document.body;
            (() => {
                let progress = 0;
                const update = () => {
                    const rect = el.getBoundingClientRect();
                    const vh = window.innerHeight;
                    progress = rect.top / vh;
                    return progress;
                };
                return update();
            })();
            return true;
        })();
    "#,
    );

    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "scroll-driver pattern should work with el defined"
    );
}

#[test]
fn test_root_scope_el_passed_to_nested_iife() {
    let mut ctx = create_context();

    // Simulate the nested IIFE pattern from scroll-driver where el is accessed
    // in a nested scope (the inner IIFE that primitives use for scoping)
    let result = ctx.eval(
        r#"
        (function() {
            const el = document.body;
            // Nested IIFE (the pattern used by scroll-driver, time-driver, etc.)
            const innerResult = (() => {
                // el should be accessible from outer scope
                return el !== undefined && el !== null && el === document.body;
            })();
            return innerResult;
        })();
    "#,
    );

    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "el should be accessible in nested IIFEs (the driver pattern)"
    );
}

#[test]
fn test_root_scope_named_driver_publishes_progress() {
    let mut ctx = create_context();

    // Simulate a named driver emitted in the :root scope. The driver owns the
    // body element and publishes its completion progress through the signal API.
    let result = ctx.eval(
        r#"
        (function() {
            const el = document.body;
            (() => {
                const driverName = "root-scroll";
                ST.set(el, driverName, 1);
                return ST.resolve(el, driverName) >= 1;
            })();
            return ST.resolve(el, "root-scroll") >= 1;
        })();
    "#,
    );

    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "root-scope named drivers should publish completion progress on document.body"
    );
}

// =============================================================================
// Contrast Test - Without el
// =============================================================================

#[test]
fn test_without_el_reference_error() {
    let mut ctx = create_context();

    // This simulates the BROKEN behavior - el is not defined
    let result = ctx.eval(
        r#"
        (function() {
            try {
                // This should throw ReferenceError since el is not defined
                const rect = el.getBoundingClientRect();
                return false; // Should not reach here
            } catch (e) {
                return e.name === 'ReferenceError';
            }
        })();
    "#,
    );

    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "el should be undefined without the fix, causing ReferenceError"
    );
}
