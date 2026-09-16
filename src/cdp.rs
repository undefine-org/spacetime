//! CDP browser substrate (PLAN-027 W2).
//!
//! Drives a warm Chromium over the Chrome DevTools Protocol (via `chromiumoxide`)
//! to provide the `layout` / `timing` / `paint` fidelity rungs that the V8 +
//! LinkeDOM backend cannot (real `getComputedStyle`, `Intl.Segmenter`, Canvas,
//! real `rAF`/layout). This is what lets `@balance`, pretext reflow, visibility
//! and dimension assertions run *for real* instead of refusing (W1) or faking.
//!
//! ## Design
//! - **Warm browser, per-test page.** The browser process is launched ONCE
//!   (lazily, behind a process-global [`OnceLock`]) and reused across every test
//!   file; each file runs in a fresh page (tab) that is closed afterwards. The
//!   launch cost is paid once, not per test.
//! - **Swappable endpoint.** The same client connects to a locally-launched
//!   Chromium today and (W7) to a remote CDP websocket (`via kernel <wsUrl>`,
//!   kernel.sh :9222) with no change to the test-facing surface.
//! - **Backend-agnostic protocol.** The page sets `window.__spacetime_test_complete`
//!   + `window.__spacetime_test_results`; the browser-side harness advertises the
//!   `paint` rung so layout+ assertions run. Identical contract to the legacy
//!   Playwright path, so the compiled test HTML is unchanged.
//! - **async↔sync bridge.** The harness exposes a synchronous [`run_test_files`]
//!   that blocks on a dedicated multi-thread Tokio runtime; the chromiumoxide
//!   `Handler` future is spawned on that runtime for the browser's lifetime.

#![cfg(feature = "cdp")]

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use chromiumoxide::Page;
use chromiumoxide::browser::{Browser, BrowserConfig};
use futures_util::StreamExt;
use tokio::runtime::Runtime;

use crate::test_runner::{CompileOptions, TestError, TestResults, compile_test_html_with_options};

/// How long to wait for a test page to finish (`__spacetime_test_complete`).
const TEST_TIMEOUT_SECS: u64 = 30;

/// Process-global warm browser + its driving Tokio runtime. Created on first use.
pub(crate) struct Infra {
    pub(crate) rt: Runtime,
    pub(crate) browser: Mutex<Browser>,
}

static INFRA: OnceLock<Result<Infra, String>> = OnceLock::new();

/// Locate a Chromium/Chrome executable: explicit env override, then chromiumoxide's
/// own detection, then common system paths (the `(a)` local-discovery path of the
/// W2 provisioning decision).
pub fn find_chrome() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("SPACETIME_CHROME").or_else(|_| std::env::var("CHROME")) {
        let pb = PathBuf::from(&p);
        if pb.exists() {
            return Some(pb);
        }
    }
    for cand in [
        "google-chrome-stable",
        "google-chrome",
        "chromium",
        "chromium-browser",
        "chrome",
    ] {
        if let Ok(out) = std::process::Command::new("which").arg(cand).output() {
            if out.status.success() {
                let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(PathBuf::from(path));
                }
            }
        }
    }
    for cand in [
        "/usr/bin/google-chrome-stable",
        "/usr/bin/google-chrome",
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
    ] {
        let pb = PathBuf::from(cand);
        if pb.exists() {
            return Some(pb);
        }
    }
    // Last resort: a Puppeteer-managed Chromium under ~/.cache/puppeteer/chrome
    // (the common CI / dev-box layout where no system Chrome is installed). Pick
    // the highest-versioned `chrome` binary so the backend works out-of-the-box
    // without requiring SPACETIME_CHROME. linux64/mac layouts both covered.
    if let Some(p) = find_puppeteer_chrome() {
        return Some(p);
    }
    None
}

/// Discover a Puppeteer-downloaded Chrome under `~/.cache/puppeteer/chrome`.
/// Each revision lives in `<rev>/chrome-<platform>/chrome` (Linux) or
/// `.../Google Chrome for Testing.app/...` (macOS). Returns the lexically
/// greatest revision dir's binary (good enough — revisions sort by version).
fn find_puppeteer_chrome() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let base = PathBuf::from(home).join(".cache/puppeteer/chrome");
    let mut revs: Vec<PathBuf> = std::fs::read_dir(&base)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    revs.sort();
    for rev in revs.into_iter().rev() {
        // Linux: <rev>/chrome-linux64/chrome ; older: chrome-linux/chrome
        for sub in ["chrome-linux64/chrome", "chrome-linux/chrome"] {
            let bin = rev.join(sub);
            if bin.exists() {
                return Some(bin);
            }
        }
        // macOS: <rev>/chrome-mac-*/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing
        if let Ok(entries) = std::fs::read_dir(&rev) {
            for e in entries.flatten() {
                let app = e
                    .path()
                    .join("Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing");
                if app.exists() {
                    return Some(app);
                }
            }
        }
    }
    None
}

fn init_infra() -> Result<Infra, String> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|e| format!("failed to build Tokio runtime: {e}"))?;

    // Unique profile dir per process so a previous (killed) run's SingletonLock
    // cannot block launch, and concurrent runs don't collide.
    let user_data = std::env::temp_dir().join(format!(
        "spacetime-cdp-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));

    // Remote CDP endpoint (PLAN-027 W7, `via kernel`): if SPACETIME_CDP_WS is set,
    // connect to an existing browser (kernel.sh :9222 / connectOverCDP model)
    // instead of launching one locally. SAME client, swapped endpoint — zero
    // surface delta for @drive.
    if let Ok(ws) = std::env::var("SPACETIME_CDP_WS") {
        if !ws.trim().is_empty() {
            let browser = rt.block_on(async move {
                let (browser, mut handler) =
                    tokio::time::timeout(std::time::Duration::from_secs(25), Browser::connect(ws))
                        .await
                        .map_err(|_| "remote CDP connect timed out after 25s".to_string())?
                        .map_err(|e| format!("failed to connect to remote CDP: {e}"))?;
                tokio::spawn(async move { while handler.next().await.is_some() {} });
                Ok::<Browser, String>(browser)
            })?;
            return Ok(Infra {
                rt,
                browser: Mutex::new(browser),
            });
        }
    }

    let browser = rt.block_on(async move {
        let mut builder = BrowserConfig::builder()
            .no_sandbox()
            .user_data_dir(&user_data)
            // PLAN-150 W4: 3D behaviour gates (@stage/@object/@particles) need a
            // working WebGL context in headless Chromium. A bare `--disable-gpu`
            // leaves `canvas.getContext('webgl')` null, so `_stThree` never
            // initialises and every 3D gate fails with "undefined (reading
            // 'scene')". SwiftShader is Chromium's software GL — deterministic,
            // no GPU required, exactly what a headless film gate wants. These
            // flags turn it on; `--use-gl=angle --use-angle=swiftshader` is the
            // supported modern spelling, with the legacy alias as fallback.
            .arg("--use-gl=angle")
            .arg("--use-angle=swiftshader")
            .arg("--enable-unsafe-swiftshader")
            .arg("--disable-dev-shm-usage");
        if let Some(chrome) = find_chrome() {
            builder = builder.chrome_executable(chrome);
        }
        let config = builder
            .build()
            .map_err(|e| format!("failed to build browser config: {e}"))?;

        // CRITICAL: `Browser::launch` only resolves once the returned `Handler`
        // future is being polled (the launch handshake flows through it). So we
        // spawn the handler-driver as a task BEFORE awaiting launch — but launch
        // gives us the handler, so instead we poll launch and the handler together.
        // chromiumoxide returns (browser, handler) from one await; spawn the
        // handler immediately and keep it alive for the runtime's lifetime.
        let (browser, mut handler) =
            tokio::time::timeout(std::time::Duration::from_secs(25), Browser::launch(config))
                .await
                .map_err(|_| "Chromium launch timed out after 25s".to_string())?
                .map_err(|e| format!("failed to launch Chromium: {e}"))?;
        tokio::spawn(async move { while handler.next().await.is_some() {} });
        Ok::<Browser, String>(browser)
    })?;

    Ok(Infra {
        rt,
        browser: Mutex::new(browser),
    })
}

pub(crate) fn infra() -> Result<&'static Infra, String> {
    INFRA
        .get_or_init(|| init_infra())
        .as_ref()
        .map_err(|e| e.clone())
}

/// Is a CDP backend available on this machine? (Chrome discoverable.)
pub fn is_available() -> bool {
    find_chrome().is_some()
}

/// Close the warm browser if one was launched. Call at the end of a CLI run so
/// the Chromium child process does not leak. Safe to call when no browser exists.
pub fn shutdown() {
    if let Some(Ok(infra)) = INFRA.get() {
        if let Ok(mut browser) = infra.browser.lock() {
            let _ = infra.rt.block_on(async {
                let _ = browser.close().await;
                browser.wait().await
            });
        }
    }
}

/// Run the given test files in a warm Chromium and return aggregated results.
///
/// Synchronous: blocks on the shared runtime. Each file is compiled to the same
/// test HTML the Playwright path used and loaded via a base64 data: URL in a fresh
/// page; results are read from `window.__spacetime_test_results`.
pub fn run_test_files(test_files: &[PathBuf], filter: Option<&str>) -> Result<TestResults, String> {
    let start = Instant::now();
    let infra = infra()?;

    let mut agg = TestResults::default();
    for file in test_files {
        let html =
            compile_test_html_with_options(std::slice::from_ref(file), &CompileOptions::default())
                .map_err(|e| format!("compile error in {}: {e}", file.display()))?;

        let one = infra
            .rt
            .block_on(run_one_page(&infra.browser, &html, filter))
            .unwrap_or_else(|e| TestResults {
                passed: 0,
                failed: 1,
                skipped: 0,
                total: 1,
                duration_ms: 0,
                errors: vec![TestError {
                    name: format!("cdp:{}", file.display()),
                    message: e,
                    stack: None,
                    file: Some(file.clone()),
                    line: None,
                }],
            });

        agg.passed += one.passed;
        agg.failed += one.failed;
        agg.skipped += one.skipped;
        agg.total += one.total;
        agg.errors.extend(one.errors);
    }
    agg.duration_ms = start.elapsed().as_millis() as u64;
    Ok(agg)
}

/// A staged HTML file under a `file://` URL, removed on Drop. Top-level `data:`
/// documents don't run inline scripts in modern Chrome (opaque origin), so the
/// CDP backend serves each test page from a real temp file instead.
pub(crate) struct HtmlFileGuard {
    pub(crate) path: PathBuf,
}

impl HtmlFileGuard {
    pub(crate) fn write(html: &str) -> Result<Self, String> {
        let path = std::env::temp_dir().join(format!(
            "spacetime-cdp-page-{}-{}.html",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::write(&path, html).map_err(|e| format!("failed to stage test HTML: {e}"))?;
        Ok(Self { path })
    }

    pub(crate) fn url(&self) -> String {
        format!("file://{}", self.path.display())
    }
}

impl Drop for HtmlFileGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Open a fresh page (tab) in the shared warm browser — isolation without a new
/// process. Shared by the test backend (`run_one_page`) and the render backend
/// (`render`, PLAN-132 W5): both drive a page over CDP, so both open one the
/// same way instead of each inlining the lock+new_page dance.
pub(crate) async fn open_page(browser: &Mutex<Browser>) -> Result<Page, String> {
    let browser = browser
        .lock()
        .map_err(|_| "browser mutex poisoned".to_string())?;
    browser
        .new_page("about:blank")
        .await
        .map_err(|e| format!("new_page failed: {e}"))
}

async fn run_one_page(

    browser: &Mutex<Browser>,
    html: &str,
    filter: Option<&str>,
) -> Result<TestResults, String> {
    // A fresh page (tab) per test file — isolation without a new process.
    let page = open_page(browser).await?;

    // Navigate to the HTML via a `file://` URL so the page's inline `<script>`
    // tags actually EXECUTE. Modern Chrome does NOT run inline scripts when the
    // top-level document is a `data:` URL (the document inherits an opaque origin
    // and the inline scripts are blocked), which left __spacetime_run_tests
    // undefined — the page rendered (title/DOM present) but no runtime ran.
    // `Page.setContent` has the same problem under CDP. A real `file://` document
    // runs inline scripts normally, so we stage the HTML to a temp file and clean
    // it up when the page is done (HtmlFileGuard on Drop).
    let html_file = HtmlFileGuard::write(html)?;
    // Set __ST_DRIVEN before ANY page script runs so the HTML's window.onload
    // doesn't auto-run the suite (the runner invokes it directly; a double run
    // would inflate counts and drive components twice).
    let _ = page
        .evaluate_on_new_document("window.__ST_DRIVEN = true;")
        .await;
    page.goto(html_file.url())
        .await
        .map_err(|e| format!("goto(file:) failed: {e}"))?;

    // Don't depend on `window.onload` (it may have already fired by the time the
    // content is parsed). Wait until the harness function is defined, then INVOKE
    // it directly and await the returned promise — deterministic, no onload race.
    let ready_deadline = Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let ready = page
            .evaluate("typeof window.__spacetime_run_tests === 'function'")
            .await
            .ok()
            .and_then(|r| r.into_value::<bool>().ok())
            .unwrap_or(false);
        if ready {
            break;
        }
        if Instant::now() > ready_deadline {
            let _ = page.close().await;
            return Err("test harness (__spacetime_run_tests) never became available".to_string());
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }

    // AUTOMATION (PLAN-027 W7): if any registered test carries a `drive` option,
    // it expects a LIVE page at that URL. Navigate there, then re-inject the
    // runtime + test registrations (the data: page's scripts don't survive
    // navigation), so the body's @when/@then run against the live DOM.
    let drive_url = page
        .evaluate(
            "(() => { const t = (window.__st_tests||[]).find(x => x.drive); return t ? t.drive : null; })()",
        )
        .await
        .ok()
        .and_then(|r| r.into_value::<Option<String>>().ok())
        .flatten();
    if let Some(url) = drive_url {
        // Capture the page's own scripts (runtime + tests) so we can replay them
        // into the live page after navigation.
        let scripts: String = page
            .evaluate(
                "Array.from(document.querySelectorAll('script')).map(s => s.textContent).join('\\n;\\n')",
            )
            .await
            .ok()
            .and_then(|r| r.into_value::<String>().ok())
            .unwrap_or_default();
        page.goto(&url)
            .await
            .map_err(|e| format!("@drive goto {url} failed: {e}"))?;
        let _ = page.wait_for_navigation().await;
        // Re-establish the harness in the live document, then mark driven.
        // NB: @then-claims throws are currently swallowed here (BUG-058); @assert
        // gates correctly. The live-DOM access + navigation (the W7 thesis) work.
        let _ = page.evaluate(scripts).await;
        let _ = page.evaluate("window.__ST_DRIVEN = true;").await;
        // Wait for the re-injected harness.
        let dl = Instant::now() + std::time::Duration::from_secs(10);
        loop {
            let ready = page
                .evaluate("typeof window.__spacetime_run_tests === 'function'")
                .await
                .ok()
                .and_then(|r| r.into_value::<bool>().ok())
                .unwrap_or(false);
            if ready || Instant::now() > dl {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
    }

    // Run the suite and serialise results in one evaluate_function call. The
    // harness awaits each async test (W0 await contract) and advertises the
    // `paint` rung (W1/W2), so layout+ assertions run for real.
    let filter_arg = match filter {
        Some(f) => serde_json::to_string(f).unwrap(),
        None => "undefined".to_string(),
    };
    let run_js = format!(
        "async () => {{ window.__ST_DRIVEN = true; \
         const r = await window.__spacetime_run_tests({filter_arg}); \
         return JSON.stringify(r); }}"
    );
    let results_json = tokio::time::timeout(
        std::time::Duration::from_secs(TEST_TIMEOUT_SECS),
        page.evaluate_function(run_js),
    )
    .await
    .map_err(|_| format!("tests did not complete within {TEST_TIMEOUT_SECS}s"))?
    .map_err(|e| format!("test run failed: {e}"))?
    .into_value::<String>()
    .map_err(|e| format!("results not a string: {e}"))?;

    // Fulfil any @capture screenshot requests the body recorded (W7). Each is
    // {kind, path}; for `screenshot` we save a full-page PNG to <path>.
    if let Ok(Some(caps)) = page
        .evaluate("JSON.stringify(window.__stCaptures || [])")
        .await
        .ok()
        .map(|r| r.into_value::<String>().ok())
        .map(|s| s.and_then(|j| serde_json::from_str::<Vec<serde_json::Value>>(&j).ok()))
        .ok_or(())
    {
        for cap in caps {
            if cap.get("kind").and_then(|k| k.as_str()) == Some("screenshot") {
                if let Some(path) = cap.get("path").and_then(|p| p.as_str()) {
                    if let Ok(png) = page
                        .screenshot(
                            chromiumoxide::page::ScreenshotParams::builder()
                                .full_page(true)
                                .build(),
                        )
                        .await
                    {
                        let _ = std::fs::write(path, png);
                    }
                }
            }
        }
    }

    // Diagnostic: if zero tests registered, surface page errors instead of a silent
    // empty result (a compiled-script exception before register_test runs).
    if results_json.contains("\"passed\":0")
        && results_json.contains("\"failed\":0")
        && results_json.contains("\"skipped\":0")
    {
        let diag = page
            .evaluate(
                "JSON.stringify({tests: (window.__st_tests||[]).length, errors: window.__st_page_errors||[]})",
            )
            .await
            .ok()
            .and_then(|r| r.into_value::<String>().ok());
        if let Some(diag) = diag {
            eprintln!("[cdp] empty-result diagnostic: {diag}");
        }
    }

    let _ = page.close().await;

    parse_results(&results_json)
}

fn parse_results(json: &str) -> Result<TestResults, String> {
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("bad results JSON: {e}"))?;
    let passed = v["passed"].as_u64().unwrap_or(0) as u32;
    let failed = v["failed"].as_u64().unwrap_or(0) as u32;
    let skipped = v["skipped"].as_u64().unwrap_or(0) as u32;
    let errors = v["errors"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|e| TestError {
                    name: e["name"].as_str().unwrap_or("").to_string(),
                    message: e["error"].as_str().unwrap_or("").to_string(),
                    stack: e["stack"].as_str().map(String::from),
                    file: None,
                    line: None,
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(TestResults {
        passed,
        failed,
        skipped,
        total: passed + failed + skipped,
        duration_ms: 0,
        errors,
    })
}
