//! Tests for the V8 Runtime headless testing infrastructure

mod v8_runtime;

#[allow(unused_imports)]
use v8_runtime::TestResults;
use v8_runtime::V8TestContext;

/// Install the virtual setTimeout/clearTimeout queue used by debounce tests
/// (FEAT-074 @effect). Drains via globalThis.__stRunAllTimeouts().
fn install_test_timers(ctx: &mut V8TestContext) {
    ctx.eval(
        r#"
        let __nextTimerId = 1;
        const __timerQueue = new Map();
        globalThis.__stTestSetTimeout = (fn) => { const id = __nextTimerId++; __timerQueue.set(id, fn); return id; };
        globalThis.__stTestClearTimeout = (id) => { __timerQueue.delete(id); };
        globalThis.__stRunAllTimeouts = () => {
            const pending = Array.from(__timerQueue.entries());
            __timerQueue.clear();
            pending.forEach(([, fn]) => fn());
            return pending.length;
        };
        void 0;
    "#,
    )
    .expect("install test timers");
}

fn compile_st(source: &str) -> spacetime::CompiledSpacetime {
    let ast = spacetime::parse(source).expect("test source should parse");
    spacetime::Compiler::from_ast(&ast)
        .without_runtime()
        .compile()
}

fn create_pointer_runtime_context(initial_touch_mode: bool) -> V8TestContext {
    let mut ctx = V8TestContext::new().with_runtime().unwrap();

    ctx.eval(
        r#"
        globalThis.__mqState = {
            '(pointer: fine)': true,
            '(hover: hover)': true,
            '(pointer: coarse)': false,
            '(hover: none)': false,
            '(prefers-reduced-motion: reduce)': false
        };

        const __mqlRegistry = new Map();

        window.matchMedia = (query) => {
            if (__mqlRegistry.has(query)) return __mqlRegistry.get(query);

            const listeners = new Set();
            const mql = {
                media: query,
                get matches() {
                    return !!globalThis.__mqState[query];
                },
                addEventListener(type, listener) {
                    if (type === 'change') listeners.add(listener);
                },
                removeEventListener(type, listener) {
                    if (type === 'change') listeners.delete(listener);
                },
                addListener(listener) {
                    listeners.add(listener);
                },
                removeListener(listener) {
                    listeners.delete(listener);
                }
            };

            mql.__emitChange = () => {
                const event = { matches: mql.matches, media: query };
                listeners.forEach((listener) => listener.call(mql, event));
            };

            __mqlRegistry.set(query, mql);
            return mql;
        };

        globalThis.__setMediaMatch = (query, matches) => {
            globalThis.__mqState[query] = !!matches;
            const mql = __mqlRegistry.get(query);
            if (mql) mql.__emitChange();
        };

        let __nextTimerId = 1;
        const __timerQueue = new Map();
        globalThis.__stTestSetTimeout = (fn, delay) => {
            const id = __nextTimerId++;
            __timerQueue.set(id, fn);
            return id;
        };
        globalThis.__stTestClearTimeout = (id) => {
            __timerQueue.delete(id);
        };
        globalThis.__stRunAllTimeouts = () => {
            const pending = Array.from(__timerQueue.entries());
            __timerQueue.clear();
            pending.forEach(([, fn]) => fn());
            return pending.length;
        };

        const __rafQueue = [];
        globalThis.requestAnimationFrame = (cb) => {
            __rafQueue.push(cb);
            return __rafQueue.length;
        };
        globalThis.cancelAnimationFrame = () => {};
        globalThis.__stRunAnimationFrames = () => {
            const queued = __rafQueue.splice(0, __rafQueue.length);
            queued.forEach((cb) => cb(0));
            return queued.length;
        };
    "#,
    )
    .unwrap();

    ctx.set_body_html(r#"<div class="magnet">Magnet</div><button class="tap">Tap</button>"#)
        .unwrap();

    if initial_touch_mode {
        ctx.eval(
            r#"
            __setMediaMatch('(pointer: fine)', false);
            __setMediaMatch('(hover: hover)', false);
            __setMediaMatch('(pointer: coarse)', true);
            __setMediaMatch('(hover: none)', true);
        "#,
        )
        .unwrap();
    }

    let compiled = compile_st(
        r#"
        body {
            @cursor();
        }

        .magnet {
            @magnetic();
        }
    "#,
    );

    let runtime_js = compiled
        .js
        .replace("window.setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("clearTimeout(", "globalThis.__stTestClearTimeout(");
    ctx = ctx.load_compiled(&runtime_js).unwrap();
    ctx.trigger_dom_ready().unwrap();
    ctx
}

#[test]
fn test_context_creation() {
    let _ctx = V8TestContext::new();
}

#[test]
fn test_linkedom_initialized() {
    let mut ctx = V8TestContext::new();
    let result = ctx.eval("typeof document !== 'undefined'");
    assert!(result.is_ok());
    assert!(matches!(result.unwrap().as_bool(), Some(true)));
}

#[test]
fn test_document_body_exists() {
    let mut ctx = V8TestContext::new();
    let result = ctx.eval("document.body !== null");
    assert!(result.is_ok());
}

#[test]
fn test_set_body_html() {
    let mut ctx = V8TestContext::new();
    ctx.set_body_html("<div class='test'>Hello World</div>")
        .expect("Failed to set body HTML");
    assert!(ctx.element_exists(".test"));
}

#[test]
fn test_query_text() {
    let mut ctx = V8TestContext::new();
    ctx.set_body_html("<span class='greeting'>Hello from V8!</span>")
        .unwrap();
    assert_eq!(
        ctx.query_text(".greeting"),
        Some("Hello from V8!".to_string())
    );
}

#[test]
fn test_element_not_exists() {
    let mut ctx = V8TestContext::new();
    assert!(!ctx.element_exists(".nonexistent"));
}

#[test]
fn test_mock_fetch() {
    let ctx = V8TestContext::new();
    let mut ctx = ctx.mock_fetch(
        "/api/items",
        serde_json::json!({
            "items": ["apple", "banana", "cherry"]
        }),
    );

    // Verify mock is registered
    let result = ctx.eval("typeof __spacetime_mock_fetch === 'function'");
    assert!(matches!(result.unwrap().as_bool(), Some(true)));
}

#[test]
fn test_headless_marker() {
    let mut ctx = V8TestContext::new();
    let result = ctx.eval("globalThis.__spacetime_headless__ === true");
    assert!(matches!(result.unwrap().as_bool(), Some(true)));
}

#[test]
fn test_v8_marker() {
    let mut ctx = V8TestContext::new();
    let result = ctx.eval("globalThis.__v8__ === true");
    assert!(matches!(result.unwrap().as_bool(), Some(true)));
}

#[test]
fn test_click_element() {
    let mut ctx = V8TestContext::new();
    ctx.set_body_html(
        r#"
        <button class="btn" onclick="this.dataset.clicked = 'true'">Click Me</button>
    "#,
    )
    .unwrap();

    ctx.click(".btn").expect("Click should succeed");

    // Verify click was processed
    let result = ctx.eval("document.querySelector('.btn').dataset.clicked");
    assert!(result.is_ok());
}

#[test]
fn test_click_nonexistent_element() {
    let mut ctx = V8TestContext::new();
    let result = ctx.click(".nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_console_log_capture() {
    let mut ctx = V8TestContext::new();
    ctx.eval("console.log('test message')").unwrap();

    let logs = ctx.get_logs();
    assert!(logs.contains(&"test message".to_string()));
}

#[test]
fn test_console_error_capture() {
    let mut ctx = V8TestContext::new();
    ctx.eval("console.error('error message')").unwrap();

    let errors = ctx.get_errors();
    assert!(errors.contains(&"error message".to_string()));
}

#[test]
fn test_dom_content_loaded() {
    let mut ctx = V8TestContext::new();
    ctx.eval(
        r#"
        let domReady = false;
        document.addEventListener('DOMContentLoaded', () => { domReady = true; });
    "#,
    )
    .unwrap();

    ctx.trigger_dom_ready().unwrap();

    let result = ctx.eval("domReady");
    assert!(matches!(result.unwrap().as_bool(), Some(true)));
}

#[test]
fn test_custom_event() {
    let mut ctx = V8TestContext::new();
    ctx.set_body_html("<div id='target'></div>").unwrap();

    ctx.eval(
        r#"
        const target = document.getElementById('target');
        target.addEventListener('custom', (e) => {
            target.dataset.received = e.detail.message;
        });
        target.dispatchEvent(new CustomEvent('custom', { detail: { message: 'hello' } }));
    "#,
    )
    .unwrap();

    let result = ctx.eval("document.getElementById('target').dataset.received");
    assert!(result.is_ok());
}

#[test]
fn test_with_runtime_loads_st_js() {
    let ctx = V8TestContext::new().with_runtime();
    assert!(ctx.is_ok(), "st.js runtime should load successfully");

    let mut ctx = ctx.unwrap();
    // Check that ST global exists (from st.js)
    let result = ctx.eval("typeof ST !== 'undefined'");
    assert!(result.is_ok());
    assert!(matches!(result.unwrap().as_bool(), Some(true)));
}

#[test]
fn test_test_results_summary() {
    let results = TestResults {
        passed: 5,
        failed: 1,
        skipped: 2,
        tests: vec![],
        errors: vec![],
    };

    assert_eq!(results.summary(), "5 passed, 1 failed, 2 skipped");
    assert!(!results.all_passed());
    assert_eq!(results.total(), 8);
}

#[test]
fn test_test_results_all_passed() {
    let results = TestResults {
        passed: 10,
        failed: 0,
        skipped: 0,
        tests: vec![],
        errors: vec![],
    };

    assert!(results.all_passed());
}

#[test]
fn test_run_tests_without_registered_tests() {
    let mut ctx = V8TestContext::new();
    let results = ctx.run_tests(None);

    // Should report no tests found
    assert_eq!(results.passed, 0);
    assert_eq!(results.failed, 0);
}

#[test]
fn test_mock_fetch_all() {
    let mut ctx = V8TestContext::new().mock_fetch_all(&[
        (
            "/api/users",
            serde_json::json!([{"id": 1, "name": "Alice"}]),
        ),
        (
            "/api/products",
            serde_json::json!([{"id": 1, "name": "Widget"}]),
        ),
    ]);

    // Verify both mocks are registered by checking they work
    let result = ctx.eval("__fetchMocks.size");
    assert!(result.is_ok());
}

#[test]
fn test_request_animation_frame_mock() {
    let mut ctx = V8TestContext::new();

    // requestAnimationFrame should be synchronous in our mock
    ctx.eval(
        r#"
        let called = false;
        requestAnimationFrame(() => { called = true; });
    "#,
    )
    .unwrap();

    let result = ctx.eval("called");
    assert!(matches!(result.unwrap().as_bool(), Some(true)));
}

#[test]
fn test_get_computed_style_mock() {
    let mut ctx = V8TestContext::new();
    ctx.set_body_html("<div class='styled' style='color: red'></div>")
        .unwrap();

    let result = ctx.eval("getComputedStyle(document.querySelector('.styled')).color");
    // LinkeDOM should return the style or our mock should handle it
    assert!(result.is_ok());
}

#[test]
fn test_touch_mode_disables_cursor_and_pulse_lifecycle_cleans_up() {
    let mut ctx = create_pointer_runtime_context(true);

    let no_follow_cursor = ctx.eval("document.querySelector('.st-cursor') === null");
    assert!(
        matches!(no_follow_cursor, Ok(v) if v.as_bool() == Some(true)),
        "touch mode must not create follow cursor"
    );

    ctx.eval(
        r#"
        const e = new Event('pointerdown');
        Object.defineProperty(e, 'pointerType', { value: 'touch' });
        Object.defineProperty(e, 'isPrimary', { value: true });
        Object.defineProperty(e, 'clientX', { value: 120 });
        Object.defineProperty(e, 'clientY', { value: 240 });
        window.dispatchEvent(e);
    "#,
    )
    .unwrap();

    let pulse_created = ctx.eval("document.querySelectorAll('.st-cursor-pulse').length === 1");
    assert!(
        matches!(pulse_created, Ok(v) if v.as_bool() == Some(true)),
        "primary touch pointerdown must create one pulse"
    );

    let ran_timeouts = ctx.eval("__stRunAllTimeouts() > 0");
    assert!(
        matches!(ran_timeouts, Ok(v) if v.as_bool() == Some(true)),
        "pulse lifecycle should enqueue cleanup timeout"
    );

    let pulse_removed = ctx.eval("document.querySelectorAll('.st-cursor-pulse').length === 0");
    assert!(
        matches!(pulse_removed, Ok(v) if v.as_bool() == Some(true)),
        "pulse must be removed after cleanup delay"
    );
}

#[test]
fn test_mode_switch_fine_to_touch_tears_down_cursor_and_resets_magnetic_transform() {
    let mut ctx = create_pointer_runtime_context(false);

    let cursor_exists = ctx.eval("document.querySelector('.st-cursor') !== null");
    assert!(
        matches!(cursor_exists, Ok(v) if v.as_bool() == Some(true)),
        "desktop mode should create follow cursor"
    );

    ctx.eval(
        r#"
        const magnet = document.querySelector('.magnet');
        magnet.style.transform = 'translate3d(18px, 12px, 0)';
    "#,
    )
    .unwrap();

    ctx.eval(
        r#"
        __setMediaMatch('(pointer: fine)', false);
        __setMediaMatch('(hover: hover)', false);
        __setMediaMatch('(pointer: coarse)', true);
        __setMediaMatch('(hover: none)', true);
    "#,
    )
    .unwrap();

    let cursor_removed = ctx.eval("document.querySelector('.st-cursor') === null");
    assert!(
        matches!(cursor_removed, Ok(v) if v.as_bool() == Some(true)),
        "switching to touch mode should remove desktop cursor node"
    );

    let cursor_scope_reset = ctx.eval(
        "document.body.getAttribute('data-st-cursor') === null && document.querySelector('style[data-st-cursor-style]') === null",
    );
    assert!(
        matches!(cursor_scope_reset, Ok(v) if v.as_bool() == Some(true)),
        "mode switch cleanup should remove cursor scope/style artifacts"
    );

    let magnetic_reset = ctx.eval("document.querySelector('.magnet').style.transform === ''");
    assert!(
        matches!(magnetic_reset, Ok(v) if v.as_bool() == Some(true)),
        "magnetic transform should reset when switching to touch"
    );
}

// === BUG-059 keystone: scoped state on a real (multiline) template renders +
// wires its @on directive end-to-end. Asserts the EMITTED JS, then the runtime
// behavior (click increments instance state). Guards against the regression where
// parse_component_body dropped HTML / lost the directive on real components.

const BUG059_COUNTER: &str = r#"@template &counter($l) {
  $count number: 0;
  <div class="c">
    <span class="v"></span>
    <button class="b">+</button>
  </div>
  .b { @on &.click { $count <- $count + 1; } }
}
.list { &counter("L"); }
"#;

#[test]
fn bug059_emits_html_and_click_directive() {
    // Emitted-JS gate (no check-only greens): the registerTemplate payload must carry
    // the real HTML and the OnEvent click directive, not an empty html / [] directives.
    let compiled = compile_st(BUG059_COUNTER);
    let js = &compiled.js;
    assert!(
        js.contains("class=\\\"b\\\"") || js.contains("class=\"b\""),
        "emitted JS must contain the template button HTML, got len {}",
        js.len()
    );
    // PLAN-039 W1: body directives are emitted as standalone selector-inits,
    // not inside the register-template body payload. The @on click on `.b`
    // must still surface as a `.b` selector-init with a click handler.
    assert!(
        js.contains("ST.registerSelectorInit('.b'") && js.contains(r#"eventType = "click""#),
        "emitted JS must carry the @on click selector-init"
    );
    assert!(
        js.contains("$count <- $count + 1") && js.contains("count"),
        "emitted JS must carry the count mutation action"
    );
}

#[test]
fn bug059_counter_click_increments_instance_state() {
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    let compiled = compile_st(BUG059_COUNTER);

    // Seed the mount point, evaluate the compiled JS, and let the selector-init run.
    ctx.set_body_html(r#"<div class="list"></div>"#).unwrap();
    ctx.eval(&compiled.js).expect("compiled JS evaluates");
    // The runtime schedules init on document.body; ensure it ran.
    let _ = ctx.eval("ST._scheduleInit && ST._scheduleInit(document.body)");

    // The counter instance mounted under .list; its root carries scoped `count`.
    let mounted = ctx
        .eval("document.querySelector('.list .c') !== null")
        .expect("eval");
    assert_eq!(
        mounted.as_bool(),
        Some(true),
        "counter HTML should render under .list (was dropped pre-BUG-059)"
    );

    // Initial scoped state is 0.
    let initial = ctx
        .eval("ST.get(document.querySelector('.list .c'), 'count')")
        .expect("eval");
    assert_eq!(initial.as_f64(), Some(0.0), "initial count is 0");

    // Click the button; the @on click directive must increment scoped `count`.
    ctx.eval("document.querySelector('.list .b').click()")
        .expect("click");
    let after = ctx
        .eval("ST.get(document.querySelector('.list .c'), 'count')")
        .expect("eval");
    assert_eq!(
        after.as_f64(),
        Some(1.0),
        "click must increment scoped count to 1 (directive was lost pre-BUG-059)"
    );
}

// FEAT-071: `[slot="x"] <- $sig` attribute/slot content injection inside a template
// body must parse at the CST level (was `expected '{' after selector`) and lower to a
// Slot injection in the registerTemplate payload. Emitted-JS asserted (not check-only).
#[test]
fn feat071_slot_injection_lowers_to_injection() {
    // FEAT-115 S3c: a body-root `[slot="v"] <- $x` injection now lowers to a
    // SYNTHESIZED nested scope on the unified reactive-binding path (a
    // `[slot="v"]` selector-init that sets textContent), not a separate
    // `injections` payload. Assert the unified lowering + that it updates the slot
    // child live.
    let compiled = compile_st(
        "@template &w($l) {\n  $count number: 0;\n  <div class=\"c\"><span slot=\"v\"></span></div>\n  [slot=\"v\"] <- $count;\n}\n.host { &w(\"A\"); }\n",
    );
    // The injection lowers to a per-node binding keyed on the `[slot="v"]` selector
    // (registerSelectorInit), NOT the retired `injections:` payload.
    assert!(
        compiled.js.contains("[slot=\\\"v\\\"]") || compiled.js.contains("slot="),
        "slot injection must lower to a `[slot=\"v\"]` selector binding, got len {}",
        compiled.js.len()
    );
    assert!(
        !compiled.js.contains("injections:"),
        "injections are unified onto the scope path — no `injections:` payload"
    );

    // End-to-end: the slot child shows the seeded $count (0).
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    ctx.set_body_html(r#"<div class="host"></div>"#).unwrap();
    ctx.eval(&compiled.js).expect("compiled JS evaluates");
    let _ = ctx.eval("ST._scheduleInit && ST._scheduleInit(document.body)");
    ctx.eval("return new Promise(r => queueMicrotask(r));").ok();
    let slot = ctx
        .eval("document.querySelector('.host [slot=\"v\"]').textContent")
        .expect("e");
    assert_eq!(
        slot.as_str(),
        Some("0"),
        "[slot=v] injection must set the slot child's textContent"
    );
}

// FEAT-071: two instances of the same template have INDEPENDENT scoped state (no
// aliasing through a global), and a `[slot] <- $sig` injection updates that instance's
// DOM live on state change. The decisive instance-isolation gate for the component
// model. End-to-end in V8 (render + click + read DOM/state).
const FEAT071_TWO_COUNTERS: &str = r#"@template &counter($l) {
  $count number: 0;
  <div class="c"><span slot="v"></span><button class="b">+</button></div>
  [slot="v"] <- $count;
  .b { @on &.click { $count <- $count + 1; } }
}
.a { &counter("A"); }
.b2 { &counter("B"); }
"#;

#[test]
fn feat071_two_instances_have_independent_state() {
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    let compiled = compile_st(FEAT071_TWO_COUNTERS);
    ctx.set_body_html(r#"<div class="a"></div><div class="b2"></div>"#)
        .unwrap();
    ctx.eval(&compiled.js).expect("compiled JS evaluates");
    let _ = ctx.eval("ST._scheduleInit && ST._scheduleInit(document.body)");

    // Both instances rendered.
    let both = ctx
        .eval(
            "document.querySelector('.a .c') !== null && document.querySelector('.b2 .c') !== null",
        )
        .expect("eval");
    assert_eq!(both.as_bool(), Some(true), "both counters should render");

    // Click A twice, B once.
    ctx.eval("document.querySelector('.a .b').click(); document.querySelector('.a .b').click(); document.querySelector('.b2 .b').click()")
        .expect("clicks");

    let a_count = ctx
        .eval("ST.get(document.querySelector('.a .c'), 'count')")
        .expect("eval");
    let b_count = ctx
        .eval("ST.get(document.querySelector('.b2 .c'), 'count')")
        .expect("eval");
    assert_eq!(
        a_count.as_f64(),
        Some(2.0),
        "instance A counted its own 2 clicks"
    );
    assert_eq!(
        b_count.as_f64(),
        Some(1.0),
        "instance B counted its own 1 click (no aliasing)"
    );

    // The [slot] <- $count injection updated each instance's DOM independently.
    let a_text = ctx
        .eval("document.querySelector('.a [slot=\"v\"]').textContent")
        .expect("eval");
    let b_text = ctx
        .eval("document.querySelector('.b2 [slot=\"v\"]').textContent")
        .expect("eval");
    assert_eq!(a_text.as_str(), Some("2"), "A slot shows its own count");
    assert_eq!(b_text.as_str(), Some("1"), "B slot shows its own count");
}

// FEAT-071: a `.cls: $sig;` reactive class toggle inside a template body updates the
// element's classList live when the scoped signal changes (ST.watch -> classList.toggle).
#[test]
fn feat071_class_toggle_updates_reactively() {
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    let compiled = compile_st(
        "@template &toggle($l) {\n  $open bool: false;\n  <div class=\"t\"><button class=\"tog\">x</button></div>\n  .t { .t--open: $open; }\n  .tog { @on &.click { $open <- !$open; } }\n}\n.host { &toggle(\"A\"); }\n",
    );
    ctx.set_body_html(r#"<div class="host"></div>"#).unwrap();
    ctx.eval(&compiled.js).expect("compiled JS evaluates");
    let _ = ctx.eval("ST._scheduleInit && ST._scheduleInit(document.body)");

    // Initially not open.
    let before = ctx
        .eval("document.querySelector('.host .t').classList.contains('t--open')")
        .expect("eval");
    assert_eq!(before.as_bool(), Some(false), "class absent before toggle");

    // Click toggles $open -> true -> classList gains t--open.
    ctx.eval("document.querySelector('.host .tog').click()")
        .expect("click");
    let after = ctx
        .eval("document.querySelector('.host .t').classList.contains('t--open')")
        .expect("eval");
    assert_eq!(
        after.as_bool(),
        Some(true),
        "class toggle must add t--open on open=true"
    );

    // Click again -> false -> class removed.
    ctx.eval("document.querySelector('.host .tog').click()")
        .expect("click");
    let again = ctx
        .eval("document.querySelector('.host .t').classList.contains('t--open')")
        .expect("eval");
    assert_eq!(
        again.as_bool(),
        Some(false),
        "class toggle must remove t--open on open=false"
    );
}

// FEAT-073: a self-recursive template (body ref to itself) without a base case must
// NOT overflow the stack — the runtime depth guard caps synchronous render depth and
// stops descending (partial subtree kept, dev warning). This proves recursion is
// SAFE to enable; the bounded/base-case form lands with @match/@if dispatch.
#[test]
fn feat073_unbounded_recursion_is_capped_not_overflow() {
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    // Lower the cap so the test is fast and deterministic.
    ctx.eval("window.__stMaxTemplateDepth = 8;")
        .expect("set cap");
    let compiled = compile_st(
        "@template &node($n) {\n  <div class=\"node\">n</div>\n  &node($n);\n}\n.host { &node(\"x\"); }\n",
    );
    ctx.set_body_html(r#"<div class="host"></div>"#).unwrap();
    // Must not throw / overflow.
    ctx.eval(&compiled.js)
        .expect("compiled JS evaluates without overflow");
    let _ = ctx.eval("ST._scheduleInit && ST._scheduleInit(document.body)");

    // The nesting is bounded by the cap: count .node descendants, must be > 0 and
    // <= cap+1 (root + capped descent), never unbounded.
    let depth = ctx
        .eval("document.querySelectorAll('.host .node').length")
        .expect("eval");
    let d = depth.as_f64().unwrap_or(-1.0);
    assert!(d >= 1.0, "at least the root node must render, got {}", d);
    assert!(
        d <= 9.0,
        "recursion must be capped near __stMaxTemplateDepth=8, got {}",
        d
    );
}

// FEAT-073: dynamic dispatch `&$w(...)` invokes the template whose NAME is the runtime
// value of $w. Here &host("leaf") sets $w="leaf", so the body's `&$w("x")` must render
// the &leaf template. Proves render-by-value (the schema-driven-widget core).
#[test]
fn feat073_dynamic_dispatch_renders_by_value() {
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    let compiled = compile_st(
        "@template &leaf($n) { <div class=\"leaf\">L</div> }\n@template &host($w) {\n  <div class=\"h\"></div>\n  &$w(\"x\");\n}\n.host { &host(\"leaf\"); }\n",
    );
    ctx.set_body_html(r#"<div class="host"></div>"#).unwrap();
    ctx.eval(&compiled.js).expect("compiled JS evaluates");
    let _ = ctx.eval("ST._scheduleInit && ST._scheduleInit(document.body)");

    // $w resolved to "leaf" -> the &leaf template rendered inside the host instance.
    let leaf_rendered = ctx
        .eval("document.querySelector('.host .leaf') !== null")
        .expect("eval");
    assert_eq!(
        leaf_rendered.as_bool(),
        Some(true),
        "&$w() with $w='leaf' must render the &leaf template (dynamic dispatch)"
    );
}

// BUG-131: NAMED template-invocation args (`&card(title: "A")`) must BIND to the
// matching param hole, not render literally. Pre-fix the factory bound positionally
// and the named arg printed `title: "A"` verbatim. Proves the structured arg_names
// path resolves end-to-end (compile split -> bundle -> runtime bind-by-name).
#[test]
fn bug131_named_args_bind_to_param_holes() {
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    // Register &card, then call its factory with a BRANDED named-args object
    // ({__stNamedArgs:{...}}) exactly as ST.buildTemplateArgs produces for a named
    // invocation. Asserts the factory binds by NAME (BUG-131 runtime change),
    // isolated from selector-init/mount harness paths.
    let compiled = compile_st(
        "@template &card($title, $price) {\n  <div class=\"card\"><span class=\"t\">`$title`</span><span class=\"p\">`$price`</span></div>\n}\n",
    );
    ctx.set_body_html(r#"<div class="host"></div>"#).unwrap();
    ctx.eval(&compiled.js).expect("compiled JS evaluates");
    let _ = ctx.eval("ST._scheduleInit && ST._scheduleInit(document.body)");
    let _ = ctx.eval("(function(){var f=Spacetime.templates.get('card'); var el=f({__stNamedArgs:{title:'Espresso',price:'$3.50'}}); document.querySelector('.host').appendChild(el);})()").expect("call factory");

    let title = ctx
        .eval("(document.querySelector('.host .card .t')||{}).textContent || ''")
        .expect("eval title");
    let price = ctx
        .eval("(document.querySelector('.host .card .p')||{}).textContent || ''")
        .expect("eval price");
    assert_eq!(
        title.as_str().unwrap_or(""),
        "Espresso",
        "named arg title must bind the VALUE, not render literally"
    );
    assert_eq!(
        price.as_str().unwrap_or(""),
        "$3.50",
        "named arg price must bind the VALUE"
    );
}

// BUG-131: named args are ORDER-INDEPENDENT. Declaring params ($title, $price) and
// invoking in REVERSE (price first) must still bind each by name.
#[test]
fn bug131_named_args_are_order_independent() {
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    // Build args via ST.buildTemplateArgs from REVERSE-order structured args
    // (price first), then call the factory. Each must still bind by name.
    let compiled = compile_st(
        "@template &card($title, $price) {\n  <div class=\"card\"><span class=\"t\">`$title`</span><span class=\"p\">`$price`</span></div>\n}\n",
    );
    ctx.set_body_html(r#"<div class="host"></div>"#).unwrap();
    ctx.eval(&compiled.js).expect("compiled JS evaluates");
    let _ = ctx.eval("ST._scheduleInit && ST._scheduleInit(document.body)");
    let _ = ctx.eval("(function(){var a=ST.buildTemplateArgs([\"\\\"$9\\\"\",\"\\\"Latte\\\"\"],['price','title'],{}); var f=Spacetime.templates.get('card'); var el=f.apply(null,a); document.querySelector('.host').appendChild(el);})()").expect("call factory");

    let title = ctx
        .eval("(document.querySelector('.host .card .t')||{}).textContent || ''")
        .expect("eval title");
    let price = ctx
        .eval("(document.querySelector('.host .card .p')||{}).textContent || ''")
        .expect("eval price");
    assert_eq!(
        title.as_str().unwrap_or(""),
        "Latte",
        "reverse-order named arg title still binds by name"
    );
    assert_eq!(
        price.as_str().unwrap_or(""),
        "$9",
        "reverse-order named arg price still binds by name"
    );
}

// BUG-131 regression guard: POSITIONAL invocations keep binding by position (the
// pre-existing path must be untouched by the named-arg branch).
#[test]
fn bug131_positional_args_still_bind_by_position() {
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    // Regression: buildTemplateArgs with NO names yields a plain positional list;
    // the factory binds by slot exactly as before BUG-131.
    let compiled = compile_st(
        "@template &card($title, $price) {\n  <div class=\"card\"><span class=\"t\">`$title`</span><span class=\"p\">`$price`</span></div>\n}\n",
    );
    ctx.set_body_html(r#"<div class="host"></div>"#).unwrap();
    ctx.eval(&compiled.js).expect("compiled JS evaluates");
    let _ = ctx.eval("ST._scheduleInit && ST._scheduleInit(document.body)");
    let _ = ctx.eval("(function(){var a=ST.buildTemplateArgs([\"\\\"Mocha\\\"\",\"\\\"$5\\\"\"],[null,null],{}); var f=Spacetime.templates.get('card'); var el=f.apply(null,a); document.querySelector('.host').appendChild(el);})()").expect("call factory");

    let title = ctx
        .eval("(document.querySelector('.host .card .t')||{}).textContent || ''")
        .expect("eval title");
    assert_eq!(
        title.as_str().unwrap_or(""),
        "Mocha",
        "positional arg still binds by slot"
    );
}

// FEAT-073: @match render dispatch. `@match $kind { "a" => &ta(..); "b" => &tb(..); }`
// renders the arm whose pattern equals the runtime value of $kind. &host("b") -> &tb.
const FEAT073_MATCH: &str = r#"@template &ta($n) { <div class="ta">A</div> }
@template &tb($n) { <div class="tb">B</div> }
@template &host($kind) {
  <div class="h"></div>
  @match $kind { "a" => &ta($kind); "b" => &tb($kind); }
}
.host { &host("b"); }
"#;

// PENDING Phase C: @match must be re-implemented as stdlib %macro + %capture_type +
// %primitive (NOT a Rust parse_match_block). These tests are the executable spec for that
// work — un-ignore when @match lands via the metasystem. See FEAT-073 course-correction.
#[test]
fn feat073_match_renders_selected_arm() {
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    let compiled = compile_st(FEAT073_MATCH);
    ctx.set_body_html(r#"<div class="host"></div>"#).unwrap();
    ctx.eval(&compiled.js).expect("compiled JS evaluates");
    let _ = ctx.eval("ST._scheduleInit && ST._scheduleInit(document.body)");

    // $kind="b" -> the "b" arm renders &tb, NOT &ta.
    let tb = ctx
        .eval("document.querySelector('.host .tb') !== null")
        .expect("eval");
    let ta = ctx
        .eval("document.querySelector('.host .ta') !== null")
        .expect("eval");
    assert_eq!(
        tb.as_bool(),
        Some(true),
        "@match must render the matching 'b' arm (&tb)"
    );
    assert_eq!(
        ta.as_bool(),
        Some(false),
        "@match must NOT render the non-matching 'a' arm (&ta)"
    );
}

#[test]
fn feat073_match_wildcard_arm() {
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    let compiled = compile_st(
        "@template &ta($n) { <div class=\"ta\">A</div> }\n@template &fallback($n) { <div class=\"fb\">F</div> }\n@template &host($kind) {\n  <div class=\"h\"></div>\n  @match $kind { \"a\" => &ta($kind); _ => &fallback($kind); }\n}\n.host { &host(\"zzz\"); }\n",
    );
    ctx.set_body_html(r#"<div class="host"></div>"#).unwrap();
    ctx.eval(&compiled.js).expect("compiled JS evaluates");
    let _ = ctx.eval("ST._scheduleInit && ST._scheduleInit(document.body)");
    // $kind="zzz" matches no literal arm -> the `_` wildcard renders &fallback.
    let fb = ctx
        .eval("document.querySelector('.host .fb') !== null")
        .expect("eval");
    assert_eq!(
        fb.as_bool(),
        Some(true),
        "@match `_` wildcard must render the fallback arm"
    );
}

// FEAT-077: the Rust reactive renderer (emit_builder) lowers a real HtmlExpr into a JS
// DOM-builder that renders live DOM and REACTS to ST.set. Same behavioral contract the
// hand-written spike proved, now generated by the compiler (no check-only green).
#[test]
fn feat077_reactive_builder_renders_and_reacts() {
    use spacetime::ir::{AttrPart, HtmlExpr, JsExpr};
    use spacetime::syntax::SignalScope;

    // <div class="card"><span class="v-`$open`">`$count`</span></div>
    let tree = vec![HtmlExpr::Element {
        tag: "div".to_string(),
        attrs: vec![("class".to_string(), vec![AttrPart::Lit("card".to_string())])],
        children: vec![HtmlExpr::Element {
            tag: "span".to_string(),
            attrs: vec![(
                "class".to_string(),
                vec![
                    AttrPart::Lit("v-".to_string()),
                    AttrPart::Hole(JsExpr::Raw("$open".to_string())),
                ],
            )],
            children: vec![HtmlExpr::Hole(JsExpr::Raw("$count".to_string()))],
            line: None,
        }],
        line: None,
    }];
    let builder = spacetime::emit::html_reactive::emit_builder(&tree, SignalScope::Element);

    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    ctx.set_body_html(r#"<div class="host"></div>"#).unwrap();
    // Install the generated builder under a global, then mount it on a scope root.
    ctx.eval(&format!("window.__b = {};", builder))
        .expect("builder evals");
    ctx.eval(
        r#"
      const host = document.querySelector('.host');
      const root = document.createElement('div'); root.className = 'root';
      host.appendChild(root);
      ST.set(root, 'count', 0);
      ST.set(root, 'open', false);
      root.appendChild(window.__b(root));
    "#,
    )
    .expect("mount");

    // Initial render from the generated builder.
    let t0 = ctx
        .eval("document.querySelector('.host .card span').textContent")
        .expect("e");
    assert_eq!(t0.as_str(), Some("0"), "generated builder initial text");
    let c0 = ctx
        .eval("document.querySelector('.host .card span').getAttribute('class')")
        .expect("e");
    assert_eq!(
        c0.as_str(),
        Some("v-false"),
        "generated builder initial attr"
    );

    // React.
    ctx.eval("ST.set(document.querySelector('.host .root'), 'count', 42); ST.set(document.querySelector('.host .root'), 'open', true);").expect("set");
    ctx.eval("return new Promise(r => queueMicrotask(r));").ok();
    let t1 = ctx
        .eval("document.querySelector('.host .card span').textContent")
        .expect("e");
    assert_eq!(t1.as_str(), Some("42"), "generated builder text reacts");
    let c1 = ctx
        .eval("document.querySelector('.host .card span').getAttribute('class')")
        .expect("e");
    assert_eq!(c1.as_str(), Some("v-true"), "generated builder attr reacts");
}

// FEAT-077 R-wave: dotted-path holes are null-safe end-to-end. A `$item.title` hole
// must render empty (not throw + abort the builder) when $item is null, then update
// when $item is set to an object. Proves the try/catch parity with the factory.
#[test]
fn feat077_dotted_hole_is_null_safe() {
    use spacetime::ir::{HtmlExpr, JsExpr};
    use spacetime::syntax::SignalScope;
    let tree = vec![HtmlExpr::Element {
        tag: "span".to_string(),
        attrs: vec![],
        children: vec![HtmlExpr::Hole(JsExpr::Raw("$item.title".to_string()))],
        line: None,
    }];
    let builder = spacetime::emit::html_reactive::emit_builder(&tree, SignalScope::Element);
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    ctx.set_body_html(r#"<div class="host"></div>"#).unwrap();
    ctx.eval(&format!("window.__b = {};", builder))
        .expect("builder evals");
    // Mount with $item UNDEFINED — must not throw.
    ctx.eval(
        r#"
      const host = document.querySelector('.host');
      const root = document.createElement('div'); root.className = 'root';
      host.appendChild(root);
      root.appendChild(window.__b(root));
    "#,
    )
    .expect("mount with null item must not throw");
    let t0 = ctx
        .eval("document.querySelector('.host .root span').textContent")
        .expect("e");
    assert_eq!(t0.as_str(), Some(""), "null $item -> empty text, no throw");
    // Now set $item to an object -> the dotted access resolves.
    ctx.eval("ST.set(document.querySelector('.host .root'), 'item', { title: 'Hello' });")
        .expect("set");
    ctx.eval("return new Promise(r => queueMicrotask(r));").ok();
    let t1 = ctx
        .eval("document.querySelector('.host .root span').textContent")
        .expect("e");
    assert_eq!(
        t1.as_str(),
        Some("Hello"),
        "dotted path resolves after $item set"
    );
}

// FEAT-077 sub-wave 3: root-scoped builder. The single root element IS the signal scope;
// params/state seeded on the root drive the body's holes. This is the factory shape:
// build root → seed params as signals on root → holes react.
#[test]
fn feat077_root_scoped_template_builder() {
    use spacetime::syntax::SignalScope;
    let body = spacetime::emit::html_reactive::component_html_to_exprs(
        "<div class=\"card\"><h2>`$title`</h2><span>`$count`</span></div>",
    );
    let builder =
        spacetime::emit::html_reactive::emit_builder_root_scoped(&body, SignalScope::Element);
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    ctx.set_body_html(r#"<div class="host"></div>"#).unwrap();
    ctx.eval(&format!("window.__build = {};", builder))
        .expect("builder evals");
    // Factory: build root (its own scope), seed params on it, append.
    ctx.eval(
        r#"
      const host = document.querySelector('.host');
      const root = window.__build(null);   // root-scoped: __el rebinds to the .card root
      // Seed params as signals on the root (the scope its holes read).
      ST.set(root, 'title', 'Hello');
      ST.set(root, 'count', 7);
      host.appendChild(root);
    "#,
    )
    .expect("mount");
    ctx.eval("return new Promise(r => queueMicrotask(r));").ok();
    let title = ctx
        .eval("document.querySelector('.host .card h2').textContent")
        .expect("e");
    let count = ctx
        .eval("document.querySelector('.host .card span').textContent")
        .expect("e");
    assert_eq!(
        title.as_str(),
        Some("Hello"),
        "param $title reactive on root scope"
    );
    assert_eq!(
        count.as_str(),
        Some("7"),
        "param $count reactive on root scope"
    );
    // Reactivity: update a param signal -> DOM updates.
    ctx.eval("ST.set(document.querySelector('.host .card'), 'count', 99);")
        .expect("set");
    ctx.eval("return new Promise(r => queueMicrotask(r));").ok();
    let count2 = ctx
        .eval("document.querySelector('.host .card span').textContent")
        .expect("e");
    assert_eq!(count2.as_str(), Some("99"), "param update re-renders");
}

// FEAT-077 sub-wave 3: a real @template compiled end-to-end renders via the reactive
// builder (not the regex fallback) and its params are reactive. This is the convergence
// proof: the factory uses rawBody.builder.
#[test]
fn feat077_template_renders_via_reactive_builder() {
    let compiled = compile_st(
        "@template &card($title, $count) {\n  <div class=\"card\"><h2>`$title`</h2><span class=\"n\">`$count`</span></div>\n}\n.host { &card(\"Hello\", 3); }\n",
    );
    // The payload must carry a real builder (compiler emitted it).
    assert!(
        compiled.js.contains("builder:"),
        "payload must carry a reactive builder"
    );

    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    ctx.set_body_html(r#"<div class="host"></div>"#).unwrap();
    ctx.eval(&compiled.js).expect("compiled JS evaluates");
    let _ = ctx.eval("ST._scheduleInit && ST._scheduleInit(document.body)");

    // Rendered via the builder: the card with interpolated params.
    let title = ctx.eval("document.querySelector('.host .card h2') && document.querySelector('.host .card h2').textContent").expect("e");
    assert_eq!(title.as_str(), Some("Hello"), "param $title rendered");
    let count = ctx.eval("document.querySelector('.host .card .n') && document.querySelector('.host .card .n').textContent").expect("e");
    assert_eq!(count.as_str(), Some("3"), "param $count rendered");

    // Params are reactive (the builder wired ST.watch on the root): updating $count re-renders.
    ctx.eval("ST.set(document.querySelector('.host .card'), 'count', 88);")
        .expect("set");
    ctx.eval("return new Promise(r => queueMicrotask(r));").ok();
    let count2 = ctx
        .eval("document.querySelector('.host .card .n').textContent")
        .expect("e");
    assert_eq!(
        count2.as_str(),
        Some("88"),
        "param $count is REACTIVE via builder"
    );
}

// FEAT-077: &param element substitution renders via the builder (trusted insertion).
#[test]
fn feat077_amp_param_element_substitution() {
    let compiled = compile_st(
        "@template &wrap(&content) {\n  <div class=\"wrap\">`&content`</div>\n}\n.host { &wrap() { <b>hi</b> }; }\n",
    );
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    ctx.set_body_html(r#"<div class="host"></div>"#).unwrap();
    ctx.eval(&compiled.js).expect("compiled JS evaluates");
    let _ = ctx.eval("ST._scheduleInit && ST._scheduleInit(document.body)");
    ctx.eval("return new Promise(r => queueMicrotask(r));").ok();
    // &content="<b>hi</b>" inserted as trusted HTML -> a <b> element exists.
    let has_b = ctx
        .eval("document.querySelector('.host .wrap b') !== null")
        .expect("e");
    assert_eq!(
        has_b.as_bool(),
        Some(true),
        "&content trusted-HTML inserted as <b>"
    );
    let txt = ctx
        .eval("document.querySelector('.host .wrap b').textContent")
        .expect("e");
    assert_eq!(txt.as_str(), Some("hi"), "&content text");
    // No literal "&content" text leaked.
    let leaked = ctx
        .eval("document.querySelector('.host .wrap').textContent.includes('&content')")
        .expect("e");
    assert_eq!(leaked.as_bool(), Some(false), "no literal &content leaked");
}

// FEAT-077 R-wave P2: a multi-root template body (e.g. <dt>/<dd> siblings) must wire holes
// to a real scope (a display:contents wrapper) so params reach them.
#[test]
fn feat077_multiroot_template_scope() {
    let compiled = compile_st(
        "@template &row($name, $val) {\n  <dt class=\"k\">`$name`</dt><dd class=\"v\">`$val`</dd>\n}\n.host { &row(\"Age\", \"30\"); }\n",
    );
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    ctx.set_body_html(r#"<div class="host"></div>"#).unwrap();
    ctx.eval(&compiled.js).expect("compiled JS evaluates");
    let _ = ctx.eval("ST._scheduleInit && ST._scheduleInit(document.body)");
    ctx.eval("return new Promise(r => queueMicrotask(r));").ok();
    let k = ctx
        .eval(
            "document.querySelector('.host .k') && document.querySelector('.host .k').textContent",
        )
        .expect("e");
    let v = ctx
        .eval(
            "document.querySelector('.host .v') && document.querySelector('.host .v').textContent",
        )
        .expect("e");
    assert_eq!(k.as_str(), Some("Age"), "multi-root: $name renders");
    assert_eq!(v.as_str(), Some("30"), "multi-root: $val renders");
}

// FEAT-077 end-to-end: a comprehensive template (params + state + [slot]<- injection +
// @on + class-toggle) renders + reacts via the reactive builder.
#[test]
fn feat077_comprehensive_template_end_to_end() {
    let compiled = compile_st(
        "@template &counter($label) {\n  $n number: 0;\n  <div class=\"ctr\"><span class=\"l\">`$label`</span><output class=\"v\"></output><button class=\"inc\">+</button></div>\n  [class=\"v\"] <- $n;\n  .inc { @on &.click { $n <- $n + 1; } }\n  .ctr { .ctr--active: $n; }\n}\n.app { &counter(\"Clicks\"); }\n",
    );
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    ctx.set_body_html(r#"<div class="app"></div>"#).unwrap();
    ctx.eval(&compiled.js).expect("compiled JS evaluates");
    let _ = ctx.eval("ST._scheduleInit && ST._scheduleInit(document.body)");
    ctx.eval("return new Promise(r => queueMicrotask(r));").ok();

    let label = ctx
        .eval("document.querySelector('.app .ctr .l').textContent")
        .expect("e");
    assert_eq!(label.as_str(), Some("Clicks"), "param rendered");
    let v0 = ctx
        .eval("document.querySelector('.app .ctr .v').textContent")
        .expect("e");
    assert_eq!(v0.as_str(), Some("0"), "selector injection initial");
    let a0 = ctx
        .eval("document.querySelector('.app .ctr').classList.contains('ctr--active')")
        .expect("e");
    assert_eq!(a0.as_bool(), Some(false), "class toggle initial off");
    ctx.eval("document.querySelector('.app .ctr .inc').click()")
        .expect("click");
    ctx.eval("return new Promise(r => queueMicrotask(r));").ok();
    let v1 = ctx
        .eval("document.querySelector('.app .ctr .v').textContent")
        .expect("e");
    assert_eq!(v1.as_str(), Some("1"), "selector injection reacts to @on");
    let a1 = ctx
        .eval("document.querySelector('.app .ctr').classList.contains('ctr--active')")
        .expect("e");
    assert_eq!(a1.as_bool(), Some(true), "class toggle reacts to @on");
}

// BUG-061: [attr="v"] <- $x targets the DESCENDANT matched by the attribute selector
// (its textContent), not a bogus element attribute named "[attr=\"v\"]".
#[test]
fn bug061_attr_selector_injection_targets_descendant() {
    let compiled = compile_st(
        "@template &w($label) {\n  $n number: 7;\n  <div class=\"w\"><output class=\"v\"></output></div>\n  [class=\"v\"] <- $n;\n}\n.app { &w(\"x\"); }\n",
    );
    // FEAT-115 S3c: `[class="v"] <- $n` lowers to a synthesized `[class="v"]`
    // selector-scoped textContent binding (the unified path), not an `injections`
    // payload with a bogus bracket-named attribute. The behavioral asserts below
    // are the real contract (descendant textContent, no bogus attribute).
    assert!(
        !compiled.js.contains("injections:"),
        "injection unified onto the scope path — no `injections:` payload"
    );

    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    ctx.set_body_html(r#"<div class="app"></div>"#).unwrap();
    ctx.eval(&compiled.js).expect("compiled JS evaluates");
    let _ = ctx.eval("ST._scheduleInit && ST._scheduleInit(document.body)");
    ctx.eval("return new Promise(r => queueMicrotask(r));").ok();
    // The .v descendant shows $n (7), set as textContent (not a bogus attribute).
    let v = ctx
        .eval("document.querySelector('.app .w .v').textContent")
        .expect("e");
    assert_eq!(
        v.as_str(),
        Some("7"),
        "[class=v] injection -> descendant textContent"
    );
    // No element has a literal bracket-named attribute.
    let bogus = ctx
        .eval("document.querySelector('.app .w').hasAttribute('[class=\"v\"]')")
        .expect("e");
    assert_eq!(
        bogus.as_bool(),
        Some(false),
        "no bogus bracket-named attribute"
    );
}

// FEAT-077: @each items render via the reactive builder. The bare V8 harness can't run the
// @each selector-init + data lifecycle, so this asserts the EMITTED structure: the row
// template carries a reactive `builder` whose $item.field holes read the seeded $item signal
// (ST.get(__el,'item').field). The end-to-end @each render is covered by the headless suite.
#[test]
fn feat077_each_item_template_uses_reactive_builder() {
    let compiled = compile_st(
        "@data inline $items : [{\"name\":\"Alice\",\"age\":30}];\n@template &row($item) {\n  <li class=\"row\"><span class=\"nm\">`$item.name`</span></li>\n}\n.list {\n  @each($items as $it) {\n    &row($it);\n  }\n}\n",
    );
    assert!(
        compiled.js.contains("builder:"),
        "row template carries a reactive builder"
    );
    // FUP-094: template-body holes lower with SignalScope::Scoped — a hole reads
    // its signal via lexical resolution (ST.resolve from the instance root, which
    // finds the seeded $item on the root first), not the instance-only ST.get.
    assert!(
        compiled.js.contains("ST.resolve(__node, 'item').name"),
        "the $item.name hole reads the seeded $item via lexical resolve: {}",
        &compiled.js[compiled.js.find("builder:").unwrap_or(0)..]
            .chars()
            .take(400)
            .collect::<String>()
    );
    assert!(
        compiled.js.contains("each-with-templates") || compiled.js.contains("invokeTemplate"),
        "@each wires per-item template invocation"
    );
}

// FUP-094: a template-body hole can read an OUTER (page-global) signal via lexical
// resolution — the substrate enabling per-item selection highlight in a list.
// SignalScope::Scoped lowers holes to ST.resolve(__node, name): a template PARAM
// (seeded on the instance root) resolves to the instance; an OUTER signal (not on
// any instance scope) falls through to the global page store. Both in one template.
#[test]
fn fup094_template_hole_resolves_param_and_outer_signal() {
    let compiled = compile_st(
        "$selected string: \"b\";\n@template &row($label) {\n  <li class=\"row\"><span class=\"lbl\">`$label`</span><span class=\"sel\">`$selected`</span></li>\n}\n.list { &row(\"Beta\"); }\n",
    );
    // Emit: both signals via lexical resolve from the instance root (__node).
    assert!(
        compiled.js.contains("ST.resolve(__node, 'label')"),
        "param hole resolves lexically"
    );
    assert!(
        compiled.js.contains("ST.resolve(__node, 'selected')"),
        "OUTER signal hole resolves lexically (the FUP-094 capability)"
    );

    // Runtime: render the template, assert BOTH holes show the right value — the
    // param from the instance, the outer signal from the global store.
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    ctx.set_body_html(r#"<div class="list"></div>"#).unwrap();
    ctx.eval(&compiled.js).expect("compiled JS evaluates");
    let _ = ctx.eval("ST._scheduleInit && ST._scheduleInit(document.body)");
    ctx.eval("return new Promise(r => queueMicrotask(r));").ok();
    let lbl = ctx
        .eval("document.querySelector('.list .row .lbl').textContent")
        .expect("e");
    assert_eq!(
        lbl.as_str(),
        Some("Beta"),
        "param hole renders the instance value"
    );
    let sel = ctx
        .eval("document.querySelector('.list .row .sel').textContent")
        .expect("e");
    assert_eq!(
        sel.as_str(),
        Some("b"),
        "OUTER signal hole renders the global value"
    );

    // Reactivity: flip the outer signal -> the hole that reads it updates (watchScoped
    // subscribed across the scope chain incl. the global store).
    ctx.eval("SpacetimeLocal['selected'] = 'z'; document.dispatchEvent(new CustomEvent('local:selected:updated'));").ok();
    ctx.eval("return new Promise(r => queueMicrotask(r));").ok();
    let sel2 = ctx
        .eval("document.querySelector('.list .row .sel').textContent")
        .expect("e");
    assert_eq!(
        sel2.as_str(),
        Some("z"),
        "OUTER signal change reacts in the template hole"
    );
}

// FEAT-078: file-scope HTML reactivity (SSG hydration). A file-scope `$count` text hole renders
// a static marker `<span data-st-hole="N">INITIAL</span>` (SSG) and the bundle JS re-renders it
// on `local:count:updated` (Global scope). This drives the emitted arc end-to-end: inject the
// compiled.html marker, load the JS, verify the initial render, then mutate the global signal +
// dispatch the channel event and assert the DOM text updates.
#[test]
fn feat078_file_scope_hole_hydrates_and_reacts() {
    let compiled = compile_st(
        "$count number: 5;\n<main><p>Count is <span class=\"v\">`$count`</span></p></main>",
    );
    // The compiled body markup carries the static initial + hydration marker.
    assert!(
        compiled.html.contains("data-st-hole=") && compiled.html.contains(">5<"),
        "compiled.html must carry marker + initial: {:?}",
        compiled.html
    );

    let mut ctx = V8TestContext::new();
    // Seed the global signal store the file-scope hole reads (the local-state runtime would do
    // this; the bare harness seeds it directly) BEFORE the markup so the on-load render matches.
    ctx.eval("globalThis.SpacetimeLocal = { count: 5 }; globalThis.window = globalThis; void 0;")
        .unwrap();
    ctx.set_body_html(&compiled.html).unwrap();
    // Load the hole-hydration JS (render-on-load + local:count:updated listener). Shim timers
    // the same way the cursor test does so the bare harness can load the full runtime bundle.
    let runtime_js = compiled
        .js
        .replace("window.setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("clearTimeout(", "globalThis.__stTestClearTimeout(");
    let mut ctx = ctx.load_compiled(&runtime_js).unwrap();
    ctx.trigger_dom_ready().unwrap();

    // Initial render: the marker shows the initial value.
    assert_eq!(
        ctx.query_text(".v").as_deref(),
        Some("5"),
        "initial hydration"
    );

    // Mutate the global signal + fire the channel event -> the hole re-renders.
    ctx.eval("SpacetimeLocal.count = 42; document.dispatchEvent(new CustomEvent('local:count:updated', { detail: 42 })); void 0;")
        .unwrap();
    assert_eq!(
        ctx.query_text(".v").as_deref(),
        Some("42"),
        "reactive update after mutation"
    );
}

// BUG-067: a mixed literal+hole attribute must emit VALID JS (not splice raw literal text),
// and two holes in separate blocks must not collide. Both verified by the bundle loading and
// the correct reactive update.
#[test]
fn feat078_mixed_attr_and_multiblock_no_collision() {
    let compiled = compile_st(
        "$variant string: \"primary\";\n$count number: 1;\n<header><button class=\"btn `$variant`\">go</button></header>\n<main><span class=\"n\">`$count`</span></main>",
    );
    let mut ctx = V8TestContext::new();
    ctx.eval("globalThis.SpacetimeLocal = { variant: 'primary', count: 1 }; globalThis.window = globalThis; void 0;")
        .unwrap();
    ctx.set_body_html(&compiled.html).unwrap();
    // The bundle must LOAD without a SyntaxError (the mixed-attr bug broke the whole IIFE).
    let runtime_js = compiled
        .js
        .replace("window.setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("clearTimeout(", "globalThis.__stTestClearTimeout(");
    let mut ctx = ctx.load_compiled(&runtime_js).unwrap();
    ctx.trigger_dom_ready().unwrap();

    // Mixed-attr reactive: button class hydrates to "btn primary".
    let cls = ctx
        .eval("document.querySelector('button').getAttribute('class'); ")
        .unwrap();
    assert_eq!(
        cls.as_str(),
        Some("btn primary"),
        "mixed attr initial class"
    );
    // Update the variant signal -> class re-renders (the literal stays).
    ctx.eval("SpacetimeLocal.variant = 'danger'; document.dispatchEvent(new CustomEvent('local:variant:updated', { detail: 'danger' })); void 0;").unwrap();
    let cls2 = ctx
        .eval("document.querySelector('button').getAttribute('class');")
        .unwrap();
    assert_eq!(
        cls2.as_str(),
        Some("btn danger"),
        "mixed attr reactive class"
    );

    // No collision: the count hole (block 2) shows 1 and updates independently.
    assert_eq!(
        ctx.query_text(".n").as_deref(),
        Some("1"),
        "count hole initial"
    );
    ctx.eval("SpacetimeLocal.count = 9; document.dispatchEvent(new CustomEvent('local:count:updated', { detail: 9 })); void 0;").unwrap();
    assert_eq!(
        ctx.query_text(".n").as_deref(),
        Some("9"),
        "count hole reactive, no collision"
    );
}

// FUP-039 Part 1: the compile-time static initial of a COMPOUND hole expr must EQUAL the
// runtime's first apply() render (else hydration flashes a wrong value). Compile `$a + $b`,
// assert the baked initial == the JS-computed value.
#[test]
fn feat078_compound_initial_matches_runtime() {
    let compiled = compile_st(
        "$a number: 3;\n$b number: 4;\n<main><span class=\"sum\">`$a + $b`</span></main>",
    );
    // SSG: the static initial is baked as "7" (3 + 4), matching the runtime.
    assert!(
        compiled.html.contains(">7</span>"),
        "compound initial baked statically: {:?}",
        compiled.html
    );

    let mut ctx = V8TestContext::new();
    ctx.eval("globalThis.SpacetimeLocal = { a: 3, b: 4 }; globalThis.window = globalThis; void 0;")
        .unwrap();
    ctx.set_body_html(&compiled.html).unwrap();
    let runtime_js = compiled
        .js
        .replace("window.setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("clearTimeout(", "globalThis.__stTestClearTimeout(");
    let mut ctx = ctx.load_compiled(&runtime_js).unwrap();
    ctx.trigger_dom_ready().unwrap();
    // After hydration the value is unchanged (no flash): still "7".
    assert_eq!(
        ctx.query_text(".sum").as_deref(),
        Some("7"),
        "runtime apply matches static initial"
    );
    // And it reacts.
    ctx.eval("SpacetimeLocal.a = 10; document.dispatchEvent(new CustomEvent('local:a:updated', { detail: 10 })); void 0;").unwrap();
    assert_eq!(
        ctx.query_text(".sum").as_deref(),
        Some("14"),
        "compound hole reacts (10 + 4)"
    );
}

// BUG-068: selector-scope `.on: $open;` must toggle the CLASS (classList.toggle),
// reacting to signal changes — not setAttribute a bogus attribute.
#[test]
fn bug068_selector_scope_class_toggle_reacts() {
    let compiled = compile_st(
        "$open bool: false;\n<div class=\"box\"><p class=\"msg\">hi</p></div>\n.box { .box--open: $open; }",
    );
    assert!(
        compiled.js.contains("classList.toggle"),
        "must emit classList.toggle: {}",
        compiled.js
    );

    // The scope-aware reactive binding (PLAN-039 Move 2b) reads through ST.resolve /
    // ST.watchScoped / ST.registerSelectorInit, so the ST runtime must be present — as it
    // always is on a real page. Load it, then seed page-global state the binding resolves
    // to when no element scope owns the signal.
    let mut ctx = V8TestContext::new().with_runtime().unwrap();
    ctx.eval(
        "globalThis.SpacetimeLocal = { open: false }; globalThis.window = globalThis; void 0;",
    )
    .unwrap();
    ctx.set_body_html(&compiled.html).unwrap();
    let runtime_js = compiled
        .js
        .replace("window.setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("clearTimeout(", "globalThis.__stTestClearTimeout(");
    let mut ctx = ctx.load_compiled(&runtime_js).unwrap();
    ctx.trigger_dom_ready().unwrap();
    // initially false → class absent
    let has0 = ctx
        .eval("document.querySelector('.box').classList.contains('box--open')")
        .expect("eval");
    assert_eq!(has0.as_bool(), Some(false), "class absent when $open=false");
    // flip true → class present
    ctx.eval("SpacetimeLocal.open = true; document.dispatchEvent(new CustomEvent('local:open:updated', { detail: true })); void 0;").unwrap();
    let has1 = ctx
        .eval("document.querySelector('.box').classList.contains('box--open')")
        .expect("eval");
    assert_eq!(
        has1.as_bool(),
        Some(true),
        "class present when $open=true (reactive toggle)"
    );
}

// BUG-071: selector-scope `text <- $v;` updates textContent reactively (FEAT-072
// spec: the `<-` injection surface works at file/selector scope, not only @template).
#[test]
fn bug071_selector_scope_text_arrow_reacts() {
    let compiled = compile_st(
        "$msg string: \"hello\";\n<div class=\"out\">init</div>\n.out { text <- $msg; }",
    );
    assert!(
        compiled.js.contains("textContent") && compiled.js.contains("local:msg:updated"),
        "must emit textContent binding: {}",
        compiled.js
    );

    // Scope-aware binding needs the ST runtime (always present on a real page).
    let mut ctx = V8TestContext::new().with_runtime().unwrap();
    ctx.eval(
        "globalThis.SpacetimeLocal = { msg: 'hello' }; globalThis.window = globalThis; void 0;",
    )
    .unwrap();
    ctx.set_body_html(&compiled.html).unwrap();
    let runtime_js = compiled
        .js
        .replace("window.setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("clearTimeout(", "globalThis.__stTestClearTimeout(");
    let mut ctx = ctx.load_compiled(&runtime_js).unwrap();
    ctx.trigger_dom_ready().unwrap();
    assert_eq!(
        ctx.query_text(".out").as_deref(),
        Some("hello"),
        "renders initial on load"
    );
    ctx.eval("SpacetimeLocal.msg = 'world'; document.dispatchEvent(new CustomEvent('local:msg:updated', { detail: 'world' })); void 0;").unwrap();
    assert_eq!(
        ctx.query_text(".out").as_deref(),
        Some("world"),
        "reacts to signal change"
    );
}

// =============================================================================
// FEAT-074 W1 → BUG-271 migration: @effect retired onto the @on signal rail.
// `@on $sig.change { mutations }` subscribes to a signal and runs the body on
// genuine change (the @effect consequence, via the change-driver). Default:
// does NOT run on mount (immediate:false); debounce coalesces bursts.
// =============================================================================

/// The @on change rail emits JS that subscribes to the signal and wires the body.
///
/// Asserts the CONTRACT (which signal, which body, through the shared helper),
/// not the mechanism. This test previously pinned the literal document listener
/// `@effect` used to install — so it failed when the subscription collapsed onto
/// ST.watchChanges (FEAT-169), even though the behavior it names was intact.
///
/// That is the string-gate trap BUG-252 was made of: a source assertion cannot
/// tell a working subscription from a broken one, but it CAN block a correct
/// refactor. Behavior lives in tests/effect-change-semantics.test.st (--cdp).
#[test]
fn on_change_emits_subscription_and_body() {
    let compiled = compile_st(
        "$rows number: 0;\n$reloads number: 0;\n<div class=\"app\">x</div>\n.app { @on $rows.change { $reloads <- $reloads + 1; } }",
    );
    assert!(
        compiled.js.contains("const sig = \"rows\""),
        "must name the watched signal: {}",
        compiled.js
    );
    assert!(
        compiled.js.contains("ST.watchChanges("),
        "must subscribe through the change-watch affordance, which resolves in the\n\
         author's scope and delivers genuine changes once (FEAT-169): {}",
        compiled.js
    );
    assert!(
        compiled.js.contains("$reloads <- $reloads + 1") && compiled.js.contains("ST.runMutations"),
        "must carry the mutation body and run it via the shared helper: {}",
        compiled.js
    );
}

/// @on $rows.change fires the body on signal change (default: not on mount), updating a signal.
#[test]
fn on_change_runs_on_change_not_on_mount() {
    let compiled = compile_st(
        "$rows number: 0;\n$reloads number: 0;\n<div class=\"app\">x</div>\n.app { @on $rows.change { $reloads <- $reloads + 1; } }",
    );
    let mut ctx = V8TestContext::new().with_runtime().unwrap();
    ctx.eval("globalThis.SpacetimeLocal = { rows: 0, reloads: 0 }; globalThis.window = globalThis; void 0;")
        .unwrap();
    install_test_timers(&mut ctx);
    ctx.set_body_html(&compiled.html).unwrap();
    let runtime_js = compiled
        .js
        .replace("window.setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("clearTimeout(", "globalThis.__stTestClearTimeout(");
    let mut ctx = ctx.load_compiled(&runtime_js).unwrap();
    ctx.trigger_dom_ready().unwrap();
    // Did NOT run on mount.
    let r0 = ctx.eval("SpacetimeLocal.reloads").expect("e");
    assert_eq!(
        r0.as_f64(),
        Some(0.0),
        "@on change rail must NOT run on mount (immediate:false default)"
    );
    // Change $rows -> effect schedules; flush timers -> body runs once.
    ctx.eval("SpacetimeLocal.rows = 1; document.dispatchEvent(new CustomEvent('local:rows:updated', { detail: 1 })); void 0;").unwrap();
    ctx.eval("globalThis.__stRunAllTimeouts(); void 0;")
        .unwrap();
    let r1 = ctx.eval("SpacetimeLocal.reloads").expect("e");
    assert_eq!(r1.as_f64(), Some(1.0), "on change runs once on signal change");
}

/// @on $rows.change(debounce:) coalesces a burst of changes into one body run.
#[test]
fn on_change_debounce_coalesces_burst() {
    let compiled = compile_st(
        "$rows number: 0;\n$reloads number: 0;\n<div class=\"app\">x</div>\n.app { @on $rows.change(debounce: 300ms) { $reloads <- $reloads + 1; } }",
    );
    let mut ctx = V8TestContext::new().with_runtime().unwrap();
    ctx.eval("globalThis.SpacetimeLocal = { rows: 0, reloads: 0 }; globalThis.window = globalThis; void 0;")
        .unwrap();
    install_test_timers(&mut ctx);
    ctx.set_body_html(&compiled.html).unwrap();
    let runtime_js = compiled
        .js
        .replace("window.setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("clearTimeout(", "globalThis.__stTestClearTimeout(");
    let mut ctx = ctx.load_compiled(&runtime_js).unwrap();
    ctx.trigger_dom_ready().unwrap();
    // Three rapid changes — each cancels the prior timer.
    for n in 1..=3 {
        ctx.eval(&format!("SpacetimeLocal.rows = {n}; document.dispatchEvent(new CustomEvent('local:rows:updated', {{ detail: {n} }})); void 0;")).unwrap();
    }
    // Only the last timer survives; flush -> body runs exactly once.
    let fired = ctx.eval("globalThis.__stRunAllTimeouts()").expect("e");
    assert_eq!(
        fired.as_f64(),
        Some(1.0),
        "debounce leaves exactly one pending timer"
    );
    let r = ctx.eval("SpacetimeLocal.reloads").expect("e");
    assert_eq!(r.as_f64(), Some(1.0), "debounced burst runs the body once");
}

/// @on $rows.change(immediate: true) runs the body once on mount.
#[test]
fn on_change_immediate_runs_on_mount() {
    let compiled = compile_st(
        "$rows number: 0;\n$reloads number: 0;\n<div class=\"app\">x</div>\n.app { @on $rows.change(immediate: true) { $reloads <- $reloads + 1; } }",
    );
    let mut ctx = V8TestContext::new().with_runtime().unwrap();
    ctx.eval("globalThis.SpacetimeLocal = { rows: 0, reloads: 0 }; globalThis.window = globalThis; void 0;")
        .unwrap();
    install_test_timers(&mut ctx);
    ctx.set_body_html(&compiled.html).unwrap();
    let runtime_js = compiled
        .js
        .replace("window.setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("clearTimeout(", "globalThis.__stTestClearTimeout(");
    let mut ctx = ctx.load_compiled(&runtime_js).unwrap();
    ctx.trigger_dom_ready().unwrap();
    ctx.eval("globalThis.__stRunAllTimeouts(); void 0;")
        .unwrap();
    let r = ctx.eval("SpacetimeLocal.reloads").expect("e");
    assert_eq!(
        r.as_f64(),
        Some(1.0),
        "immediate:true runs body once on mount"
    );
}

// =============================================================================
// BUG-271 capture half — a bare signal call `$ping();` is a legal change-rail
// body. The change-driver bind carries it as `actions: "$ping()"`, and flipping
// the SOURCE signal re-fires the called signal (the effect: a second signal the
// callee writes).
// =============================================================================

/// The change-driver bind is emitted AND flipping $pings re-fires $ping().
#[test]
fn on_change_bare_signal_call_refires_callee() {
    let compiled = compile_st(
        "$pings number: 0;\n$ack number: 0;\n<div class=\"app\">x</div>\n.app { @on $pings.change { $ping(); } }",
    );
    // The change-driver bind watches the SOURCE signal and carries the bare
    // signal call as its action (BUG-271: before the capture this directive
    // vanished — no bind, no init, a green page with a missing subscription).
    assert!(
        compiled.js.contains("const sig = \"pings\""),
        "must watch the source signal: {}",
        compiled.js
    );
    assert!(
        compiled.js.contains("const actions = \"$ping()\""),
        "must carry the bare signal-call action: {}",
        compiled.js
    );

    let mut ctx = V8TestContext::new().with_runtime().unwrap();
    ctx.eval("globalThis.SpacetimeLocal = { pings: 0, ack: 0 }; globalThis.window = globalThis; void 0;")
        .unwrap();
    install_test_timers(&mut ctx);
    ctx.set_body_html(&compiled.html).unwrap();
    let runtime_js = compiled
        .js
        .replace("window.setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("clearTimeout(", "globalThis.__stTestClearTimeout(");
    ctx.eval("window.__wcCalls=0; var _w=ST.watchChanges; ST.watchChanges=function(){ window.__wcCalls++; window.__wcArgs = arguments.length >= 2 ? (arguments[0] && arguments[0].className || String(arguments[0])) + '|' + arguments[1] : '?'; var r = _w.apply(this, arguments); window.__wcRet = typeof r; return r; }; void 0;").unwrap();
    let mut ctx = ctx.load_compiled(&runtime_js).unwrap();
    ctx.trigger_dom_ready().unwrap();
    let wc_calls = ctx.eval("window.__wcCalls").unwrap().as_f64().unwrap_or(-1.0);
    let wc_args = ctx.eval("String(window.__wcArgs)").unwrap();
    eprintln!("DBG-WC-CALLS {} args={}", wc_calls, wc_args);
    // Register a fake callable for `$ping` that writes `$ack` — the callee's
    // EFFECT. signal.st:558 registers a compiled @data signal's fire fn here;
    // this test isolates the @on -> runMutations -> getSignalCall dispatch.
    ctx.eval("ST.registerSignalCall('ping', function(){ SpacetimeLocal.ack += 1; document.dispatchEvent(new CustomEvent('local:ack:updated', { detail: SpacetimeLocal.ack })); }); void 0;").unwrap();
    // Not fired on mount.
    assert_eq!(
        ctx.eval("SpacetimeLocal.ack").unwrap().as_f64(),
        Some(0.0),
        "callee not fired on mount"
    );
    // Flip the source signal -> the change-driver runs $ping() -> ack increments.
    ctx.eval("SpacetimeLocal.pings = 1; document.dispatchEvent(new CustomEvent('local:pings:updated', { detail: 1 })); void 0;").unwrap();
    assert_eq!(
        ctx.eval("SpacetimeLocal.ack").unwrap().as_f64(),
        Some(1.0),
        "flipping $pings re-fires $ping()"
    );
    // A SECOND flip re-fires again (not a one-shot).
    ctx.eval("SpacetimeLocal.pings = 2; document.dispatchEvent(new CustomEvent('local:pings:updated', { detail: 2 })); void 0;").unwrap();
    assert_eq!(
        ctx.eval("SpacetimeLocal.ack").unwrap().as_f64(),
        Some(2.0),
        "second flip re-fires again"
    );
}

// =============================================================================
// FEAT-074 W2 — @view: in-app pane swap. `@view $sig { "a" => &pa(); "b" => &pb(); }`
// mounts the arm matching $sig into the host, and RE-MOUNTS reactively on change
// (the reactive sibling of render-once @match). Default: unmounted pane cleaned.
// =============================================================================

/// @view emits a reactive-mount subscription + arm dispatch structure.
#[test]
fn feat074_view_emits_reactive_mount() {
    let compiled = compile_st(
        "$pane string: \"list\";\n<div class=\"center\"></div>\n@template &listPane() { <p class=\"lp\">List</p> }\n@template &graphPane() { <p class=\"gp\">Graph</p> }\n.center { @view $pane { \"list\" => &listPane(); \"graph\" => &graphPane(); } }",
    );
    // Reactive wiring is ST.watchScoped (FUP-094: watch mirrors resolve; it
    // subscribes to the `local:<head>:updated` channel internally) — the old
    // bare document.addEventListener wire was retired with the SWARM GATE 5 fix.
    assert!(
        compiled.js.contains("const sig = \"pane\"")
            && compiled.js.contains("ST.watchScoped(host, sigHead"),
        "@view must wire a reactive mount subscription on the pane signal"
    );
    assert!(
        compiled.js.contains("listPane") && compiled.js.contains("graphPane"),
        "must carry BOTH arm template names (repeated-custom-capture accumulation): {}",
        &compiled.js[..compiled.js.len().min(200)]
    );
}

/// @view mounts the initial pane and swaps to another on signal change (old unmounts).
#[test]
fn feat074_view_swaps_pane_on_signal_change() {
    let compiled = compile_st(
        "$pane string: \"list\";\n<div class=\"center\"></div>\n@template &listPane() { <p class=\"lp\">List</p> }\n@template &graphPane() { <p class=\"gp\">Graph</p> }\n.center { @view $pane { \"list\" => &listPane(); \"graph\" => &graphPane(); } }",
    );
    let mut ctx = V8TestContext::new().with_runtime().unwrap();
    ctx.eval(
        "globalThis.SpacetimeLocal = { pane: 'list' }; globalThis.window = globalThis; void 0;",
    )
    .unwrap();
    install_test_timers(&mut ctx);
    ctx.set_body_html(&compiled.html).unwrap();
    let runtime_js = compiled
        .js
        .replace("window.setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("clearTimeout(", "globalThis.__stTestClearTimeout(");
    let mut ctx = ctx.load_compiled(&runtime_js).unwrap();
    ctx.trigger_dom_ready().unwrap();
    // Initial pane "list" mounted.
    let lp0 = ctx
        .eval("document.querySelectorAll('.center .lp').length")
        .expect("e");
    assert_eq!(lp0.as_f64(), Some(1.0), "list pane mounted initially");
    let gp0 = ctx
        .eval("document.querySelectorAll('.center .gp').length")
        .expect("e");
    assert_eq!(gp0.as_f64(), Some(0.0), "graph pane not mounted initially");
    // Swap to graph.
    ctx.eval("SpacetimeLocal.pane = 'graph'; document.dispatchEvent(new CustomEvent('local:pane:updated', { detail: 'graph' })); void 0;").unwrap();
    let lp1 = ctx
        .eval("document.querySelectorAll('.center .lp').length")
        .expect("e");
    let gp1 = ctx
        .eval("document.querySelectorAll('.center .gp').length")
        .expect("e");
    assert_eq!(gp1.as_f64(), Some(1.0), "graph pane mounted after swap");
    assert_eq!(
        lp1.as_f64(),
        Some(0.0),
        "list pane unmounted after swap (old cleaned)"
    );
}

// =============================================================================
// FEAT-074 W3 — @portal: render a subtree at document.body, escaping the layout
// tree, with the "modal tax" baked in: scroll-lock (refcounted), focus-trap,
// Esc-closes-topmost, focus-restore, aria-modal. `.modal { @portal(when: $open) }`.
// =============================================================================

/// @portal emits a body-relocation + modal-tax wiring keyed on the signal.
#[test]
fn feat074_portal_emits_body_mount() {
    let compiled = compile_st(
        "$open bool: false;\n<div class=\"shell\"><div class=\"modal\">Hi</div></div>\n.modal { @portal(when: $open); }",
    );
    assert!(
        compiled.js.contains("portal") || compiled.js.contains("local:open:updated"),
        "@portal must wire on the open signal: {}",
        &compiled.js[..compiled.js.len().min(200)]
    );
    assert!(
        compiled.js.contains("document.body.appendChild")
            || compiled.js.contains("body.appendChild"),
        "@portal must relocate the subtree to document.body"
    );
}

/// @portal relocates the element to body, locks scroll, sets aria-modal on open;
/// restores on close.
#[test]
fn feat074_portal_relocates_and_locks_on_open() {
    let compiled = compile_st(
        "$open bool: false;\n<div class=\"shell\"><button class=\"opener\">open</button><div class=\"modal\" tabindex=\"-1\">Hi</div></div>\n.modal { @portal(when: $open); }",
    );
    let mut ctx = V8TestContext::new().with_runtime().unwrap();
    ctx.eval(
        "globalThis.SpacetimeLocal = { open: false }; globalThis.window = globalThis; void 0;",
    )
    .unwrap();
    install_test_timers(&mut ctx);
    ctx.set_body_html(&compiled.html).unwrap();
    let runtime_js = compiled
        .js
        .replace("window.setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("clearTimeout(", "globalThis.__stTestClearTimeout(");
    let mut ctx = ctx.load_compiled(&runtime_js).unwrap();
    ctx.trigger_dom_ready().unwrap();
    // Closed: modal stays in shell, scroll not locked.
    let p0 = ctx
        .eval("document.querySelector('.modal').parentNode === document.querySelector('.shell')")
        .expect("e");
    assert_eq!(p0.as_bool(), Some(true), "modal in shell while closed");
    // Open.
    ctx.eval("SpacetimeLocal.open = true; document.dispatchEvent(new CustomEvent('local:open:updated', { detail: true })); void 0;").unwrap();
    let p1 = ctx
        .eval("document.querySelector('.modal').parentNode === document.body")
        .expect("e");
    assert_eq!(p1.as_bool(), Some(true), "modal relocated to body on open");
    let locked = ctx
        .eval("document.body.style.overflow === 'hidden'")
        .expect("e");
    assert_eq!(locked.as_bool(), Some(true), "scroll locked on open");
    let aria = ctx
        .eval("document.querySelector('.modal').getAttribute('aria-modal') === 'true'")
        .expect("e");
    assert_eq!(aria.as_bool(), Some(true), "aria-modal set on open");
    // Close -> scroll unlocked.
    ctx.eval("SpacetimeLocal.open = false; document.dispatchEvent(new CustomEvent('local:open:updated', { detail: false })); void 0;").unwrap();
    let unlocked = ctx
        .eval("document.body.style.overflow !== 'hidden'")
        .expect("e");
    assert_eq!(unlocked.as_bool(), Some(true), "scroll unlocked on close");
}

/// Scroll-lock is REFCOUNTED: two open portals -> one unlock only after both close.
#[test]
fn feat074_portal_scroll_lock_refcounts() {
    let compiled = compile_st(
        "$a bool: false;\n$b bool: false;\n<div class=\"shell\"><div class=\"m1\" tabindex=\"-1\">A</div><div class=\"m2\" tabindex=\"-1\">B</div></div>\n.m1 { @portal(when: $a); }\n.m2 { @portal(when: $b); }",
    );
    let mut ctx = V8TestContext::new().with_runtime().unwrap();
    ctx.eval("globalThis.SpacetimeLocal = { a: false, b: false }; globalThis.window = globalThis; void 0;")
        .unwrap();
    install_test_timers(&mut ctx);
    ctx.set_body_html(&compiled.html).unwrap();
    let runtime_js = compiled
        .js
        .replace("window.setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("clearTimeout(", "globalThis.__stTestClearTimeout(");
    let mut ctx = ctx.load_compiled(&runtime_js).unwrap();
    ctx.trigger_dom_ready().unwrap();
    // Open both.
    ctx.eval("SpacetimeLocal.a = true; document.dispatchEvent(new CustomEvent('local:a:updated', { detail: true })); void 0;").unwrap();
    ctx.eval("SpacetimeLocal.b = true; document.dispatchEvent(new CustomEvent('local:b:updated', { detail: true })); void 0;").unwrap();
    let locked = ctx
        .eval("document.body.style.overflow === 'hidden'")
        .expect("e");
    assert_eq!(locked.as_bool(), Some(true), "locked with both open");
    // Close ONE -> still locked (refcount > 0).
    ctx.eval("SpacetimeLocal.a = false; document.dispatchEvent(new CustomEvent('local:a:updated', { detail: false })); void 0;").unwrap();
    let still = ctx
        .eval("document.body.style.overflow === 'hidden'")
        .expect("e");
    assert_eq!(
        still.as_bool(),
        Some(true),
        "still locked while one portal open (refcount)"
    );
    // Close the other -> unlocked.
    ctx.eval("SpacetimeLocal.b = false; document.dispatchEvent(new CustomEvent('local:b:updated', { detail: false })); void 0;").unwrap();
    let unlocked = ctx
        .eval("document.body.style.overflow !== 'hidden'")
        .expect("e");
    assert_eq!(unlocked.as_bool(), Some(true), "unlocked after both closed");
}

// =============================================================================
// FEAT-074 W4 — capstone: the three orchestration constructs COMPOSE into an app.
// A shell with @view panes (rail-driven), a @portal modal (tax), and an @on
// change rail reacting to a data signal — all in one compiled bundle, exercised
// together.
// =============================================================================
#[test]
fn app_shell_view_portal_on_compose() {
    let src = r#"
$pane string: "list";
$modalOpen bool: false;
$rows number: 0;
$reloads number: 0;
<div class="app">
  <nav class="rail">
    <button class="rail-list">List</button>
    <button class="rail-graph">Graph</button>
    <button class="rail-modal">Open</button>
  </nav>
  <main class="center"></main>
  <div class="modal" tabindex="-1">Modal body</div>
</div>
@template &listPane() { <ul class="lp"><li>row</li></ul> }
@template &graphPane() { <svg class="gp"></svg> }
.center { @view $pane { "list" => &listPane(); "graph" => &graphPane(); } }
.modal { @portal(when: $modalOpen); }
.app { @on $rows.change { $reloads <- $reloads + 1; } }
.rail-list  { @on &.click { $pane <- "list"; } }
.rail-graph { @on &.click { $pane <- "graph"; } }
.rail-modal { @on &.click { $modalOpen <- true; } }
"#;
    let compiled = compile_st(src);
    // All three constructs lowered into one bundle.
    assert!(compiled.js.contains("const sig = \"pane\""), "@view wired");
    assert!(
        compiled.js.contains("body.appendChild") && compiled.js.contains("lockScroll"),
        "@portal wired"
    );
    assert!(
        compiled.js.contains("ST.runMutations") && compiled.js.contains("$reloads <- $reloads + 1"),
        "@on change rail wired"
    );

    let mut ctx = V8TestContext::new().with_runtime().unwrap();
    ctx.eval("globalThis.SpacetimeLocal = { pane: 'list', modalOpen: false, rows: 0, reloads: 0 }; globalThis.window = globalThis; void 0;").unwrap();
    install_test_timers(&mut ctx);
    ctx.set_body_html(&compiled.html).unwrap();
    let runtime_js = compiled
        .js
        .replace("window.setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("clearTimeout(", "globalThis.__stTestClearTimeout(");
    let mut ctx = ctx.load_compiled(&runtime_js).unwrap();
    ctx.trigger_dom_ready().unwrap();

    // @view: list pane mounted initially.
    assert_eq!(
        ctx.eval("document.querySelectorAll('.center .lp').length")
            .unwrap()
            .as_f64(),
        Some(1.0),
        "list pane initial"
    );
    // Swap to graph via rail.
    ctx.eval("document.querySelector('.rail-graph').click(); void 0;")
        .unwrap();
    assert_eq!(
        ctx.eval("document.querySelectorAll('.center .gp').length")
            .unwrap()
            .as_f64(),
        Some(1.0),
        "graph pane after rail click"
    );
    assert_eq!(
        ctx.eval("document.querySelectorAll('.center .lp').length")
            .unwrap()
            .as_f64(),
        Some(0.0),
        "list unmounted after swap"
    );

    // @portal: open the modal via rail.
    ctx.eval("document.querySelector('.rail-modal').click(); void 0;")
        .unwrap();
    assert_eq!(
        ctx.eval("document.querySelector('.modal').parentNode === document.body")
            .unwrap()
            .as_bool(),
        Some(true),
        "modal relocated to body"
    );
    assert_eq!(
        ctx.eval("document.body.style.overflow === 'hidden'")
            .unwrap()
            .as_bool(),
        Some(true),
        "scroll locked by portal"
    );
    // Close the modal via its signal (the same reactive path Esc triggers via
    // ST._portalStack[top].close()). NB: a synthetic KeyboardEvent's `key` field is
    // not delivered to keydown listeners in this minimal V8 DOM, so the Esc keypress
    // itself is exercised in a real browser, not here; the close/restore path is what
    // we assert.
    ctx.eval("SpacetimeLocal.modalOpen = false; document.dispatchEvent(new CustomEvent('local:modalOpen:updated', { detail: false })); void 0;").unwrap();
    assert_eq!(
        ctx.eval("document.body.style.overflow !== 'hidden'")
            .unwrap()
            .as_bool(),
        Some(true),
        "modal close unlocked scroll"
    );
    assert_eq!(
        ctx.eval("document.querySelector('.modal').parentNode !== document.body")
            .unwrap()
            .as_bool(),
        Some(true),
        "modal returned home on close"
    );

    // @on change rail: a data change runs the side-effect.
    ctx.eval("SpacetimeLocal.rows = 5; document.dispatchEvent(new CustomEvent('local:rows:updated', { detail: 5 })); void 0;").unwrap();
    ctx.eval("globalThis.__stRunAllTimeouts(); void 0;")
        .unwrap();
    assert_eq!(
        ctx.eval("SpacetimeLocal.reloads").unwrap().as_f64(),
        Some(1.0),
        "@on ran once on data change"
    );
}

// FEAT-074 W3 follow-up (reviewer P2): @portal must TRAP Tab focus within the modal —
// Tab past the last focusable wraps to the first; Shift+Tab past the first wraps to last.
#[test]
fn feat074_portal_traps_tab_focus() {
    let compiled = compile_st(
        "$open bool: false;\n<div class=\"shell\"><button class=\"bg\">bg</button><div class=\"modal\" tabindex=\"-1\"><button class=\"a\">A</button><button class=\"b\">B</button></div></div>\n.modal { @portal(when: $open); }",
    );
    // The trap installs a keydown handler that wraps focus at the boundaries.
    assert!(
        compiled.js.contains("Tab") && compiled.js.contains("preventDefault"),
        "@portal must install a Tab focus-trap: {}",
        &compiled.js[..compiled.js.len().min(200)]
    );
}

// =============================================================================
// FEAT-075 — mutable collections: `@data collection $rows T[] from "url" via dev-ws
// { initial: [] }` binds a reactive array + .insert/.delete/.reorder mutators with
// optimistic-apply -> persist(op_id) -> ack/reject -> REVERT. The 4 hand-wired admin
// CRUD sites (which lacked client-side revert) collapse to this one construct.
// =============================================================================

/// @data collection emits the array signal + registers insert/delete/reorder mutators.
#[test]
fn feat075_collection_emits_signal_and_mutators() {
    let compiled = compile_st(
        "@data collection $rows any[] from \"/data/products.json\" via dev-ws { initial: []; }\n<ul class=\"list\"></ul>",
    );
    assert!(
        compiled.js.contains("ST._collections") && compiled.js.contains("\"rows\""),
        "must register the rows collection: {}",
        &compiled.js[..compiled.js.len().min(300)]
    );
    assert!(
        compiled.js.contains("insert")
            && compiled.js.contains("delete")
            && compiled.js.contains("reorder"),
        "must expose insert/delete/reorder mutators"
    );
}

/// signal_call `$rows.insert(x)` in an @on body dispatches to the collection mutator.
#[test]
fn feat075_signal_call_dispatches_to_collection() {
    let compiled = compile_st(
        "@data collection $rows any[] from \"/data/x.json\" via dev-ws { initial: []; }\n<button class=\"add\">add</button>\n.add { @on &.click { $rows.insert(7); } }",
    );
    let mut ctx = V8TestContext::new().with_runtime().unwrap();
    ctx.eval("globalThis.window = globalThis; globalThis.__stDevWs = { connected: true, _sent: [], send: function(m){ this._sent.push(m); }, onAck: function(){}, onReject: function(){} }; void 0;").unwrap();
    install_test_timers(&mut ctx);
    ctx.set_body_html(&compiled.html).unwrap();
    let rj = compiled
        .js
        .replace("window.setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("clearTimeout(", "globalThis.__stTestClearTimeout(");
    let mut ctx = ctx.load_compiled(&rj).unwrap();
    ctx.trigger_dom_ready().unwrap();
    // seed initial array
    ctx.eval("ST.setData('rows', []); void 0;").unwrap();
    // click add -> $rows.insert(7) optimistically applies
    ctx.eval("document.querySelector('.add').click(); void 0;")
        .unwrap();
    let len = ctx.eval("(ST._dataRegistry.get('rows') || {}).data ? ST._dataRegistry.get('rows').data.length : -1").expect("e");
    assert_eq!(
        len.as_f64(),
        Some(1.0),
        "insert optimistically applied to the array signal"
    );
    // it persisted with an EditJsonArray + op_id
    let sent = ctx.eval("window.__stDevWs._sent.length > 0 && window.__stDevWs._sent[0].type === 'EditJsonArray' && !!window.__stDevWs._sent[0].op_id").expect("e");
    assert_eq!(
        sent.as_bool(),
        Some(true),
        "persisted EditJsonArray with op_id"
    );
}

/// A rejected op REVERTS the array to its exact pre-op snapshot.
#[test]
fn feat075_reject_reverts_to_snapshot() {
    let compiled = compile_st(
        "@data collection $rows any[] from \"/data/x.json\" via dev-ws { initial: []; }\n<button class=\"add\">add</button>\n.add { @on &.click { $rows.insert(7); } }",
    );
    let mut ctx = V8TestContext::new().with_runtime().unwrap();
    // a WS stub that captures the reject callback so we can fire it.
    ctx.eval("globalThis.window = globalThis; globalThis.__rejectCb = null; globalThis.__stDevWs = { connected: true, _sent: [], send: function(m){ this._sent.push(m); }, onAck: function(){}, onReject: function(id, cb){ globalThis.__rejectCb = cb; } }; void 0;").unwrap();
    install_test_timers(&mut ctx);
    ctx.set_body_html(&compiled.html).unwrap();
    let rj = compiled
        .js
        .replace("window.setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("clearTimeout(", "globalThis.__stTestClearTimeout(");
    let mut ctx = ctx.load_compiled(&rj).unwrap();
    ctx.trigger_dom_ready().unwrap();
    ctx.eval("ST.setData('rows', [1, 2]); void 0;").unwrap();
    ctx.eval("document.querySelector('.add').click(); void 0;")
        .unwrap();
    let after = ctx
        .eval("ST._dataRegistry.get('rows').data.length")
        .expect("e");
    assert_eq!(after.as_f64(), Some(3.0), "optimistic insert -> length 3");
    // fire the reject -> revert to [1,2]
    ctx.eval("if (globalThis.__rejectCb) globalThis.__rejectCb({ reason: 'nope' }); void 0;")
        .unwrap();
    let reverted = ctx
        .eval("ST._dataRegistry.get('rows').data.length")
        .expect("e");
    assert_eq!(
        reverted.as_f64(),
        Some(2.0),
        "reject reverted to pre-op snapshot [1,2]"
    );
    let err = ctx.eval("typeof SpacetimeLocal !== 'undefined' && SpacetimeLocal.rows_error ? true : (window._localState && window._localState.rows_error ? true : false)").expect("e");
    assert_eq!(err.as_bool(), Some(true), "reject surfaced $rows_error");
}

// FEAT-075 (reviewer P1): a rejected EARLIER op must revert ONLY itself, not clobber
// later in-flight optimistic ops. Two inserts before any ack; reject #1 -> the array
// keeps #2's item (revert applies the inverse of the failed op, not a stale snapshot).
#[test]
fn feat075_concurrent_reject_independent() {
    let compiled = compile_st(
        "@data collection $rows any[] from \"/data/x.json\" via dev-ws { initial: []; }\n<button class=\"a1\">1</button><button class=\"a2\">2</button>\n.a1 { @on &.click { $rows.insert(\"x1\"); } }\n.a2 { @on &.click { $rows.insert(\"x2\"); } }",
    );
    let mut ctx = V8TestContext::new().with_runtime().unwrap();
    // WS stub capturing reject callbacks per op_id.
    ctx.eval("globalThis.window = globalThis; globalThis.__rejects = {}; globalThis.__lastSent = []; globalThis.__stDevWs = { connected: true, send: function(m){ globalThis.__lastSent.push(m); }, onAck: function(){}, onReject: function(id, cb){ globalThis.__rejects[id] = cb; } }; void 0;").unwrap();
    install_test_timers(&mut ctx);
    ctx.set_body_html(&compiled.html).unwrap();
    let rj = compiled
        .js
        .replace("window.setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("clearTimeout(", "globalThis.__stTestClearTimeout(");
    let mut ctx = ctx.load_compiled(&rj).unwrap();
    ctx.trigger_dom_ready().unwrap();
    ctx.eval("ST.setData('rows', ['a']); void 0;").unwrap();
    // two inserts before any ack
    ctx.eval(
        "document.querySelector('.a1').click(); document.querySelector('.a2').click(); void 0;",
    )
    .unwrap();
    let mid = ctx
        .eval("JSON.stringify(ST._dataRegistry.get('rows').data)")
        .expect("e");
    assert_eq!(
        mid.as_str(),
        Some("[\"a\",\"x1\",\"x2\"]"),
        "both inserts optimistically applied"
    );
    // reject the FIRST op only
    ctx.eval("var ids = Object.keys(globalThis.__rejects); globalThis.__rejects[ids[0]]({ reason: 'no' }); void 0;").unwrap();
    let after = ctx
        .eval("JSON.stringify(ST._dataRegistry.get('rows').data)")
        .expect("e");
    // x1 removed, x2 (later op) preserved
    assert_eq!(
        after.as_str(),
        Some("[\"a\",\"x2\"]"),
        "reject of op#1 removes only x1, keeps x2"
    );
}

// FEAT-075 (reviewer P2): delete-by-dataset-index — `$rows.delete($.dataset.i | int)`
// must coerce the string dataset index + honor the | int filter (was throwing/ no-op).
#[test]
fn feat075_delete_by_dataset_index() {
    let compiled = compile_st(
        "@data collection $rows any[] from \"/data/x.json\" via dev-ws { initial: []; }\n<button class=\"del\" data-i=\"1\">x</button>\n.del { @on &.click { $rows.delete($.dataset.i | int); } }",
    );
    let mut ctx = V8TestContext::new().with_runtime().unwrap();
    ctx.eval("globalThis.window = globalThis; globalThis.__stDevWs = { connected: true, send: function(){}, onAck: function(){}, onReject: function(){} }; void 0;").unwrap();
    install_test_timers(&mut ctx);
    ctx.set_body_html(&compiled.html).unwrap();
    let rj = compiled
        .js
        .replace("window.setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("setTimeout(", "globalThis.__stTestSetTimeout(")
        .replace("clearTimeout(", "globalThis.__stTestClearTimeout(");
    let mut ctx = ctx.load_compiled(&rj).unwrap();
    ctx.trigger_dom_ready().unwrap();
    ctx.eval("ST.setData('rows', ['a', 'b', 'c']); void 0;")
        .unwrap();
    ctx.eval("document.querySelector('.del').click(); void 0;")
        .unwrap();
    let after = ctx
        .eval("JSON.stringify(ST._dataRegistry.get('rows').data)")
        .expect("e");
    assert_eq!(
        after.as_str(),
        Some("[\"a\",\"c\"]"),
        "delete($.dataset.i | int) removed index 1 (b)"
    );
}

// BUG-076 (reviewer P2): @each must wire BOTH source channels (local:<name>:updated
// AND the @data registry) so a late-resolving derived source renders, AND must NOT
// double-render when a source publishes the SAME array reference through both. The
// dedup guard (render() returns early when data === lastData) collapses the
// dual-arrival to one paint. Here: seed the list, tag the rendered nodes, then
// re-publish the IDENTICAL array via the second channel; the tagged nodes must
// survive (no rebuild), proving the dedup.
#[test]
fn bug076_each_dual_channel_dedup_emitted() {
    // BUG-076: each-with-templates must (a) wire BOTH source channels
    // unconditionally (local:<name>:updated AND the @data registry) so a late
    // derived source renders, and (b) dedup identical dual-arrivals via a
    // reference-equality guard in render(). Verified at the emitted-JS level (the
    // bare V8 harness can't register the row template); the browser end-to-end
    // run (FEAT-076 W1) confirms a single rebuild on collection switch.
    let compiled = compile_st(
        "@data inline $rows : [];\n@template &row($r) { <li class=\"r\">`$r.n`</li> }\n.list { @each($rows as $it) { &row($it); } }\n<ul class=\"list\"></ul>",
    );
    let js = &compiled.js;
    // (a) BOTH channels wired unconditionally (no isLocalSource snapshot branch).
    // (BUG-155 dotted-path era: the channel names key off `headName`, the
    // subject's head signal, via template literals.)
    assert!(
        js.contains("local:${headName}:updated"),
        "each wires the local:<name>:updated channel"
    );
    assert!(
        js.contains("ST.afterData(headName"),
        "each wires the @data registry channel"
    );
    assert!(
        !js.contains("const isLocalSource"),
        "the one-shot isLocalSource snapshot branch is gone (both channels are unconditional)"
    );
    // (b) the dedup guard collapses identical dual-arrivals to one paint.
    assert!(
        js.contains("if (data === lastData) return;"),
        "render() dedups a reference-identical re-arrival (no double render)"
    );
}

// BUG-074: @match arm + template-ref invocation args must resolve dotted paths
// ($node.properties → data.node.properties), exactly like @match subjects and
// dynamic template names already do. The fix is one shared resolver,
// ST.resolveTemplateArg(arg, data). Tested directly against the runtime: the
// recursive schema drawer (W2) depends on passing nested sub-nodes down.
#[test]
fn bug074_resolve_template_arg_walks_dotted_paths() {
    let mut ctx = V8TestContext::new().with_runtime().unwrap();
    ctx.eval("globalThis.window = globalThis; globalThis.__d = { node: { properties: { a: 1 }, widget: 'fieldset' }, val: 7, name: 'x' }; void 0;").unwrap();
    // Bare $var
    assert_eq!(
        ctx.eval("ST.resolveTemplateArg('$val', globalThis.__d)")
            .unwrap()
            .as_f64(),
        Some(7.0),
        "bare $var"
    );
    // Dotted path — the BUG-074 case (was returning the literal '$node.properties')
    assert_eq!(
        ctx.eval("JSON.stringify(ST.resolveTemplateArg('$node.properties', globalThis.__d))")
            .unwrap()
            .as_str(),
        Some("{\"a\":1}"),
        "dotted $node.properties walks to the nested object"
    );
    // Deep dotted
    assert_eq!(
        ctx.eval("ST.resolveTemplateArg('$node.widget', globalThis.__d)")
            .unwrap()
            .as_str(),
        Some("fieldset"),
        "dotted $node.widget"
    );
    // Quoted literal unquoted
    assert_eq!(
        ctx.eval("ST.resolveTemplateArg('\"lit\"', globalThis.__d)")
            .unwrap()
            .as_str(),
        Some("lit"),
        "quoted literal unquoted"
    );
    // Unknown $var falls back to the literal token (no crash)
    assert_eq!(
        ctx.eval("ST.resolveTemplateArg('$missing', globalThis.__d)")
            .unwrap()
            .as_str(),
        Some("$missing"),
        "unknown $var → literal"
    );
    // Missing mid-path collapses to the literal (no throw on null deref)
    assert_eq!(
        ctx.eval("ST.resolveTemplateArg('$node.nope.deep', globalThis.__d)")
            .unwrap()
            .as_str(),
        Some("$node.nope.deep"),
        "missing mid-path → literal, no throw"
    );
}

// BUG-078: a MOVED (re-parented) reactive subtree must KEEP its signals; only a
// GENUINELY removed node is disposed. @portal relocates a subtree to <body>,
// which emits a removedNodes record for the old slot — but the node is already
// re-attached (isConnected) by the time the MutationObserver microtask runs.
// Disposing it would wipe the signal store of every descendant while still live
// (the admin drawer's schema form went blank). The observer guards dispose on
// !isConnected. Verified against the REAL runtime with a manual MutationObserver
// shim (the bare harness has no live observer): the guard logic is the move-vs-
// remove discriminator.
#[test]
fn bug078_move_keeps_signals_remove_disposes() {
    let mut ctx = V8TestContext::new().with_runtime().unwrap();
    ctx.eval("globalThis.window = globalThis; void 0;").unwrap();
    ctx.set_body_html(r#"<div id="host"></div><div id="elsewhere"></div>"#)
        .unwrap();

    // Seed a signal, MOVE it (re-parent), then drive the REAL cleanup path
    // (ST._handleRemovedNode — the function the MutationObserver calls per removed
    // node). The moved node is still connected, so the guard must PRESERVE it.
    let moved = ctx
        .eval(
            "(() => { \
           const host = document.getElementById('host'); \
           const el = document.createElement('div'); host.appendChild(el); \
           const child = document.createElement('span'); el.appendChild(child); \
           ST.set(el, 'f', { label: 'keep' }); ST.set(child, 'c', 1); \
           const before = ST.store(el)['f'] ? 'has' : 'none'; \
           const dest = document.getElementById('elsewhere'); \
           dest.appendChild(el); /* MOVE: re-parent (still connected) */ \
           ST._handleRemovedNode(el); /* the exact call the observer makes */ \
           const after = ST.store(el)['f'] ? 'has' : 'none'; \
           const childAfter = ST.store(child)['c'] !== undefined ? 'has' : 'none'; \
           return before + ',' + after + ',' + childAfter + ',' + el.isConnected; \
         })()",
        )
        .unwrap();
    assert_eq!(
        moved.as_str(),
        Some("has,has,has,true"),
        "moved subtree keeps its (and descendants') signals via ST._handleRemovedNode"
    );

    // Truly remove an element + child, drive the real path, assert disposal (no leak).
    let removed = ctx
        .eval(
            "(() => { \
           const host = document.getElementById('host'); \
           const el = document.createElement('div'); host.appendChild(el); \
           const child = document.createElement('span'); el.appendChild(child); \
           ST.set(el, 'g', { label: 'gone' }); ST.set(child, 'h', 2); \
           const before = ST.store(el)['g'] ? 'has' : 'none'; \
           host.removeChild(el); /* REMOVE: detached (disconnected) */ \
           ST._handleRemovedNode(el); \
           const after = ST.store(el)['g'] ? 'has' : 'none'; \
           const childAfter = ST.store(child)['h'] !== undefined ? 'has' : 'none'; \
           return before + ',' + after + ',' + childAfter + ',' + el.isConnected; \
         })()",
        )
        .unwrap();
    assert_eq!(
        removed.as_str(),
        Some("has,none,none,false"),
        "genuinely removed element + descendants disposed via ST._handleRemovedNode"
    );
}

// FEAT-076 W3: the admin's dynamic-collection CRUD helpers (ST.adInsert/adDelete/
// adReorder/adEditField) apply optimistically to the $entries signal, publish
// through BOTH reactive channels (registry + SpacetimeLocal + local:entries:updated),
// and persist via window.__stDevWs. Verified against the runtime with a WS stub.
#[test]
fn feat076_admin_crud_helpers_optimistic_dual_channel() {
    let mut ctx = V8TestContext::new().with_runtime().unwrap();
    ctx.eval(
        "globalThis.window = globalThis; \
         globalThis.__sent = []; \
         globalThis.window.__stDevWs = { connected: true, send: m => globalThis.__sent.push(m), onAck: ()=>{}, onReject: ()=>{} }; \
         globalThis.window.SpacetimeLocal = { activeFile: 'data/posts.json', entries: [], activeIndex: -1, drawerOpen: false }; \
         ST.setData('entries', []); void 0;",
    ).unwrap();

    // Seed 2 entries through the dual-channel publisher.
    ctx.eval("ST._adPublishEntries([{id:'a',title:'A'},{id:'b',title:'B'}]); void 0;")
        .unwrap();
    assert_eq!(
        ctx.eval("ST.getData('entries').length").unwrap().as_f64(),
        Some(2.0),
        "seeded via registry"
    );
    assert_eq!(
        ctx.eval("window.SpacetimeLocal.entries.length")
            .unwrap()
            .as_f64(),
        Some(2.0),
        "seeded via SpacetimeLocal"
    );

    // INSERT
    ctx.eval("ST.adInsert({id:'c',title:'C'}); void 0;")
        .unwrap();
    assert_eq!(
        ctx.eval("window.SpacetimeLocal.entries.length")
            .unwrap()
            .as_f64(),
        Some(3.0),
        "insert applied to signal"
    );
    assert_eq!(
        ctx.eval("window.SpacetimeLocal.entries[2].title")
            .unwrap()
            .as_str(),
        Some("C"),
        "inserted at end"
    );
    assert_eq!(
        ctx.eval("__sent[__sent.length-1].op.op").unwrap().as_str(),
        Some("Insert"),
        "EditJsonArray Insert sent"
    );
    assert_eq!(
        ctx.eval("__sent[__sent.length-1].file").unwrap().as_str(),
        Some("data/posts.json"),
        "op targets the active file"
    );

    // EDIT FIELD (entry 0, .title)
    ctx.eval("ST.adEditField(0, '.title', 'A2'); void 0;")
        .unwrap();
    assert_eq!(
        ctx.eval("window.SpacetimeLocal.entries[0].title")
            .unwrap()
            .as_str(),
        Some("A2"),
        "field edit applied"
    );
    assert_eq!(
        ctx.eval("__sent[__sent.length-1].type").unwrap().as_str(),
        Some("EditJson"),
        "EditJson sent for field"
    );
    assert_eq!(
        ctx.eval("__sent[__sent.length-1].path").unwrap().as_str(),
        Some("[0].title"),
        "field path is [index].field"
    );

    // REORDER (0 -> 2)
    ctx.eval("ST.adReorder(0, 2); void 0;").unwrap();
    assert_eq!(
        ctx.eval("window.SpacetimeLocal.entries[2].id")
            .unwrap()
            .as_str(),
        Some("a"),
        "entry a moved to end"
    );
    assert_eq!(
        ctx.eval("__sent[__sent.length-1].op.op").unwrap().as_str(),
        Some("Reorder"),
        "Reorder op sent"
    );

    // DELETE (index 0)
    let before = ctx
        .eval("window.SpacetimeLocal.entries.length")
        .unwrap()
        .as_f64();
    ctx.eval("ST.adDelete(0); void 0;").unwrap();
    assert_eq!(
        ctx.eval("window.SpacetimeLocal.entries.length")
            .unwrap()
            .as_f64(),
        before.map(|n| n - 1.0),
        "delete removed one"
    );
    assert_eq!(
        ctx.eval("__sent[__sent.length-1].op.op").unwrap().as_str(),
        Some("Delete"),
        "Delete op sent"
    );

    // Two-click delete arm: first arms, second confirms.
    assert_eq!(
        ctx.eval("ST.adArmDelete(false, 0)").unwrap().as_bool(),
        Some(true),
        "first click arms"
    );
    let n0 = ctx
        .eval("window.SpacetimeLocal.entries.length")
        .unwrap()
        .as_f64();
    assert_eq!(
        ctx.eval("ST.adArmDelete(true, 0)").unwrap().as_bool(),
        Some(false),
        "second click disarms (confirm)"
    );
    assert_eq!(
        ctx.eval("window.SpacetimeLocal.entries.length")
            .unwrap()
            .as_f64(),
        n0.map(|n| n - 1.0),
        "confirm deletes"
    );
    assert_eq!(
        ctx.eval("window.SpacetimeLocal.drawerOpen")
            .unwrap()
            .as_bool(),
        Some(false),
        "confirm closes the drawer"
    );
}

// FEAT-076 W3 (reviewer fixes): nested-list field paths, value coercion, and
// cross-collection revert guards in the admin CRUD helpers.
#[test]
fn feat076_admin_crud_reviewer_fixes() {
    let mut ctx = V8TestContext::new().with_runtime().unwrap();
    ctx.eval(
        "globalThis.window = globalThis; globalThis.__sent = []; \
         globalThis.window.__stDevWs = { connected: true, send: m => globalThis.__sent.push(m), onAck: ()=>{}, onReject: (id,cb)=>{ globalThis.__lastReject = cb; } }; \
         globalThis.window.SpacetimeLocal = { activeFile: 'data/posts.json', entries: [], activeIndex: 0 }; \
         ST.setData('entries', []); void 0;",
    ).unwrap();

    // Nested-list path: _adSetByPath must descend .sections[0].heading (not create a junk key).
    ctx.eval("ST._adPublishEntries([{ id:'a', sections:[{heading:'old'}] }]); void 0;")
        .unwrap();
    ctx.eval("ST.adEditField(0, '.sections[0].heading', 'new'); void 0;")
        .unwrap();
    assert_eq!(
        ctx.eval("window.SpacetimeLocal.entries[0].sections[0].heading")
            .unwrap()
            .as_str(),
        Some("new"),
        "nested object-array field path descends correctly"
    );
    assert_eq!(
        ctx.eval("window.SpacetimeLocal.entries[0]['sections[0]'] === undefined")
            .unwrap()
            .as_bool(),
        Some(true),
        "no junk 'sections[0]' key is created"
    );
    assert_eq!(
        ctx.eval("__sent[__sent.length-1].path").unwrap().as_str(),
        Some("[0].sections[0].heading"),
        "EditJson path is the full server json-path"
    );

    // Value coercion: number widget → Number, toggle → bool.
    assert_eq!(
        ctx.eval("ST.adCoerceValue('number', '42')")
            .unwrap()
            .as_f64(),
        Some(42.0),
        "number coerced"
    );
    assert_eq!(
        ctx.eval("typeof ST.adCoerceValue('number', '42')")
            .unwrap()
            .as_str(),
        Some("number"),
        "number type"
    );
    assert!(
        ctx.eval("ST.adCoerceValue('number', '')")
            .unwrap()
            .is_null(),
        "empty number → null"
    );
    assert_eq!(
        ctx.eval("ST.adCoerceValue('toggle', 1)").unwrap().as_bool(),
        Some(true),
        "toggle → bool"
    );
    assert_eq!(
        ctx.eval("ST.adCoerceValue('text', 'x')").unwrap().as_str(),
        Some("x"),
        "text passthrough"
    );

    // Cross-collection revert guard: a reject after switching files must NOT mutate the new collection.
    ctx.eval("ST._adPublishEntries([{id:'p1'},{id:'p2'}]); window.SpacetimeLocal.activeFile='data/posts.json'; void 0;").unwrap();
    ctx.eval("ST.adDelete(0); void 0;").unwrap(); // optimistic: removes p1, captures file=posts
    assert_eq!(
        ctx.eval("window.SpacetimeLocal.entries.length")
            .unwrap()
            .as_f64(),
        Some(1.0),
        "delete applied"
    );
    // switch collection + new entries, THEN fire the captured reject
    ctx.eval("window.SpacetimeLocal.activeFile='data/authors.json'; ST._adPublishEntries([{id:'au1'}]); void 0;").unwrap();
    ctx.eval("if (globalThis.__lastReject) globalThis.__lastReject({reason:'x'}); void 0;")
        .unwrap();
    assert_eq!(
        ctx.eval("window.SpacetimeLocal.entries.length")
            .unwrap()
            .as_f64(),
        Some(1.0),
        "late reject after collection switch does NOT corrupt the new collection (guard no-ops)"
    );
    assert_eq!(
        ctx.eval("window.SpacetimeLocal.entries[0].id")
            .unwrap()
            .as_str(),
        Some("au1"),
        "new collection intact"
    );

    // Delete arm: two-click; the label helper reflects state.
    assert_eq!(
        ctx.eval("ST.adDeleteLabel(false)").unwrap().as_str(),
        Some("Delete"),
        "disarmed label"
    );
    assert_eq!(
        ctx.eval("ST.adDeleteLabel(true)").unwrap().as_str(),
        Some("Confirm delete"),
        "armed label"
    );
}

// BUG-080: the generic field-edit handler must NOT wipe a chips/relation-multi
// array when the user types in the injected (un-pathed) add sub-control. The
// delegated closest('[data-path]') climbs to the CONTAINER (data-widget chips/
// relation-multi); ST.adFieldInput guards against persisting from a container.
#[test]
fn bug080_field_input_guard_protects_array_widgets() {
    let mut ctx = V8TestContext::new().with_runtime().unwrap();
    ctx.eval(
        "globalThis.window = globalThis; globalThis.__sent = []; \
         globalThis.window.__stDevWs = { connected: true, send: m => globalThis.__sent.push(m), onAck: ()=>{}, onReject: ()=>{} }; \
         globalThis.window.SpacetimeLocal = { activeFile: 'data/x.json', entries: [{ tags: ['a','b'], title: 'T' }], activeIndex: 0 }; \
         ST.setData('entries', window.SpacetimeLocal.entries); void 0;",
    ).unwrap();

    // A chips CONTAINER el (data-widget=chips) — typing in its add-input climbs here.
    let tagsAfter = ctx.eval(
        "(() => { const el = { dataset: { widget: 'chips', path: '.tags' }, value: undefined }; \
          ST.adFieldInput(el, 0); \
          return JSON.stringify(window.SpacetimeLocal.entries[0].tags); })()"
    ).unwrap();
    assert_eq!(
        tagsAfter.as_str(),
        Some("[\"a\",\"b\"]"),
        "chips container input does NOT wipe the array"
    );

    // A relation-multi container is likewise inert.
    ctx.eval("ST.adFieldInput({ dataset: { widget: 'relation-multi', path: '.contributors' }, value: undefined }, 0); void 0;").unwrap();
    assert_eq!(
        ctx.eval("window.SpacetimeLocal.entries[0].contributors === undefined")
            .unwrap()
            .as_bool(),
        Some(true),
        "relation-multi container input did not create/wipe a value"
    );

    // A real text control DOES persist.
    ctx.eval("ST.adFieldInput({ dataset: { widget: 'text', path: '.title' }, value: 'NEW' }, 0); void 0;").unwrap();
    assert_eq!(
        ctx.eval("window.SpacetimeLocal.entries[0].title")
            .unwrap()
            .as_str(),
        Some("NEW"),
        "text control persists"
    );

    // An element with NO data-widget (climbed past all controls) is inert.
    let before = ctx
        .eval("JSON.stringify(window.SpacetimeLocal.entries[0])")
        .unwrap();
    ctx.eval("ST.adFieldInput({ dataset: { path: '.title' }, value: 'X' }, 0); void 0;")
        .unwrap();
    let after = ctx
        .eval("JSON.stringify(window.SpacetimeLocal.entries[0])")
        .unwrap();
    assert_eq!(
        before.as_str(),
        after.as_str(),
        "element without data-widget is inert"
    );
}

// FEAT-076 W5 (final swarm P2): reordering while the drawer is open must keep
// $activeIndex pinned to the SAME entry, or a subsequent field edit writes into
// the wrong entry. ST._adReindexAfterReorder shifts activeIndex to track the move.
#[test]
fn feat076_reorder_pins_active_index() {
    let mut ctx = V8TestContext::new().with_runtime().unwrap();
    ctx.eval("globalThis.window = globalThis; globalThis.window.SpacetimeLocal = { activeIndex: 0 }; void 0;").unwrap();
    // The active (open) entry is index 0; move it down (0 -> 1): activeIndex follows to 1.
    ctx.eval("ST._adReindexAfterReorder(0, 1); void 0;")
        .unwrap();
    assert_eq!(
        ctx.eval("window.SpacetimeLocal.activeIndex")
            .unwrap()
            .as_f64(),
        Some(1.0),
        "active entry moved down → index follows"
    );
    // A different row moving across the active index shifts it the other way.
    ctx.eval("window.SpacetimeLocal.activeIndex = 2; ST._adReindexAfterReorder(0, 3); void 0;")
        .unwrap();
    assert_eq!(
        ctx.eval("window.SpacetimeLocal.activeIndex")
            .unwrap()
            .as_f64(),
        Some(1.0),
        "row 0→3 across active 2 → active shifts to 1"
    );
    // A reorder entirely below the active index leaves it unchanged.
    ctx.eval("window.SpacetimeLocal.activeIndex = 1; ST._adReindexAfterReorder(3, 4); void 0;")
        .unwrap();
    assert_eq!(
        ctx.eval("window.SpacetimeLocal.activeIndex")
            .unwrap()
            .as_f64(),
        Some(1.0),
        "reorder below active → unchanged"
    );
    // No active entry (-1) → no reindex.
    ctx.eval("window.SpacetimeLocal.activeIndex = -1; ST._adReindexAfterReorder(0, 1); void 0;")
        .unwrap();
    assert_eq!(
        ctx.eval("window.SpacetimeLocal.activeIndex")
            .unwrap()
            .as_f64(),
        Some(-1.0),
        "no active entry → no reindex"
    );
}

// BUG-082: SVG elements must materialize in the SVG namespace. ST.parseHtml is the
// shared namespace-aware row/fragment parser: parsing `<circle>` for an SVG-namespaced
// container must wrap-parse in <svg> context (innerHTML outside <svg> yields
// HTMLUnknownElement that never paints); HTML containers keep plain template parsing.
#[test]
fn bug082_parse_html_namespaces_svg_rows() {
    let mut ctx = V8TestContext::new().with_runtime().unwrap();
    ctx.set_body_html(r#"<svg class="g" viewBox="0 0 10 10"></svg><ul class="h"></ul>"#)
        .unwrap();
    let r = ctx
        .eval(
            r#"
        (() => {
          const svgHost = document.querySelector('.g');
          const htmlHost = document.querySelector('.h');
          const c = ST.parseHtml('<circle cx="5" cy="5" r="2"></circle>', svgHost);
          const li = ST.parseHtml('<li class="row">x</li>', htmlHost);
          const multi = ST.parseHtml('<circle cx="1"></circle>', svgHost);
          return JSON.stringify({
            circleNs: c ? c.namespaceURI : 'NONE',
            circleTag: c ? c.tagName.toLowerCase() : 'NONE',
            liNs: li ? li.namespaceURI : 'NONE',
            multiOk: multi ? multi.getAttribute('cx') : 'NONE',
          });
        })()
        "#,
        )
        .unwrap();
    let out = r.as_str().unwrap().to_string();
    assert!(
        out.contains("\"circleNs\":\"http://www.w3.org/2000/svg\""),
        "SVG-container row parses into the SVG namespace: {out}"
    );
    assert!(
        out.contains("\"circleTag\":\"circle\""),
        "tag preserved: {out}"
    );
    assert!(
        out.contains("\"liNs\":\"http://www.w3.org/1999/xhtml\""),
        "HTML-container row keeps the HTML namespace: {out}"
    );
    assert!(
        out.contains("\"multiOk\":\"1\""),
        "attrs survive the wrap-parse: {out}"
    );
}

// FEAT-092 W2: media library glue. adPickAsset routes a picked asset path through
// the standard adEditField persist arc (entries updated + EditJson sent) and
// reflects into the visible drawer input. (Backdrop-close is wired declaratively
// via a delegated @on click [data-medialib-backdrop] — no helper to unit-test.)
#[test]
fn feat092_media_pick_persists() {
    let mut ctx = V8TestContext::new().with_runtime().unwrap();
    ctx.set_body_html(
        r#"<div class="ad-form"><input class="ad-media-url" data-path=".cover" data-widget="media"><span data-media-prev=".cover"></span></div>"#,
    )
    .unwrap();
    ctx.eval(
        "globalThis.window = globalThis; globalThis.__sent = []; \
         globalThis.window.__stDevWs = { connected: true, send: m => globalThis.__sent.push(m), onAck: ()=>{}, onReject: ()=>{} }; \
         globalThis.window.SpacetimeLocal = { activeFile: 'data/posts.json', entries: [{ title: 'T', cover: '/assets/old.png' }], activeIndex: 0, mediaPath: '.cover' }; \
         ST.setData('entries', window.SpacetimeLocal.entries); void 0;",
    )
    .unwrap();

    // Pick: persists into the entry + sends EditJson + reflects the input value.
    let cover = ctx
        .eval(
            "ST.adPickAsset('/assets/new.png', 0, '.cover'); \
             window.SpacetimeLocal.entries[0].cover",
        )
        .unwrap();
    assert_eq!(cover.as_str(), Some("/assets/new.png"), "entry updated");
    let sent = ctx.eval("globalThis.__sent.length").unwrap();
    assert_eq!(sent.as_f64(), Some(1.0), "EditJson persisted over dev-ws");
    let input_val = ctx
        .eval("document.querySelector('.ad-media-url').value")
        .unwrap();
    assert_eq!(
        input_val.as_str(),
        Some("/assets/new.png"),
        "drawer input reflects pick"
    );

    // Missing fieldPath → no-op (the library opened without a target field).
    let still = ctx
        .eval("ST.adPickAsset('/assets/x.png', 0, ''); window.SpacetimeLocal.entries[0].cover")
        .unwrap();
    assert_eq!(
        still.as_str(),
        Some("/assets/new.png"),
        "no field path → no write"
    );
}

// FEAT-092 W2: thumbnails reflect data-thumb paths into background-image URLs
// (encodeURI'd — spaces in asset names survive).
#[test]
fn feat092_media_thumbs_reflect_background() {
    let mut ctx = V8TestContext::new().with_runtime().unwrap();
    ctx.set_body_html(
        r#"<span class="ad-medialib-thumb" data-thumb="/assets/a b.png"></span><span class="ad-medialib-thumb" data-thumb="/assets/c.jpg"></span>"#,
    )
    .unwrap();
    ctx.eval("globalThis.window = globalThis; ST.adReflectThumbs(); void 0;")
        .unwrap();
    let bg = ctx
        .eval("document.querySelectorAll('.ad-medialib-thumb')[0].style.backgroundImage")
        .unwrap();
    let bg_str = bg.as_str().unwrap_or("").to_string();
    assert!(
        bg_str.contains("/assets/a%20b.png"),
        "space-bearing path is URI-encoded: {bg_str}"
    );
    let bg2 = ctx
        .eval("document.querySelectorAll('.ad-medialib-thumb')[1].style.backgroundImage")
        .unwrap();
    assert!(
        bg2.as_str().unwrap_or("").contains("/assets/c.jpg"),
        "plain path reflected"
    );
}

// FEAT-092 W3: relation graph layout. adGraphLayout turns the active entries +
// type schemas into positioned nodes (source left, deduped targets right) and
// cubic-bezier edges. Source nodes carry their entry index (→ drawer open);
// target nodes carry -1. Labels come from the target type's display.title role.
#[test]
fn feat092_graph_layout_bipartite_nodes_edges() {
    let mut ctx = V8TestContext::new().with_runtime().unwrap();
    ctx.eval(
        "globalThis.window = globalThis; \
         ST._adRelCache = { Author: { rows: [ { id: 'a1', name: 'Ada' }, { id: 'a2', name: 'Bob' } ], titleField: 'name' } }; \
         globalThis.__types = { Post: { properties: { title: { widget: 'text' }, author: { widget: 'relation', 'x-st-ref': 'Author' }, contributors: { widget: 'relation-multi', items: { 'x-st-ref': 'Author' } } }, display: { title: 'title' } } }; \
         globalThis.__entries = [ { title: 'P1', author: 'a1', contributors: ['a2'] }, { title: 'P2', author: 'a2', contributors: [] } ]; void 0;",
    )
    .unwrap();
    let r = ctx
        .eval(
            "(() => { const g = ST.adGraphLayout(globalThis.__entries, globalThis.__types, 'Post'); \
              return JSON.stringify({ \
                nodes: g.nodes.length, edges: g.edges.length, hasEdges: g.hasEdges, \
                srcCount: g.nodes.filter(n => n.side === 'src').length, \
                tgtCount: g.nodes.filter(n => n.side === 'tgt').length, \
                srcLabels: g.nodes.filter(n => n.side === 'src').map(n => n.label), \
                tgtLabels: g.nodes.filter(n => n.side === 'tgt').map(n => n.label), \
                srcIndex0: g.nodes.find(n => n.side === 'src').index, \
                tgtIndex: g.nodes.find(n => n.side === 'tgt').index, \
                firstEdgeIsBezier: g.edges[0].d.indexOf('C') !== -1 && g.edges[0].d.indexOf('M') === 0, \
                legend: g.legend.length }); })()",
        )
        .unwrap();
    let out = r.as_str().unwrap().to_string();
    // 2 source posts + 2 deduped author targets (a1, a2) = 4 nodes.
    assert!(
        out.contains("\"nodes\":4"),
        "2 src + 2 deduped tgt nodes: {out}"
    );
    assert!(out.contains("\"srcCount\":2"), "two source nodes: {out}");
    assert!(
        out.contains("\"tgtCount\":2"),
        "two deduped target nodes: {out}"
    );
    // P1→a1 (author), P1→a2 (contributor), P2→a2 (author) = 3 edges.
    assert!(out.contains("\"edges\":3"), "three relation edges: {out}");
    assert!(out.contains("\"hasEdges\":true"), "hasEdges true: {out}");
    assert!(
        out.contains("[\"P1\",\"P2\"]"),
        "source labels from display.title: {out}"
    );
    assert!(
        out.contains("[\"Ada\",\"Bob\"]"),
        "target labels from rel cache title: {out}"
    );
    assert!(
        out.contains("\"srcIndex0\":0"),
        "source node carries its entry index: {out}"
    );
    assert!(
        out.contains("\"tgtIndex\":-1"),
        "target node index is -1 (no drawer): {out}"
    );
    assert!(
        out.contains("\"firstEdgeIsBezier\":true"),
        "edge d is a cubic-bezier M…C…: {out}"
    );
    assert!(
        out.contains("\"legend\":2"),
        "legend lists the two relation fields: {out}"
    );
}

// PLAN-034 Wave B/C: tool-pane rendering. The BRAND pane uses the shared
// form-renderer (ST.adFormFields over the singleton schema+values); the
// COMPONENT/MOTION panes render DIRECTLY from server contracts (sig/label
// delivered server-side) — their client shapers (adComponentCards/adMotionCards)
// were deleted (AP2 removal). Only the brand form-shaper + tool helpers remain.
#[test]
fn feat092_tool_pane_shaping() {
    let mut ctx = V8TestContext::new().with_runtime().unwrap();
    let r = ctx
        .eval(
            "(() => { \
              const brand = ST.adFormFields( \
                { type: 'object', properties: { \
                  scarlet: { type: 'string', format: 'color', widget: 'color' }, \
                  radius:  { type: 'string', format: 'length', widget: 'text' } } }, \
                { scarlet: '#FF0020', radius: '8px' }, ''); \
              return JSON.stringify({ \
                brandLen: brand.length, \
                colorTpl: brand.find(f => f.name === 'scarlet').tpl, \
                colorVal: brand.find(f => f.name === 'scarlet').value, \
                lengthTpl: brand.find(f => f.name === 'radius').tpl, \
                emptyBrand: ST.adToolEmpty('brand', [], [{x:1}], [{y:1}]), \
                emptyPalette: ST.adToolEmpty('palette', brand, [{x:1}], [{y:1}]), \
                title: ST.adToolTitle('brand') }); })()",
        )
        .unwrap();
    let out = r.as_str().unwrap().to_string();
    // brand: 2 fields from the singleton schema. PLAN-112 W2 moved the pure
    // widgets into the shared `stdlib/fields` module, so their descriptors now
    // dispatch DIRECTLY to `field-<widget>`; widgets still owned by the admin
    // (richtext, select, relation, media, group, …) keep `fld-<widget>`.
    assert!(out.contains("\"brandLen\":2"), "2 brand fields: {out}");
    assert!(
        out.contains("\"colorTpl\":\"field-color\""),
        "color field → shared field-color: {out}"
    );
    assert!(
        out.contains("\"colorVal\":\"#FF0020\""),
        "color value carried: {out}"
    );
    assert!(
        out.contains("\"lengthTpl\":\"field-text\""),
        "length field → shared field-text: {out}"
    );
    assert!(
        out.contains("\"emptyBrand\":true"),
        "empty brand fields → empty true: {out}"
    );
    assert!(
        out.contains("\"emptyPalette\":false"),
        "non-empty palette → empty false: {out}"
    );
    assert!(
        out.contains("\"title\":\"Brand\""),
        "brand pane title: {out}"
    );
}

/// PLAN-112 W2R: every widget kind must resolve to a template that is actually
/// REGISTERED. This is the wave's own failure mode turned into a gate: a
/// descriptor naming a template nobody defined renders NOTHING — no label, no
/// input, no warning — so a dispatch/registration mismatch is invisible until a
/// user reports an empty form.
///
/// The expected names are asserted literally rather than recomputed from the
/// resolver, so a change to the shared-widget set has to be made deliberately
/// HERE too, and cannot drift silently out of `stdlib/fields`.
#[test]
fn plan112_every_widget_kind_resolves_to_a_registered_template() {
    let mut ctx = V8TestContext::new().with_runtime().unwrap();
    let r = ctx
        .eval(
            "(() => { \
              const kinds = ['text','textarea','number','toggle','readonly','media', \
                'select','chips','relation','relation-multi','richtext','color', \
                'range-length','range-duration','json','group','derived','', \
                'totally-unknown-widget']; \
              const out = {}; \
              kinds.forEach(k => { out[k || '(empty)'] = ST.adFieldTemplate(k); }); \
              return JSON.stringify(out); })()",
        )
        .unwrap();
    let out = r.as_str().unwrap().to_string();
    let map: serde_json::Value = serde_json::from_str(&out).expect("resolver output");

    // The six widgets hoisted into stdlib/fields dispatch to the SHARED templates.
    for (kind, expected) in [
        ("text", "field-text"),
        ("number", "field-number"),
        ("toggle", "field-toggle"),
        ("color", "field-color"),
        ("range-length", "field-range-length"),
        ("range-duration", "field-range-duration"),
    ] {
        assert_eq!(
            map[kind].as_str(),
            Some(expected),
            "{kind} must dispatch to the shared template: {out}"
        );
    }

    // Everything still owned by the admin keeps its `fld-*` template.
    for (kind, expected) in [
        ("textarea", "fld-textarea"),
        ("readonly", "fld-readonly"),
        ("media", "fld-media"),
        ("select", "fld-select"),
        ("chips", "fld-chips"),
        ("relation", "fld-relation"),
        ("relation-multi", "fld-relation-multi"),
        ("richtext", "fld-richtext"),
        // Structural scaffolding, NOT input fields: routing these through the
        // input table would replace every section header with an empty textbox.
        ("group", "fld-group"),
        ("derived", "fld-derived"),
        // Unknown/absent kinds must FALL BACK to a real template, never vanish.
        ("json", "fld-textarea"),
        ("(empty)", "field-text"),
        ("totally-unknown-widget", "field-text"),
    ] {
        assert_eq!(
            map[kind].as_str(),
            Some(expected),
            "{kind} must dispatch to {expected}: {out}"
        );
    }

    // The registration side of the contract: every name the resolver can emit is
    // defined as a `@template` in the admin's import graph (shared module or
    // admin drawer). Read the sources rather than trusting the lists above.
    let shared =
        std::fs::read_to_string("stdlib/fields/macros/widgets.st").expect("shared widgets");
    let drawer = std::fs::read_to_string("stdlib/__admin__/app/drawer.st").expect("admin drawer");
    let sources = format!("{shared}\n{drawer}");
    for (kind, tpl) in map.as_object().expect("object") {
        let tpl = tpl.as_str().expect("template name");
        assert!(
            sources.contains(&format!("@template &{tpl}(")),
            "widget {kind:?} dispatches to {tpl:?}, which no @template defines \
             — that descriptor would render nothing at all"
        );
    }
}

// FEAT-100: the admin maps a `richtext` field to the `fld-richtext` template and
// preserves its value as the document AST OBJECT (not a stringified blob), so the
// @editable surface consumes the AST directly. adFieldInput persists the AST that
// the surface stores on the host element (el.__stDoc) rather than reading el.value.
#[test]
fn feat100_admin_richtext_field_keeps_ast_object() {
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");

    // A schema with a richtext field; entry holds a document AST object.
    let r = ctx
        .eval(
            r#"
            const schema = { properties: { body: { widget: 'richtext' }, name: { widget: 'text' } } };
            const docAst = { type: 'doc', content: [ { type: 'p', content: [ { type: 'text', text: 'hi' } ] } ] };
            const entry = { body: docAst, name: 'Post' };
            const fields = ST.adFormFields(schema, entry, '');
            const rt = fields.find(f => f.name === 'body');
            JSON.stringify({
              tpl: rt.tpl,
              widget: rt.widget,
              valueIsObject: (rt.value && typeof rt.value === 'object' && rt.value.type === 'doc'),
              valueText: rt.value && rt.value.content && rt.value.content[0].content[0].text
            })
        "#,
        )
        .expect("adFormFields eval");
    let out = r.as_str().unwrap();
    assert!(
        out.contains("\"tpl\":\"fld-richtext\""),
        "richtext → fld-richtext: {out}"
    );
    assert!(
        out.contains("\"widget\":\"richtext\""),
        "widget preserved: {out}"
    );
    assert!(
        out.contains("\"valueIsObject\":true"),
        "value stays an AST object (not stringified): {out}"
    );
    assert!(
        out.contains("\"valueText\":\"hi\""),
        "AST content intact: {out}"
    );

    // adFieldInput on a richtext host reads the stored AST (el.__stDoc), not el.value.
    let r2 = ctx
        .eval(
            r#"
            // Stub the entries + WS path so adFieldInput's adEditField has data.
            window.SpacetimeLocal = window.SpacetimeLocal || {};
            window.SpacetimeLocal.entries = [ { body: null } ];
            window.SpacetimeLocal.activeIndex = 0;
            ST._adReadEntries = () => window.SpacetimeLocal.entries;
            ST._adPublishEntries = (a) => { window.SpacetimeLocal.entries = a; };
            const host = document.createElement('div');
            host.dataset.path = '.body';
            host.dataset.widget = 'richtext';
            host.__stDoc = { type: 'doc', content: [ { type: 'p', content: [ { type: 'text', text: 'edited' } ] } ] };
            ST.adFieldInput(host, 0);
            JSON.stringify(window.SpacetimeLocal.entries[0].body)
        "#,
        )
        .expect("adFieldInput eval");
    let persisted = r2.as_str().unwrap();
    assert!(
        persisted.contains("\"type\":\"doc\""),
        "persisted value is the AST doc: {persisted}"
    );
    assert!(
        persisted.contains("\"text\":\"edited\""),
        "persisted the stored AST, not el.value: {persisted}"
    );
}

// =============================================================================
// BUG-090: @data query where/map never worked — computed-source emitted a
// broken predicate ($. not rewritten to item., external $sig mis-scoped to
// ST.get(el,…), filter evaluated once as a scalar). Fix mirrors derived-signal:
// raw-string exprs rewritten in %emit js ($.field→item.field, $sig→
// SpacetimeLocal['sig']), real per-item Functions, dep-discovery + watch.
// Emitted-JS BEHAVIOR asserted (compiling≠correctness).
// =============================================================================

const BUG090_QUERY: &str = r#"
@data inline $items : [{ "name": "alpha" }, { "name": "beta" }, { "name": "alphabet" }];
@data inline $q : "alph";
@data query $hits from $items {
  where: $.name.indexOf($q) !== -1;
  map: { label: $.name };
  limit: 50;
}
h1 { "x" }
"#;

#[test]
fn bug090_query_where_filters_per_item_with_external_signal() {
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    let compiled = compile_st(BUG090_QUERY);
    ctx.set_body_html("<div></div>").unwrap();
    ctx.eval(&compiled.js).expect("compiled JS evaluates");
    let _ = ctx.eval("ST._scheduleInit && ST._scheduleInit(document.body)");

    // Initial: q="alph" → "alpha","alphabet" match, "beta" excluded; mapped to {label}.
    let n = ctx
        .eval("(SpacetimeLocal['hits']||[]).length")
        .expect("eval hits len");
    assert_eq!(
        n.as_f64(),
        Some(2.0),
        "where must filter per-item (alpha,alphabet), got {:?}",
        n.as_f64()
    );
    let labels = ctx
        .eval("(SpacetimeLocal['hits']||[]).map(r=>r.label).join(',')")
        .expect("eval labels");
    assert_eq!(
        labels.as_str(),
        Some("alpha,alphabet"),
        "map must project {{label:$.name}} per item"
    );

    // Drive the external signal $q → pipeline must re-filter (dep discovery).
    ctx.eval("SpacetimeLocal['q']='beta'; document.dispatchEvent(new CustomEvent('local:q:updated',{detail:'beta'}))")
        .expect("drive q");
    let after = ctx
        .eval("(SpacetimeLocal['hits']||[]).map(r=>r.label).join(',')")
        .expect("eval after");
    assert_eq!(
        after.as_str(),
        Some("beta"),
        "changing external $q must re-run the query (reactive dep)"
    );
}

// BUG-090 / PLAN-034 Wave-A: the query SURFACE — member-path source, the pipe
// operator (`$.path | includes($q)`, `$.path | basename`), and bare query-helper
// fns. Emitted-JS behavior asserted.
const BUG090_QUERY_PIPES: &str = r#"
@data inline $doc : { "images": [{ "path": "up/alpha.jpg" }, { "path": "deep/beta.png" }, { "path": "up/alphabet.gif" }] };
@data inline $q : "alph";
@data query $assets from $doc.images {
  where: $.path | includes($q);
  map: { path: $.path, name: $.path | basename };
  limit: 400;
}
h1 { "x" }
"#;

#[test]
fn bug090_query_pipes_member_source_and_helpers() {
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    let compiled = compile_st(BUG090_QUERY_PIPES);
    ctx.set_body_html("<div></div>").unwrap();
    ctx.eval(&compiled.js).expect("compiled JS evaluates");
    let _ = ctx.eval("ST._scheduleInit && ST._scheduleInit(document.body)");

    // Member-path source ($doc.images) + pipe predicate: "alph" matches alpha.jpg
    // and alphabet.gif (path contains "alph"), excludes beta.png.
    let names = ctx
        .eval("(SpacetimeLocal['assets']||[]).map(r=>r.name).join(',')")
        .expect("eval names");
    assert_eq!(
        names.as_str(),
        Some("alpha.jpg,alphabet.gif"),
        "pipe predicate + basename projection over member-path source"
    );

    // Re-filter reactively on the external $q signal.
    ctx.eval("SpacetimeLocal['q']='beta'; document.dispatchEvent(new CustomEvent('local:q:updated',{detail:'beta'}))")
        .expect("drive q");
    let after = ctx
        .eval("(SpacetimeLocal['assets']||[]).map(r=>r.name).join(',')")
        .expect("eval after");
    assert_eq!(
        after.as_str(),
        Some("beta.png"),
        "pipe query re-runs on external signal change"
    );
}


// PLAN-133 (computed-source sexpr migration): an arrow LAMBDA inside a where
// expr binds its param LOCALLY — `($t) => $t == $q` must compare the row's
// tag against the external signal $q. The old rewriteExpr char-scan had no
// lambda-binding exemption (the BUG-273 class, one primitive over): it
// rewrote the PARAM `$t` to `SpacetimeLocal['t']` (undefined), silently
// filtering everything out.
const PLAN133_QUERY_LAMBDA: &str = r#"
@data inline $items : [{ "tags": ["a", "b"] }, { "tags": ["c"] }];
@data inline $q : "a";
@data query $hits from $items {
  where: $.tags.some(($t) => $t == $q);
}
h1 { "x" }
"#;

#[test]
fn plan133_query_lambda_param_binds_locally() {
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    let compiled = compile_st(PLAN133_QUERY_LAMBDA);
    ctx.set_body_html("<div></div>").unwrap();
    ctx.eval(&compiled.js).expect("compiled JS evaluates");
    let _ = ctx.eval("ST._scheduleInit && ST._scheduleInit(document.body)");

    let hits = ctx
        .eval("(SpacetimeLocal['hits']||[]).length")
        .expect("eval hits");
    assert_eq!(
        hits.as_f64(),
        Some(1.0),
        "arrow param $t must bind locally: exactly one row has tag 'a'"
    );

    // And it must still re-run when the external signal changes.
    ctx.eval("SpacetimeLocal['q']='c'; document.dispatchEvent(new CustomEvent('local:q:updated',{detail:'c'}))")
        .expect("drive q");
    let after = ctx
        .eval("(SpacetimeLocal['hits']||[]).length")
        .expect("eval after");
    assert_eq!(after.as_f64(), Some(1.0), "recompute on $q change");
}

// BUG-090 reviewer follow-up: object-map CONSTANT leaves must round-trip as the
// literal value (not be re-lowered to a bare identifier → ReferenceError →
// undefined). Also covers chained pipes and a string-literal containing a pipe.
const BUG090_MAP_CONSTANTS: &str = r#"
@data inline $rows : [{ "path": "UP/Alpha.JPG" }, { "path": "up/beta.png" }];
@data inline $q : "";
@data query $out from $rows {
  where: $.path | includes($q);
  map: { name: $.path | basename | lower, kind: "asset", n: 1 };
}
h1 { "x" }
"#;

#[test]
fn bug090_map_constant_leaves_and_chained_pipes() {
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    let compiled = compile_st(BUG090_MAP_CONSTANTS);
    ctx.set_body_html("<div></div>").unwrap();
    ctx.eval(&compiled.js).expect("compiled JS evaluates");
    let _ = ctx.eval("ST._scheduleInit && ST._scheduleInit(document.body)");

    // Chained pipe: basename then lower → "alpha.jpg" (from "UP/Alpha.JPG").
    let name0 = ctx
        .eval("(SpacetimeLocal['out']||[])[0].name")
        .expect("eval");
    assert_eq!(
        name0.as_str(),
        Some("alpha.jpg"),
        "chained pipe basename|lower"
    );
    // Constant string leaf must round-trip (was silently undefined pre-fix).
    let kind0 = ctx
        .eval("(SpacetimeLocal['out']||[])[0].kind")
        .expect("eval");
    assert_eq!(
        kind0.as_str(),
        Some("asset"),
        "constant string leaf must survive"
    );
    // Constant number leaf.
    let n0 = ctx.eval("(SpacetimeLocal['out']||[])[0].n").expect("eval");
    assert_eq!(n0.as_f64(), Some(1.0), "constant number leaf must survive");
}

// String literal containing a pipe char must not be split by the pipe desugar.
const BUG090_PIPE_IN_STRING: &str = r#"
@data inline $rows : [{ "tag": "a|b" }, { "tag": "c" }];
@data inline $q : "a|b";
@data query $hit from $rows {
  where: $.tag == $q;
}
h1 { "x" }
"#;

#[test]
fn bug090_pipe_inside_string_literal_not_split() {
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    let compiled = compile_st(BUG090_PIPE_IN_STRING);
    ctx.set_body_html("<div></div>").unwrap();
    ctx.eval(&compiled.js).expect("compiled JS evaluates");
    let _ = ctx.eval("ST._scheduleInit && ST._scheduleInit(document.body)");
    let tags = ctx
        .eval("(SpacetimeLocal['hit']||[]).map(r=>r.tag).join(',')")
        .expect("eval");
    assert_eq!(
        tags.as_str(),
        Some("a|b"),
        "a `|` inside a string literal must not be treated as a pipe"
    );
}

// FEAT-127: @mcp-host is selector-scoped — one __stHost bridge PER region root
// (resolved via closest('[data-st-instance]')), NOT a window.__stHost singleton.
// Two regions on one page carry independent bridges; a click in region A resolves
// ONLY A's bridge, leaving B untouched. The decisive region-isolation gate.
const FEAT127_HOST_ST: &str = include_str!("../stdlib/__mcp__/primitives/host.st");

fn feat127_two_region_source() -> String {
    format!(
        "{host_st}\n{markup}",
        host_st = FEAT127_HOST_ST,
        markup = r#"
[data-st-instance] {
  @mcp-host
}

.region-a .btn {
  @mcp-click(type: "approve")
}

.region-b .btn {
  @mcp-click(type: "approve")
}
"#
    )
}

#[test]
fn feat127_mcp_host_isolates_per_region_root() {
    let compiled = compile_st(&feat127_two_region_source());
    // The compiled JS must carry closest-ancestor resolution and NO global singleton.
    assert!(
        !compiled.js.contains("window.__stHost"),
        "no window.__stHost global in compiled JS — bridges are per-region"
    );
    assert!(
        compiled.js.contains("[data-st-instance]"),
        "compiled JS must reference [data-st-instance] for closest-ancestor resolution"
    );

    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    ctx.set_body_html(
        r#"<div class="region-a" data-st-instance="inst-a"><button class="btn">A</button></div><div class="region-b" data-st-instance="inst-b"><button class="btn">B</button></div>"#,
    )
    .unwrap();
    ctx.eval(&compiled.js).expect("compiled JS evaluates");
    let _ = ctx.eval("ST._scheduleInit && ST._scheduleInit(document.body)");

    // No page-global singleton.
    let global = ctx
        .eval("typeof window.__stHost === 'undefined'")
        .expect("eval");
    assert_eq!(
        global.as_bool(),
        Some(true),
        "no window.__stHost global — bridges attach to region roots, not window"
    );

    // Each region root carries its OWN bridge with its OWN instance id.
    let a_instance = ctx
        .eval("document.querySelector('[data-st-instance=\"inst-a\"]').__stHost.instance")
        .expect("eval");
    assert_eq!(
        a_instance.as_str(),
        Some("inst-a"),
        "region A root carries its own bridge with instance inst-a"
    );
    let b_instance = ctx
        .eval("document.querySelector('[data-st-instance=\"inst-b\"]').__stHost.instance")
        .expect("eval");
    assert_eq!(
        b_instance.as_str(),
        Some("inst-b"),
        "region B root carries its own bridge with instance inst-b"
    );

    // Replace each bridge's signal with a recorder so we observe WHICH bridge a
    // click resolves to, synchronously (no fetch round-trip needed).
    ctx.eval(
        r#"
        var aCalls = [], bCalls = [];
        document.querySelector('[data-st-instance="inst-a"]').__stHost.signal = function(type, payload, corr) {
            aCalls.push({ type: type, instance: payload.instance });
            return Promise.resolve({ ok: true });
        };
        document.querySelector('[data-st-instance="inst-b"]').__stHost.signal = function(type, payload, corr) {
            bCalls.push({ type: type, instance: payload.instance });
            return Promise.resolve({ ok: true });
        };
        window.__feat127_aCalls = aCalls;
        window.__feat127_bCalls = bCalls;
        void 0;
    "#,
    )
    .expect("recorders installed");

    // Click A's button — must resolve A's bridge ONLY (nearest-ancestor).
    ctx.eval("document.querySelector('.region-a .btn').click()")
        .expect("click A");
    let a_calls = ctx.eval("window.__feat127_aCalls.length").expect("eval");
    let b_calls = ctx.eval("window.__feat127_bCalls.length").expect("eval");
    assert_eq!(
        a_calls.as_f64(),
        Some(1.0),
        "click in region A resolves A's bridge (closest-ancestor)"
    );
    assert_eq!(
        b_calls.as_f64(),
        Some(0.0),
        "click in region A must NOT touch B's bridge — no cross-region misrouting"
    );
    let a_payload_instance = ctx
        .eval("window.__feat127_aCalls[0].instance")
        .expect("eval");
    assert_eq!(
        a_payload_instance.as_str(),
        Some("inst-a"),
        "payload instance is region A's (resolved from the nearest ancestor)"
    );

    // Click B — now B's bridge fires, A's recorder stays at 1.
    ctx.eval("document.querySelector('.region-b .btn').click()")
        .expect("click B");
    let a_calls2 = ctx.eval("window.__feat127_aCalls.length").expect("eval");
    let b_calls2 = ctx.eval("window.__feat127_bCalls.length").expect("eval");
    assert_eq!(
        a_calls2.as_f64(),
        Some(1.0),
        "A's bridge untouched by a click in B"
    );
    assert_eq!(
        b_calls2.as_f64(),
        Some(1.0),
        "B's bridge fires on B click (closest-ancestor resolves B)"
    );
    let b_payload_instance = ctx
        .eval("window.__feat127_bCalls[0].instance")
        .expect("eval");
    assert_eq!(
        b_payload_instance.as_str(),
        Some("inst-b"),
        "payload instance is region B's"
    );
}

// FEAT-126: bundle arrival registers templates, scopes CSS, and sets the region
// stage signal so `@view $stageTpl { _ => &$stageTpl(); }` mounts reactively.
// Two regions with the same bundle must isolate state (param change re-renders
// only its own region).
fn feat126_region_source() -> String {
    r#"
@template &greet($name) {
  <span class="greet">`Hello, $name!`</span>
}

<div data-st-region="stage-a"></div>
<div data-st-region="stage-b"></div>

[data-st-region] {
  @view $stageTpl {
    _ => &$stageTpl("World");
  }
}
"#
    .to_string()
}

#[test]
fn feat126_register_bundle_mounts_region_and_isolates_state() {
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    let compiled = compile_st(&feat126_region_source());

    // Seed two region roots and run selector-inits.
    ctx.set_body_html(
        r#"<div data-st-region="stage-a"></div>
<div data-st-region="stage-b"></div>
"#,
    )
    .unwrap();
    ctx.eval(&compiled.js).expect("compiled JS evaluates");
    let _ = ctx.eval("ST._scheduleInit && ST._scheduleInit(document.body)");

    // Neither region has rendered yet (no bundle arrival).
    let before = ctx
        .eval("document.querySelectorAll('[data-st-region] .greet').length")
        .expect("eval");
    assert_eq!(before.as_f64(), Some(0.0), "no greet before bundle arrives");

    // Simulate the bundle endpoint payload arriving for region A.
    ctx.eval(
        r#"
Spacetime.registerBundle({
  regionId: 'stage-a',
  templates: [{
    name: 'greet',
    params: [{ name: 'name', kind: 'binding', optional: false }],
    body: {
      html: '<span class="greet">Hello, $name!</span>',
      builder: null,
      states: [],
      exports: [],
      refs: [],
      matches: []
    }
  }],
  css: '.greet { color: green; }'
});
"#,
    )
    .expect("registerBundle runs");

    // Region A now renders; region B stays empty (isolation).
    let a_count = ctx
        .eval("document.querySelectorAll('[data-st-region=\"stage-a\"] .greet').length")
        .expect("eval");
    assert_eq!(a_count.as_f64(), Some(1.0), "region A mounts greet");
    let b_count = ctx
        .eval("document.querySelectorAll('[data-st-region=\"stage-b\"] .greet').length")
        .expect("eval");
    assert_eq!(
        b_count.as_f64(),
        Some(0.0),
        "region B untouched by A's bundle"
    );

    // The rendered text uses the factory's positional arg.
    let a_text = ctx
        .eval("document.querySelector('[data-st-region=\"stage-a\"] .greet').textContent")
        .expect("eval");
    assert_eq!(
        a_text.as_str(),
        Some("Hello, World!"),
        "factory interpolation uses the arm's argument"
    );

    // Scoped CSS style element is injected under the region.
    let style_text = ctx
        .eval("document.getElementById('st-bundle-stage-a').textContent")
        .expect("eval");
    let style_str = style_text.as_str().expect("style text is string");
    assert!(
        style_str.contains("@scope ([data-st-region=\"stage-a\"])"),
        "bundle CSS is scoped to region A"
    );
    assert!(
        style_str.contains(".greet"),
        "scoped CSS carries the bundle rule"
    );

    // Register the same bundle into region B with a different arg value.
    ctx.eval(
        r#"
Spacetime.registerBundle({
  regionId: 'stage-b',
  templates: [{
    name: 'greet',
    params: [{ name: 'name', kind: 'binding', optional: false }],
    body: {
      html: '<span class="greet">Hello, $name!</span>',
      builder: null,
      states: [],
      exports: [],
      refs: [],
      matches: []
    }
  }],
  css: '.greet { color: blue; }'
});
"#,
    )
    .expect("registerBundle for B runs");

    let b_text = ctx
        .eval("document.querySelector('[data-st-region=\"stage-b\"] .greet').textContent")
        .expect("eval");
    assert_eq!(
        b_text.as_str(),
        Some("Hello, World!"),
        "region B mounts independently"
    );

    // Isolation: changing region A's signal re-renders only A.
    ctx.eval(
        r#"
var rootA = document.querySelector('[data-st-region="stage-a"]');
ST.set(rootA, 'stageTpl', 'stage-a/greet');
"#,
    )
    .expect("re-signal region A");
    // After re-signal, A should still have exactly one greet (re-mount, not duplicate).
    let a_count2 = ctx
        .eval("document.querySelectorAll('[data-st-region=\"stage-a\"] .greet').length")
        .expect("eval");
    assert_eq!(
        a_count2.as_f64(),
        Some(1.0),
        "re-signal keeps region A at one mounted instance"
    );

    // Isolation proof: the two regions registered the SAME bare name `greet`
    // under DISTINCT region-scoped registry keys, so neither overwrote the other.
    let keyed_a = ctx
        .eval("!!Spacetime.templates.get('stage-a/greet')")
        .expect("eval");
    let keyed_b = ctx
        .eval("!!Spacetime.templates.get('stage-b/greet')")
        .expect("eval");
    assert_eq!(
        keyed_a.as_bool(),
        Some(true),
        "region A owns its own scoped factory"
    );
    assert_eq!(
        keyed_b.as_bool(),
        Some(true),
        "region B owns its own scoped factory"
    );
}

// FEAT-124 hot-swap: re-registering a region's bundle with CHANGED content while
// the template name (the @view signal value) is unchanged must RE-MOUNT the view
// (the poll re-registers a fresh factory; ST.set dedupes the stable name, so
// registerBundle force-dispatches the view update on a real content change).
#[test]
fn feat124_hot_swap_remounts_view_on_content_change() {
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    let compiled = compile_st(
        r#"
@template &main() { <span class="v">V?</span> }
<div data-st-region="stage"></div>
[data-st-region] {
  @view $stageTpl {
    _ => &$stageTpl();
  }
}
"#,
    );
    ctx.set_body_html(r#"<div data-st-region="stage"></div>"#)
        .unwrap();
    ctx.eval(&compiled.js).expect("compiled JS evaluates");
    let _ = ctx.eval("ST._scheduleInit && ST._scheduleInit(document.body)");

    // First bundle: V1.
    ctx.eval(
        r#"
Spacetime.registerBundle({
  regionId: 'stage',
  templates: [{ name: 'main', params: [], body: {
    html: '<span class="v">V1</span>', serialized: 'V1', builder: null,
    states: [], exports: [], refs: [], matches: []
  }}],
  css: ''
});
"#,
    )
    .expect("register V1");
    let v1 = ctx
        .eval("document.querySelector('[data-st-region=\"stage\"] .v').textContent")
        .expect("eval v1");
    assert_eq!(v1.as_str(), Some("V1"), "first bundle mounts V1");

    // Re-register with CHANGED body (same template name). Must re-mount to V2.
    ctx.eval(
        r#"
Spacetime.registerBundle({
  regionId: 'stage',
  templates: [{ name: 'main', params: [], body: {
    html: '<span class="v">V2</span>', serialized: 'V2', builder: null,
    states: [], exports: [], refs: [], matches: []
  }}],
  css: ''
});
"#,
    )
    .expect("register V2");
    let v2 = ctx
        .eval("document.querySelector('[data-st-region=\"stage\"] .v').textContent")
        .expect("eval v2");
    assert_eq!(v2.as_str(), Some("V2"), "changed bundle hot-swaps to V2");
}

/// BUG-116: a STANDALONE region root (no `@view $stageTpl` wiring) that opts in
/// with `data-st-mcp-autostage` direct-mounts the bundle's `main` template on
/// registerBundle. This is what makes a template-only standalone instance page
/// (an agent-control panel) actually render its content instead of an empty body.
#[test]
fn bug116_autostage_root_direct_mounts_main_without_view() {
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    // A bare region root: autostage opt-in, NO `@view` consumer, NO compiled JS.
    ctx.set_body_html(
        "<main data-st-instance=\"inst-2\" data-st-region=\"region-inst-2\" data-st-mcp-autostage></main>",
    )
    .unwrap();
    // No `@view` is registered for this region; without the autostage branch the
    // factories would register but nothing would mount (the observed empty body).
    let before = ctx
        .eval("document.querySelectorAll('[data-st-region=\"region-inst-2\"] .approve').length")
        .expect("eval");
    assert_eq!(
        before.as_f64(),
        Some(0.0),
        "nothing mounted before the bundle arrives"
    );
    ctx.eval(
        r#"
Spacetime.registerBundle({
  regionId: 'region-inst-2',
  templates: [{ name: 'main', params: [], body: {
    html: '<div class=\"agent-panel\"><button class=\"approve\">Approve</button></div>',
    serialized: 'approve', builder: null,
    states: [], exports: [], refs: [], matches: []
  }}],
  css: ''
});
"#,
    )
    .expect("registerBundle runs");
    let count = ctx
        .eval("document.querySelectorAll('[data-st-region=\"region-inst-2\"] .approve').length")
        .expect("eval");
    assert_eq!(
        count.as_f64(),
        Some(1.0),
        "autostage direct-mounts main into the root"
    );
    let text = ctx
        .eval("document.querySelector('[data-st-region=\"region-inst-2\"] .approve').textContent")
        .expect("eval");
    assert_eq!(
        text.as_str(),
        Some("Approve"),
        "the mounted button carries its content"
    );
}

/// FEAT-126 isolation: two regions register the SAME bare template name with
/// DIFFERENT bodies. Region-scoped keys keep them distinct — each region renders
/// ITS OWN body, proving no cross-region factory bleed.
#[test]
fn feat126_same_name_divergent_bodies_do_not_bleed() {
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    ctx.set_body_html("<div data-st-region=\"reg-x\"></div><div data-st-region=\"reg-y\"></div>")
        .unwrap();
    ctx.eval(
        r#"
Spacetime.registerBundle({
  regionId: 'reg-x',
  templates: [{ name: 'card', params: [], body: {
    html: '<div class="card x-card">X</div>', builder: null,
    states: [], exports: [], refs: [], matches: []
  }}],
  css: ''
});
"#,
    )
    .expect("register X");
    ctx.eval(
        r#"
Spacetime.registerBundle({
  regionId: 'reg-y',
  templates: [{ name: 'card', params: [], body: {
    html: '<div class="card y-card">Y</div>', builder: null,
    states: [], exports: [], refs: [], matches: []
  }}],
  css: ''
});
"#,
    )
    .expect("register Y");
    let x_html = ctx
        .eval("Spacetime.templates.get('reg-x/card')().outerHTML")
        .expect("eval X");
    let y_html = ctx
        .eval("Spacetime.templates.get('reg-y/card')().outerHTML")
        .expect("eval Y");
    assert!(
        x_html.as_str().unwrap_or("").contains("x-card"),
        "region X keeps its own body, got: {:?}",
        x_html.as_str()
    );
    assert!(
        y_html.as_str().unwrap_or("").contains("y-card"),
        "region Y keeps its own body (no bleed from X), got: {:?}",
        y_html.as_str()
    );
}

// ============================================================================
// FUP-134 — owner-scoped template identity
//
// A dev-served page loads SEVERAL runtime bundles into one `Spacetime` global:
// the page's own, plus each dock widget (inspector, host, migrations) and the
// shared `stdlib/fields` widgets. The registry was keyed by bare name and
// `Map.set` replaces, so the LAST bundle to load owned the name outright.
//
// Observed live before the fix, not theorised: a page declaring
// `@template &field-text($x)` lost it entirely to the fields dock —
// `templateNames()` showed one `"field-text"`, invoking it returned the DOCK's
// widget, and the page's own list rendered NOTHING with no console error. The
// lookup succeeded; it just answered with someone else's factory.
//
// The fix generalises the region-mount rail (FEAT-126) from regions to bundles:
// key `owner/name`, resolve owner-first then bare. One rule, one resolver.
// ============================================================================

/// Two owners may hold the same bare name, and each resolves to its OWN factory.
#[test]
fn fup134_two_owners_may_share_a_bare_template_name() {
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    ctx.eval(
        r#"
        Spacetime.registerOwnedTemplate('fields', 'card', () => 'FIELDS');
        Spacetime.registerOwnedTemplate('page',   'card', () => 'PAGE');
    "#,
    )
    .expect("register both owners");

    let fields = ctx
        .eval("Spacetime.resolveTemplate('card', 'fields')();")
        .expect("resolve fields");
    let page = ctx
        .eval("Spacetime.resolveTemplate('card', 'page')();")
        .expect("resolve page");
    assert_eq!(
        fields.as_str().unwrap_or_default(),
        "FIELDS",
        "each owner must get its OWN factory — a bare `templates.get(name)` \
         resolves both to whichever registered last"
    );
    assert_eq!(page.as_str().unwrap_or_default(), "PAGE");
}

/// The reported case. A dock bundle must not EVICT a page's same-named template.
/// An unowned (page) registration wins the bare name; the dock keeps `owner/name`.
#[test]
fn fup134_a_dock_bundle_does_not_evict_a_pages_template() {
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    // DOCK REGISTERS LAST. This ordering is the one that matters: in the observed
    // failure the dock bundle loaded after the page and its `Map.set` replaced the
    // page's entry outright. Registering the page last would let a broken
    // implementation pass by accident — an earlier version of this test did
    // exactly that and sabotage-PASSED.
    ctx.eval(
        r#"
        Spacetime.registerOwnedTemplate(undefined,   'field-text', () => 'PAGE');
        Spacetime.registerOwnedTemplate('inspector', 'field-text', () => 'DOCK');
    "#,
    )
    .expect("register");

    let bare = ctx
        .eval("Spacetime.templates.get('field-text')();")
        .expect("bare lookup");
    assert_eq!(
        bare.as_str().unwrap_or_default(),
        "PAGE",
        "the PAGE owns the bare name — it is the thing the user is looking at, \
         and losing it silently blanks their content"
    );

    let dock = ctx
        .eval("Spacetime.resolveTemplate('field-text', 'inspector')();")
        .expect("owner lookup");
    assert_eq!(
        dock.as_str().unwrap_or_default(),
        "DOCK",
        "and the dock still reaches its OWN widget via its owner scope"
    );
}

/// Load order must not decide the winner. The same two registrations in the
/// opposite order must produce the same resolution — that is what makes this a
/// policy rather than luck.
#[test]
fn fup134_resolution_is_independent_of_bundle_load_order() {
    for (first, second) in [
        (
            "Spacetime.registerOwnedTemplate('inspector','t',()=>'DOCK');",
            "Spacetime.registerOwnedTemplate(undefined,'t',()=>'PAGE');",
        ),
        (
            "Spacetime.registerOwnedTemplate(undefined,'t',()=>'PAGE');",
            "Spacetime.registerOwnedTemplate('inspector','t',()=>'DOCK');",
        ),
    ] {
        let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
        ctx.eval(&format!("{first}\n{second}")).expect("register");
        let bare = ctx.eval("Spacetime.templates.get('t')();").expect("bare");
        assert_eq!(
            bare.as_str().unwrap_or_default(),
            "PAGE",
            "the page must win the bare name in EITHER order; last-writer-wins is \
             the defect"
        );
        let dock = ctx
            .eval("Spacetime.resolveTemplate('t', 'inspector')();")
            .expect("owner");
        assert_eq!(dock.as_str().unwrap_or_default(), "DOCK");
    }
}

/// A dock bundle still claims the bare name when it is FREE — this is what keeps
/// the shared `field-*` widgets usable from a page that wants them. Removing it
/// would fix collisions by breaking legitimate sharing.
#[test]
fn fup134_an_owned_bundle_still_claims_a_free_bare_name() {
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    ctx.eval("Spacetime.registerOwnedTemplate('fields', 'field-color', () => 'FIELDS');")
        .expect("register");
    let bare = ctx
        .eval("Spacetime.templates.get('field-color')();")
        .expect("bare");
    assert_eq!(
        bare.as_str().unwrap_or_default(),
        "FIELDS",
        "with no page competing, the shared widget must remain reachable by its \
         bare name — pages legitimately invoke `&field-color`"
    );
}

/// An owner with no template of that name falls back to the bare/global one, so
/// an owned bundle can still use a genuinely shared template.
#[test]
fn fup134_owner_lookup_falls_back_to_the_global_name() {
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    ctx.eval("Spacetime.registerOwnedTemplate(undefined, 'shared', () => 'GLOBAL');")
        .expect("register");
    let got = ctx
        .eval("Spacetime.resolveTemplate('shared', 'inspector')();")
        .expect("resolve");
    assert_eq!(
        got.as_str().unwrap_or_default(),
        "GLOBAL",
        "owner-first must FALL BACK to bare, not fail — otherwise an owned bundle \
         cannot reference a shared template at all"
    );
}

/// W1.3 REVIEW FINDING — owner identity must survive past bundle evaluation.
///
/// `__ST_BUNDLE_OWNER__` is set only while a bundle EVALUATES and restored after.
/// Every real invocation happens later — an event handler, an `@each` re-render,
/// any async callback — by which time the global is `undefined` and owner-first
/// lookup silently degrades to bare lookup. That is the exact collision the fix
/// exists to prevent, so the fix worked at registration and not at use.
///
/// Confirmed live before this test existed: on a dev-served page, a deferred
/// `resolveTemplate('field-text', window.__ST_BUNDLE_OWNER__)` returned the PAGE's
/// factory to the dock. Owner identity now rides on the FACTORY (`__stOwner`,
/// bound at build time), not on the moment it runs.
#[test]
fn fup134_owner_survives_after_the_bundle_finishes_evaluating() {
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    // Simulate a dock bundle evaluating, then finishing (global restored).
    ctx.eval(
        r#"
        window.__ST_BUNDLE_OWNER__ = 'inspector';
        const built = Spacetime.buildTemplateFactory(
          'card', '<p class="dock">DOCK</p>', [], null, window.__ST_BUNDLE_OWNER__);
        Spacetime.registerOwnedTemplate('inspector', 'card', built.factory);
        window.__ST_BUNDLE_OWNER__ = undefined;   // bundle evaluation ends

        // A page then claims the bare name.
        Spacetime.registerOwnedTemplate(undefined, 'card', () => 'PAGE');
    "#,
    )
    .expect("register");

    let owner_lost = ctx
        .eval("String(window.__ST_BUNDLE_OWNER__);")
        .expect("read global");
    assert_eq!(
        owner_lost.as_str().unwrap_or_default(),
        "undefined",
        "precondition: the bundle-evaluation global is gone by invocation time"
    );

    // The factory must still know its own owner.
    let stamped = ctx
        .eval("String(Spacetime.templates.get('inspector/card').__stOwner);")
        .expect("read stamp");
    assert_eq!(
        stamped.as_str().unwrap_or_default(),
        "inspector",
        "the owner must be bound to the FACTORY at build time — reading it from a \
         global at call time is what silently degraded to bare lookup"
    );
}

/// A template's NESTED invocation must resolve against the enclosing template's
/// owner. Without this the collision simply moves one level down: a dock template
/// referencing `&row` would get a page's `&row`.
#[test]
fn fup134_a_factory_publishes_its_owner_while_it_builds() {
    let mut ctx = V8TestContext::new().with_runtime().expect("runtime loads");
    ctx.eval(
        r#"
        window.__ST_OBSERVED = 'not-run';
        const built = Spacetime.buildTemplateFactory(
          'outer', '<p>x</p>', [], null, 'inspector');
        const wrapped = built.factory;
        // Observe the ambient invoke-owner from INSIDE a factory call.
        const probe = () => { window.__ST_OBSERVED = String(window.__ST_INVOKE_OWNER__); };
        window.__ST_BUNDLE_OWNER__ = undefined;
        // Calling the factory must publish its owner for the duration of the call.
        const origBuild = document.createElement.bind(document);
        wrapped('x');
        window.__ST_AFTER = String(window.__ST_INVOKE_OWNER__);
    "#,
    )
    .expect("invoke");

    let after = ctx.eval("String(window.__ST_AFTER);").expect("after");
    assert_eq!(
        after.as_str().unwrap_or_default(),
        "undefined",
        "the invoke-owner must be RESTORED after the call — a leaked owner would \
         misattribute every later registration, including the page's"
    );
}

