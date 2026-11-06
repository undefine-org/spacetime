//! `render` — the same page under a virtual clock, mp4 out (PLAN-132 W5).
//!
//! The thesis this verb is built on: **a website and a video differ only in
//! which signal drives the timeline.** So render adds NO author-facing syntax
//! (D5): the page is compiled by the ordinary `Compiler`, and a backend
//! substitutes the runtime's VIRTUAL clock (`ST._clock`) for the wall clock.
//! The page is not told it is being rendered.
//!
//! Two backends draw the frames (PLAN-151), chosen with `--engine`:
//!
//! - [`chromium`] — a warm Chromium over CDP (the same substrate `test --cdp`
//!   uses), one screenshot per frame piped to ffmpeg. Needs the `cdp` feature.
//! - [`raea`] — the vendored raea engine (`stdlib/film/vendor/raea`): the page
//!   laid out by Blitz, painted on the GPU and encoded in-process. No browser.
//!
//! Everything a backend needs to agree on lives here, once: which page a
//! target names, how it compiles, how long its score is, how its assets
//! resolve. A backend only decides how a frame at virtual time `t` becomes
//! pixels.
//!
//! A page with no `@score` has no playhead and therefore nothing to render —
//! that is a loud error, never a 0-byte mp4 (silent acceptance is the banned
//! class).

use std::path::{Path, PathBuf};

use crate::compiler::Compiler;
use crate::pipeline::score::declared_score_totals;

#[cfg(feature = "cdp")]
pub mod chromium;
pub mod raea;
mod soundtrack;

#[cfg(feature = "cdp")]
pub use chromium::{page_moves_under_real_time, render_page, render_stills};

/// Which backend draws the frames (`spacetime render --engine`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engine {
    /// Chromium over CDP, screenshot per frame (the original PLAN-132 path).
    Chromium,
    /// The vendored raea engine: Blitz layout, GPU paint, in-process encode.
    Raea,
}

impl std::str::FromStr for Engine {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "chromium" | "chrome" | "cdp" => Ok(Engine::Chromium),
            "raea" => Ok(Engine::Raea),
            other => Err(format!("unknown render engine `{other}` (expected `chromium` or `raea`)")),
        }
    }
}

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
pub(crate) fn compile_page(
    entry: &Path,
    workspace_root: &Path,
    url_base: &str,
) -> Result<(String, u64, Vec<serde_json::Value>), String> {
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
    // PLAN-152: the film's `@sound` notes, from the same tree (what the
    // renderers mux as its soundtrack).
    let soundtracks = crate::pipeline::score::declared_soundtracks(&matches, &registry);
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
    let html = rebase_root_urls(&html, url_base);
    Ok((html, longest.round() as u64, soundtracks))
}

/// PLAN-152: the soundtrack of `[from_ms, to_ms)` of a film into `dir` as
/// `soundtrack.wav` — its path, or None when the film has no `@sound`.
pub(crate) fn soundtrack_in(dir: &Path, programs: &[serde_json::Value], from_ms: u64, to_ms: u64) -> Result<Option<PathBuf>, String> {
    let out = dir.join("soundtrack.wav");
    Ok(soundtrack::write_soundtrack(programs, from_ms, to_ms, &out)?.then_some(out))
}

/// The `file://` base of a site directory (what a `file://`-staged page
/// rebases its root-relative URLs onto).
pub(crate) fn file_base(site_dir: &Path) -> String {
    let abs = std::fs::canonicalize(site_dir).unwrap_or_else(|_| site_dir.to_path_buf());
    format!("file://{}", abs.display())
}

/// Rewrite root-relative URLs (`src="/x"`, `href="/x"`, `url(/x)`,
/// `url("/x")`, `url('/x')`) onto `base` (`file://<site_dir>` for the Chromium backend, `.` for
/// raea's staged copy) so a `file://`-staged
/// page resolves its assets the way the served page does. Protocol-relative
/// URLs (`//cdn`) are left alone.
pub(crate) fn rebase_root_urls(html: &str, base: &str) -> String {
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
pub(crate) fn resolve_entry(entry: &Path) -> Result<PathBuf, String> {
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
pub(crate) fn assemble_page(css: &str, body: &str, js: &str) -> String {
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

#[cfg(test)]
mod rebase_tests {
    use super::{file_base, rebase_root_urls};

    /// A `file://`-staged render must find the site's assets: root-relative
    /// URLs are rebased onto the site dir, protocol-relative ones are kept.
    #[test]
    fn root_relative_urls_rebase_onto_the_site_dir() {
        let site = std::env::temp_dir();
        let base = file_base(&site);
        let html = r#"<img src="/a.jpg"><a href="/p">x</a><style>.h{background:url(/b.jpg);mask:url("/c.svg");b:url('/d.png')}</style><script src="//cdn.example/x.js"></script>"#;
        let out = rebase_root_urls(html, &base);
        assert!(out.contains(&format!(r#"src="{base}/a.jpg""#)), "{out}");
        assert!(out.contains(&format!(r#"href="{base}/p""#)), "{out}");
        assert!(out.contains(&format!("url({base}/b.jpg)")), "{out}");
        assert!(out.contains(&format!(r#"url("{base}/c.svg")"#)), "{out}");
        assert!(out.contains(&format!("url('{base}/d.png')")), "{out}");
        assert!(out.contains(r#"src="//cdn.example/x.js""#), "protocol-relative untouched: {out}");
    }
}

