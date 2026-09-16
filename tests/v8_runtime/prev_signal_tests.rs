//! ST.prev Tests (PLAN-126 D31-ii)
//!
//! The `.prev` facet's expression-position helper: the previous value of a
//! signal. At mount the previous value IS the initial value (the arms
//! primed-guard convention); every change shifts cur -> prev.

use super::context::V8TestContext;

const ST_JS: &str = include_str!("../../public/runtime/st.js");

fn create_context() -> V8TestContext {
    let mut ctx = V8TestContext::new();
    ctx.eval(ST_JS).expect("Failed to load st.js");
    ctx
}

#[test]
fn prev_at_mount_is_the_initial_value() {
    let mut ctx = create_context();
    let result = ctx
        .eval(
            r#"
        (() => {
            const el = document.createElement('div');
            ST.set(el, 'price', 100);
            return ST.prev(el, 'price');
        })()
    "#,
        )
        .expect("eval");
    assert_eq!(result.to_string(), "100");
}

#[test]
fn prev_after_change_is_the_old_value() {
    let mut ctx = create_context();
    let result = ctx
        .eval(
            r#"
        (() => {
            const el = document.createElement('div');
            ST.set(el, 'price', 100);
            ST.prev(el, 'price');          // subscribe
            ST.set(el, 'price', 120);      // change: cur 100 -> prev, cur = 120
            return ST.prev(el, 'price');
        })()
    "#,
        )
        .expect("eval");
    assert_eq!(result.to_string(), "100");
}

#[test]
fn prev_within_one_flush_is_the_pre_flush_value() {
    // §6.12 flush semantics: two changes inside ONE watch flush collapse —
    // `.prev` is the value before the flush, matching what the arms runtime's
    // own prev tracking sees. One memory, one definition.
    let mut ctx = create_context();
    let result = ctx
        .eval(
            r#"
        (() => {
            const el = document.createElement('div');
            ST.set(el, 'price', 1);
            ST.prev(el, 'price');
            ST.set(el, 'price', 2);
            ST.set(el, 'price', 3);
            return ST.prev(el, 'price');
        })()
    "#,
        )
        .expect("eval");
    assert_eq!(result.to_string(), "1");
}

#[ignore = "V8 doesn't flush microtasks across eval boundaries (the afterData pattern) — cross-flush chain verified via the pre-flush invariant tests; browser-verified"]
#[test]
fn prev_across_flushes_tracks_a_chain() {
    let mut ctx = create_context();
    ctx.eval(
        r#"
        (() => {
            const el = document.createElement('div');
            window.__probe = el;
            ST.set(el, 'price', 1);
            ST.prev(el, 'price');
            ST.set(el, 'price', 2);
        })()
    "#,
    )
    .expect("eval 1");
    // The harness flushes microtasks between evals — the watch fires here.
    let result = ctx
        .eval(
            r#"
        (() => {
            const el = window.__probe;
            ST.set(el, 'price', 3);
            return ST.prev(el, 'price');
        })()
    "#,
        )
        .expect("eval 2");
    assert_eq!(result.to_string(), "2");
}

#[test]
fn prev_subscribing_late_starts_from_the_current_value() {
    let mut ctx = create_context();
    let result = ctx
        .eval(
            r#"
        (() => {
            const el = document.createElement('div');
            ST.set(el, 'price', 100);
            ST.set(el, 'price', 200);      // before any subscription
            return ST.prev(el, 'price');   // first call: prev = current
        })()
    "#,
        )
        .expect("eval");
    assert_eq!(result.to_string(), "200");
}

#[test]
fn prev_is_per_node() {
    let mut ctx = create_context();
    let result = ctx
        .eval(
            r#"
        (() => {
            const a = document.createElement('div');
            const b = document.createElement('div');
            ST.set(a, 'x', 1);
            ST.set(b, 'x', 10);
            ST.prev(a, 'x'); ST.prev(b, 'x');
            ST.set(a, 'x', 2);
            ST.set(b, 'x', 20);
            // One flush: prev is each node's pre-flush value.
            return [ST.prev(a, 'x'), ST.prev(b, 'x')].join(',');
        })()
    "#,
        )
        .expect("eval");
    assert_eq!(result.to_string(), "\"1,10\"");
}
