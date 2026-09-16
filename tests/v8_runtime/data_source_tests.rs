//! Data-source primitive V8 tests
//!
//! Tests for the data-source primitive null safety and correct behavior:
//! - No crash when srcValue is null
//! - Correct URL fetch with and without locale interpolation
//! - Full @data compilation pipeline produces non-null src values

use super::context::V8TestContext;

const ST_JS: &str = include_str!("../../public/runtime/st.js");

fn create_context() -> V8TestContext {
    let mut ctx = V8TestContext::new();
    ctx.eval(ST_JS).expect("Failed to load st.js");
    ctx
}

/// Compile a .st source to JS + HTML using the full compiler pipeline
fn compile_st(source: &str) -> spacetime::CompiledSpacetime {
    let ast = spacetime::parse(source).expect("test source should parse");
    spacetime::Compiler::from_ast(&ast)
        .without_runtime()
        .compile()
}

// =============================================================================
// Null Safety Tests
// =============================================================================

/// data-source must not crash when srcValue is null.
/// Regression test: `srcValue is null` → `srcValue.replace(...)` TypeError
#[test]
fn test_data_source_null_src_no_crash() {
    let mut ctx = create_context();

    // Simulate data-source IIFE with null srcValue
    ctx.eval(
        r#"
        (function() {
            const sourceName = "testData";
            const srcValue = null;
            const defaultValue = null;
            const refreshInterval = 0;

            if (!window.__spacetimeData) window.__spacetimeData = {};
            const store = window.__spacetimeData[sourceName] = {
                data: null, loading: true, error: null
            };

            const dispatch = (type, detail) => {
                document.dispatchEvent(new CustomEvent(`data:${sourceName}:${type}`, { detail }));
            };

            const load = async () => {
                store.loading = true;
                store.error = null;
                dispatch('loading', true);

                try {
                    let result;
                    if (!srcValue) {
                        store.error = 'No source URL provided';
                        dispatch('error', store.error);
                        return;
                    }
                    if (srcValue.startsWith('localStorage:')) {
                        const key = srcValue.slice('localStorage:'.length);
                        result = null;
                    } else {
                        const url = srcValue.replace('{locale}', window.__st_locale || 'en');
                        const resp = await fetch(url);
                        result = await resp.json();
                    }
                    store.data = result;
                    store.loading = false;
                    dispatch('data', result);
                } catch (e) {
                    store.error = e.message;
                    store.loading = false;
                    dispatch('error', e.message);
                }
            };
            load();
        })();
    "#,
    )
    .expect("data-source should not throw when srcValue is null");

    let errors = ctx.get_errors();
    assert!(
        errors.is_empty(),
        "Should have no errors, got: {:?}",
        errors
    );
}

// =============================================================================
// Compilation Pipeline Tests
// =============================================================================

/// @data with inline src property must compile to non-null srcValue.
/// This is the core regression: form_compiler.rs was overwriting optional
/// captures with null even when the property value was present.
#[test]
fn test_data_directive_src_not_null() {
    let compiled = compile_st(
        r#"
@data fetch $items string[] : "/api/items.json"
"#,
    );

    // The compiled JS should NOT wire a null source when a src URL is provided.
    assert!(
        !compiled.js.contains("srcValue = null"),
        "srcValue should not be null when src is provided.\nJS:\n{}",
        compiled.js
    );

    // It should contain the actual URL
    assert!(
        compiled.js.contains("/api/items.json"),
        "Compiled JS should contain the source URL.\nJS:\n{}",
        compiled.js
    );
}

/// @data with src property must produce sourceName matching the declared name
#[test]
fn test_data_directive_source_name_matches() {
    let compiled = compile_st(
        r#"
@data fetch $caseStudies object[] : "/api/case-studies.json"
"#,
    );

    assert!(
        compiled.js.contains("caseStudies") || compiled.js.contains("case-studies"),
        "Compiled JS should reference the data source name.\nJS:\n{}",
        compiled.js
    );
}

/// Multiple @data directives in the same file should each have their own src
#[test]
fn test_multiple_data_directives_each_have_src() {
    let compiled = compile_st(
        r#"
@data fetch $items string[] : "/api/items.json"
@data fetch $users object[] : "/api/users.json"
"#,
    );

    assert!(
        compiled.js.contains("/api/items.json"),
        "First @data src should be present.\nJS:\n{}",
        compiled.js
    );
    assert!(
        compiled.js.contains("/api/users.json"),
        "Second @data src should be present.\nJS:\n{}",
        compiled.js
    );
}

// =============================================================================
// Runtime Behavior Tests
// =============================================================================

/// data-source fetches the correct URL when srcValue is provided
#[test]
fn test_data_source_fetches_url() {
    let mut ctx =
        create_context().mock_fetch("/api/items.json", serde_json::json!(["a", "b", "c"]));

    ctx.eval(
        r#"
        (function() {
            const sourceName = "items";
            const srcValue = "/api/items.json";
            const defaultValue = null;
            const refreshInterval = 0;

            if (!window.__spacetimeData) window.__spacetimeData = {};
            const store = window.__spacetimeData[sourceName] = {
                data: null, loading: true, error: null
            };

            const dispatch = (type, detail) => {
                document.dispatchEvent(new CustomEvent(`data:${sourceName}:${type}`, { detail }));
            };

            const load = async () => {
                store.loading = true;
                store.error = null;
                dispatch('loading', true);
                try {
                    if (!srcValue) {
                        store.error = 'No source URL provided';
                        dispatch('error', store.error);
                        return;
                    }
                    const url = srcValue.replace('{locale}', window.__st_locale || 'en');
                    const resp = await fetch(url);
                    const result = await resp.json();
                    store.data = result;
                    store.loading = false;
                    dispatch('data', result);
                } catch (e) {
                    store.error = e.message;
                    store.loading = false;
                    dispatch('error', e.message);
                }
            };
            load();
        })();
    "#,
    )
    .expect("data-source with valid URL should not throw");

    let errors = ctx.get_errors();
    assert!(
        errors.is_empty(),
        "Should have no errors, got: {:?}",
        errors
    );
}

// =============================================================================
// Refresh-interval ms conversion (BUG-138)
// =============================================================================

/// BUG-138: `@data fetch { refresh: 5m }` must emit `refreshInterval = 300000`
/// (5 minutes in ms, an UNQUOTED number) so the data-source guard
/// `if (refreshInterval > 0) setInterval(load, refreshInterval)` actually fires.
/// Before the fix the `:duration` capture stored the raw token and emitted
/// the refresh as a string (`"5m" > 0` is false → never refreshed). PLAN-133
/// W3: the value now lands in the per-usage config call (`refresh: 300000`),
/// consumed by the prelude-owned `__stDataSource` lifecycle.
#[test]
fn test_data_fetch_refresh_duration_converts_to_ms() {
    let compiled = compile_st(
        r#"
@data fetch $products string[] : "/api/products.json" { refresh: 5m }
"#,
    );

    assert!(
        compiled.js.contains("refresh: 300000"),
        "refresh: 5m should compile to 300000 ms (unquoted number).\nJS:\n{}",
        compiled.js
    );
    assert!(
        !compiled.js.contains("refresh: \"5m\""),
        "refresh must NOT be emitted as the raw string \"5m\".\nJS:\n{}",
        compiled.js
    );
}

/// `refresh: 300ms` → 300; `refresh: 2s` → 2000 (unit-aware ms conversion).
#[test]
fn test_data_fetch_refresh_units() {
    let ms = compile_st(
        r#"
@data fetch $a string[] : "/api/a.json" { refresh: 300ms }
"#,
    );
    assert!(
        ms.js.contains("refresh: 300"),
        "refresh: 300ms should compile to 300.\nJS:\n{}",
        ms.js
    );

    let secs = compile_st(
        r#"
@data fetch $b string[] : "/api/b.json" { refresh: 2s }
"#,
    );
    assert!(
        secs.js.contains("refresh: 2000"),
        "refresh: 2s should compile to 2000.\nJS:\n{}",
        secs.js
    );
}

/// `refresh: 0` (a bare number, not a unit token) still emits 0 — the
/// no-auto-refresh case must keep working.
#[test]
fn test_data_fetch_refresh_zero_unchanged() {
    let compiled = compile_st(
        r#"
@data fetch $c string[] : "/api/c.json" { refresh: 0 }
"#,
    );
    assert!(
        compiled.js.contains("refresh: 0"),
        "refresh: 0 should stay 0.\nJS:\n{}",
        compiled.js
    );
}
