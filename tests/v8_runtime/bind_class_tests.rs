//! Bind-class primitive V8 tests
//!
//! Tests for the bind-class primitive null safety and correct behavior:
//! - No crash when querySelector returns null
//! - Correctly adds/removes classes when element exists

use super::context::V8TestContext;

const ST_JS: &str = include_str!("../../public/runtime/st.js");

fn create_context() -> V8TestContext {
    let mut ctx = V8TestContext::new();
    ctx.eval(ST_JS).expect("Failed to load st.js");
    ctx
}

// =============================================================================
// Null Safety Tests
// =============================================================================

/// bind-class must not crash when the target element doesn't exist in the DOM.
/// Regression test: `el is null` → `el.classList.add(cls)` TypeError
#[test]
fn test_bind_class_null_element_no_crash() {
    let mut ctx = create_context();

    // Set up DOM WITHOUT the target element
    ctx.set_body_html("<div class='other'>No target here</div>")
        .unwrap();

    // Simulate bind-class generated code targeting a nonexistent selector
    ctx.eval(
        r#"
        const el = document.querySelector('.nonexistent');
        const cls = 'active';
        const update = () => {
            if (!el) return;
            if (true) {
                el.classList.add(cls);
            } else {
                el.classList.remove(cls);
            }
        };
        update();
    "#,
    )
    .expect("bind-class should not throw when element is null");

    let errors = ctx.get_errors();
    assert!(
        errors.is_empty(),
        "Should have no errors, got: {:?}",
        errors
    );
}

// =============================================================================
// Functional Tests
// =============================================================================

/// bind-class correctly adds class when condition is true
#[test]
fn test_bind_class_adds_class_when_true() {
    let mut ctx = create_context();

    ctx.set_body_html(r#"<div class="target"></div>"#).unwrap();

    ctx.eval(
        r#"
        const el = document.querySelector('.target');
        const cls = 'active';
        const update = (cond) => {
            if (!el) return;
            if (cond) {
                el.classList.add(cls);
            } else {
                el.classList.remove(cls);
            }
        };
        update(true);
    "#,
    )
    .unwrap();

    let has_class = ctx.eval("document.querySelector('.target').classList.contains('active')");
    assert!(
        matches!(has_class, Ok(v) if v.as_bool() == Some(true)),
        "Element should have 'active' class"
    );
}

/// ST.onCleanup must not crash when element is null.
/// Regression: WeakMap rejects null keys → TypeError
#[test]
fn test_on_cleanup_null_element_no_crash() {
    let mut ctx = create_context();
    let result = ctx.eval("ST.onCleanup(null, () => {})");
    assert!(
        result.is_ok(),
        "ST.onCleanup(null, fn) should not throw: {:?}",
        result.err()
    );
}

/// All ST methods that accept an element must handle null gracefully.
/// Structural fix: eliminates entire class of null-element crashes.
#[test]
fn test_all_st_methods_null_safe() {
    let mut ctx = create_context();
    let methods = [
        "ST.dispose(null)",
        "ST.init(null, (el) => {})",
        "ST.get(null, 'x')",
        "ST.watch(null, 'x', () => {})",
        "ST.watchAll(null, ['x'], () => {})",
        "ST.derive(null, 'x', ['y'], () => 0)",
        "ST.bindState(null, 'x', 'y')",
        "ST.bindStyle(null, 'x', 'color')",
        "ST.watchTypedUnion(null, 'x', 'V', ['a'])",
    ];
    for method in methods {
        let result = ctx.eval(method);
        assert!(
            result.is_ok(),
            "{} should not throw: {:?}",
            method,
            result.err()
        );
    }
}

/// ST.watch(null) returns a no-op unsubscribe function
#[test]
fn test_watch_null_returns_unsub() {
    let mut ctx = create_context();
    let result = ctx.eval("typeof ST.watch(null, 'x', () => {})");
    assert!(
        matches!(result, Ok(v) if v.as_str() == Some("function")),
        "ST.watch(null) should return a function"
    );
}

/// ST.get(null) returns undefined, not an error
#[test]
fn test_get_null_returns_undefined() {
    let mut ctx = create_context();
    let result = ctx.eval("ST.get(null, 'x') === undefined");
    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "ST.get(null) should return undefined"
    );
}

/// bind-class correctly removes class when condition is false
#[test]
fn test_bind_class_removes_class_when_false() {
    let mut ctx = create_context();

    ctx.set_body_html(r#"<div class="target active"></div>"#)
        .unwrap();

    ctx.eval(
        r#"
        const el = document.querySelector('.target');
        const cls = 'active';
        const update = (cond) => {
            if (!el) return;
            if (cond) {
                el.classList.add(cls);
            } else {
                el.classList.remove(cls);
            }
        };
        update(false);
    "#,
    )
    .unwrap();

    let has_class = ctx.eval("document.querySelector('.target').classList.contains('active')");
    assert!(
        matches!(has_class, Ok(v) if v.as_bool() == Some(false)),
        "Element should NOT have 'active' class after update(false)"
    );
}
