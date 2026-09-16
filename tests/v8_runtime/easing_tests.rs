//! Easing Registry Tests (BUG-297)
//!
//! `public/runtime/easing.js` owns `ST.easings` / `ST.registerEasing` /
//! `ST.getEasing`. The compiler emits one `ST.registerEasing(name, curve)`
//! call per `@form easing` declaration into the page bundle.
//!
//! The regression pinned here: the dev server loads SEVERAL bundles that each
//! vendor easing.js and share one `window.ST` (site runtime, then
//! comments/migrations/inspect/host). A late copy that re-assigns `ST.easings`
//! wholesale WIPES the page's custom curves — silently back to linear
//! (observed live 2026-08-05: the site bundle registered the curve, the host
//! bundle then replaced the map, and the element animated linear). easing.js
//! must MERGE: fresh stdlib definitions win for the names they know;
//! page-registered names they don't know survive a reload.

use super::context::V8TestContext;

const ST_JS: &str = include_str!("../../public/runtime/st.js");
const EASING_JS: &str = include_str!("../../public/runtime/easing.js");

fn create_easing_context() -> V8TestContext {
    let mut ctx = V8TestContext::new();
    ctx.eval(ST_JS).expect("st.js loads");
    ctx.eval(EASING_JS).expect("easing.js loads");
    ctx
}

#[test]
fn test_register_easing_makes_a_custom_curve_resolvable() {
    let mut ctx = create_easing_context();
    ctx.eval(
        r#"
        ST.registerEasing('gate-curve', ST.cubicBezier(0.4, 0, 0.2, 1));
        window.__result = {
            mid: ST.getEasing('gate-curve')(0.5),
            kind: typeof ST.easings['gate-curve'],
            fallback: ST.getEasing('never-declared')(0.5)
        };
    "#,
    )
    .expect("registration evaluates");
    let mid = ctx
        .eval("window.__result.mid")
        .expect("mid reads")
        .as_f64().expect("numeric result");
    let kind = ctx.eval("window.__result.kind").expect("kind reads");
    let fallback = ctx
        .eval("window.__result.fallback")
        .expect("fallback reads")
        .as_f64().expect("numeric result");
    assert_eq!(kind.as_str(), Some("function"), "registered curve is stored");
    assert!(
        (mid - 0.776).abs() < 0.01,
        "cubic-bezier(0.4,0,0.2,1) at t=0.5 must be ~0.776, got {mid}"
    );
    assert_eq!(fallback, 0.5, "unknown names still fall back to linear");
}

#[test]
fn test_reloading_easing_js_preserves_page_registrations() {
    let mut ctx = create_easing_context();
    // The page bundle registers its custom curve; then the dev server loads
    // ANOTHER bundle that vendors easing.js (host/comments/inspect/...). The
    // registration must survive the second load, and stdlib definitions must
    // still be the fresh copy.
    ctx.eval(
        r#"
        ST.registerEasing('gate-curve', ST.cubicBezier(0.4, 0, 0.2, 1));
    "#,
    )
    .expect("page registration");
    ctx.eval(EASING_JS).expect("easing.js reloads (second bundle)");
    let mid = ctx
        .eval("ST.getEasing('gate-curve')(0.5)")
        .expect("custom curve survives reload")
        .as_f64().expect("numeric result");
    let linear = ctx
        .eval("ST.getEasing('linear')(0.25)")
        .expect("stdlib curve still present")
        .as_f64().expect("numeric result");
    assert!(
        (mid - 0.776).abs() < 0.01,
        "a page-registered curve must SURVIVE a second easing.js load — got {mid} \
         (0.5 means the late bundle wiped ST.easings, the BUG-297 dev-server regression)"
    );
    assert_eq!(linear, 0.25, "stdlib definitions remain intact after reload");
}

#[test]
fn test_spring_curve_factory_is_registered() {
    let mut ctx = create_easing_context();
    // `spring(...)` is the second `@form easing` body factory the compiler
    // emits; without it a declared spring body would emit a call to an
    // undefined helper (E0952-class runtime break).
    let kind = ctx
        .eval("typeof ST.springCurve")
        .expect("springCurve lookup");
    assert_eq!(
        kind.as_str(),
        Some("function"),
        "ST.springCurve must exist for `@form easing --x {{ spring(...) }}` bodies"
    );
    let val = ctx
        .eval("ST.registerEasing('springy', ST.springCurve(180, 12)) || ST.getEasing('springy')(0)")
        .expect("spring registers")
        .as_f64().expect("numeric result");
    assert!(val.abs() < 1e-9, "a spring curve starts at 0, got {val}");
}
