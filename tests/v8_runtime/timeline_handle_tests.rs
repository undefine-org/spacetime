//! Named Animation Progress Signal Tests
//!
//! Animation drivers publish progress directly on their owning element. Consumers
//! resolve the named signal or subscribe with `ST.watch`; progress at or above one
//! represents completion.

use super::context::V8TestContext;

const ST_JS: &str = include_str!("../../public/runtime/st.js");

fn create_timeline_context() -> V8TestContext {
    let mut ctx = V8TestContext::new();
    ctx.eval(ST_JS).expect("Failed to load st.js");
    ctx
}

#[test]
fn test_named_progress_is_absent_until_published() {
    let mut ctx = create_timeline_context();
    ctx.set_body_html(r#"<div class="test">Test</div>"#)
        .unwrap();

    ctx.eval(
        r#"
        const el = document.querySelector('.test');
        window.__result = ST.resolve(el, 'progress-test') === undefined;
    "#,
    )
    .unwrap();

    let result = ctx.eval("window.__result");
    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "an unpublished named progress signal should resolve to undefined"
    );
}

#[test]
fn test_named_progress_resolves_from_its_owner() {
    let mut ctx = create_timeline_context();
    ctx.set_body_html(r#"<div class="test">Test</div>"#)
        .unwrap();

    ctx.eval(
        r#"
        const el = document.querySelector('.test');
        ST.set(el, 'progress-test', 0.5);
        window.__result = ST.resolve(el, 'progress-test');
    "#,
    )
    .unwrap();

    let result = ctx.eval("window.__result");
    assert!(
        matches!(result, Ok(v) if v.as_f64() == Some(0.5)),
        "ST.set should publish progress resolved from its owning element"
    );
}

#[test]
fn test_named_progress_watch_receives_published_value() {
    let mut ctx = create_timeline_context();
    ctx.set_body_html(r#"<div class="test">Test</div>"#)
        .unwrap();

    ctx.eval(
        r#"
        const el = document.querySelector('.test');
        ST.set(el, 'progress-test', 0.75);
        ST.watch(el, 'progress-test', progress => { window.__watched = progress; });
    "#,
    )
    .unwrap();

    let result = ctx.eval("window.__watched");
    assert!(
        matches!(result, Ok(v) if v.as_f64() == Some(0.75)),
        "ST.watch should receive an already-published named progress value"
    );
}

#[test]
fn test_named_progress_at_or_above_one_is_complete() {
    let mut ctx = create_timeline_context();
    ctx.set_body_html(r#"<div class="test">Test</div>"#)
        .unwrap();

    ctx.eval(
        r#"
        const el = document.querySelector('.test');
        ST.set(el, 'progress-test', 1.5);
        window.__result = ST.resolve(el, 'progress-test');
    "#,
    )
    .unwrap();

    let result = ctx.eval("window.__result");
    assert!(
        matches!(result, Ok(v) if v.as_f64().is_some_and(|progress| progress >= 1.0)),
        "progress at or above one should represent completion"
    );
}
