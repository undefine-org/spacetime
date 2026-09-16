//! V8TestContext - Headless JavaScript test execution using V8 via rustyscript
//!
//! Memory optimization is achieved through:
//! - .cargo/config.toml: RUST_TEST_THREADS=4 (limits parallelism)
//! - #[serial] attributes on heavy tests (prevents concurrent execution)

use rustyscript::{Runtime, RuntimeOptions};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Once;

static V8_INIT: Once = Once::new();

fn ensure_v8_initialized() {
    V8_INIT.call_once(|| {
        rustyscript::init_platform(4, true);
    });
}

/// LinkeDOM bundle providing a full DOM implementation
const LINKEDOM_JS: &str = include_str!("linkedom.bundle.js");

/// Console setup JS - overrides built-in console to capture output
const CONSOLE_SETUP_JS: &str = r#"
// Console logs storage
const __consoleLogs = [];
const __consoleErrors = [];
const __consoleWarns = [];

globalThis.console = {
    log: (...args) => { __consoleLogs.push(args.join(' ')); },
    error: (...args) => { __consoleErrors.push(args.join(' ')); },
    warn: (...args) => { __consoleWarns.push(args.join(' ')); },
    info: (...args) => { __consoleLogs.push(args.join(' ')); },
    debug: (...args) => { __consoleLogs.push(args.join(' ')); },
    trace: (...args) => { __consoleLogs.push(args.join(' ')); },
    dir: () => {},
    table: () => {},
    group: () => {},
    groupEnd: () => {},
    time: () => {},
    timeEnd: () => {},
    clear: () => { __consoleLogs.length = 0; __consoleErrors.length = 0; },
};

globalThis.__spacetime_get_logs = () => __consoleLogs;
globalThis.__spacetime_get_errors = () => __consoleErrors;
"#;

/// Headless environment markers
const HEADLESS_MARKERS_JS: &str = r#"
globalThis.__spacetime_headless__ = true;
globalThis.__v8__ = true;
"#;

/// DOM mocks and fetch setup
const DOM_MOCKS_JS: &str = r#"
// Track events for assertions
const __events = [];
window.__spacetime_events = __events;

// Override dispatchEvent to track events
const _originalDispatchEvent = Element.prototype.dispatchEvent;
Element.prototype.dispatchEvent = function(event) {
    __events.push({ type: event.type, target: this, detail: event.detail });
    return _originalDispatchEvent.call(this, event);
};

// Mock fetch with configurable responses
const __fetchMocks = new Map();
globalThis.__spacetime_mock_fetch = (url, response) => {
    __fetchMocks.set(url, response);
};

globalThis.fetch = async (url, options) => {
    const mock = __fetchMocks.get(url);
    if (mock) {
        return {
            ok: true,
            status: 200,
            json: async () => typeof mock === 'string' ? JSON.parse(mock) : mock,
            text: async () => typeof mock === 'string' ? mock : JSON.stringify(mock),
        };
    }
    throw new Error(`No mock for fetch: ${url}`);
};

// Performance timing for tests
const __testStart = Date.now();
globalThis.__spacetime_test_elapsed = () => Date.now() - __testStart;
"#;

/// Individual test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub error: Option<String>,
    pub duration_ms: u64,
}

/// Aggregate test results
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TestResults {
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub tests: Vec<TestResult>,
    pub errors: Vec<String>,
}

impl TestResults {
    /// Returns true if all tests passed
    pub fn all_passed(&self) -> bool {
        self.failed == 0 && self.errors.is_empty()
    }

    /// Returns total number of tests run
    pub fn total(&self) -> usize {
        self.passed + self.failed + self.skipped
    }

    /// Returns formatted summary string
    pub fn summary(&self) -> String {
        format!(
            "{} passed, {} failed, {} skipped",
            self.passed, self.failed, self.skipped
        )
    }
}

/// V8-based headless test execution context
///
/// Provides a complete JavaScript environment with:
/// - LinkeDOM for DOM simulation
/// - Spacetime runtime (st.js) for signals
/// - Mock fetch for data loading
/// - Synchronous requestAnimationFrame
pub struct V8TestContext {
    runtime: Runtime,
    fetch_mocks: HashMap<String, String>,
}

impl Default for V8TestContext {
    fn default() -> Self {
        Self::new()
    }
}

impl V8TestContext {
    /// Create a new test context with LinkeDOM initialized
    pub fn new() -> Self {
        ensure_v8_initialized();
        let mut runtime =
            Runtime::new(RuntimeOptions::default()).expect("Failed to create V8 runtime");

        // Set up console capture FIRST (required by LinkeDOM)
        let _ = runtime.eval::<Value>(CONSOLE_SETUP_JS);

        // Add headless environment marker (before LinkeDOM)
        let _ = runtime.eval::<Value>(HEADLESS_MARKERS_JS);

        // Initialize LinkeDOM bundle
        if let Err(e) = runtime.eval::<Value>(LINKEDOM_JS) {
            eprintln!("Warning: Failed to initialize LinkeDOM: {:?}", e);
        }

        // Add enhanced DOM mocks for testing
        let _ = runtime.eval::<Value>(DOM_MOCKS_JS);

        Self {
            runtime,
            fetch_mocks: HashMap::new(),
        }
    }

    /// Load the Spacetime runtime (st.js)
    ///
    /// This provides the signal system (ST.set, ST.get, ST.watch)
    pub fn with_runtime(mut self) -> Result<Self, String> {
        // AbortController/AbortSignal ONLY, sliced out of src/headless_shims.js
        // (the BUG-196 block). The change-driver (and every interactive
        // animation driver) builds one at init; without it the init throws
        // AFTER the _st_init flag is set and the driver silently never
        // attaches (BUG-271). Loading the WHOLE shims file aborts V8
        // (SIGTRAP, isolate stack check) in the FEAT-077 builder tests — the
        // full file is written for the headless backend's own world, not for
        // this harness. One contract, one slice, source of truth stays
        // headless_shims.js.
        let shims = include_str!("../../src/headless_shims.js");
        let abort_start = shims
            .find("if (typeof globalThis.AbortController === 'undefined')")
            .expect("BUG-196 shim marker");
        let abort_end = shims[abort_start..]
            .find("// Signal-aware addEventListener")
            .map(|i| abort_start + i)
            .expect("end of BUG-196 shim block");
        self.runtime
            .eval::<Value>(&shims[abort_start..abort_end])
            .map_err(|e| format!("Failed to load AbortController shim: {:?}", e))?;
        // Load st.js runtime (core signals)
        let st_js = include_str!("../../public/runtime/st.js");
        self.runtime
            .eval::<Value>(st_js)
            .map_err(|e| format!("Failed to load st.js: {:?}", e))?;
        // Load templates.js (template registry + factory builder). Production pages
        // ALWAYS emit this alongside st.js (pipeline/emit.rs::format_js_with_runtime),
        // so the harness must too — register-template and the FEAT-126 region-mount
        // registerBundle both build factories via Spacetime.buildTemplateFactory, which
        // lives here. Without it the v8 context diverges from production.
        let templates_js = include_str!("../../public/runtime/templates.js");
        self.runtime
            .eval::<Value>(templates_js)
            .map_err(|e| format!("Failed to load templates.js: {:?}", e))?;

        Ok(self)
    }

    /// Load the editable AST-model runtime (PLAN-031 / FEAT-098). Pure kernel:
    /// document shape, integer positions, ops, projection. Needs `ST` from st.js.
    #[allow(dead_code)]
    pub fn with_editable(mut self) -> Result<Self, String> {
        let editable_js = include_str!("../../public/runtime/editable.js");
        self.runtime
            .eval::<Value>(editable_js)
            .map_err(|e| format!("Failed to load editable.js: {:?}", e))?;
        Ok(self)
    }

    // `with_data_binding` lived here. public/runtime/data-binding.js was deleted in
    // cdd3e397 (BUG-328) as the THIRD copy of the filter table; this loader was its
    // only remaining reference and had zero callers, so the `include_str!` broke the
    // whole v8_runtime_test binary at COMPILE time — 139 tests silently not running.
    // `there_is_no_second_runtime_filter_table` (src/html/ssg_filters.rs) guards the
    // file's return; nothing needs to guard the loader.

    /// Load compiled Spacetime code (generated JS)
    #[allow(dead_code)]
    pub fn load_compiled(mut self, code: &str) -> Result<Self, String> {
        self.runtime
            .eval::<Value>(code)
            .map_err(|e| format!("Failed to load compiled code: {:?}", e))?;

        Ok(self)
    }

    /// Mock a fetch URL with JSON response
    pub fn mock_fetch(mut self, url: &str, data: serde_json::Value) -> Self {
        let json_str = data.to_string();
        self.fetch_mocks.insert(url.to_string(), json_str.clone());

        let mock_code = format!(
            r#"__spacetime_mock_fetch({}, {});"#,
            serde_json::to_string(url).unwrap(),
            json_str
        );

        let _ = self.runtime.eval::<Value>(&mock_code);
        self
    }

    /// Mock multiple fetch URLs at once
    pub fn mock_fetch_all(mut self, mocks: &[(&str, serde_json::Value)]) -> Self {
        for (url, data) in mocks {
            self = self.mock_fetch(url, data.clone());
        }
        self
    }

    /// Evaluate raw JavaScript code
    pub fn eval(&mut self, code: &str) -> Result<Value, rustyscript::Error> {
        self.runtime.eval::<Value>(code)
    }

    /// Trigger DOMContentLoaded event
    pub fn trigger_dom_ready(&mut self) -> Result<(), String> {
        self.runtime
            .eval::<Value>(
                r#"
                document.dispatchEvent(new Event('DOMContentLoaded'));
                "#,
            )
            .map_err(|e| format!("Failed to trigger DOMContentLoaded: {:?}", e))?;
        Ok(())
    }

    /// Run all registered tests with optional filter
    ///
    /// Returns test results including pass/fail counts and error messages.
    /// Tests are registered via `window.__spacetime_register_test()`.
    pub fn run_tests(&mut self, filter: Option<&str>) -> TestResults {
        // First trigger DOM ready to initialize everything
        let _ = self.trigger_dom_ready();

        // Run tests via the stdlib test runner (synchronous approach)
        self.run_tests_sync(filter)
    }

    /// Synchronous test runner
    fn run_tests_sync(&mut self, filter: Option<&str>) -> TestResults {
        let filter_arg = filter
            .map(|f| format!("\"{}\"", f))
            .unwrap_or_else(|| "undefined".to_string());

        let check_code = format!(
            r#"
            (() => {{
                if (typeof window.__spacetime_run_tests !== 'function') {{
                    // Get registered tests
                    const tests = window.__st_tests || [];
                    let passed = 0, failed = 0, skipped = 0;
                    const errors = [];
                    const filter = {};

                    for (const test of tests) {{
                        if (test.skip) {{
                            skipped++;
                            continue;
                        }}
                        if (filter && !test.name.includes(filter)) {{
                            skipped++;
                            continue;
                        }}

                        try {{
                            const result = test.fn();
                            if (result && typeof result.then === 'function') {{
                                passed++;
                            }} else {{
                                passed++;
                            }}
                        }} catch (e) {{
                            failed++;
                            errors.push({{ name: test.name, error: e.message }});
                        }}
                    }}

                    return JSON.stringify({{ passed, failed, skipped, errors, tests: tests.length }});
                }}

                // Use the registered test runner
                try {{
                    const results = window.__spacetime_run_tests({});
                    return JSON.stringify({{
                        passed: results.passed || 0,
                        failed: results.failed || 0,
                        skipped: results.skipped || 0,
                        errors: results.errors || []
                    }});
                }} catch (e) {{
                    return JSON.stringify({{
                        passed: 0,
                        failed: 0,
                        skipped: 0,
                        errors: ["Test runner error: " + e.message]
                    }});
                }}
            }})()
            "#,
            filter_arg, filter_arg
        );

        match self.runtime.eval::<Value>(&check_code) {
            Ok(value) => {
                if let Some(json_str) = value.as_str()
                    && let Ok(results) = serde_json::from_str::<TestResults>(json_str)
                {
                    return results;
                }
                TestResults::default()
            }
            Err(e) => TestResults {
                errors: vec![format!("Test run failed: {:?}", e)],
                ..Default::default()
            },
        }
    }

    /// Get console logs captured during test execution
    pub fn get_logs(&mut self) -> Vec<String> {
        match self
            .runtime
            .eval::<Value>("JSON.stringify(__spacetime_get_logs())")
        {
            Ok(value) => {
                if let Some(json_str) = value.as_str() {
                    serde_json::from_str(json_str).unwrap_or_default()
                } else {
                    vec![]
                }
            }
            Err(_) => vec![],
        }
    }

    /// Get console errors captured during test execution
    pub fn get_errors(&mut self) -> Vec<String> {
        match self
            .runtime
            .eval::<Value>("JSON.stringify(__spacetime_get_errors())")
        {
            Ok(value) => {
                if let Some(json_str) = value.as_str() {
                    serde_json::from_str(json_str).unwrap_or_default()
                } else {
                    vec![]
                }
            }
            Err(_) => vec![],
        }
    }

    /// Set HTML content in the document body
    pub fn set_body_html(&mut self, html: &str) -> Result<(), String> {
        let escaped = html.replace('\\', "\\\\").replace('`', "\\`");
        let code = format!("document.body.innerHTML = `{}`;", escaped);
        self.runtime
            .eval::<Value>(&code)
            .map_err(|e| format!("Failed to set body HTML: {:?}", e))?;
        Ok(())
    }

    /// Query an element and get its text content
    pub fn query_text(&mut self, selector: &str) -> Option<String> {
        let code = format!(
            r#"
            (() => {{
                const el = document.querySelector("{}");
                return el ? el.textContent : null;
            }})()
            "#,
            selector.replace('"', r#"\""#)
        );

        match self.runtime.eval::<Value>(&code) {
            Ok(value) => {
                if value.is_null() {
                    None
                } else {
                    value.as_str().map(|s| s.to_string())
                }
            }
            Err(_) => None,
        }
    }

    /// Check if an element exists
    pub fn element_exists(&mut self, selector: &str) -> bool {
        let code = format!(
            r#"document.querySelector("{}") !== null"#,
            selector.replace('"', r#"\""#)
        );

        matches!(
            self.runtime.eval::<Value>(&code),
            Ok(value) if value.as_bool() == Some(true)
        )
    }

    /// Simulate a click on an element
    pub fn click(&mut self, selector: &str) -> Result<(), String> {
        let code = format!(
            r#"
            (() => {{
                const el = document.querySelector("{}");
                if (el) {{
                    el.click();
                    return true;
                }}
                return false;
            }})()
            "#,
            selector.replace('"', r#"\""#)
        );

        match self.runtime.eval::<Value>(&code) {
            Ok(value) if value.as_bool() == Some(true) => Ok(()),
            Ok(_) => Err(format!("Element not found: {}", selector)),
            Err(e) => Err(format!("Click failed: {:?}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let ctx = V8TestContext::new();
        assert!(ctx.fetch_mocks.is_empty());
    }

    #[test]
    fn test_linkedom_initialized() {
        let mut ctx = V8TestContext::new();
        let result = ctx.eval("typeof document !== 'undefined'");
        assert!(result.is_ok());
    }

    #[test]
    fn test_query_element() {
        let mut ctx = V8TestContext::new();
        ctx.set_body_html("<div class='test'>Hello World</div>")
            .unwrap();
        assert!(ctx.element_exists(".test"));
        assert_eq!(ctx.query_text(".test"), Some("Hello World".to_string()));
    }

    #[test]
    fn test_mock_fetch() {
        let mut ctx = V8TestContext::new();
        ctx = ctx.mock_fetch("/api/data", serde_json::json!({"items": [1, 2, 3]}));

        let result = ctx.eval(
            r#"
            (async () => {
                const res = await fetch('/api/data');
                const data = await res.json();
                return data.items.length;
            })()
            "#,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_headless_marker() {
        let mut ctx = V8TestContext::new();
        let result = ctx.eval("globalThis.__spacetime_headless__ === true");
        assert!(matches!(
            result,
            Ok(value) if value.as_bool() == Some(true)
        ));
    }
}
