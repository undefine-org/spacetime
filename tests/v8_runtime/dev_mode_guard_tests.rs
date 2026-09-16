//! Dev-Mode Export Enforcement Guard Tests (PROJ-106 ITEM-106-004)
//!
//! Tests that ST.set and ST.get check __stExports metadata in dev mode
//! (window.__ST_DEV === true) and block/warn on invalid access.

use super::context::V8TestContext;

const ST_JS: &str = include_str!("../../public/runtime/st.js");

fn create_context() -> V8TestContext {
    let mut ctx = V8TestContext::new();
    ctx.eval(ST_JS).expect("Failed to load st.js");
    ctx
}

// =============================================================================
// ST.set Dev-Mode Guards
// =============================================================================

#[test]
fn test_set_non_exported_var_blocked_in_dev_mode() {
    let mut ctx = create_context();

    let result = ctx.eval(
        r#"
        (() => {
            window.__ST_DEV = true;

            const el = document.createElement('div');

            // Set up exports metadata: only 'count' is exported
            el.__stExports = { count: { mutable: true } };

            // Initialize state
            ST.set(el, 'count', 0);

            // Now try to set a non-exported var — should be blocked
            // Temporarily disable __ST_DEV to seed the state first
            window.__ST_DEV = false;
            ST.set(el, 'secret', 100);
            window.__ST_DEV = true;

            // Attempt to write 'secret' (not in exports) — guard should block
            ST.set(el, 'secret', 999);

            // 'secret' should still be 100 (the guard returned early)
            const secretVal = ST.get(el, 'secret');

            return JSON.stringify({
                secretVal: secretVal,
                blocked: secretVal === 100
            });
        })();
    "#,
    );

    let val = result.expect("JS evaluation failed");
    let obj: serde_json::Value = serde_json::from_str(val.as_str().unwrap()).unwrap();
    assert_eq!(
        obj["secretVal"], 100,
        "Non-exported var should NOT be updated in dev mode"
    );
    assert_eq!(
        obj["blocked"], true,
        "ST.set on non-exported var must be blocked by dev guard"
    );
}

#[test]
fn test_set_readonly_export_blocked_in_dev_mode() {
    let mut ctx = create_context();

    let result = ctx.eval(
        r#"
        (() => {
            window.__ST_DEV = true;

            const el = document.createElement('div');

            // 'count' exported as read-only (mutable: false)
            el.__stExports = { count: { mutable: false } };

            // Seed initial state with dev mode off
            window.__ST_DEV = false;
            ST.set(el, 'count', 42);
            window.__ST_DEV = true;

            // Attempt to write read-only export — guard should block
            ST.set(el, 'count', 999);

            const countVal = ST.get(el, 'count');

            return JSON.stringify({
                countVal: countVal,
                blocked: countVal === 42
            });
        })();
    "#,
    );

    let val = result.expect("JS evaluation failed");
    let obj: serde_json::Value = serde_json::from_str(val.as_str().unwrap()).unwrap();
    assert_eq!(
        obj["countVal"], 42,
        "Read-only export should NOT be updated in dev mode"
    );
    assert_eq!(
        obj["blocked"], true,
        "ST.set on read-only export must be blocked by dev guard"
    );
}

#[test]
fn test_set_mutable_export_succeeds_in_dev_mode() {
    let mut ctx = create_context();

    let result = ctx.eval(
        r#"
        (() => {
            window.__ST_DEV = true;

            const el = document.createElement('div');

            // 'count' exported as mutable
            el.__stExports = { count: { mutable: true } };

            // Initialize state (mutable export — should pass guard)
            ST.set(el, 'count', 0);

            // Update mutable export — should succeed even in dev mode
            ST.set(el, 'count', 77);

            const countVal = ST.get(el, 'count');

            return JSON.stringify({
                countVal: countVal,
                succeeded: countVal === 77
            });
        })();
    "#,
    );

    let val = result.expect("JS evaluation failed");
    let obj: serde_json::Value = serde_json::from_str(val.as_str().unwrap()).unwrap();
    assert_eq!(
        obj["countVal"], 77,
        "Mutable export should be updated in dev mode"
    );
    assert_eq!(
        obj["succeeded"], true,
        "ST.set on mutable export must succeed in dev mode"
    );
}

#[test]
fn test_set_production_mode_no_guard() {
    let mut ctx = create_context();

    let result = ctx.eval(
        r#"
        (() => {
            // Ensure __ST_DEV is NOT set (production mode)
            delete window.__ST_DEV;

            const el = document.createElement('div');

            // Set up exports metadata (read-only)
            el.__stExports = { count: { mutable: false } };

            // In production mode, guards are inactive — all writes should succeed
            ST.set(el, 'count', 42);
            ST.set(el, 'secret', 100);

            const countVal = ST.get(el, 'count');
            const secretVal = ST.get(el, 'secret');

            return JSON.stringify({
                countVal: countVal,
                secretVal: secretVal,
                bothSucceeded: countVal === 42 && secretVal === 100
            });
        })();
    "#,
    );

    let val = result.expect("JS evaluation failed");
    let obj: serde_json::Value = serde_json::from_str(val.as_str().unwrap()).unwrap();
    assert_eq!(
        obj["countVal"], 42,
        "Production mode should allow writing read-only exports"
    );
    assert_eq!(
        obj["secretVal"], 100,
        "Production mode should allow writing non-exported vars"
    );
    assert_eq!(
        obj["bothSucceeded"], true,
        "All writes must succeed in production mode (no guard)"
    );
}

#[test]
fn test_get_non_exported_var_warns_in_dev_mode() {
    let mut ctx = create_context();

    // ST.get should still return the value (warning only, not blocking)
    let result = ctx.eval(
        r#"
        (() => {
            window.__ST_DEV = true;

            const el = document.createElement('div');
            el.__stExports = { count: { mutable: true } };

            // Seed state with dev mode off
            window.__ST_DEV = false;
            ST.set(el, 'count', 10);
            ST.set(el, 'secret', 99);
            window.__ST_DEV = true;

            // Read exported var — no warning
            const countVal = ST.get(el, 'count');

            // Read non-exported var — should warn but still return value
            const secretVal = ST.get(el, 'secret');

            return JSON.stringify({
                countVal: countVal,
                secretVal: secretVal,
                readSucceeded: countVal === 10 && secretVal === 99
            });
        })();
    "#,
    );

    let val = result.expect("JS evaluation failed");
    let obj: serde_json::Value = serde_json::from_str(val.as_str().unwrap()).unwrap();
    assert_eq!(obj["countVal"], 10, "Exported var read should return value");
    assert_eq!(
        obj["secretVal"], 99,
        "Non-exported var read should still return value (warning only)"
    );
    assert_eq!(
        obj["readSucceeded"], true,
        "ST.get warns but does not block reads in dev mode"
    );
}

#[test]
fn test_no_guard_without_exports_metadata() {
    let mut ctx = create_context();

    let result = ctx.eval(
        r#"
        (() => {
            window.__ST_DEV = true;

            const el = document.createElement('div');
            // No __stExports set — template without @exports

            ST.set(el, 'anything', 42);
            const val = ST.get(el, 'anything');

            return JSON.stringify({
                val: val,
                succeeded: val === 42
            });
        })();
    "#,
    );

    let val = result.expect("JS evaluation failed");
    let obj: serde_json::Value = serde_json::from_str(val.as_str().unwrap()).unwrap();
    assert_eq!(
        obj["succeeded"], true,
        "Templates without __stExports should have zero overhead even in dev mode"
    );
}
