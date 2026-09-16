//! Scoped State Tests (PROJ-102)
//!
//! Tests that template instances have independent state via ST.set/get/watch
//! scoped to each root element, not global SpacetimeLocal.

use super::context::V8TestContext;

const ST_JS: &str = include_str!("../../public/runtime/st.js");

fn create_context() -> V8TestContext {
    let mut ctx = V8TestContext::new();
    ctx.eval(ST_JS).expect("Failed to load st.js");
    ctx
}

// =============================================================================
// Instance Independence Tests
// =============================================================================

#[test]
fn test_two_instances_independent_state() {
    let mut ctx = create_context();

    // Create two separate DOM elements (simulating two template instances)
    // Each gets its own $count state via ST.set
    let result = ctx.eval(
        r#"
        (() => {
            // Simulate two template instances with their own root elements
            const root1 = document.createElement('div');
            root1.className = 'counter-1';
            const root2 = document.createElement('div');
            root2.className = 'counter-2';

            // Initialize scoped state on each instance (as register-template would)
            ST.set(root1, 'count', 0);
            ST.set(root2, 'count', 0);

            // Increment only instance 1
            ST.set(root1, 'count', ST.get(root1, 'count') + 1);
            ST.set(root1, 'count', ST.get(root1, 'count') + 1);
            ST.set(root1, 'count', ST.get(root1, 'count') + 1);

            // Verify independence: instance 2 should still be 0
            const count1 = ST.get(root1, 'count');
            const count2 = ST.get(root2, 'count');

            return JSON.stringify({ count1, count2, independent: count1 === 3 && count2 === 0 });
        })();
    "#,
    );

    let val = result.expect("JS evaluation failed");
    let obj: serde_json::Value = serde_json::from_str(val.as_str().unwrap()).unwrap();
    assert_eq!(obj["count1"], 3, "Instance 1 should have count=3");
    assert_eq!(obj["count2"], 0, "Instance 2 should still have count=0");
    assert_eq!(obj["independent"], true, "Instances must be independent");
}

#[test]
fn test_st_set_creates_signal_on_element() {
    let mut ctx = create_context();

    let result = ctx.eval(
        r#"
        (() => {
            const el = document.createElement('div');
            ST.set(el, 'open', false);

            const v1 = ST.get(el, 'open');
            ST.set(el, 'open', true);
            const v2 = ST.get(el, 'open');

            return JSON.stringify({ v1, v2 });
        })();
    "#,
    );

    let val = result.expect("JS evaluation failed");
    let obj: serde_json::Value = serde_json::from_str(val.as_str().unwrap()).unwrap();
    assert_eq!(obj["v1"], false);
    assert_eq!(obj["v2"], true);
}

#[test]
fn test_st_watch_fires_on_scoped_state_change() {
    let mut ctx = create_context();

    let result = ctx.eval(
        r#"
        (() => {
            const el = document.createElement('div');
            ST.set(el, 'name', 'initial');

            const watched = [];
            ST.watch(el, 'name', v => watched.push(v));

            ST.set(el, 'name', 'updated');
            ST.set(el, 'name', 'final');

            // watched should have: 'initial' (from immediate call), then microtask updates
            return JSON.stringify({ watchedCount: watched.length, firstValue: watched[0] });
        })();
    "#,
    );

    let val = result.expect("JS evaluation failed");
    let obj: serde_json::Value = serde_json::from_str(val.as_str().unwrap()).unwrap();
    // watch() calls immediately with current value
    assert!(
        obj["watchedCount"].as_u64().unwrap() >= 1,
        "Watch should fire at least once (immediate)"
    );
    assert_eq!(
        obj["firstValue"], "initial",
        "First watch value should be 'initial'"
    );
}

#[test]
fn test_multiple_state_vars_per_instance() {
    let mut ctx = create_context();

    let result = ctx.eval(
        r#"
        (() => {
            const el = document.createElement('div');
            // Simulate component_body states: $count number: 0; $label string: "default";
            ST.set(el, 'count', 0);
            ST.set(el, 'label', 'default');
            ST.set(el, 'open', false);

            // Modify each independently
            ST.set(el, 'count', 42);
            ST.set(el, 'label', 'updated');
            // Leave 'open' as-is

            return JSON.stringify({
                count: ST.get(el, 'count'),
                label: ST.get(el, 'label'),
                open: ST.get(el, 'open')
            });
        })();
    "#,
    );

    let val = result.expect("JS evaluation failed");
    let obj: serde_json::Value = serde_json::from_str(val.as_str().unwrap()).unwrap();
    assert_eq!(obj["count"], 42);
    assert_eq!(obj["label"], "updated");
    assert_eq!(obj["open"], false);
}

#[test]
fn test_scoped_state_does_not_leak_to_global() {
    let mut ctx = create_context();

    let result = ctx.eval(
        r#"
        (() => {
            // Set up global SpacetimeLocal
            if (!window._localState) window._localState = {};
            if (!window.SpacetimeLocal) {
                window.SpacetimeLocal = new Proxy(window._localState, {
                    set(target, prop, value) {
                        target[prop] = value;
                        return true;
                    },
                    get(target, prop) { return target[prop]; }
                });
            }

            // Create scoped state on an element
            const el = document.createElement('div');
            ST.set(el, 'myVar', 'scoped-value');

            // Global should NOT have this var
            const globalHas = 'myVar' in window._localState;
            const scopedValue = ST.get(el, 'myVar');

            return JSON.stringify({ globalHas, scopedValue });
        })();
    "#,
    );

    let val = result.expect("JS evaluation failed");
    let obj: serde_json::Value = serde_json::from_str(val.as_str().unwrap()).unwrap();
    assert_eq!(
        obj["globalHas"], false,
        "Scoped state should not leak to global"
    );
    assert_eq!(
        obj["scopedValue"], "scoped-value",
        "Scoped state should be readable"
    );
}

#[test]
fn test_scoped_state_css_var_set() {
    let mut ctx = create_context();

    let result = ctx.eval(
        r#"
        (() => {
            const el = document.createElement('div');
            document.body.appendChild(el);

            ST.set(el, 'count', 5);

            // ST.set should also set CSS variable --st-count
            const cssVal = el.style.getPropertyValue('--st-count');
            return JSON.stringify({ cssVal });
        })();
    "#,
    );

    let val = result.expect("JS evaluation failed");
    let obj: serde_json::Value = serde_json::from_str(val.as_str().unwrap()).unwrap();
    assert!(
        obj["cssVal"] == "5" || obj["cssVal"] == 5,
        "ST.set should set --st-count CSS variable, got: {:?}",
        obj["cssVal"]
    );
}

#[test]
fn test_factory_with_state_init_pattern() {
    let mut ctx = create_context();

    // This test simulates the exact pattern that register-template now emits
    let result = ctx.eval(r#"
        (() => {
            // Simulate the rawBody from component_body parser
            const rawBody = {
                html: '<div class="counter"><span class="count">0</span><button>+1</button></div>',
                states: ['$count number: 0', '$label string: "Click me"'],
                directives: [],
                css_rules: [],
                exports: [],
                injections: []
            };

            // Factory function (mimicking register-template output)
            function createCounter() {
                const template = document.createElement('template');
                template.innerHTML = rawBody.html.trim();
                const element = template.content.firstElementChild;

                // State init (as emitted by register-template PROJ-102)
                const rawStates = rawBody.states || [];
                rawStates.forEach(stateDecl => {
                    if (typeof stateDecl !== 'string') return;
                    const match = stateDecl.match(/^\$(\w+)\s+\w+(?:\[\])?:\s*(.+?)\s*;?$/);
                    if (match && element) {
                        const [, name, rawInitial] = match;
                        let initial;
                        try { initial = JSON.parse(rawInitial.trim()); } catch { initial = rawInitial.trim(); }
                        ST.set(element, name, initial);
                    }
                });

                return element;
            }

            // Create two instances
            const inst1 = createCounter();
            const inst2 = createCounter();

            // Both should start with count=0
            const init1 = ST.get(inst1, 'count');
            const init2 = ST.get(inst2, 'count');

            // Modify instance 1 only
            ST.set(inst1, 'count', 10);

            const final1 = ST.get(inst1, 'count');
            const final2 = ST.get(inst2, 'count');

            return JSON.stringify({
                init1, init2, final1, final2,
                label1: ST.get(inst1, 'label'),
                label2: ST.get(inst2, 'label'),
                independent: final1 === 10 && final2 === 0
            });
        })();
    "#);

    let val = result.expect("JS evaluation failed");
    let obj: serde_json::Value = serde_json::from_str(val.as_str().unwrap()).unwrap();
    assert_eq!(obj["init1"], 0, "Instance 1 should start at 0");
    assert_eq!(obj["init2"], 0, "Instance 2 should start at 0");
    assert_eq!(
        obj["final1"], 10,
        "Instance 1 should be 10 after modification"
    );
    assert_eq!(
        obj["final2"], 0,
        "Instance 2 should still be 0 (independent)"
    );
    assert_eq!(
        obj["label1"], "Click me",
        "String state should parse correctly"
    );
    assert_eq!(
        obj["label2"], "Click me",
        "String state should be set on each instance"
    );
    assert_eq!(obj["independent"], true, "Instances MUST be independent");
}
