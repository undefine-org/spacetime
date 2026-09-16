//! Test runner for Spacetime tests
//!
//! Discovers and compiles .test.st files, generating HTML that can be
//! executed in a browser environment. The test runner supports:
//!
//! - Test discovery from directories
//! - Compilation to browser-executable HTML
//! - Result parsing from JSON output
//!
//! ## Usage
//!
//! ```rust,ignore
//! use spacetime::test_runner::{discover_tests, compile_test_html};
//!
//! let tests = discover_tests(&PathBuf::from("tests/"));
//! let html = compile_test_html(&tests)?;
//! ```

use std::path::PathBuf;

/// Testing stdlib (auto-imported for test files)
const TESTING_STDLIB: &str = include_str!("../stdlib/testing/test.st");
const TESTING_ASSERTIONS_TIMELINE: &str = include_str!("../stdlib/testing/assertions/timeline.st");
const TESTING_ASSERTIONS_STATE: &str = include_str!("../stdlib/testing/assertions/state.st");
const TESTING_ASSERTIONS_BINDING: &str = include_str!("../stdlib/testing/assertions/binding.st");
const TESTING_GENERATIVE: &str = include_str!("../stdlib/testing/generative.st");
const TESTING_TIMING: &str = include_str!("../stdlib/testing/timing.st");
const TESTING_AUTOMATION: &str = include_str!("../stdlib/testing/automation.st");

// Full ST runtime modules (same order as pipeline/emit.rs)
const RUNTIME_CORE: &str = include_str!("../public/runtime/st.js");
const RUNTIME_RAF: &str = include_str!("../public/runtime/raf-coordinator.js");
const RUNTIME_EASING: &str = include_str!("../public/runtime/easing.js");
const RUNTIME_COLOR: &str = include_str!("../public/runtime/color.js");
const RUNTIME_INTERPOLATE: &str = include_str!("../public/runtime/interpolate.js");
const RUNTIME_STAGGER: &str = include_str!("../public/runtime/stagger.js");
const RUNTIME_VALUE_FUNCTIONS: &str = include_str!("../public/runtime/value-functions.js");
const RUNTIME_TEMPLATES: &str = include_str!("../public/runtime/templates.js");
const RUNTIME_EDITABLE: &str = include_str!("../public/runtime/editable.js");

/// Results from a test run
#[derive(Debug, Clone, Default)]
pub struct TestResults {
    /// Number of tests that passed
    pub passed: u32,
    /// Number of tests that failed
    pub failed: u32,
    /// Number of tests that were skipped
    pub skipped: u32,
    /// Details about each failed test
    pub errors: Vec<TestError>,
    /// Total test count
    pub total: u32,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

impl TestResults {
    /// Check if all tests passed
    pub fn all_passed(&self) -> bool {
        self.failed == 0 && self.errors.is_empty()
    }

    /// Number of claims that actually EXECUTED (a skip runs nothing).
    pub fn executed(&self) -> u32 {
        self.passed + self.failed
    }

    /// BUG-332: tests existed, and none of them ran.
    ///
    /// This run proved NOTHING, so it must not be reported as success. It is the
    /// shape a whole dead suite takes — when stdlib load breaks, every `.st` file
    /// executes zero claims and every file reports `0 passed, 0 failed`.
    ///
    /// Deliberately NOT `skipped > 0`: skipping is correct and load-bearing (a
    /// `needs layout` claim refuses a layout-less backend by design — see
    /// docs/testing/FIDELITY_LADDER.md), so a partially-skipped run is a good run.
    /// And an EMPTY selection is not vacant either: nothing was claimed, so
    /// nothing is falsely claimed proven.
    pub fn is_vacant(&self) -> bool {
        self.total > 0 && self.executed() == 0
    }

    /// The single success predicate for a run: nothing failed, AND something ran.
    pub fn is_success(&self) -> bool {
        self.all_passed() && !self.is_vacant()
    }

    /// Get a summary string
    pub fn summary(&self) -> String {
        format!(
            "{} passed, {} failed, {} skipped ({}ms)",
            self.passed, self.failed, self.skipped, self.duration_ms
        )
    }
}

/// Error from a failed test
#[derive(Debug, Clone)]
pub struct TestError {
    /// Name of the failed test
    pub name: String,
    /// Error message
    pub message: String,
    /// Stack trace if available
    pub stack: Option<String>,
    /// File where the test was defined
    pub file: Option<PathBuf>,
    /// Line number in the file
    pub line: Option<u32>,
}

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FAIL: {}\n  {}", self.name, self.message)?;
        if let Some(ref stack) = self.stack {
            write!(f, "\n  Stack:\n    {}", stack.replace('\n', "\n    "))?;
        }
        if let Some(ref file) = self.file {
            write!(f, "\n  at {}", file.display())?;
            if let Some(line) = self.line {
                write!(f, ":{}", line)?;
            }
        }
        Ok(())
    }
}

/// Options for test discovery
#[derive(Debug, Clone, Default)]
pub struct DiscoverOptions {
    /// File pattern to match (default: "*.test.st")
    pub pattern: Option<String>,
    /// Whether to search recursively
    pub recursive: bool,
    /// Directories to exclude
    pub exclude: Vec<String>,
}

impl DiscoverOptions {
    pub fn new() -> Self {
        Self {
            pattern: None,
            recursive: true,
            exclude: vec!["node_modules".to_string(), ".git".to_string()],
        }
    }

    pub fn with_pattern(mut self, pattern: &str) -> Self {
        self.pattern = Some(pattern.to_string());
        self
    }

    pub fn recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }

    pub fn exclude(mut self, dir: &str) -> Self {
        self.exclude.push(dir.to_string());
        self
    }
}

/// Test layer for organizing tests by scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestLayer {
    /// Unit tests - single primitive/macro tests
    Unit,
    /// Integration tests - composed behavior tests
    Integration,
    /// E2E tests - full scenario tests
    E2e,
    /// Meta tests - tests of the testing framework itself
    Meta,
    /// All test layers
    All,
}

impl TestLayer {
    /// Get the subdirectory name for this layer
    pub fn subdir(&self) -> Option<&'static str> {
        match self {
            TestLayer::Unit => Some("unit"),
            TestLayer::Integration => Some("integration"),
            TestLayer::E2e => Some("e2e"),
            TestLayer::Meta => Some("meta"),
            TestLayer::All => None,
        }
    }
}

/// Discover test files in a directory
///
/// Finds all `.test.st` files in the given directory and its subdirectories.
///
/// # Arguments
///
/// * `dir` - Directory to search for test files
///
/// # Returns
///
/// Vector of paths to test files, sorted by path
pub fn discover_tests(dir: &PathBuf) -> Vec<PathBuf> {
    discover_tests_with_options(dir, &DiscoverOptions::new())
}

/// Discover test files by layer
///
/// Finds test files in the appropriate subdirectory for the given layer.
///
/// # Arguments
///
/// * `base_dir` - Base test directory (e.g., "tests/")
/// * `layer` - Which test layer to discover
///
/// # Returns
///
/// Vector of paths to test files for that layer
pub fn discover_tests_by_layer(base_dir: &PathBuf, layer: TestLayer) -> Vec<PathBuf> {
    match layer {
        TestLayer::All => discover_tests(base_dir),
        _ => {
            if let Some(subdir) = layer.subdir() {
                let layer_dir = base_dir.join(subdir);
                if layer_dir.exists() {
                    discover_tests(&layer_dir)
                } else {
                    vec![]
                }
            } else {
                vec![]
            }
        }
    }
}

/// Discover meta-tests (tests of the testing framework)
///
/// Finds test files in stdlib/testing/meta/
pub fn discover_meta_tests(stdlib_dir: &PathBuf) -> Vec<PathBuf> {
    let meta_dir = stdlib_dir.join("testing").join("meta");
    if meta_dir.exists() {
        discover_tests(&meta_dir)
    } else {
        vec![]
    }
}

/// Discover test files with custom options
pub fn discover_tests_with_options(dir: &PathBuf, options: &DiscoverOptions) -> Vec<PathBuf> {
    let mut tests = vec![];
    discover_recursive(dir, options, &mut tests);
    tests.sort();
    tests
}

fn discover_recursive(dir: &PathBuf, options: &DiscoverOptions, tests: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        // Check if directory should be excluded
        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str())
                && options.exclude.iter().any(|e| e == name)
            {
                continue;
            }

            if options.recursive {
                discover_recursive(&path, options, tests);
            }
            continue;
        }

        // Check if file matches pattern
        if is_test_file(&path, options) {
            tests.push(path);
        }
    }
}

fn is_test_file(path: &PathBuf, options: &DiscoverOptions) -> bool {
    let pattern = options.pattern.as_deref().unwrap_or("*.test.st");

    // Simple pattern matching
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if let Some(suffix) = pattern.strip_prefix('*') {
            return name.ends_with(suffix);
        } else {
            return name == pattern;
        }
    }

    false
}

/// Compile test files to HTML for browser execution
///
/// Takes a list of test files, parses and compiles them, then wraps
/// the output in an HTML document that can run tests in a browser.
///
/// # Arguments
///
/// * `test_files` - List of paths to .test.st files
///
/// # Returns
///
/// HTML string ready to be served or written to a file
pub fn compile_test_html(test_files: &[PathBuf]) -> Result<String, CompileError> {
    compile_test_html_with_options(test_files, &CompileOptions::default())
}

/// Options for test compilation
#[derive(Debug, Clone, Default)]
pub struct CompileOptions {
    /// Filter tests by name pattern
    pub filter: Option<String>,
    /// Include source maps for debugging
    pub source_maps: bool,
    /// Custom HTML template
    pub template: Option<String>,
    /// Additional CSS to include
    pub extra_css: Option<String>,
    /// Additional JS to include
    pub extra_js: Option<String>,
}

/// Error during test compilation
#[derive(Debug)]
pub struct CompileError {
    pub kind: CompileErrorKind,
    pub file: Option<PathBuf>,
    pub message: String,
}

#[derive(Debug)]
pub enum CompileErrorKind {
    IoError,
    ParseError,
    CompileError,
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.file {
            Some(path) => write!(f, "{}: {}", path.display(), self.message),
            None => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for CompileError {}

/// Compile test files with custom options
pub fn compile_test_html_with_options(
    test_files: &[PathBuf],
    options: &CompileOptions,
) -> Result<String, CompileError> {
    let mut all_js = String::new();
    let mut all_css = String::new();
    let mut test_names = Vec::new();
    // Union of meta-registries across test files, used to demand-inject vendored
    // blobs (e.g. pretext) into the combined test JS — mirrors the site path.
    // Seeded from the stdlib tree (see vendor::stdlib_test_vendor_registry).
    let mut vendor_registry = crate::vendor::stdlib_test_vendor_registry();
    // Files whose compile produced ERRORS. Collected rather than returned at
    // the first one so a run reports EVERY broken file, not just the earliest.
    let mut compile_failures: Vec<String> = Vec::new();

    // Note: test runtime is added via {test_runtime} in the HTML template
    // Do NOT add it to all_js or it will be declared twice

    for file in test_files {
        let source = std::fs::read_to_string(file).map_err(|e| CompileError {
            kind: CompileErrorKind::IoError,
            file: Some(file.clone()),
            message: e.to_string(),
        })?;

        // Prepend the testing stdlib + assertion modules so macros are available during expansion
        let mut combined_source = format!(
            "{}\n{}\n{}\n{}\n{}\n{}\n{}\n\n// === Test File: {} ===\n\n{}",
            TESTING_STDLIB,
            TESTING_ASSERTIONS_TIMELINE,
            TESTING_ASSERTIONS_STATE,
            TESTING_ASSERTIONS_BINDING,
            TESTING_GENERATIVE,
            TESTING_TIMING,
            TESTING_AUTOMATION,
            file.display(),
            source
        );

        // Parse the file (with testing stdlib prepended)
        let mut ast = crate::parser::parse(&combined_source).map_err(|e| CompileError {
            kind: CompileErrorKind::ParseError,
            file: Some(file.clone()),
            message: format!("{:?}", e),
        })?;

        // Match Compiler::from_file/check: rewrite retired syntax before import
        // resolution and rematching, so a test source cannot compile differently
        // from the same source through the site path.
        let migration_registry = crate::compiler::cached_stdlib_registry().0;
        let version = crate::migrate::read_syntax_version(&ast.matches).version;
        let shim = crate::migrate::apply_migration_shim(
            combined_source,
            ast,
            &migration_registry,
            version.as_deref(),
        );
        combined_source = shim.source;
        ast = shim.ast;

        // Resolve @import so a .test.st can pull in modular stdlib (e.g.
        // `@import "stdlib/text"`). The testing stdlib is ALREADY prepended into
        // combined_source, so skip the `stdlib/testing/*` imports (re-resolving
        // them would disturb the working prepend-based expansion). Only genuinely
        // external module imports (text, mobile, …) need resolution here.
        let needs_resolution = ast
            .imports
            .iter()
            .any(|i| !i.path.starts_with("stdlib/testing"));
        if needs_resolution {
            let workspace_root =
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            // Resolve against the combined source's path-base via `file`; rematch
            // over combined_source so spans align with the parsed AST.
            ast = crate::parser::resolve_imports(&ast, file, &workspace_root).map_err(|e| {
                CompileError {
                    kind: CompileErrorKind::ParseError,
                    file: Some(file.clone()),
                    message: format!("import resolution failed: {e}"),
                }
            })?;
            if !ast.meta_defs.is_empty() {
                let main_sf = file.to_string_lossy().to_string();
                crate::parser::rematch_with_user_macros_in(
                    &mut ast,
                    &combined_source,
                    Some(&main_sf),
                );
            }
        }

        // Collect vendor declarations for demand injection after all files compile.
        for def in &ast.meta_defs {
            if let crate::parser::meta_ast::MetaDef::Vendor(v) = def {
                let _ = vendor_registry.register_vendor(v.clone());
            }
        }

        // Compile. A test under `<project>/tests/` mounts that project's
        // fixtures, so the project's `_prelude.st` (brand forms) participates
        // exactly as it does under serve/build/check — the nearest prelude
        // names the project; a repo-level test has none and compiles as before.
        let site_dir = crate::compiler::project_root_for(file);
        let site_dir = site_dir
            .join(crate::compiler::PROJECT_PRELUDE)
            .is_file()
            .then_some(site_dir);
        let compiled = crate::compiler::Compiler::from_ast(&ast)
            .with_site_dir(site_dir)
            .compile();
        if !compiled.pipeline_errors.is_empty() {
            // These are `pipeline_errors` — ERRORS. They were printed as
            // "⚠ Compile warnings" and then DISCARDED, so a test file that
            // failed to compile still ran, and could report PASSES. A gate
            // written against a feature that does not compile is exactly the
            // silent-acceptance class: `tests/score/clip-consumer.test.st`
            // emitted two E0949s, ran anyway, and reported "1 passed" — the
            // one that passed being a negative assertion satisfied vacuously
            // by an element nothing had wired up.
            //
            // A test file that does not compile is a FAILED test file. There
            // is no reading under which continuing is correct.
            eprintln!(
                "\x1b[31m✗\x1b[0m {} FAILED TO COMPILE — its tests cannot be trusted and were not run:",
                file.display()
            );
            for err in &compiled.pipeline_errors {
                eprintln!("  - {}: {}", err.code, err.message);
            }
            compile_failures.push(file.display().to_string());
            continue;
        }

        all_js.push_str(&format!("// From: {}\n", file.display()));
        all_js.push_str(&compiled.js);
        all_js.push('\n');

        all_css.push_str(&compiled.css);
        all_css.push('\n');

        // Extract test names for filtering
        for line in source.lines() {
            if let Some(name) = extract_test_name(line) {
                test_names.push(name);
            }
        }
    }

    // A test file that does not compile is a FAILED test file — never a
    // warning. This is a HARD ERROR because the alternative (printing and
    // continuing) is what let `tests/score/clip-consumer.test.st` report
    // "1 passed" against a feature that does not exist: two E0949s, printed
    // as warnings, discarded, and the vacuously-satisfied negative counted
    // as a pass.
    if !compile_failures.is_empty() {
        return Err(CompileError {
            kind: CompileErrorKind::CompileError,
            file: None,
            message: format!(
                "{} test file(s) failed to compile:\n  {}\n\nA test that cannot compile cannot make a claim. Fix the compile error, or the gate is asserting nothing.",
                compile_failures.len(),
                compile_failures.join("\n  ")
            ),
        });
    }

    // Add extra CSS/JS if provided
    if let Some(ref css) = options.extra_css {
        all_css.push_str(css);
        all_css.push('\n');
    }

    if let Some(ref js) = options.extra_js {
        all_js.push_str(js);
        all_js.push('\n');
    }

    // Demand-inject vendored blobs (e.g. pretext) into the combined test JS,
    // exactly like the site path: a blob's IIFE is prepended only when its global
    // is referenced in the emitted JS. Keeps unused-vendor test pages lean.
    all_js = crate::vendor::inject_vendor_preludes(&all_js, &vendor_registry);

    // Generate HTML
    let filter_js = match &options.filter {
        Some(f) => format!("'{}'", f),
        None => "null".to_string(),
    };

    let html = options.template.clone().unwrap_or_else(|| {
        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Spacetime Tests</title>
  <style>
    /* Test reporter base styles */
    :root {{
      --st-pass: oklch(0.6 0.2 145);
      --st-fail: oklch(0.6 0.2 25);
      --st-skip: oklch(0.6 0 0);
      --st-bg: oklch(0.98 0 0);
      --st-text: oklch(0.2 0 0);
    }}

    body {{
      font-family: system-ui, -apple-system, sans-serif;
      margin: 0;
      padding: 2rem;
      background: var(--st-bg);
      color: var(--st-text);
    }}

    #test-container {{
      max-width: 800px;
      margin: 0 auto;
    }}

    .test-summary {{
      padding: 1rem;
      border-radius: 0.5rem;
      margin-bottom: 1rem;
      font-weight: 500;
    }}

    .test-summary.passed {{
      background: oklch(0.9 0.1 145);
      color: oklch(0.3 0.15 145);
    }}

    .test-summary.failed {{
      background: oklch(0.9 0.1 25);
      color: oklch(0.3 0.15 25);
    }}

    .test-summary.running {{
      background: oklch(0.9 0.08 60);
      color: oklch(0.3 0.1 60);
    }}

    .test-list {{
      list-style: none;
      padding: 0;
      margin: 0;
    }}

    .test-item {{
      padding: 0.5rem 0.75rem;
      margin: 0.25rem 0;
      border-radius: 0.25rem;
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }}

    .test-item.passed {{
      background: oklch(0.95 0.05 145);
    }}

    .test-item.failed {{
      background: oklch(0.95 0.05 25);
    }}

    .test-item.skipped {{
      background: oklch(0.95 0 0);
      opacity: 0.6;
    }}

    .test-item .icon {{
      width: 1rem;
      text-align: center;
    }}

    .test-item.passed .icon::before {{ content: 'v'; color: var(--st-pass); }}
    .test-item.failed .icon::before {{ content: 'x'; color: var(--st-fail); }}
    .test-item.skipped .icon::before {{ content: '-'; color: var(--st-skip); }}

    .error-details {{
      margin-top: 1rem;
      padding: 1rem;
      background: oklch(0.95 0.05 25);
      border-left: 3px solid var(--st-fail);
      border-radius: 0 0.25rem 0.25rem 0;
    }}

    .error-details h4 {{
      margin: 0 0 0.5rem 0;
      color: oklch(0.3 0.15 25);
    }}

    .error-details pre {{
      margin: 0;
      font-size: 0.875rem;
      white-space: pre-wrap;
      color: oklch(0.4 0.1 25);
    }}

    /* User styles */
    {all_css}
  </style>
</head>
<body>
  <div id="test-container">
    <div id="test-summary" class="test-summary running">Running tests...</div>
    <ul id="test-list" class="test-list"></ul>
    <div id="error-details"></div>
  </div>

  <script>
    // Full Spacetime runtime (modular, loaded in order)
    {st_runtime}

    // Test runtime
    {test_runtime}
  </script>

  <script>
    // Compiled test code (separate script so parse errors are isolated)
    {all_js}
  </script>

  <script>
    // Run tests
    window.onerror = (msg, src, line, col, err) => {{
      // If test runner hasn't completed, report the error and signal done
      if (!window.__spacetime_test_complete) {{
        window.__spacetime_test_complete = true;
        window.__spacetime_test_results = {{
          passed: 0, failed: 1, skipped: 0,
          errors: [{{ name: 'script error', error: msg + ' (line ' + line + ')' }}]
        }};
        console.log('__ST_TEST_RESULTS__' + JSON.stringify(window.__spacetime_test_results));
      }}
    }};

    window.onload = async () => {{
      // An external driver (CDP runner) may invoke __spacetime_run_tests directly;
      // if so it sets __ST_DRIVEN to avoid this onload running the suite twice.
      if (window.__ST_DRIVEN) return;
      const summary = document.getElementById('test-summary');
      const list = document.getElementById('test-list');
      const errorDetails = document.getElementById('error-details');

      try {{
        const results = await window.__spacetime_run_tests(
          (typeof window.__ST_FILTER !== 'undefined') ? window.__ST_FILTER : {filter_js}
        );

        // Update summary
        summary.className = 'test-summary ' + (results.failed > 0 ? 'failed' : 'passed');
        summary.textContent = `${{results.passed}} passed, ${{results.failed}} failed, ${{results.skipped}} skipped`;

        // Populate test list
        const tests = window.__st_tests || [];
        for (const test of tests) {{
          const item = document.createElement('li');
          item.className = 'test-item ' + (test.status || 'pending');
          item.innerHTML = `<span class="icon"></span><span>${{test.name}}</span>`;
          list.appendChild(item);
        }}

        // Show errors
        if (results.errors.length > 0) {{
          let html = '<h3>Errors</h3>';
          for (const err of results.errors) {{
            html += `
              <div class="error-details">
                <h4>${{err.name}}</h4>
                <pre>${{err.error}}</pre>
              </div>
            `;
          }}
          errorDetails.innerHTML = html;
        }}

        // Signal completion for CLI runner
        window.__spacetime_test_complete = true;
        window.__spacetime_test_results = results;

        // Output JSON for CLI parsing
        console.log('__ST_TEST_RESULTS__' + JSON.stringify(results));

      }} catch (e) {{
        summary.className = 'test-summary failed';
        summary.textContent = 'Test runner error: ' + e.message;
        console.error(e);

        // Signal completion even on error so Playwright doesn't timeout
        window.__spacetime_test_complete = true;
        window.__spacetime_test_results = {{
          passed: 0, failed: 1, skipped: 0,
          errors: [{{ name: 'runner error', error: e.message, stack: e.stack }}]
        }};
        console.log('__ST_TEST_RESULTS__' + JSON.stringify(window.__spacetime_test_results));
      }}
    }};
  </script>
</body>
</html>"#,
            all_css = all_css,
            st_runtime = escape_script_close(&full_st_runtime()),
            test_runtime = escape_script_close(&generate_test_runtime()),
            all_js = escape_script_close(&all_js),
            filter_js = filter_js,
        )
    });

    Ok(html)
}

/// Escape any `</script` sequence inside JS destined for an inline `<script>`
/// block. When the test HTML is parsed by a REAL HTML parser (the `--cdp`
/// Chromium backend's `file://`/`data:` documents), a literal `</script` inside
/// the embedded runtime — e.g. the string `<script>alert(1)</script>` in a
/// sanitizer comment in editable.js — prematurely CLOSES the script element,
/// truncating it mid-source. The browser then reports "Unexpected end of input"
/// and NONE of the runtime runs, so `__spacetime_run_tests` is never defined and
/// every CDP test fails with "harness never became available". The V8 `logic`
/// backend evals the JS directly (no HTML parse) so it never hit this. The
/// standard fix is the HTML-script-embedding escape `<\/script`, which the JS
/// tokenizer reads identically (a `\/` in a regex/comment/string is just `/`)
/// but the HTML parser no longer sees as a close tag. Case-insensitive because
/// HTML end-tag matching is case-insensitive (`</SCRIPT>` closes too).
fn escape_script_close(js: &str) -> String {
    // Match `</script` followed by whitespace, `>`, or `/` (the HTML end-tag
    // open). Replace the `<` with `<\` — cheap, allocation-light, and only when
    // the sequence is actually present.
    if !js.to_ascii_lowercase().contains("</script") {
        return js.to_string();
    }
    let mut out = String::with_capacity(js.len() + 16);
    let bytes = js.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Look for `</script` case-insensitively at position i.
        if bytes[i] == b'<'
            && i + 8 <= bytes.len()
            && bytes[i + 1] == b'/'
            && js[i + 2..i + 8].eq_ignore_ascii_case("script")
        {
            out.push_str("<\\/script");
            i += 8; // consumed `</script`
        } else {
            // Push this full UTF-8 char (handle multibyte safely).
            let ch_len = utf8_char_len(bytes[i]);
            out.push_str(&js[i..i + ch_len]);
            i += ch_len;
        }
    }
    out
}

/// Byte length of the UTF-8 character starting at a lead byte.
fn utf8_char_len(lead: u8) -> usize {
    if lead < 0x80 {
        1
    } else if lead >> 5 == 0b110 {
        2
    } else if lead >> 4 == 0b1110 {
        3
    } else if lead >> 3 == 0b11110 {
        4
    } else {
        1
    }
}

/// Concatenate the full ST runtime (all modules in dependency order).
fn full_st_runtime() -> String {
    [
        RUNTIME_CORE,
        RUNTIME_RAF,
        RUNTIME_EASING,
        RUNTIME_COLOR,
        RUNTIME_INTERPOLATE,
        RUNTIME_STAGGER,
        RUNTIME_VALUE_FUNCTIONS,
        RUNTIME_TEMPLATES,
        RUNTIME_EDITABLE,
        &dev_hooks_js(),
    ]
    .join("\n\n")
}

/// The dev-hooks primitive source (debug-instrumentation globals). ONE source of
/// truth shared by the V8 `logic` backend (main.rs) and the Chromium `--cdp`
/// backend (this module's test HTML).
const DEV_HOOKS_PRIMITIVE_ST: &str = include_str!("../stdlib/__dev__/primitives/dev-hooks.st");

/// Extract the `%emit js { … }` body from the dev-hooks primitive so BOTH test
/// backends seed the debug globals (`__stDebugHook` / `__ST_DEBUG__`) that the
/// dev-runtime tests assert. The dev tier is macro-mounted and never loads in the
/// bare test runtime, so without this those globals are absent and the dev-hook
/// tests fail ("__stDebugHook should be an object"). Extracting from the
/// primitive (vs a hand-copied JS file) keeps it the single source of truth.
pub fn dev_hooks_js() -> String {
    let src = DEV_HOOKS_PRIMITIVE_ST;
    let start = match src.find("%emit js {") {
        Some(i) => i + "%emit js {".len(),
        None => return String::new(),
    };
    let tail = &src[start..];
    let last = tail.rfind('}').unwrap_or(tail.len());
    let body = &tail[..last];
    let last2 = body.rfind('}').unwrap_or(body.len());
    format!("(function(){{\n{}\n}})();", &body[..last2])
}

/// Generate the test runtime JavaScript
fn generate_test_runtime() -> String {
    r#"
// Spacetime Test Runtime
const __st_tests = [];
const __st_results = { passed: 0, failed: 0, skipped: 0, errors: [] };

// =============================================================================
// State Machine Runtime for Tests
// =============================================================================

class SpacetimeStateMachine {
  constructor(element, config) {
    this.element = element;
    this.state = config.initial || 'idle';
    this.transitions = config.transitions || [];
    this.states = config.states || {};

    // Set initial state
    this.element.dataset.stState = this.state;

    // Bind event listeners for transitions
    this._boundHandlers = [];
    for (const t of this.transitions) {
      if (t.on) {
        const handler = (e) => {
          if (this.state === t.from) {
            this._transitionTo(t.to);
          }
        };
        this._boundHandlers.push({ event: t.on, handler });
        this.element.addEventListener(t.on, handler);
      }
    }
  }

  _transitionTo(newState) {
    const oldState = this.state;
    this.state = newState;
    this.element.dataset.stState = newState;

    // Apply state styles if defined
    if (this.states[newState]) {
      Object.assign(this.element.style, this.states[newState]);
    }

    // Dispatch state change event
    this.element.dispatchEvent(new CustomEvent('st:statechange', {
      detail: { from: oldState, to: newState }
    }));
  }

  destroy() {
    for (const { event, handler } of this._boundHandlers) {
      this.element.removeEventListener(event, handler);
    }
    this._boundHandlers = [];
  }
}

// Global registry for state machines in tests
window.__st_machines = new Map();

// Register a state machine on an element
window.__st_register_machine = (element, config) => {
  // Clean up existing machine if any
  if (window.__st_machines.has(element)) {
    window.__st_machines.get(element).destroy();
  }
  const machine = new SpacetimeStateMachine(element, config);
  window.__st_machines.set(element, machine);
  return machine;
};

// Helper to dispatch custom events (for @when)
window.__st_dispatch = (element, eventName) => {
  element.dispatchEvent(new CustomEvent(eventName, { bubbles: true }));
};

// Helper to create objects from alternating key/value pairs.
// Used in tests to avoid object literals with colons (INIT-040 parser limitation).
// Usage: __obj("delay", 100, "from", "first") => { delay: 100, from: "first" }
window.__obj = (...args) => {
  const o = {};
  for (let i = 0; i < args.length; i += 2) {
    o[args[i]] = args[i + 1];
  }
  return o;
};

// =============================================================================
// Test Registration and Runner
// =============================================================================

// =============================================================================
// Generative testing (PLAN-027 W4): @property / @fuzz support.
// A seeded PRNG + structural value generators + a shrinker. Values are generated
// from a compact type spec the macros emit (see stdlib/testing/generative.st):
//   { kind:'number', min, max } | { kind:'int', min, max } | { kind:'bool' }
//   { kind:'string', corpus } | { kind:'array', of:<spec>, max } | { kind:'union', of:[...] }
// =============================================================================

// Mulberry32 — a tiny deterministic PRNG so failures are reproducible by seed.
window.__stRng = (seed) => {
  let a = seed >>> 0;
  return () => {
    a |= 0; a = (a + 0x6D2B79F5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
};

window.__stGenString = (rng, corpus) => {
  const pools = {
    ascii: 'abcdefghijklmnopqrstuvwxyz ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789',
    unicode: 'a\u00e9\u4e2d\ud83d\ude00\n\t"\'\\<>&{} \u0000\u00ff',
    json: '{}[]":,0123456789truefalsenul \\',
    html: '<>/="\' abcdivspan&;{}',
  };
  const pool = pools[corpus] || pools.ascii;
  const len = Math.floor(rng() * 24);
  let s = '';
  for (let i = 0; i < len; i++) s += pool[Math.floor(rng() * pool.length)];
  return s;
};

// Split a binding string `name: spec` (the raw text inside forall(...)/inputs(...))
// into { name, spec } where spec is the parsed generator spec object.
window.__stParseBinding = (text) => {
  let t = (text || '').trim();
  if (t.startsWith('(')) t = t.slice(1);
  if (t.endsWith(')')) t = t.slice(0, -1);
  t = t.trim();
  const i = t.indexOf(':');
  if (i < 0) return { name: t || 'x', spec: window.__stSpecFromText('string') };
  const name = t.slice(0, i).trim();
  const spec = window.__stSpecFromText(t.slice(i + 1).trim());
  return { name, spec };
};

// Parse a compact type-spec STRING (as written after `:` in a @property/@fuzz
// binding) into the spec object __stGen understands. Examples:
//   'int 0..100' 'number' 'number 1..2' 'bool' 'string' 'string ~ unicode' 'int[]'
window.__stSpecFromText = (text) => {
  let t = (text || '').trim();
  let isArray = false;
  if (t.endsWith('[]')) { isArray = true; t = t.slice(0, -2).trim(); }
  let spec;
  // string ~ corpus
  const corpusMatch = t.match(/^string\s*~\s*(\w+)$/);
  const rangeMatch = t.match(/^(int|number)\s+(-?[0-9.]+)\s*\.\.\s*(-?[0-9.]+)$/);
  if (corpusMatch) {
    spec = { kind: 'string', corpus: corpusMatch[1] };
  } else if (rangeMatch) {
    spec = { kind: rangeMatch[1], min: parseFloat(rangeMatch[2]), max: parseFloat(rangeMatch[3]) };
  } else if (t === 'int' || t === 'number') {
    spec = { kind: t };
  } else if (t === 'bool') {
    spec = { kind: 'bool' };
  } else if (t === 'string') {
    spec = { kind: 'string', corpus: 'ascii' };
  } else {
    // Unknown / @type name — fall back to a string (FEAT-065 will resolve @type).
    spec = { kind: 'string', corpus: 'ascii' };
  }
  return isArray ? { kind: 'array', of: spec, max: 16 } : spec;
};

window.__stGen = (spec, rng) => {
  switch (spec.kind) {
    case 'number': {
      const min = spec.min ?? 0, max = spec.max ?? 100;
      return min + rng() * (max - min);
    }
    case 'int': {
      const min = Math.ceil(spec.min ?? 0), max = Math.floor(spec.max ?? 100);
      return min + Math.floor(rng() * (max - min + 1));
    }
    case 'bool': return rng() < 0.5;
    case 'string': return window.__stGenString(rng, spec.corpus || 'ascii');
    case 'array': {
      const n = Math.floor(rng() * ((spec.max ?? 16) + 1));
      return Array.from({ length: n }, () => window.__stGen(spec.of, rng));
    }
    case 'union': return spec.of[Math.floor(rng() * spec.of.length)];
    default: return undefined;
  }
};

// Structural shrink: yield 'simpler' candidates toward a minimal failing case.
window.__stShrinkCandidates = (spec, value) => {
  const out = [];
  switch (spec.kind) {
    case 'number': case 'int':
      if (value !== 0) { out.push(0); out.push(spec.kind === 'int' ? Math.trunc(value / 2) : value / 2); }
      break;
    case 'string':
      if (value.length > 0) { out.push(''); out.push(value.slice(0, Math.floor(value.length / 2))); }
      break;
    case 'array':
      if (value.length > 0) {
        out.push([]);
        out.push(value.slice(0, Math.floor(value.length / 2)));
        if (value.length > 1) out.push(value.slice(1));
      }
      break;
  }
  return out;
};

// Run a property: generate `cases` inputs, find a failure, shrink it, throw the
// minimal counterexample. `check(value)` returns true on PASS, false/throw on FAIL.
window.__stProperty = (spec, cases, seed, check) => {
  const rng = window.__stRng(seed);
  const fails = (v) => { try { return !check(v); } catch (_e) { return true; } };
  for (let i = 0; i < cases; i++) {
    const v = window.__stGen(spec, rng);
    if (fails(v)) {
      // Shrink: greedily replace with a simpler still-failing candidate.
      let cur = v;
      let improved = true;
      while (improved) {
        improved = false;
        for (const cand of window.__stShrinkCandidates(spec, cur)) {
          if (fails(cand)) { cur = cand; improved = true; break; }
        }
      }
      throw new Error('property failed (seed ' + seed + ') minimal counterexample: ' + JSON.stringify(cur));
    }
  }
};

// Run a fuzz target: generate `cases` inputs; FAIL if `body(value)` throws or
// exceeds the per-case time budget (a hang proxy).
window.__stFuzz = (spec, cases, seed, budgetMs, body) => {
  const rng = window.__stRng(seed);
  for (let i = 0; i < cases; i++) {
    const v = window.__stGen(spec, rng);
    const t0 = (typeof performance !== 'undefined' ? performance.now() : Date.now());
    try {
      body(v);
    } catch (e) {
      throw new Error('fuzz crash (seed ' + seed + ') on input ' + JSON.stringify(v) + ': ' + e.message);
    }
    const dt = (typeof performance !== 'undefined' ? performance.now() : Date.now()) - t0;
    if (dt > budgetMs) {
      throw new Error('fuzz timeout (' + dt.toFixed(0) + 'ms > ' + budgetMs + 'ms, seed ' + seed + ') on input ' + JSON.stringify(v));
    }
  }
};

window.__spacetime_register_test = (name, fn, options = {}) => {
  __st_tests.push({ name, fn, status: 'pending', ...options });
};

// Vacuous-test detection (BUG-309): a test that executes ZERO assertions has
// asserted nothing, so it must FAIL rather than pass. Every assertion macro
// calls __st_noteAssertion() as its first emitted statement; a directive whose
// grammar matches no %form (e.g. `@then $x on document should equal …` —
// `document` is not a Selector) is dropped by the expander and emits NO note,
// leaving the per-test counter at 0. The runner turns that into a failure
// naming the test, so the whole vacuity class is self-detecting regardless of
// which directive shape vanished.
let __st_currentAssertions = 0;
let __st_allowZeroAssertions = false;
window.__st_noteAssertion = () => { __st_currentAssertions++; };
// Explicit opt-out for tests whose purpose is 'this compiles' (no claim to
// make). A body directive emits __st_markNoAssert() to opt in.
window.__st_markNoAssert = () => { __st_allowZeroAssertions = true; };

window.__spacetime_run_tests = async (filter) => {
  // Fidelity ladder (PLAN-027 W1/W2): this harness runs in a REAL browser
  // (CDP / Playwright), so it can faithfully provide every rung up to `paint`.
  // Advertise that ceiling so layout/timing/paint assertions RUN here instead of
  // refusing as they (correctly) do on the V8 backend. An explicit override may
  // be injected as window.__ST_BACKEND_RUNG.
  if (window.ST && ST._rung) {
    ST._rung.setBackend(window.__ST_BACKEND_RUNG || 'paint');
  }

  __st_results.passed = 0;
  __st_results.failed = 0;
  __st_results.skipped = 0;
  __st_results.errors = [];

  const tests = filter
    ? __st_tests.filter(t => t.name.includes(filter))
    : __st_tests;

  // Handle .only tests
  const onlyTests = tests.filter(t => t.only);
  const testsToRun = onlyTests.length > 0 ? onlyTests : tests;

  for (const test of testsToRun) {
    // Clean up state machines from previous test
    for (const machine of window.__st_machines.values()) {
      machine.destroy();
    }
    window.__st_machines.clear();

    if (test.skip) {
      test.status = 'skipped';
      __st_results.skipped++;
      console.log(`SKIP: ${test.name}`);
      continue;
    }

    // Skip non-.only tests if there are .only tests
    if (onlyTests.length > 0 && !test.only) {
      test.status = 'skipped';
      __st_results.skipped++;
      continue;
    }

    try {
      test.status = 'running';
      __st_currentAssertions = 0;
      __st_allowZeroAssertions = false;
      await test.fn();
      // A test that ran but made no assertions has verified nothing — treat it
      // as a failure, not a pass (BUG-309). Skipped tests never reach here.
      if (__st_currentAssertions === 0 && !__st_allowZeroAssertions) {
        throw new Error(
          'Test made no assertions — its claims did not compile to anything ' +
          '(e.g. a directive whose shape matched no macro was silently dropped). ' +
          'Every @test must execute at least one assertion; if this test is ' +
          'intentionally compile-only, mark its body with @no_assert.');
      }
      test.status = 'passed';
      __st_results.passed++;
      console.log(`PASS: ${test.name}`);
    } catch (e) {
      // A fidelity refusal (ST._rung.require on a backend that can't provide the
      // needed rung) is a DEFERRAL to a higher-fidelity backend (CDP), not a
      // failure. Count it as skipped so a logic-rung run reports 0 failures while
      // still surfacing the deferral; the same test actually runs on --cdp.
      if (e && e.__stRungSkip) {
        test.status = 'skipped';
        __st_results.skipped++;
        console.log(`SKIP (needs '${e.__stRungNeed}' fidelity): ${test.name}`);
      } else {
        test.status = 'failed';
        __st_results.failed++;
        __st_results.errors.push({
          name: test.name,
          error: e.message,
          stack: e.stack
        });
        console.error(`FAIL: ${test.name}`);
        console.error(`  ${e.message}`);
      }
    }
  }

  console.log(`\nResults: ${__st_results.passed} passed, ${__st_results.failed} failed, ${__st_results.skipped} skipped`);
  return __st_results;
};

// Make tests available globally
window.__st_tests = __st_tests;
"#
    .to_string()
}

/// Extract test name from a source line
fn extract_test_name(line: &str) -> Option<String> {
    let line = line.trim();

    // Match @test "name" or @test.skip "name" or @test.only "name"
    if line.starts_with("@test") {
        // Find the quoted string
        if let Some(start) = line.find('"')
            && let Some(end) = line[start + 1..].find('"')
        {
            return Some(line[start + 1..start + 1 + end].to_string());
        }
    }

    None
}

/// Parse test results from JSON output
///
/// Used when running tests in headless browser mode
pub fn parse_test_results(json: &str) -> Result<TestResults, String> {
    // Find the magic marker in the output
    const MARKER: &str = "__ST_TEST_RESULTS__";
    let json = if let Some(pos) = json.find(MARKER) {
        &json[pos + MARKER.len()..]
    } else {
        json
    };

    // Parse JSON
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("Failed to parse results: {}", e))?;

    let passed = value["passed"].as_u64().unwrap_or(0) as u32;
    let failed = value["failed"].as_u64().unwrap_or(0) as u32;
    let skipped = value["skipped"].as_u64().unwrap_or(0) as u32;

    let mut errors = Vec::new();
    if let Some(arr) = value["errors"].as_array() {
        for err in arr {
            errors.push(TestError {
                name: err["name"].as_str().unwrap_or("").to_string(),
                message: err["error"].as_str().unwrap_or("").to_string(),
                stack: err["stack"].as_str().map(|s| s.to_string()),
                file: None,
                line: None,
            });
        }
    }

    Ok(TestResults {
        passed,
        failed,
        skipped,
        errors,
        total: passed + failed + skipped,
        duration_ms: value["duration"].as_u64().unwrap_or(0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_tests() {
        let tests = discover_tests(&PathBuf::from("tests/"));
        // The function should work even if no test files exist
        assert!(
            tests.is_empty()
                || tests
                    .iter()
                    .all(|p| p.to_string_lossy().contains(".test.st"))
        );
    }

    #[test]
    fn test_escape_script_close() {
        // No `</script` — returned unchanged (cheap path).
        assert_eq!(escape_script_close("const x = 1;"), "const x = 1;");
        // A literal `</script>` in a comment/string is escaped so a real HTML
        // parser (the --cdp backend) doesn't close the inline <script> early.
        assert_eq!(
            escape_script_close("// <script>alert(1)</script>"),
            "// <script>alert(1)<\\/script>",
        );
        // Case-insensitive match (HTML end-tags match case-insensitively); the
        // replacement normalizes to lowercase `script`, which is JS-equivalent.
        assert_eq!(escape_script_close("a</SCRIPT>b"), "a<\\/script>b");
        // `</scripted` is NOT a close tag boundary, but the `</script` prefix IS
        // still neutralized (harmless to JS, safe for HTML) — matches prefix.
        assert_eq!(escape_script_close("</scripts"), "<\\/scripts");
        // The escaped form is JS-equivalent: `<\/script` evals the same as
        // `</script` inside a string/comment/regex (the `\/` is just `/`).
        // Multibyte UTF-8 around the match survives intact.
        assert_eq!(escape_script_close("é</script>é"), "é<\\/script>é",);
        // The real-world trigger: editable.js's sanitizer comment. The full ST
        // runtime must contain NO bare `</script` after escaping.
        let escaped = escape_script_close(&full_st_runtime());
        assert!(
            !escaped.to_ascii_lowercase().contains("</script"),
            "escaped runtime still contains a bare </script close sequence",
        );
    }

    #[test]
    fn test_is_test_file() {
        let options = DiscoverOptions::new();

        assert!(is_test_file(&PathBuf::from("foo.test.st"), &options));
        assert!(is_test_file(&PathBuf::from("bar.test.st"), &options));
        assert!(!is_test_file(&PathBuf::from("foo.st"), &options));
        assert!(!is_test_file(&PathBuf::from("test.rs"), &options));
    }

    #[test]
    fn test_extract_test_name() {
        assert_eq!(
            extract_test_name(r#"@test "my test" {"#),
            Some("my test".to_string())
        );
        assert_eq!(
            extract_test_name(r#"@test.skip "skipped test" {"#),
            Some("skipped test".to_string())
        );
        assert_eq!(
            extract_test_name(r#"@test.only "focused test" {"#),
            Some("focused test".to_string())
        );
        assert_eq!(extract_test_name("// comment"), None);
        assert_eq!(extract_test_name("@given .box {}"), None);
    }

    #[test]
    fn test_parse_test_results() {
        let json = r#"{"passed": 5, "failed": 2, "skipped": 1, "errors": [{"name": "test1", "error": "assertion failed"}]}"#;

        let results = parse_test_results(json).unwrap();
        assert_eq!(results.passed, 5);
        assert_eq!(results.failed, 2);
        assert_eq!(results.skipped, 1);
        assert_eq!(results.errors.len(), 1);
        assert_eq!(results.errors[0].name, "test1");
    }

    #[test]
    fn test_parse_test_results_with_marker() {
        let output = r#"
PASS: test1
PASS: test2
FAIL: test3
__ST_TEST_RESULTS__{"passed": 2, "failed": 1, "skipped": 0, "errors": []}
"#;

        let results = parse_test_results(output).unwrap();
        assert_eq!(results.passed, 2);
        assert_eq!(results.failed, 1);
    }

    /// BUG-332: a run that executed NOTHING must not read as success.
    ///
    /// `0 passed, 0 failed, N skipped` was printed GREEN and exited 0, because
    /// success was decided by `failed == 0` alone. That is the shape a whole
    /// dead suite takes: when stdlib load breaks, every `.st` file executes zero
    /// claims and CI goes green over a corpus that compiled nothing.
    ///
    /// The invariant is NOT "skipped > 0 is a failure" — skipping is CORRECT and
    /// load-bearing (a `needs layout` claim refuses the logic backend by design,
    /// see docs/testing/FIDELITY_LADDER.md). It is: tests EXISTED and none RAN,
    /// so the run proved nothing and must not claim it did.
    #[test]
    fn a_run_that_executed_nothing_is_vacant() {
        let vacant = TestResults {
            passed: 0,
            failed: 0,
            skipped: 4,
            errors: vec![],
            total: 4,
            duration_ms: 10,
        };

        assert!(
            vacant.is_vacant(),
            "0 executed of 4 tests proves nothing — must be vacant"
        );
        assert!(
            !vacant.is_success(),
            "a vacant run must NOT be reported as success (it exited 0 before BUG-332)"
        );
    }

    /// The other half: a PARTIALLY skipped run is a perfectly good run.
    ///
    /// Without this, the obvious over-fix (`skipped > 0` ⇒ fail) would break every
    /// legitimate `--headless` run of a file carrying `needs layout` claims, and
    /// the fidelity ladder would become unusable.
    #[test]
    fn a_partially_skipped_run_is_not_vacant() {
        let mixed = TestResults {
            passed: 2,
            failed: 0,
            skipped: 2,
            errors: vec![],
            total: 4,
            duration_ms: 10,
        };

        assert!(
            !mixed.is_vacant(),
            "2 claims DID execute — skipping the layout half is the ladder working"
        );
        assert!(mixed.is_success(), "a partially-skipped green run is green");
    }

    /// An empty selection (no tests matched) is not vacant either: there was
    /// nothing to prove, so there is no false claim of proof. Reporting it as a
    /// failure would break `--filter` workflows that legitimately match nothing.
    #[test]
    fn an_empty_selection_is_not_vacant() {
        let empty = TestResults::default();

        assert!(!empty.is_vacant(), "no tests selected ≠ tests that did not run");
        assert!(empty.is_success());
    }

    /// Vacancy must not mask a real failure: failures still decide success.
    #[test]
    fn a_failing_run_is_not_success_regardless_of_vacancy() {
        let failing = TestResults {
            passed: 0,
            failed: 1,
            skipped: 3,
            errors: vec![],
            total: 4,
            duration_ms: 10,
        };

        assert!(!failing.is_vacant(), "a claim ran and failed — not vacant");
        assert!(!failing.is_success());
    }

    #[test]
    fn test_results_summary() {
        let results = TestResults {
            passed: 10,
            failed: 2,
            skipped: 3,
            errors: vec![],
            total: 15,
            duration_ms: 150,
        };

        assert_eq!(results.summary(), "10 passed, 2 failed, 3 skipped (150ms)");
        assert!(!results.all_passed());
    }

    #[test]
    fn test_results_all_passed() {
        let results = TestResults {
            passed: 10,
            failed: 0,
            skipped: 0,
            errors: vec![],
            total: 10,
            duration_ms: 100,
        };

        assert!(results.all_passed());
    }

    #[test]
    fn test_layer_subdir() {
        assert_eq!(TestLayer::Unit.subdir(), Some("unit"));
        assert_eq!(TestLayer::Integration.subdir(), Some("integration"));
        assert_eq!(TestLayer::E2e.subdir(), Some("e2e"));
        assert_eq!(TestLayer::Meta.subdir(), Some("meta"));
        assert_eq!(TestLayer::All.subdir(), None);
    }

    #[test]
    fn test_discover_tests_by_layer() {
        let base = PathBuf::from("tests/");

        // Unit tests should be in tests/unit/
        let unit_tests = discover_tests_by_layer(&base, TestLayer::Unit);
        for test in &unit_tests {
            assert!(test.to_string_lossy().contains("/unit/"));
        }

        // Integration tests should be in tests/integration/
        let integration_tests = discover_tests_by_layer(&base, TestLayer::Integration);
        for test in &integration_tests {
            assert!(test.to_string_lossy().contains("/integration/"));
        }
    }
}
