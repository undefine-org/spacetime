//! The raea backend (PLAN-151): the same compiled page, drawn without a
//! browser by the vendored raea engine (`stdlib/film/vendor/raea`).
//!
//! raea renders an *HTML composition*: a page whose root element declares
//! `data-composition-id`, `data-width`/`data-height`, `data-fps` and
//! `data-duration`, and whose timelines are registered on
//! `window.__timelines` as objects answering `totalDuration()` and
//! `totalTime(t)`. raea lays the page out with Blitz, runs its scripts on its
//! own DOM, and for every frame calls `totalTime(t)` before painting it on the
//! GPU and encoding it in-process.
//!
//! Spacetime already has exactly one clock a page's motion reads:
//! `ST._clock`. So this backend adds NO page syntax and NO second runtime —
//! it is one adapter in two parts:
//!
//! 1. [`film_document`] marks the compiled page's `<body>` as the root
//!    composition and appends a bridge that registers `ST._clock` as the
//!    composition's single timeline (`totalTime(t)` = advance the virtual
//!    clock to `t`).
//! 2. [`render_page`] / [`render_stills`] stage that document beside the
//!    site's files and hand it to the `raea` binary.
//!
//! This file is the ONLY place in Spacetime that knows raea exists (the
//! vendoring rule: own every dependency behind one surface — blast radius of
//! an upstream change is this file).

use std::path::{Path, PathBuf};
use std::process::Command;

use super::{RenderOptions, RenderReport, Stage, StillsOptions, compile_page, resolve_entry};

/// The composition id the bridge registers its timeline under.
const COMPOSITION_ID: &str = "spacetime";

/// The browser canvas default, declared before any page CSS (see
/// [`film_document`]).
const CANVAS_DEFAULT: &str = "\n<style>html{background:#fff}</style>";

/// The runtime's 3D mode under raea (PLAN-152): `three-stage` builds its parts
/// as `data-3d` markup for raea to draw, and touches no WebGL.
const STAGE_HEADLESS: &str = "\n<script>window.ST = Object.assign(window.ST || {}, { _stage3d: { renderer: \"none\" } });</script>";

/// Where the vendored engine lives, relative to the repository root.
const VENDOR_DIR: &str = "stdlib/film/vendor/raea";

/// Find the `raea` binary: `$SPACETIME_RAEA`, else the vendored submodule's
/// release build, else `raea` on `PATH`. Never guesses silently — the error
/// names every place it looked and how to build one.
pub fn locate() -> Result<PathBuf, String> {
    if let Ok(p) = std::env::var("SPACETIME_RAEA") {
        let p = PathBuf::from(p);
        return if p.is_file() {
            Ok(p)
        } else {
            Err(format!("SPACETIME_RAEA={} is not a file", p.display()))
        };
    }
    let vendored = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(VENDOR_DIR)
        .join("target/release/raea");
    if vendored.is_file() {
        return Ok(vendored);
    }
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join("raea");
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    Err(format!(
        "render --engine raea needs the raea binary, but none was found \
         (looked at $SPACETIME_RAEA, {}, and PATH). Build the vendored engine with \
         `cargo build --release -p raea-cli --no-default-features` inside {VENDOR_DIR}.",
        vendored.display()
    ))
}

/// The time window a render covers, in ms of the page's score.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Window {
    pub from_ms: u64,
    pub to_ms: u64,
}

impl Window {
    pub(crate) fn of(duration_ms: u64, from_ms: Option<u64>, to_ms: Option<u64>) -> Self {
        let from_ms = from_ms.unwrap_or(0).min(duration_ms);
        let to_ms = to_ms.unwrap_or(duration_ms).min(duration_ms).max(from_ms);
        if to_ms == from_ms {
            Window { from_ms: 0, to_ms: duration_ms }
        } else {
            Window { from_ms, to_ms }
        }
    }

    fn span_s(&self) -> f64 {
        (self.to_ms - self.from_ms) as f64 / 1000.0
    }
}

/// Turn a compiled Spacetime document into a raea HTML composition.
///
/// The `<body>` becomes the root composition (its size, frame rate and
/// length declared as data), and a bridge script — appended AFTER the
/// runtime, so `ST` exists — installs the virtual clock and registers it as
/// the composition's timeline. raea seeks absolute time; the clock only steps
/// forward, so `totalTime(t)` advances by the difference (raea renders frames
/// in order; a backwards seek has nothing to rewind and is ignored).
///
/// The clock is installed exactly as the Chromium backend installs it
/// (`ST._clock.install()`, its default tick), so a film's drivers step through
/// the same virtual instants on both engines and a frame at `t` shows the same
/// picture under either (a score's drivers seed their start from the first
/// tick, so a different tick would shift every frame by the difference).
pub(crate) fn film_document(html: &str, window: Window, opts: &RenderOptions) -> Result<String, String> {
    let root = format!(
        "<body data-composition-id=\"{COMPOSITION_ID}\" data-root=\"true\" \
         data-width=\"{w}\" data-height=\"{h}\" data-fps=\"{fps}\" data-duration=\"{d}\">",
        w = opts.width,
        h = opts.height,
        fps = opts.fps,
        d = window.span_s(),
    );
    let bridge = format!(
        "<script>\n(function () {{\n\
         \x20 if (typeof ST === 'undefined' || !ST._clock) throw new Error('spacetime render: the runtime did not load (no ST._clock)');\n\
         \x20 var clock = ST._clock, from = {from};\n\
         \x20 clock.install();\n\
         \x20 if (from > 0) clock.advance(from);\n\
         \x20 (window.__timelines = window.__timelines || {{}})[\"{COMPOSITION_ID}\"] = {{\n\
         \x20   totalDuration: function () {{ return {d}; }},\n\
         \x20   totalTime: function (t) {{ var ms = from + t * 1000 - clock.now; if (ms > 0) clock.advance(ms); return this; }}\n\
         \x20 }};\n\
         }})();\n</script>\n</body>",
        from = window.from_ms,
        d = window.span_s(),
    );
    if !html.contains("<body>") || !html.contains("</body>") {
        return Err("the compiled document has no <body> to mark as the composition root".into());
    }
    let html = html.replacen("<body>", &root, 1);
    // A browser paints its canvas white beneath the page; raea composites the
    // page over a black camera. Declare the browser's default FIRST, so a
    // page's own background (any later rule) still wins — the film looks the
    // same under either backend.
    //
    // PLAN-152: a film with a 3D world (a `@stage`) is drawn OVER that world —
    // raea lays the page over the 3D it reads from the stage's `data-3d` parts —
    // so its canvas stays transparent, and the runtime is told before it boots
    // that there is no WebGL here (the stage then builds parts, not a renderer).
    let has_world = html.contains("data-st-world");
    let head = if has_world { STAGE_HEADLESS } else { CANVAS_DEFAULT };
    let html = match html.find("<head>") {
        Some(at) => format!("{}{head}{}", &html[..at + "<head>".len()], &html[at + "<head>".len()..]),
        None => format!("{head}{html}"),
    };
    // The LAST </body>: the bridge runs after the runtime's script.
    let at = html.rfind("</body>").expect("checked above");
    Ok(format!("{}{}{}", &html[..at], bridge, &html[at + "</body>".len()..]))
}


/// Compile `entry` and stage it as a raea composition over `window`.
fn prepare(entry: &Path, opts: &RenderOptions) -> Result<(Stage, Window, u64), String> {
    if (opts.dpr - 1.0).abs() > f64::EPSILON {
        return Err(format!(
            "--dpr {} is a Chromium device-scale option; with --engine raea render at the \
             size you want (--width/--height) instead",
            opts.dpr
        ));
    }
    let entry = resolve_entry(entry)?;
    let workspace_root = crate::compiler::project_root_for(&entry);
    // Root-relative assets rebase onto the staged copy, whose entries are the
    // site's own files (see `Stage`).
    let (html, duration_ms, soundtracks) = compile_page(&entry, &workspace_root, ".")?;
    let window = Window::of(duration_ms, opts.from_ms, opts.to_ms);
    let document = film_document(&html, window, opts)?;
    let stage = Stage::new(&document, &workspace_root)?;
    // PLAN-152: the film's `@sound`, synthesized for the rendered window, as the
    // composition's `<audio>` (raea mixes a page's audio into the mp4)
    if super::soundtrack_in(&stage.dir, &soundtracks, window.from_ms, window.to_ms)?.is_some() {
        let page = stage.page();
        let doc = std::fs::read_to_string(&page).map_err(|e| format!("failed to read the staged page: {e}"))?;
        let at = doc.rfind("</body>").ok_or("the staged page has no </body>")?;
        let doc = format!("{}<audio src=\"soundtrack.wav\" data-start=\"0\"></audio>\n{}", &doc[..at], &doc[at..]);
        std::fs::write(&page, doc).map_err(|e| format!("failed to write the staged page: {e}"))?;
    }
    Ok((stage, window, duration_ms))
}

/// Run `raea render <page> -o <out> [extra…]`, surfacing its own words on
/// failure (raea prints `REFUSED  …` or panics with a reason).
fn run_raea(page: &Path, out: &Path, extra: &[String]) -> Result<(), String> {
    let bin = locate()?;
    let output = Command::new(&bin)
        .arg("render")
        .arg(page)
        .arg("-o")
        .arg(out)
        .args(extra)
        .output()
        .map_err(|e| format!("failed to run {}: {e}", bin.display()))?;
    if !output.status.success() {
        let mut why = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !stdout.is_empty() {
            why = format!("{stdout}\n{why}");
        }
        return Err(format!("raea exited with {}:\n{}", output.status, why.trim()));
    }
    Ok(())
}

/// Render a page to an mp4 with raea. Returns the same report shape as the
/// Chromium backend.
pub fn render_page(entry: &Path, opts: &RenderOptions) -> Result<RenderReport, String> {
    let (stage, window, duration_ms) = prepare(entry, opts)?;
    if opts.out.exists() {
        eprintln!("\x1b[33m⚠\x1b[0m overwriting existing output {}", opts.out.display());
        let _ = std::fs::remove_file(&opts.out);
    }
    let out = absolute(&opts.out);
    run_raea(&stage.page(), &out, &[])?;
    if !out.is_file() {
        return Err(format!("raea reported success but wrote no {}", out.display()));
    }
    let frames = (window.span_s() * opts.fps as f64 - 1e-6).ceil().max(1.0) as u32;
    Ok(RenderReport {
        out: opts.out.clone(),
        frames,
        duration_ms,
        fps: opts.fps,
        width: opts.width,
        height: opts.height,
    })
}

/// Capture timestamps as PNG stills with raea (`raea render --at F,F -o DIR`),
/// named `still-<ms>ms.png` exactly as the Chromium backend names them.
pub fn render_stills(entry: &Path, sopts: &StillsOptions, fps: u32) -> Result<Vec<PathBuf>, String> {
    if sopts.times_s.is_empty() {
        return Err("--stills needs at least one timestamp (e.g. --stills 1.5,9,24)".to_string());
    }
    let opts = RenderOptions {
        out: sopts.out_dir.clone(),
        fps,
        width: sopts.width,
        height: sopts.height,
        dpr: sopts.dpr,
        from_ms: None,
        to_ms: None,
    };
    let (stage, window, duration_ms) = prepare(entry, &opts)?;
    let last = (window.span_s() * fps as f64 - 1e-6).ceil().max(1.0) as i64 - 1;
    let mut wanted: Vec<(u64, i64)> = sopts
        .times_s
        .iter()
        .map(|s| ((s * 1000.0).round().max(0.0) as u64).min(duration_ms))
        .map(|ms| (ms, ((ms as f64 / 1000.0) * fps as f64).round() as i64))
        .map(|(ms, f)| (ms, f.min(last)))
        .collect();
    wanted.sort_unstable();
    wanted.dedup();

    let shots = stage.dir.join("stills");
    std::fs::create_dir_all(&shots).map_err(|e| format!("failed to create {}: {e}", shots.display()))?;
    let at = wanted.iter().map(|(_, f)| f.to_string()).collect::<Vec<_>>().join(",");
    run_raea(&stage.page(), &shots, &["--at".to_string(), at])?;

    std::fs::create_dir_all(&sopts.out_dir)
        .map_err(|e| format!("failed to create stills dir {}: {e}", sopts.out_dir.display()))?;
    let mut pngs: Vec<PathBuf> = std::fs::read_dir(&shots)
        .map_err(|e| format!("failed to read raea's stills: {e}"))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "png"))
        .collect();
    pngs.sort();
    if pngs.len() != wanted.len() {
        return Err(format!(
            "asked raea for {} still(s) (frames {}), it wrote {}",
            wanted.len(),
            wanted.iter().map(|(_, f)| f.to_string()).collect::<Vec<_>>().join(","),
            pngs.len()
        ));
    }
    let mut written = Vec::new();
    for ((ms, _), png) in wanted.iter().zip(pngs) {
        let path = sopts.out_dir.join(format!("still-{ms}ms.png"));
        std::fs::copy(&png, &path).map_err(|e| format!("failed to write {}: {e}", path.display()))?;
        written.push(path);
    }
    Ok(written)
}

fn absolute(p: &Path) -> PathBuf {
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir().map(|d| d.join(p)).unwrap_or_else(|_| p.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> RenderOptions {
        RenderOptions { fps: 30, width: 1280, height: 720, ..RenderOptions::default() }
    }

    /// The compiled page becomes a raea composition: the body is the root,
    /// declaring the render's size, rate and length as data.
    #[test]
    fn body_becomes_the_root_composition() {
        let html = "<html><body>\n<div class=\"stage\"></div>\n<script>var ST={};</script>\n</body></html>";
        let doc = film_document(html, Window { from_ms: 0, to_ms: 4500 }, &opts()).unwrap();
        assert!(doc.contains(
            "<body data-composition-id=\"spacetime\" data-root=\"true\" data-width=\"1280\" \
             data-height=\"720\" data-fps=\"30\" data-duration=\"4.5\">"
        ), "{doc}");
    }

    /// The bridge comes AFTER the runtime (so `ST` exists when it runs),
    /// registers the virtual clock as the one timeline, and installs it as the
    /// Chromium backend does.
    #[test]
    fn bridge_registers_the_virtual_clock_after_the_runtime() {
        let html = "<html><body>\n<script>var ST={_clock:{}};</script>\n</body></html>";
        let doc = film_document(html, Window { from_ms: 0, to_ms: 2000 }, &opts()).unwrap();
        let runtime = doc.find("var ST=").unwrap();
        let bridge = doc.find("__timelines").unwrap();
        assert!(bridge > runtime, "bridge must follow the runtime: {doc}");
        // the Chromium backend's install, so both engines tick the same instants
        assert!(doc.contains("clock.install();"), "{doc}");
        assert!(doc.contains("[\"spacetime\"]"), "{doc}");
        assert!(doc.trim_end().ends_with("</body></html>"), "{doc}");
    }

    /// The browser's white canvas is declared BEFORE the page's own styles, so
    /// a page background (a later rule) overrides it.
    #[test]
    fn the_canvas_default_precedes_the_page_styles() {
        let html = "<html><head><style>html{background:#000}</style></head><body>\n</body></html>";
        let doc = film_document(html, Window { from_ms: 0, to_ms: 1000 }, &opts()).unwrap();
        let canvas = doc.find("html{background:#fff}").unwrap();
        let page = doc.find("html{background:#000}").unwrap();
        assert!(canvas < page, "{doc}");
    }

    /// A windowed render (`--from/--to`) is a shorter composition whose clock
    /// starts at the window's head.
    #[test]
    fn a_window_offsets_the_clock_and_shortens_the_composition() {
        let w = Window::of(30_000, Some(7_400), Some(12_800));
        assert_eq!(w, Window { from_ms: 7_400, to_ms: 12_800 });
        let doc = film_document("<body>\n</body>", w, &opts()).unwrap();
        assert!(doc.contains("data-duration=\"5.4\""), "{doc}");
        assert!(doc.contains("from = 7400"), "{doc}");
        // An empty window renders the whole score, as the Chromium backend does.
        assert_eq!(Window::of(3_000, Some(5_000), None), Window { from_ms: 0, to_ms: 3_000 });
    }

    /// A document with no body cannot be a composition — a loud error, never a
    /// silently empty film.
    /// A film with a 3D world stays transparent (raea draws the world under
    /// it) and tells the runtime, before it boots, that there is no WebGL.
    #[test]
    fn a_film_with_a_stage_is_headless_and_transparent() {
        let html = "<html><head></head><body>\n<script>var ST={};w.setAttribute('data-st-world','');</script>\n</body></html>";
        let doc = film_document(html, Window { from_ms: 0, to_ms: 1000 }, &opts()).unwrap();
        assert!(!doc.contains("html{background:#fff}"), "{doc}");
        let preset = doc.find("_stage3d").unwrap();
        assert!(preset < doc.find("var ST={}").unwrap(), "the preset must precede the runtime: {doc}");
    }

    #[test]
    fn a_bodyless_document_is_refused() {
        assert!(film_document("<div></div>", Window { from_ms: 0, to_ms: 1000 }, &opts()).is_err());
    }
}
