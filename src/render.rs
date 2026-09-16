//! `render` — the same page under a virtual clock, mp4 out (PLAN-132 W5).
//!
//! The thesis this verb is built on: **a website and a video differ only in
//! which signal drives the timeline.** So render adds NO author-facing syntax
//! (D5) and no second browser stack (AGENTS.md §2): it compiles + serves the
//! page through the SAME path `test --cdp` uses (`cdp.rs` + the compiler) and
//! then substitutes the runtime's VIRTUAL clock (`ST._clock`) for the wall
//! clock. The page is not told it is being rendered — its `.time` scores just
//! run against the same driver-source clock the test harness drives, instead
//! of the wall clock.
//!
//! # Mechanism
//!
//! 1. Compile the page with the ordinary `Compiler` and assemble the same
//!    self-contained HTML the CDP test backend loads via `file://`.
//! 2. The render duration is the LONGEST declared `@score` total on the page,
//!    surfaced by `score::declared_score_totals` — computed with the exact
//!    helpers the window-map lowering uses, so render and score agree and
//!    render never re-parses source or emitted JS.
//! 3. Drive a warm Chromium (reusing `cdp::infra`). A rAF gate swallows the
//!    page's real rAFs while it loads, so drivers mount at progress 0; then
//!    `ST._clock.install()` takes over callback dispatch and each render frame
//!    calls `ST._clock.advance(1000/fps)` to tick every driver by exactly that
//!    much virtual time. After a short paint settle, a `Page.captureScreenshot`
//!    PNG is piped into an ffmpeg image2pipe → h264 mp4.
//!
//! A page with no `@score` has no playhead and therefore nothing to render —
//! that is a loud error, never a 0-byte mp4 (silent acceptance is the banned
//! class).

#![cfg(feature = "cdp")]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command as ProcCommand, Stdio};
use std::time::{Duration, Instant};

use chromiumoxide::cdp::browser_protocol::emulation::SetDeviceMetricsOverrideParams;
use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;
use chromiumoxide::page::{Page, ScreenshotParams};

use crate::cdp;
use crate::compiler::Compiler;
use crate::pipeline::score::declared_score_totals;

/// How long to let the compositor paint after advancing virtual time, before
/// capturing. Virtual time freezes timers/rAF; real painting still happens on
/// real frames, so a short real sleep lets the updated styles reach the
/// compositor. The sleep does not affect WHAT is painted (that is a pure
/// function of virtual time), so determinism is unaffected.
const PAINT_SETTLE_MS: u64 = 40;

/// Renderer configuration.
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// Output .mp4 path (overwrites loudly; never silently).
    pub out: PathBuf,
    /// Frames per second in the output video (default 60).
    pub fps: u32,
    /// Output frame width in pixels (default 1920).
    pub width: u32,
    /// Output frame height in pixels (default 1080).
    pub height: u32,
    /// PLAN-150 W11: device pixel ratio. Frames render at width*dpr × height*dpr
    /// (a `--dpr 2` render of a 1920×1080 film is 3840×2160). Default 1.
    pub dpr: f64,
    /// PLAN-150 W11: render only this time window (ms). `None` = the whole
    /// score. `from` clamps the first frame's virtual time; `to` the last.
    pub from_ms: Option<u64>,
    pub to_ms: Option<u64>,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            out: PathBuf::from("film.mp4"),
            fps: 60,
            width: 1920,
            height: 1080,
            dpr: 1.0,
            from_ms: None,
            to_ms: None,
        }
    }
}

/// PLAN-150 W11: options for a STILLS render — capture specific virtual
/// timestamps as PNGs, no video mux. The fastest iteration loop: three frames
/// of a 30s film in ~5s instead of rendering the whole thing.
#[derive(Debug, Clone)]
pub struct StillsOptions {
    /// Timestamps (seconds) to capture.
    pub times_s: Vec<f64>,
    /// Output directory for the PNGs (one `still-<ms>.png` per timestamp).
    pub out_dir: PathBuf,
    pub width: u32,
    pub height: u32,
    pub dpr: f64,
}

/// What a render produced — the shape a caller (CLI or gate) reports.
#[derive(Debug, Clone)]
pub struct RenderReport {
    pub out: PathBuf,
    pub frames: u32,
    pub duration_ms: u64,
    pub fps: u32,
    pub width: u32,
    pub height: u32,
}

/// Compile a page and return its self-contained HTML + the longest declared
/// `@score` total (ms). Mirrors the `test --cdp` compile path: the ordinary
/// `Compiler`, whose emitted JS already carries the runtime.
///
/// The totals come from `score::declared_score_totals` over the SAME source
/// the compiler parsed (the compiler's own parser, not a regex over text or a
/// scrape of emitted JS). Only declared-domain scores (`.time`, `.loop`) give
/// a playhead; a page with none renders nothing and errors loudly.
fn compile_page(
    entry: &Path,
    workspace_root: &Path,
) -> Result<(String, u64), String> {
    let compiler = Compiler::from_file(entry, workspace_root)
        .map_err(|e| format!("failed to compile {}: {e}", entry.display()))?;

    // The render duration is the longest declared `@score` total on the page.
    // It comes from the SAME fully import-resolved AST the compiler is about
    // to emit (compiler.merged_ast()) — NOT a re-parse of just the entry file.
    // A score living in an imported module would otherwise be invisible to an
    // entry-only walk: it would loudly claim "no score" when that module is the
    // only playhead, or — worse — SILENTLY truncate the video to the entry's
    // own (shorter) longest score. Reading the merged tree keeps render and
    // score agreeing on what the page actually compiles to.
    let merged = compiler.merged_ast();
    let mut matches = merged.matches.clone();
    collect_scope_matches(&merged.scopes, &mut matches);

    let registry = crate::compiler::cached_stdlib_registry().0;
    let totals = declared_score_totals(&matches, &registry);
    let longest = totals
        .into_iter()
        .fold(0.0_f64, |acc, t| if t > acc { t } else { acc });

    // The page's directory is its project: `_prelude.st` beside it (brand
    // forms) participates exactly as it does under `serve`/`build`
    // (PLAN-135 W1) — a film that eases with `--lm-glide` would otherwise
    // refuse to compile only under `render`.
    let compiled = compiler
        .with_site_dir(Some(workspace_root.to_path_buf()))
        .compile();

    if !compiled.pipeline_errors.is_empty() {
        let first = &compiled.pipeline_errors[0];
        return Err(format!(
            "compilation failed ({}): {}",
            first.code, first.message
        ));
    }

    if longest <= 0.0 {
        return Err(format!(
            "{} declares no `@score` with a playhead — a render needs a score to have \
             something to time, so refusing rather than producing a 0-byte mp4",
            entry.display()
        ));
    }

    let html = assemble_page(&compiled.css, &compiled.html, &compiled.js);
    // The staged document is loaded via `file://` from the temp dir, so a
    // root-relative asset URL (`/assets/hero.jpg`, the shape `serve` and
    // `build` both resolve against the site root) would point at the
    // filesystem root and 404 silently — a film with a photograph would
    // render its photograph as nothing. Rebase them onto the site directory.
    let html = rebase_root_urls(&html, workspace_root);
    Ok((html, longest.round() as u64))
}

/// Rewrite root-relative URLs (`src="/x"`, `href="/x"`, `url(/x)`,
/// `url("/x")`, `url('/x')`) to `file://<site_dir>/x` so a `file://`-staged
/// page resolves its assets the way the served page does. Protocol-relative
/// URLs (`//cdn`) are left alone.
fn rebase_root_urls(html: &str, site_dir: &Path) -> String {
    let abs = std::fs::canonicalize(site_dir).unwrap_or_else(|_| site_dir.to_path_buf());
    let base = format!("file://{}", abs.display());
    let mut out = html.to_string();
    for (needle, replacement) in [
        ("src=\"/", format!("src=\"{base}/")),
        ("href=\"/", format!("href=\"{base}/")),
        ("url(/", format!("url({base}/")),
        ("url(\"/", format!("url(\"{base}/")),
        ("url('/", format!("url('{base}/")),
    ] {
        // Guard the protocol-relative form: `src="//cdn…` must stay untouched.
        let mut rebuilt = String::with_capacity(out.len());
        let mut rest = out.as_str();
        while let Some(pos) = rest.find(needle) {
            let after = &rest[pos + needle.len()..];
            rebuilt.push_str(&rest[..pos]);
            if after.starts_with('/') {
                rebuilt.push_str(needle);
            } else {
                rebuilt.push_str(&replacement);
            }
            rest = after;
        }
        rebuilt.push_str(rest);
        out = rebuilt;
    }
    out
}

/// Collect every `FormMatch` from a scope tree (scores live in selectors).
fn collect_scope_matches(
    scopes: &[crate::parser::ast::ScopeBlock],
    out: &mut Vec<crate::syntax::FormMatch>,
) {
    for scope in scopes {
        out.extend(scope.matches.clone());
        collect_nested_matches(&scope.nested_scopes, out);
    }
}

/// Recurse into a nested-scope tree collecting every `FormMatch` (scores live
/// in selectors at any depth; imported modules bring their own nesting).
fn collect_nested_matches(
    scopes: &[crate::parser::ast::NestedScope],
    out: &mut Vec<crate::syntax::FormMatch>,
) {
    for scope in scopes {
        out.extend(scope.matches.clone());
        collect_nested_matches(&scope.nested_scopes, out);
    }
}

/// Resolve a render target to a concrete source page: a directory names its
/// root page (`index.st`, else `index.st.md` — the same `index.st` convention
/// `test`/`build` use to find a project's entry, extended for a literate-only
/// project); a file is its own page. THE single place a CLI/gate path becomes
/// a page to compile, so the documented `Project directory or page` form is
/// true.
fn resolve_entry(entry: &Path) -> Result<PathBuf, String> {
    if entry.is_dir() {
        let st = entry.join("index.st");
        if st.is_file() {
            return Ok(st);
        }
        let md = entry.join("index.st.md");
        if md.is_file() {
            return Ok(md);
        }
        Err(format!(
            "{} is a directory but has neither index.st nor index.st.md — nothing to render",
            entry.display()
        ))
    } else {
        Ok(entry.to_path_buf())
    }
}

/// Assemble the self-contained document the CDP backend loads via `file://`
/// (the same staging `cdp::HtmlFileGuard` uses for test pages).
fn assemble_page(css: &str, body: &str, js: &str) -> String {
    // The compiler's emitted JS may itself contain the literal sequence
    // `</script>` (e.g. in a comment about richtext sanitization), which would
    // terminate this inline `<script>` element early and leave the runtime
    // undefined (ST absent -> nothing mounts -> "nothing to render"). Escape
    // the closing tag so the HTML parser cannot close the element: `\/` is
    // still just `/` in a JS string, regex, or comment, so the code is
    // unchanged at runtime. (The production page shell avoids this entirely by
    // loading JS from an external file — `src/html/page_shell.rs` — but a
    // self-contained file:// document must inline it.)
    let js = js.replace("</script", "<\\/script");
    format!(
        "<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>spacetime render</title>\n<style>\nhtml,body{{margin:0;width:100%;height:100%;}}\n{css}\n</style>\n\
         </head>\n<body>\n{body}\n<script>\n{js}\n</script>\n</body>\n</html>\n"
    )
}

/// Render a page to an mp4. Synchronous: blocks on the shared CDP runtime,
/// exactly like `cdp::run_test_files`. Returns a report describing the output.
pub fn render_page(entry: &Path, opts: &RenderOptions) -> Result<RenderReport, String> {
    let entry = resolve_entry(entry)?;
    let workspace_root = crate::compiler::project_root_for(&entry);
    let workspace_root = workspace_root.as_path();
    let (html, duration_ms) = compile_page(&entry, workspace_root)?;

    // PLAN-150 W11: an optional [from, to] window trims the render to one scene.
    let start_ms = opts.from_ms.unwrap_or(0).min(duration_ms);
    let end_ms = opts.to_ms.unwrap_or(duration_ms).min(duration_ms).max(start_ms);
    let window_ms = end_ms - start_ms;
    let span_ms = if window_ms > 0 { window_ms } else { duration_ms };
    let frames = ((span_ms as f64 / 1000.0) * opts.fps as f64).round() as u32;
    if frames == 0 {
        return Err(format!(
            "{}ms of score at {}fps produces 0 frames — raise the duration or fps",
            span_ms, opts.fps
        ));
    }
    let frame_ms = 1000.0 / opts.fps as f64;

    // Overwrite loudly: an existing output is not silently replaced.
    if opts.out.exists() {
        eprintln!(
            "\x1b[33m⚠\x1b[0m overwriting existing output {}",
            opts.out.display()
        );
    }

    // dpr scales the capture viewport; the mux still targets width×height×dpr.
    let out_w = (opts.width as f64 * opts.dpr).round() as u32;
    let out_h = (opts.height as f64 * opts.dpr).round() as u32;
    let mut ffmpeg = spawn_ffmpeg(&opts.out, opts.fps, out_w, out_h)?;
    let write_result = capture_into(
        &html,
        &mut ffmpeg,
        frames,
        frame_ms,
        opts.width,
        opts.height,
        opts.dpr,
        start_ms as f64,
    );
    // Close ffmpeg's stdin so it finalizes the mux.
    drop(ffmpeg.stdin.take());

    if let Err(e) = write_result {
        let _ = ffmpeg.child.kill();
        let _ = ffmpeg.child.wait();
        return Err(e);
    }
    let status = ffmpeg
        .child
        .wait()
        .map_err(|e| format!("ffmpeg wait failed: {e}"))?;
    if !status.success() {
        return Err(format!("ffmpeg exited with {status}"));
    }

    Ok(RenderReport {
        out: opts.out.clone(),
        frames,
        duration_ms,
        fps: opts.fps,
        width: opts.width,
        height: opts.height,
    })
}

/// PLAN-150 W11: capture specific virtual timestamps as PNG stills — the fast
/// iteration loop. No ffmpeg, no whole-film render: open the page once under the
/// virtual clock, advance to each requested time, screenshot to a PNG. Returns
/// the paths written.
pub fn render_stills(entry: &Path, opts: &StillsOptions) -> Result<Vec<PathBuf>, String> {
    let entry = resolve_entry(entry)?;
    let workspace_root = crate::compiler::project_root_for(&entry);
    let workspace_root = workspace_root.as_path();
    let (html, duration_ms) = compile_page(&entry, workspace_root)?;

    if opts.times_s.is_empty() {
        return Err("--stills needs at least one timestamp (e.g. --stills 1.5,9,24)".to_string());
    }
    std::fs::create_dir_all(&opts.out_dir)
        .map_err(|e| format!("failed to create stills dir {}: {e}", opts.out_dir.display()))?;

    // Sort + clamp the requested times into the score's span (ms).
    let mut times_ms: Vec<u64> = opts
        .times_s
        .iter()
        .map(|s| ((s * 1000.0).round() as i64).max(0) as u64)
        .map(|ms| ms.min(duration_ms))
        .collect();
    times_ms.sort_unstable();

    let out_dir = opts.out_dir.clone();
    let (w, h, dpr) = (opts.width, opts.height, opts.dpr);
    let infra = cdp::infra()?;
    let written: Vec<PathBuf> = infra.rt.block_on(async {
        let page = cdp::open_page(&infra.browser).await?;
        page.execute(SetDeviceMetricsOverrideParams::new(
            w as i64, h as i64, dpr, false,
        ))
        .await
        .map_err(|e| format!("set device metrics failed: {e}"))?;
        let _ = page
            .evaluate_on_new_document(
                "window.__stGate = true; const __st_raf = window.requestAnimationFrame.bind(window); \
                 window.requestAnimationFrame = function(cb){ return __st_raf(function(t){ if (!window.__stGate) cb(t); }); };",
            )
            .await
            .map_err(|e| format!("inject rAF gate failed: {e}"))?;
        let html_file = cdp::HtmlFileGuard::write(&html)?;
        page.goto(html_file.url())
            .await
            .map_err(|e| format!("goto(file:) failed: {e}"))?;
        wait_for_driver(&page).await?;
        page.evaluate("ST._clock.install();")
            .await
            .map_err(|e| format!("install virtual clock failed: {e}"))?;
        page.evaluate("window.__stGate = false;")
            .await
            .map_err(|e| format!("release rAF gate failed: {e}"))?;

        let mut written = Vec::new();
        let mut now = 0.0f64;
        for ms in &times_ms {
            let delta = *ms as f64 - now;
            if delta > 0.0 {
                page.evaluate(format!("ST._clock.advance({delta});"))
                    .await
                    .map_err(|e| format!("advance to {ms}ms failed: {e}"))?;
                now = *ms as f64;
            }
            tokio::time::sleep(Duration::from_millis(PAINT_SETTLE_MS)).await;
            let png = page
                .screenshot(ScreenshotParams::builder().format(CaptureScreenshotFormat::Png).build())
                .await
                .map_err(|e| format!("captureScreenshot failed: {e}"))?;
            let path = out_dir.join(format!("still-{ms}ms.png"));
            std::fs::write(&path, &png)
                .map_err(|e| format!("failed to write {}: {e}", path.display()))?;
            written.push(path);
        }
        let _ = page.close().await;
        Ok::<Vec<PathBuf>, String>(written)
    })?;

    Ok(written)
}

/// A running ffmpeg child with its stdin pipe (for PNG frames).
struct FfmpegPipe {
    child: Child,
    stdin: Option<std::process::ChildStdin>,
}

fn spawn_ffmpeg(out: &Path, fps: u32, width: u32, height: u32) -> Result<FfmpegPipe, String> {
    let mut child = ProcCommand::new("ffmpeg")
        .arg("-y")
        .arg("-loglevel")
        .arg("error")
        .arg("-f")
        .arg("image2pipe")
        .arg("-framerate")
        .arg(fps.to_string())
        .arg("-i")
        .arg("-")
        .arg("-c:v")
        .arg("libx264")
        .arg("-preset")
        .arg("medium")
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg("-vf")
        .arg(format!("scale={width}:{height}"))
        .arg("-movflags")
        .arg("+faststart")
        .arg(out)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn ffmpeg: {e}"))?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "ffmpeg stdin not available".to_string())?;
    Ok(FfmpegPipe { child, stdin: Some(stdin) })
}

/// Drive one page over CDP: load the compiled HTML under a PAUSED virtual
/// clock, then advance it one frame's worth at a time and pipe each screenshot
/// to ffmpeg.
fn capture_into(
    html: &str,
    ffmpeg: &mut FfmpegPipe,
    frames: u32,
    frame_ms: f64,
    width: u32,
    height: u32,
    dpr: f64,
    start_ms: f64,
) -> Result<(), String> {
    let infra = cdp::infra()?;
    infra.rt.block_on(async {
        let page = cdp::open_page(&infra.browser).await?;

        // Fixed output viewport, device scale = dpr (PLAN-150 W11), so frames
        // are byte-for-byte the same size and scale across runs — determinism's
        // first requirement. dpr multiplies the captured pixels (a 2× render).
        page.execute(SetDeviceMetricsOverrideParams::new(
            width as i64,
            height as i64,
            dpr,
            false,
        ))
        .await
        .map_err(|e| format!("set device metrics failed: {e}"))?;

        // Install a rAF GATE before ANY page script runs. The page's `.time`
    // drivers seed their `startTime` from the FIRST rAF timestamp, so a real-
    // time rAF during page load would poison every later virtual-time one
    // (a real timestamp followed by virtual ones => a nonsensical negative
    // elapsed => a frozen, non-deterministic page). The gate drops real rAFs
    // while `__stGate` is true, so the drivers stay at progress 0 until we
    // release them under the frozen virtual clock. This is a substitution at
    // the driver source (D5), not author-facing syntax: the page never knows.
    let _ = page
        .evaluate_on_new_document(
            "window.__stGate = true; const __st_raf = window.requestAnimationFrame.bind(window); \
             window.requestAnimationFrame = function(cb){ return __st_raf(function(t){ if (!window.__stGate) cb(t); }); };",
        )
        .await
        .map_err(|e| format!("inject rAF gate failed: {e}"))?;

    // Stage the compiled page under a `file://` URL — inline scripts run,
    // unlike a data: URL (the exact reason cdp::HtmlFileGuard exists). The
    // page loads under REAL time (the gate swallows its rAFs); virtual time
    // is frozen next, from a deterministic baseline.
    let html_file = cdp::HtmlFileGuard::write(html)?;
    page.goto(html_file.url())
        .await
        .map_err(|e| format!("goto(file:) failed: {e}"))?;

    // Wait for the score's driver to mount (its rAF callback registered) —
    // still at progress 0, the gate having swallowed every real-time rAF that
    // would otherwise have seeded its `startTime`.
    wait_for_driver(&page).await?;

    // Install the runtime's VIRTUAL clock (ST._clock — the D5 substitution at
    // the driver source, the SAME clock stdlib/testing/timing.st drives). This
    // cancels the real rAF loop the drivers started on load and hands timestamp
    // ownership to us: from here every registered rAF callback fires ONLY when
    // we advance the clock. (Browser-level Emulation.setVirtualTimePolicy does
    // NOT dispatch the runtime's rAF callbacks — virtual time advances, but
    // ST._raf stays unticked — so the driver-source clock is the mechanism, not
    // the browser's.)
    let _ = page
        .evaluate("ST._clock.install();")
        .await
        .map_err(|e| format!("install virtual clock failed: {e}"))?;

    // Release the gate: ST._clock now owns dispatch, so the gate (which only
    // ever throttled the REAL loop, now cancelled) has no further work. The
    // drivers stay at progress 0 until the first advance below.
    let _ = page
        .evaluate("window.__stGate = false;")
        .await
        .map_err(|e| format!("release rAF gate failed: {e}"))?;

    // PLAN-150 W11: for a windowed render, fast-forward the virtual clock to
    // the window START before the first frame, so frame 0 is the scene's head.
    if start_ms > 0.0 {
        page.evaluate(format!("ST._clock.advance({start_ms});"))
            .await
            .map_err(|e| format!("advance to window start failed: {e}"))?;
    }

    // Frame 0 is the initial state at the window start (t = start_ms).
    write_frame(&page, ffmpeg, width, height).await?;

    for _i in 1..frames {
        // Advance the runtime's virtual clock by exactly one frame's worth of
        // time. advance() ticks every registered rAF callback with a
        // deterministic timestamp (ST._clock.now) and leaves the clock at
        // exactly i*frame_ms — so the screenshot below captures the page at
        // that virtual instant, reproducibly (this is what makes two renders
        // of the same page bit-identical).
        page.evaluate(format!("ST._clock.advance({frame_ms});"))
            .await
            .map_err(|e| format!("advance virtual clock failed: {e}"))?;

        write_frame(&page, ffmpeg, width, height).await?;
    }

    let _ = page.close().await;
    Ok::<(), String>(())
    })
}

/// Capture one PNG at the current (frozen) virtual time and pipe it to ffmpeg.
async fn write_frame(
    page: &Page,
    ffmpeg: &mut FfmpegPipe,
    _width: u32,
    _height: u32,
) -> Result<(), String> {
    // Let the compositor paint the current style before capturing. Virtual
    // time froze timers, but real frames still paint; a short real sleep makes
    // the capture reflect the latest style deterministically.
    tokio::time::sleep(Duration::from_millis(PAINT_SETTLE_MS)).await;

    let png = page
        .screenshot(ScreenshotParams::builder().format(CaptureScreenshotFormat::Png).build())
        .await
        .map_err(|e| format!("captureScreenshot failed: {e}"))?;

    let stdin = ffmpeg
        .stdin
        .as_mut()
        .ok_or_else(|| "ffmpeg stdin closed".to_string())?;
    stdin
        .write_all(&png)
        .and_then(|_| stdin.flush())
        .map_err(|e| format!("writing frame to ffmpeg failed: {e}"))?;
    Ok(())
}

/// THE THESIS GATE'S browser half (PLAN-132): does the SAME page that render
/// virtual-clocks also move under the real `.time` clock in a browser? Loads
/// the page under REAL time and compares two screenshots ~1.3s apart. If the
/// page moves at all, the frames differ. `render_page` and this function
/// compile the IDENTICAL source through the same path, so a page that passes
/// both has run one fragment under two clocks with zero page changes.
pub fn page_moves_under_real_time(entry: &Path) -> Result<bool, String> {
    let entry = resolve_entry(entry)?;
    let workspace_root = crate::compiler::project_root_for(&entry);
    let workspace_root = workspace_root.as_path();
    let (html, _duration_ms) = compile_page(&entry, workspace_root)?;
    let infra = cdp::infra()?;
    infra.rt.block_on(async {
        let page = cdp::open_page(&infra.browser).await?;
        page.execute(SetDeviceMetricsOverrideParams::new(480, 360, 1.0, false))
            .await
            .map_err(|e| format!("set device metrics failed: {e}"))?;
        let html_file = cdp::HtmlFileGuard::write(&html)?;
        page.goto(html_file.url())
            .await
            .map_err(|e| format!("goto(file:) failed: {e}"))?;
        wait_for_driver(&page).await?;

        // Two samples under the real clock, far enough apart that the first
        // clip's 0..1s window has definitely crossed between them.
        tokio::time::sleep(Duration::from_millis(150)).await;
        let a = shot(&page).await?;
        tokio::time::sleep(Duration::from_millis(1300)).await;
        let b = shot(&page).await?;

        let _ = page.close().await;
        Ok::<bool, String>(a != b)
    })
}

async fn shot(page: &Page) -> Result<Vec<u8>, String> {
    page.screenshot(ScreenshotParams::builder().format(CaptureScreenshotFormat::Png).build())
        .await
        .map_err(|e| format!("captureScreenshot failed: {e}"))
}

/// Wait until the page's runtime has mounted the score driver (its rAF

/// callback is registered). Under a paused clock no rAF has fired, so this is
/// purely "has the page finished wiring its drivers".
async fn wait_for_driver(page: &Page) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let ready = page
            .evaluate("typeof ST !== 'undefined' && ST._raf && ST._raf.callbacks && ST._raf.callbacks.size > 0")
            .await
            .ok()
            .and_then(|r| r.into_value::<bool>().ok())
            .unwrap_or(false);
        if ready {
            return Ok(());
        }
        if Instant::now() > deadline {
            return Err(
                "the page's runtime never mounted a driver (no @score consumed its progress) — \
                 nothing to render"
                    .to_string(),
            );
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

#[cfg(test)]
mod rebase_tests {
    use super::rebase_root_urls;
    use std::path::Path;

    /// A `file://`-staged render must find the site's assets: root-relative
    /// URLs are rebased onto the site dir, protocol-relative ones are kept.
    #[test]
    fn root_relative_urls_rebase_onto_the_site_dir() {
        let site = std::env::temp_dir();
        let base = format!("file://{}", std::fs::canonicalize(&site).unwrap().display());
        let html = r#"<img src="/a.jpg"><a href="/p">x</a><style>.h{background:url(/b.jpg);mask:url("/c.svg");b:url('/d.png')}</style><script src="//cdn.example/x.js"></script>"#;
        let out = rebase_root_urls(html, Path::new(&site));
        assert!(out.contains(&format!(r#"src="{base}/a.jpg""#)), "{out}");
        assert!(out.contains(&format!(r#"href="{base}/p""#)), "{out}");
        assert!(out.contains(&format!("url({base}/b.jpg)")), "{out}");
        assert!(out.contains(&format!(r#"url("{base}/c.svg")"#)), "{out}");
        assert!(out.contains(&format!("url('{base}/d.png')")), "{out}");
        assert!(out.contains(r#"src="//cdn.example/x.js""#), "protocol-relative untouched: {out}");
    }
}
