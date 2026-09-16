//! PLAN-132 W5 gates — the negative discipline applied to video.
//!
//! `spacetime render` compiles the SAME page `test --cdp` compiles and drives
//! it under Chromium's VIRTUAL clock instead of the wall clock. These gates
//! assert what that buys, and — in the BUG-252 spirit — the one thing it must
//! NOT do:
//!
//!   1. FRAME COUNT     a 2s @score fixture @ 10fps -> exactly 20 frames / 2.0s
//!   2. MOTION          frame 0 vs frame 10 DIFFER (a frozen video must fail)
//!   3. DETERMINISM     two renders -> identical per-frame hashes
//!   4. DISCRIMINATION  the motion gate FAILS on a page whose score nobody
//!                      consumes (BUG-258's inert state) — proof it is real
//!   5. THESIS          one `@form score` fragment runs under BOTH the real
//!                      `.time` clock (browser) and the virtual clock (render)
//!                      with zero page changes
//!
//! Requires the `cdp` feature, a discoverable Chrome, and ffmpeg/ffprobe on
//! PATH. All video inspection is done with ffmpeg/ffprobe (the same tools the
//! render backend pipes into), never by re-reading our own output.

#![cfg(feature = "cdp")]

use std::path::{Path, PathBuf};
use std::process::Command;

use spacetime::render::{RenderOptions, page_moves_under_real_time, render_page};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/render/fixtures")
        .join(name)
}

/// Render a fixture to a unique temp mp4 and return (path, report).
fn render(fixture_name: &str, fps: u32, tag: &str) -> (PathBuf, spacetime::render::RenderReport) {
    let out = std::env::temp_dir().join(format!(
        "render_{tag}_{fixture_name}_{}.mp4",
        std::process::id()
    ));
    let opts = RenderOptions {
        out: out.clone(),
        fps,
        width: 320,
        height: 180,
        ..Default::default()
    };
    let rep = render_page(&fixture(fixture_name), &opts)
        .unwrap_or_else(|e| panic!("render {fixture_name} failed: {e}"));
    (out, rep)
}

/// Frame count + duration (s) reported by ffprobe for the video stream.
fn ffprobe_frames(path: &Path) -> (u32, f64) {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-count_frames",
            "-show_entries",
            "stream=nb_read_frames",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("ffprobe failed: {e}"));
    assert!(
        out.status.success(),
        "ffprobe errored: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut it = text.split_whitespace();
    let frames: u32 = it
        .next()
        .unwrap_or_else(|| panic!("ffprobe gave no frame count: {text:?}"))
        .parse()
        .unwrap();
    let duration: f64 = it
        .next()
        .unwrap_or_else(|| panic!("ffprobe gave no duration: {text:?}"))
        .parse()
        .unwrap();
    (frames, duration)
}

/// The raw RGB24 pixels of frame `index` (deterministic, directly comparable).
fn frame_pixels(path: &Path, index: u32) -> Vec<u8> {
    let out = Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-i",
            path.to_str().unwrap(),
            "-vf",
            &format!("select=eq(n\\,{index})"),
            "-frames:v",
            "1",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb24",
            "-",
        ])
        .output()
        .unwrap_or_else(|e| panic!("ffmpeg frame extract failed: {e}"));
    assert!(
        out.status.success(),
        "ffmpeg frame extract errored: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}

fn frame_md5(path: &Path, index: u32) -> String {
    let mut h = Command::new("md5sum").stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).spawn().unwrap();
    std::io::Write::write_all(h.stdin.as_mut().unwrap(), &frame_pixels(path, index)).unwrap();
    let out = h.wait_with_output().unwrap();
    String::from_utf8_lossy(&out.stdout).split_whitespace().next().unwrap().to_string()
}

/// MOTION: do frames `a` and `b` differ beyond pixel noise? (Any difference in
/// raw RGB means the video is not frozen.)
fn frames_differ(path: &Path, a: u32, b: u32) -> bool {
    frame_pixels(path, a) != frame_pixels(path, b)
}

// =============================================================================
// 1. FRAME COUNT — a 2s fixture @ 10fps is exactly 20 frames / 2.0s.
// =============================================================================
#[test]
fn render_2s_fixture_at_10fps_is_exactly_20_frames() {
    let (path, rep) = render("film.st", 10, "count");
    let (frames, duration) = ffprobe_frames(&path);
    assert_eq!(
        frames, 20,
        "2s @ 10fps must encode exactly 20 frames, got {frames} (report said {})",
        rep.frames
    );
    assert!(
        (duration - 2.0).abs() < 0.05,
        "2s @ 10fps must be ~2.0s, got {duration}"
    );
    assert_eq!(rep.frames, 20, "render report must agree with ffprobe");
    std::fs::remove_file(&path).ok();
}

// =============================================================================
// 2. MOTION — frame 0 vs frame 10 must differ. A FROZEN video must FAIL this,
//    exactly as a frozen page fails the CDP gates (BUG-252 doctrine).
// =============================================================================
#[test]
fn motion_frames_0_and_10_differ() {
    let (path, _rep) = render("film.st", 10, "motion");
    assert!(
        frames_differ(&path, 0, 10),
        "a rendered score that moves must differ between frame 0 and frame 10 — \
         identical frames mean the video is frozen (the score published progress \
         but nothing consumed it)"
    );
    std::fs::remove_file(&path).ok();
}

// =============================================================================
// 3. DETERMINISM — two renders of the same fixture have identical per-frame
//    hashes.
// =============================================================================
#[test]
fn two_renders_are_bit_identical_per_frame() {
    let (path_a, _) = render("film.st", 10, "detA");
    let (path_b, _) = render("film.st", 10, "detB");
    // Same frame count in both, and every frame matches.
    let (fa, _) = ffprobe_frames(&path_a);
    let (fb, _) = ffprobe_frames(&path_b);
    assert_eq!(fa, fb, "both renders must have the same frame count");
    for i in 0..fa {
        assert_eq!(
            frame_md5(&path_a, i),
            frame_md5(&path_b, i),
            "frame {i} differs between two renders of the same page — \
             the virtual clock is not deterministic"
        );
    }
    std::fs::remove_file(&path_a).ok();
    std::fs::remove_file(&path_b).ok();
}

// =============================================================================
// 4. DISCRIMINATION — run the motion gate against a STATIC page (same score,
//    but nothing consumes the clips' progress => BUG-258's inert state). The
//    motion gate MUST fail on it. A motion gate that passed on a frozen page
//    is not a gate.
// =============================================================================
#[test]
fn motion_gate_fails_on_a_static_page() {
    let (path, _rep) = render("static.st", 10, "disc");
    assert!(
        !frames_differ(&path, 0, 10),
        "a page whose score publishes progress nothing consumes must render a FROZEN \
         video — if frame 0 and frame 10 differ, the motion gate is satisfied by \
         something unrelated to the score (or by noise)"
    );
    std::fs::remove_file(&path).ok();
}

// =============================================================================
// 5. THESIS — the fixture's `@form score --story` fragment runs under BOTH
//    clocks with ZERO page changes. The render above proves it moves under the
//    virtual clock; here the SAME source is loaded in a real browser and must
//    move under the real `.time` clock too.
// =============================================================================
#[test]
fn one_fragment_two_clocks_zero_page_changes() {
    // Virtual clock: already proven by the motion gate on this exact source.
    let (path, _) = render("film.st", 10, "thesis");
    assert!(
        frames_differ(&path, 0, 10),
        "precondition: the fixture must move under the virtual clock (see motion gate)"
    );
    std::fs::remove_file(&path).ok();

    // Real `.time` clock, same source, real browser: the fragment must move here too.
    let moved = page_moves_under_real_time(&fixture("film.st"))
        .unwrap_or_else(|e| panic!("real-time motion probe failed: {e}"));
    assert!(
        moved,
        "the SAME `@form score --story` fragment that renders to a moving mp4 must \
         also move under `.time` in a real browser — if not, render and the website \
         have diverged (the thesis is false)"
    );
}

// =============================================================================
// 6. DIRECTORY FORM — `spacetime render proj/` (a directory, per the CLI doc
//    "Project directory or page") must resolve to the project's root page and
//    render it. proj/index.st declares a 1s score, so 1s @ 10fps is exactly 10
//    frames. Before the fix, render passed the directory verbatim to the
//    compiler, which fs::read_to_string'd it into 'Is a directory'.
// =============================================================================
#[test]
fn directory_entry_resolves_to_root_page() {
    let out = std::env::temp_dir().join(format!("render_dir_{}.mp4", std::process::id()));
    let opts = RenderOptions {
        out: out.clone(),
        fps: 10,
        width: 320,
        height: 180,
        ..Default::default()
    };
    let rep = render_page(&fixture("proj"), &opts)
        .unwrap_or_else(|e| panic!("render of a directory must resolve to its root page: {e}"));
    let (frames, duration) = ffprobe_frames(&out);
    assert_eq!(
        frames, 10,
        "proj/index.st is a 1s score; 1s @ 10fps must be exactly 10 frames, got {frames}"
    );
    assert!(
        (duration - 1.0).abs() < 0.05,
        "1s @ 10fps must be ~1.0s, got {duration}"
    );
    assert_eq!(rep.frames, 10, "render report must agree with ffprobe");
    std::fs::remove_file(&out).ok();
}

// =============================================================================
// 7. IMPORTED SCORE — the page's LONGEST (here: only) score lives in an
//    IMPORTED module (`import_only/score_long.st`, a 4s playhead). Render must
//    time the video to it: 4s @ 10fps -> exactly 40 frames. DISCRIMINATION:
//    with entry-only totals the fix replaced, this page declared no score and
//    refused to render — the loud failure the merged-AST fix closes.
// =============================================================================
#[test]
fn longest_score_in_imported_module_sets_duration() {
    let out = std::env::temp_dir().join(format!("render_imp_{}.mp4", std::process::id()));
    let opts = RenderOptions {
        out: out.clone(),
        fps: 10,
        width: 320,
        height: 180,
        ..Default::default()
    };
    let rep = render_page(&fixture("import_only"), &opts)
        .unwrap_or_else(|e| panic!("render must see an imported module's score: {e}"));
    let (frames, duration) = ffprobe_frames(&out);
    assert_eq!(
        frames, 40,
        "the imported 4s score must set the timeline (40 frames @ 10fps), got {frames}"
    );
    assert!(
        (duration - 4.0).abs() < 0.05,
        "imported 4s score must yield ~4.0s, got {duration}"
    );
    assert_eq!(rep.frames, 40, "render report must agree with ffprobe");
    std::fs::remove_file(&out).ok();
}

// =============================================================================
// 8. MIXED, NO TRUNCATION — the NEGATIVE gate. import_mixed/index.st has an
//    entry-LOCAL 1s score AND imports a 4s score. Render must run to the LONGEST
//    total (4s = 40 frames @ 10fps), never truncate the video to the entry's own
//    shorter 1s (10 frames) — the SILENT failure mode when totals were computed
//    entry-only.
// =============================================================================
#[test]
fn imported_longest_score_not_truncated_by_entry_local() {
    let out = std::env::temp_dir().join(format!("render_mix_{}.mp4", std::process::id()));
    let opts = RenderOptions {
        out: out.clone(),
        fps: 10,
        width: 320,
        height: 180,
        ..Default::default()
    };
    let rep = render_page(&fixture("import_mixed"), &opts)
        .unwrap_or_else(|e| panic!("mixed import render failed: {e}"));
    let (frames, duration) = ffprobe_frames(&out);
    assert_eq!(
        frames, 40,
        "the longest score is the imported 4s, so render must be 40 frames @ 10fps — \
         a 10-frame result means the entry-local 1s silently truncated it, got {frames}"
    );
    assert!(
        (duration - 4.0).abs() < 0.05,
        "must be ~4.0s (imported longest), got {duration}"
    );
    assert_eq!(rep.frames, 40, "render report must agree with ffprobe");
    std::fs::remove_file(&out).ok();
}

// ===========================================================================
// PLAN-150 W11 — render tooling: stills, dpr, ranges.
// The film's build gets the flags a build has (film.st.md §13). Output goes to
// the repo-local `scratch/` (never /tmp), per the workspace convention.
// ===========================================================================

use spacetime::render::{StillsOptions, render_stills};

fn scratch_dir(tag: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scratch/render-gates")
        .join(format!("{tag}_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

/// PNG dimensions from the IHDR chunk (bytes 16..24: width, height BE u32).
fn png_dims(path: &Path) -> (u32, u32) {
    let d = std::fs::read(path).expect("read png");
    assert!(d.len() > 24 && &d[1..4] == b"PNG", "not a PNG: {}", path.display());
    let w = u32::from_be_bytes([d[16], d[17], d[18], d[19]]);
    let h = u32::from_be_bytes([d[20], d[21], d[22], d[23]]);
    (w, h)
}

#[test]
fn w11_stills_capture_requested_timestamps_that_differ() {
    // A 2s score: stills at 0/1/2s must all be written, and DIFFER (the film
    // advanced — a frozen page would produce identical bytes, BUG-252).
    let dir = scratch_dir("stills");
    let opts = StillsOptions {
        times_s: vec![0.0, 1.0, 2.0],
        out_dir: dir.clone(),
        width: 320,
        height: 180,
        dpr: 1.0,
    };
    let paths = render_stills(&fixture("film.st"), &opts)
        .unwrap_or_else(|e| panic!("stills render failed: {e}"));
    assert_eq!(paths.len(), 3, "one PNG per requested timestamp");
    for p in &paths {
        assert!(p.exists(), "still not written: {}", p.display());
    }
    // The film moves: the three frames must not be byte-identical.
    let bytes: Vec<Vec<u8>> = paths.iter().map(|p| std::fs::read(p).unwrap()).collect();
    assert!(
        bytes[0] != bytes[2],
        "the first and last still must DIFFER — a `@on &.clip` opacity rise means \
         the film advanced; identical bytes mean the virtual clock never moved"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn w11_dpr_scales_the_captured_pixels() {
    // `--dpr 2` doubles the captured resolution: a 320x180 still becomes 640x360.
    let dir = scratch_dir("dpr");
    let opts = StillsOptions {
        times_s: vec![1.0],
        out_dir: dir.clone(),
        width: 320,
        height: 180,
        dpr: 2.0,
    };
    let paths = render_stills(&fixture("film.st"), &opts)
        .unwrap_or_else(|e| panic!("dpr stills render failed: {e}"));
    let (w, h) = png_dims(&paths[0]);
    assert_eq!((w, h), (640, 360), "dpr 2 must render at 2x the pixels, got {w}x{h}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn w11_from_to_window_trims_the_render() {
    // A 2s score rendered `--from 1 --to 2` at 10fps is a 1s window -> 10 frames.
    let out = scratch_dir("window").join("window.mp4");
    let opts = RenderOptions {
        out: out.clone(),
        fps: 10,
        width: 320,
        height: 180,
        dpr: 1.0,
        from_ms: Some(1000),
        to_ms: Some(2000),
    };
    let rep = render_page(&fixture("film.st"), &opts)
        .unwrap_or_else(|e| panic!("windowed render failed: {e}"));
    assert_eq!(
        rep.frames, 10,
        "a [1s,2s] window at 10fps is 10 frames — a full-score render would be 20"
    );
    std::fs::remove_dir_all(out.parent().unwrap()).ok();
}
