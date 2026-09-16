//! Timeline Macro Tests using V8
//!
//! Tests that compile .st source containing timeline macros (@on &.load, @on
//! &.scroll, @on &.visible, @on &.hover, @on &.loop — the SIP-001 unified
//! `@on <driver>` head) and verify the generated JS runtime behavior by loading
//! it into V8 with the required runtime modules.

use super::context::V8TestContext;

/// ST runtime source
const ST_JS: &str = include_str!("../../public/runtime/st.js");
/// RAF coordinator runtime source
const RAF_JS: &str = include_str!("../../public/runtime/raf-coordinator.js");
/// Purity runtime source
const PURITY_JS: &str = include_str!("../../public/runtime/purity.js");

fn create_timeline_context() -> V8TestContext {
    let mut ctx = V8TestContext::new();
    ctx.eval(ST_JS).expect("Failed to load st.js");
    ctx.eval(RAF_JS).expect("Failed to load raf-coordinator.js");
    ctx.eval(PURITY_JS).expect("Failed to load purity.js");
    ctx
}

/// Helper: compile .st source and return the generated JS
fn compile_to_js(source: &str) -> String {
    let ast = spacetime::parse(source).expect("Parse should succeed");
    let compiled = spacetime::compile(&ast, spacetime::CompileOptions::default());
    compiled.js
}

// =============================================================================
// @on &.load Macro Tests
// =============================================================================

#[test]
fn test_load_macro_emits_named_driver_publication() {
    let js = compile_to_js(
        r#"
.hero {
    @on &.load(duration: 600ms) as $intro {
        opacity: 0 -> 1;
    }
}
"#,
    );

    assert!(
        js.contains("ST.set") && js.contains("intro"),
        "Compiled named @on &.load driver should publish intro progress via ST.set.\nGenerated JS:\n{}",
        &js[..js.len().min(500)]
    );
}

#[test]
#[ignore = "V8+LinkeDOM lacks full DOM API (Element.animate, getComputedStyle) — verified working in browser"]
fn test_load_macro_compiled_js_loads_in_runtime() {
    let js = compile_to_js(
        r#"
.fade-in {
    @on &.load(name: fade, duration: 800ms) {
        opacity: 0 -> 1;
    }
}
"#,
    );

    let mut ctx = create_timeline_context();
    ctx.set_body_html(r#"<div class="fade-in">Hello</div>"#)
        .unwrap();

    // The compiled JS should load without errors
    let result = ctx.eval(&js);
    assert!(
        result.is_ok(),
        "Compiled @on &.load JS should load without errors: {:?}",
        result.err()
    );
}

// =============================================================================
// @on &.scroll Macro Tests
// =============================================================================

#[test]
fn test_scroll_macro_emits_scroll_listener() {
    let js = compile_to_js(
        r#"
.parallax {
    @on &.scroll(name: reveal, start: 0, end: 1) {
        opacity: 0 -> 1;
    }
}
"#,
    );

    // SIP-001 cutover: `@scroll reveal(start, end)` → `@on &.scroll(name: reveal, start, end)`.
    // The scroll driver registers a real window scroll listener (shared handler table).
    assert!(
        js.contains("window.addEventListener('scroll'"),
        "Compiled @on &.scroll should register a window scroll listener.\nGenerated JS:\n{}",
        &js[..js.len().min(500)]
    );
}

#[test]
fn test_compiled_scroll_multi_property() {
    let js = compile_to_js(
        r#"
.hero {
    @on &.scroll(name: parallax, start: 0, end: 1) {
        opacity: 0 -> 1;
        translate-y: 100px -> 0;
    }
}
"#,
    );

    // SIP-001 cutover: `@scroll parallax(start, end)` → `@on &.scroll(name: parallax, start, end)`.
    // BOTH animated properties must land in the structured `anims.properties` array.
    assert!(
        js.contains("property: \"opacity\"") && js.contains("translate-y"),
        "Compiled multi-property @on &.scroll should carry both animation properties.\nGenerated JS:\n{}",
        &js[..js.len().min(500)]
    );
}

// =============================================================================
// @on &.visible / &.load (retired @on visible) Macro Tests
// =============================================================================

#[test]
fn test_on_visible_emits_intersection_observer() {
    let js = compile_to_js(
        r#"
.card {
    @on &.load(name: reveal, duration: 800ms) {
        opacity: 0 -> 1;
    }
}
"#,
    );

    // SIP-001 cutover: `@on visible reveal(…)` → `@on &.load(name: reveal, …)`. The retired
    // `@on visible` WAS the load driver (never an observer), so the cutover target keeps the
    // load-driver threshold IntersectionObserver entry gate.
    assert!(
        js.contains("IntersectionObserver"),
        "Compiled @on &.load (the @on visible cutover target) should emit IntersectionObserver.\nGenerated JS:\n{}",
        &js[..js.len().min(500)]
    );
}

// =============================================================================
// @on &.hover Macro Tests
// =============================================================================

#[test]
fn test_on_hover_emits_mouseenter_listener() {
    let js = compile_to_js(
        r#"
.button {
    @on &.hover(name: lift, duration: 300ms) {
        scale: 1 -> 1.1;
    }
}
"#,
    );

    // SIP-001 cutover: `@on hover lift(300ms)` → `@on &.hover(name: lift, duration: 300ms)`.
    // The hover driver triggers on the real `mouseenter` event (event-driver row).
    assert!(
        js.contains("mouseenter"),
        "Compiled @on &.hover should emit the mouseenter trigger.\nGenerated JS:\n{}",
        &js[..js.len().min(500)]
    );
}

// =============================================================================
// @on &.loop Macro Tests
// =============================================================================

#[test]
fn test_loop_macro_emits_raf_callback() {
    let js = compile_to_js(
        r#"
.spinner {
    @on &.loop(name: spin, duration: 1s) {
        rotate: 0deg -> 360deg;
    }
}
"#,
    );

    // SIP-001 cutover: `@loop spin(1s)` → `@on &.loop(name: spin, duration: 1s)`.
    // The loop driver advances through the RAF coordinator (ST._raf.add).
    assert!(
        js.contains("ST._raf.add"),
        "Compiled @on &.loop should drive via the RAF coordinator.\nGenerated JS:\n{}",
        &js[..js.len().min(500)]
    );
}

// =============================================================================
// Cleanup Registration Tests
// =============================================================================

#[test]
fn test_compiled_animation_registers_cleanup() {
    let js = compile_to_js(
        r#"
.element {
    @on &.load(name: intro, duration: 500ms) {
        opacity: 0 -> 1;
    }
}
"#,
    );

    // SIP-001 cutover: `@load intro(…)` → `@on &.load(name: intro, …)`. The load driver
    // registers teardown with ST.onCleanup for every element it touches.
    assert!(
        js.contains("ST.onCleanup"),
        "Compiled @on &.load should register cleanup via ST.onCleanup.\nGenerated JS:\n{}",
        &js[..js.len().min(500)]
    );
}

// =============================================================================
// Runtime Integration Tests
// =============================================================================

#[test]
fn test_progress_signal_api_available_in_context() {
    let mut ctx = create_timeline_context();

    let result = ctx.eval(
        "typeof ST.set === 'function' && typeof ST.resolve === 'function' && typeof ST.watch === 'function'",
    );
    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "ST.set, ST.resolve, and ST.watch should be available"
    );
}

#[test]
fn test_raf_coordinator_available_in_context() {
    let mut ctx = create_timeline_context();

    let result = ctx.eval("typeof ST._raf.add === 'function'");
    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "ST._raf.add should be available"
    );
}

#[test]
fn test_named_driver_progress_publish_resolve_and_watch_flow() {
    let mut ctx = create_timeline_context();
    ctx.set_body_html(r#"<div class="test">Test</div>"#)
        .unwrap();

    ctx.eval(
        r#"
        const el = document.querySelector('.test');
        window.__observedProgress = undefined;
        const unwatch = ST.watch(el, 'intro', (progress) => {
            window.__observedProgress = progress;
        });
        ST.set(el, 'intro', 1);
        window.__isComplete = ST.resolve(el, 'intro') >= 1;
        queueMicrotask(unwatch);
    "#,
    )
    .unwrap();

    let result = ctx.eval("window.__isComplete && window.__observedProgress === 1");
    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "Named driver progress should publish through ST.set, resolve at completion, and notify ST.watch"
    );
}

// =============================================================================
// Structured Animation Body Tests
// =============================================================================

#[test]
fn test_on_visible_scoped_animation_produces_structured_anims() {
    let js = compile_to_js(
        r#"
.hero {
    @on &.load(name: h-lines, duration: 2200ms, delay: 100ms) {
        .line--h1 {
            opacity: 0 -> 0.35;
            scale-x: 0 -> 1;
            transform-origin: left;
            easing: &ease-out-expo;
            range: 0 to 0.5;
        }
        .line--h2 {
            opacity: 0 -> 0.55;
            scale-x: 0 -> 1;
            transform-origin: right;
            range: 0.15 to 0.65;
        }
    }
}
"#,
    );
    // SIP-001 cutover: `@on visible h-lines(…)` → `@on &.load(name: h-lines, …)` — the
    // retired `@on visible` was the load driver. The nested `.line--h1`/`.line--h2`
    // selectors must lower into the structured `anims.scopes` array (NOT a raw string).
    assert!(
        !js.contains(r#"const anims = ""#),
        "Animation body should NOT be emitted as a quoted string.\nJS:\n{}",
        &js[..js.len().min(1000)]
    );
    assert!(
        js.contains("scopes"),
        "Animation body should carry nested scopes for the inner selectors.\nJS:\n{}",
        &js[..js.len().min(1000)]
    );
}

#[test]
fn test_load_animation_body_is_structured_object() {
    let js = compile_to_js(
        r#"
.fade {
    @on &.load(name: intro, duration: 600ms) {
        opacity: 0 -> 1;
        translate-y: 20px -> 0;
    }
}
"#,
    );
    // Verify anims is a JS object with properties array
    assert!(
        js.contains("properties"),
        "Animation body should be a structured object with 'properties'.\nJS:\n{}",
        &js[..js.len().min(1000)]
    );
}

// =============================================================================
// Debug Runtime Tests
// =============================================================================

#[test]
fn test_debug_runtime_creates_st_debug_object() {
    let mut ctx = V8TestContext::new();
    let config = spacetime::DebuggerConfig::default();
    let debug_js = spacetime::generate_debug_runtime(&config);
    ctx.eval(&debug_js)
        .expect("Debug runtime should load without errors");

    let result = ctx.eval("typeof window.__ST_DEBUG__").unwrap();
    assert_eq!(
        result.as_str().map(|s| s.to_string()),
        Some("object".to_string()),
        "__ST_DEBUG__ should be an object after loading debug runtime"
    );
}

// =============================================================================
// @on &.load immediate Tests
// =============================================================================

#[test]
fn test_on_visible_immediate_skips_observer() {
    let js = compile_to_js(
        r#"
.hero {
    @on &.load(name: reveal, duration: 800ms, immediate: true) {
        opacity: 0 -> 1;
    }
}
"#,
    );

    // SIP-001 cutover: `@on visible reveal(800ms, immediate: true)` →
    // `@on &.load(name: reveal, duration: 800ms, immediate: true)`. With
    // `immediate: true` the load driver must compile the SKIP-OBSERVER branch
    // (`if (true)` → `queueMicrotask(() => play())`), not the IntersectionObserver
    // entry gate (which is `if (false)` in the default compile).
    assert!(
        js.contains("if (true)") && js.contains("queueMicrotask"),
        "immediate: true should compile the queueMicrotask immediate-play branch.\nGenerated JS:\n{}",
        &js[..js.len().min(1000)]
    );
}

#[test]
fn test_on_visible_default_uses_observer() {
    // PLAN-124 W3.4: the new surface — `@on &.visible` resolves through the
    // driver registry to the intersection primitive (viewport-entry reveals;
    // the retired `@on visible` was the load driver).
    let js = compile_to_js(
        r#"
.hero {
    @on &.visible {
        opacity: 0 -> 1;
    }
}
"#,
    );

    assert!(
        js.contains("IntersectionObserver"),
        "@on &.visible should use IntersectionObserver.\nGenerated JS:\n{}",
        &js[..js.len().min(1000)]
    );
}

#[test]
#[ignore = "V8+LinkeDOM lacks full DOM API (IntersectionObserver) — verified working in browser"]
fn test_on_visible_immediate_js_loads_in_runtime() {
    let js = compile_to_js(
        r#"
.hero {
    @on &.load(name: reveal, duration: 800ms, immediate: true) {
        opacity: 0 -> 1;
    }
}
"#,
    );

    let mut ctx = create_timeline_context();
    ctx.set_body_html(r#"<div class="hero">Hello</div>"#)
        .unwrap();

    let result = ctx.eval(&js);
    assert!(
        result.is_ok(),
        "Compiled @on &.load immediate JS should load without errors: {:?}",
        result.err()
    );
}

#[test]
fn test_template_def_and_invoke_no_e0402() {
    let source = r#"
@template &card($title) {
    <div class="card">`$title`</div>
}

.section {
    &card("Hello") {}
}
"#;
    let ast = spacetime::parse(source).expect("Parse should succeed");
    let compiled = spacetime::compile(&ast, spacetime::CompileOptions::default());
    let e0402s: Vec<_> = compiled
        .pipeline_errors
        .iter()
        .filter(|e| e.code == "E0402")
        .collect();
    assert!(
        e0402s.is_empty(),
        "Template def + invoke should produce no E0402, got: {:?}",
        e0402s.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}
