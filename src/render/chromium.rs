//! The Chromium backend (PLAN-132 W5): compile + serve the page through the
//! SAME path `test --cdp` uses (`cdp.rs` + the compiler), then drive the
//! runtime's virtual clock one frame at a time.
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

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command as ProcCommand, Stdio};
use std::time::{Duration, Instant};

use chromiumoxide::cdp::browser_protocol::emulation::SetDeviceMetricsOverrideParams;
use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;
use chromiumoxide::page::{Page, ScreenshotParams};

use super::{RenderOptions, RenderReport, Served, Stage, StillsOptions, compile_page, resolve_entry};
use crate::cdp;

/// How long to let the compositor paint after advancing virtual time, before
/// capturing. Virtual time freezes timers/rAF; real painting still happens on
/// real frames, so a short real sleep lets the updated styles reach the
/// compositor. The sleep does not affect WHAT is painted (that is a pure
/// function of virtual time), so determinism is unaffected.
const PAINT_SETTLE_MS: u64 = 40;

/// Render a page to an mp4. Synchronous: blocks on the shared CDP runtime,
/// exactly like `cdp::run_test_files`. Returns a report describing the output.
pub fn render_page(entry: &Path, opts: &RenderOptions) -> Result<RenderReport, String> {
    let entry = resolve_entry(entry)?;
    let workspace_root = crate::compiler::project_root_for(&entry);
    let workspace_root = workspace_root.as_path();
    let (html, duration_ms, soundtracks) = compile_page(&entry, workspace_root, ".")?;
    // PLAN-152: served over loopback HTTP from a stage beside the site's files
    // (Chrome won't fetch `file://`: a glTF character never arrived)
    let stage = Stage::new(&html, workspace_root)?;
    let served = Served::start(&stage.dir)?;

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
    // PLAN-152: the film's `@sound`, synthesized for exactly the rendered window
    let sound_dir = std::env::temp_dir().join(format!("spacetime-sound-{}", std::process::id()));
    std::fs::create_dir_all(&sound_dir).map_err(|e| format!("failed to create {}: {e}", sound_dir.display()))?;
    let soundtrack = super::soundtrack_in(&sound_dir, &soundtracks, start_ms, end_ms.max(start_ms + 1))?;
    let mut ffmpeg = spawn_ffmpeg(&opts.out, opts.fps, out_w, out_h, soundtrack.as_deref())?;
    let write_result = capture_into(
        &served.url,
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
    let _ = std::fs::remove_dir_all(&sound_dir);
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
    let (html, duration_ms, soundtracks) = compile_page(&entry, workspace_root, ".")?;
    // PLAN-152: served over loopback HTTP from a stage beside the site's files
    // (Chrome won't fetch `file://`: a glTF character never arrived)
    let stage = Stage::new(&html, workspace_root)?;
    let served = Served::start(&stage.dir)?;

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
        page.goto(served.url.clone())
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

fn spawn_ffmpeg(out: &Path, fps: u32, width: u32, height: u32, soundtrack: Option<&Path>) -> Result<FfmpegPipe, String> {
    let mut cmd = ProcCommand::new("ffmpeg");
    cmd.arg("-y")
        .arg("-loglevel")
        .arg("error")
        .arg("-f")
        .arg("image2pipe")
        .arg("-framerate")
        .arg(fps.to_string())
        .arg("-i")
        .arg("-");
    // PLAN-152: the soundtrack as a second input, AAC beside the picture
    if let Some(wav) = soundtrack {
        cmd.arg("-i").arg(wav).arg("-c:a").arg("aac").arg("-b:a").arg("192k").arg("-shortest");
    }
    let mut child = cmd
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
    url: &str,
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
    page.goto(url.to_string())
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
    let (html, _duration_ms, _soundtracks) = compile_page(&entry, workspace_root, ".")?;
    let stage = Stage::new(&html, workspace_root)?;
    let served = Served::start(&stage.dir)?;
    let infra = cdp::infra()?;
    infra.rt.block_on(async {
        let page = cdp::open_page(&infra.browser).await?;
        page.execute(SetDeviceMetricsOverrideParams::new(480, 360, 1.0, false))
            .await
            .map_err(|e| format!("set device metrics failed: {e}"))?;
        page.goto(served.url.clone())
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
            // PLAN-152: and every asset a stage is loading (a glTF model) has
            // arrived, so frame 0 is the scene, not the scene minus its cast
            .evaluate("typeof ST !== 'undefined' && ST._raf && ST._raf.callbacks && ST._raf.callbacks.size > 0 && !(ST._loading > 0)")
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

