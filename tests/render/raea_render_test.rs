//! PLAN-151 gates — `spacetime render --engine raea`, the browserless backend.
//!
//! The SAME pages `render_test.rs` drives through Chromium, drawn instead by
//! the vendored raea engine (`stdlib/film/vendor/raea`). The first four gates
//! are the Chromium backend's, unchanged in intent — a second engine has to
//! clear the same bar:
//!
//!   1. FRAME COUNT     a 2s @score fixture @ 10fps -> exactly 20 frames / 2.0s
//!   2. MOTION          frame 0 vs frame 10 DIFFER (a frozen video must fail)
//!   3. DETERMINISM     two renders -> identical per-frame hashes
//!   4. DISCRIMINATION  the motion gate FAILS on a page nobody animates
//!
//! The rest pin the engine gaps a real Spacetime film found in raea, each a
//! RED-first gate seen failing against the unpatched engine (see
//! `stdlib/film/vendor/PROVENANCE.md`, *Local patches*):
//!
//!   5. SHOT            a `@form shot` stamped through `<template>` appears
//!   6. CLIP TEXT       `background-clip: text` paints letters, not a slab
//!   7. MASK VAR        a mask reading an animated custom property grows
//!   8. COUNTER         a CSS counter reset from an animated property ticks
//!   9. STILLS          `--stills` writes `still-<ms>ms.png` per timestamp
//!
//! These need the raea binary and a Vulkan GPU, so they are `#[ignore]`d by
//! default — run them explicitly (and READ the count: an ignored gate proves
//! nothing):
//!
//!   cargo test --test raea_render_test -- --ignored
//!
//! A missing binary FAILS a gate loudly; it never passes it.

use std::path::{Path, PathBuf};
use std::process::Command;

use spacetime::render::raea::{render_page, render_stills};
use spacetime::render::{RenderOptions, RenderReport, StillsOptions};

const NEEDS: &str = "needs the raea binary and a Vulkan GPU: cargo test --test raea_render_test -- --ignored";

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/render/fixtures").join(name)
}

fn tmp(tag: &str, ext: &str) -> PathBuf {
    std::env::temp_dir().join(format!("raea_{tag}_{}.{ext}", std::process::id()))
}

fn render(fixture_name: &str, fps: u32, tag: &str) -> (PathBuf, RenderReport) {
    let out = tmp(&format!("{tag}_{fixture_name}"), "mp4");
    let opts = RenderOptions { out: out.clone(), fps, width: 320, height: 180, ..Default::default() };
    let rep = render_page(&fixture(fixture_name), &opts)
        .unwrap_or_else(|e| panic!("raea render of {fixture_name} failed: {e}"));
    (out, rep)
}

/// Stills of `fixture_name` at `times_s`, 480×270, as raw RGB24 per still.
fn stills(fixture_name: &str, times_s: &[f64], tag: &str) -> Vec<Vec<u8>> {
    let out_dir = tmp(&format!("{tag}_stills"), "d");
    let sopts = StillsOptions { times_s: times_s.to_vec(), out_dir: out_dir.clone(), width: 480, height: 270, dpr: 1.0 };
    let paths = render_stills(&fixture(fixture_name), &sopts, 10)
        .unwrap_or_else(|e| panic!("raea stills of {fixture_name} failed: {e}"));
    let pixels = paths.iter().map(|p| rgb_of(p, 0)).collect();
    std::fs::remove_dir_all(&out_dir).ok();
    pixels
}

fn ffprobe_frames(path: &Path) -> (u32, f64) {
    let out = Command::new("ffprobe")
        .args(["-v", "error", "-select_streams", "v:0", "-count_frames", "-show_entries", "stream=nb_read_frames"])
        .args(["-show_entries", "format=duration", "-of", "default=noprint_wrappers=1:nokey=1"])
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("ffprobe failed: {e}"));
    assert!(out.status.success(), "ffprobe errored: {}", String::from_utf8_lossy(&out.stderr));
    let text = String::from_utf8_lossy(&out.stdout);
    let mut it = text.split_whitespace();
    let frames = it.next().and_then(|s| s.parse().ok()).unwrap_or_else(|| panic!("no frame count: {text:?}"));
    let duration = it.next().and_then(|s| s.parse().ok()).unwrap_or_else(|| panic!("no duration: {text:?}"));
    (frames, duration)
}

/// Raw RGB24 of frame `index` of a video, or of a PNG (index 0).
fn rgb_of(path: &Path, index: u32) -> Vec<u8> {
    let out = Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(path)
        .args(["-vf", &format!("select=eq(n\\,{index})"), "-frames:v", "1", "-f", "rawvideo", "-pix_fmt", "rgb24", "-"])
        .output()
        .unwrap_or_else(|e| panic!("ffmpeg frame extract failed: {e}"));
    assert!(out.status.success(), "ffmpeg errored: {}", String::from_utf8_lossy(&out.stderr));
    out.stdout
}

/// How many pixels are clearly red (the fixtures' only colour on white).
fn red_pixels(rgb: &[u8]) -> usize {
    rgb.chunks_exact(3).filter(|p| p[0] > 180 && p[1] < 90 && p[2] < 90).count()
}

// 1. FRAME COUNT
#[test]
#[ignore = "needs the raea binary and a Vulkan GPU: cargo test --test raea_render_test -- --ignored"]
fn raea_2s_fixture_at_10fps_is_exactly_20_frames() {
    let (path, rep) = render("film.st", 10, "count");
    let (frames, duration) = ffprobe_frames(&path);
    assert_eq!(frames, 20, "2s @ 10fps must encode exactly 20 frames, got {frames}");
    assert!((duration - 2.0).abs() < 0.05, "2s @ 10fps must be ~2.0s, got {duration}");
    assert_eq!(rep.frames, 20, "the render report must agree with ffprobe");
    std::fs::remove_file(&path).ok();
}

// 2. MOTION — the score drives raea's frames through the clock bridge.
#[test]
#[ignore = "needs the raea binary and a Vulkan GPU: cargo test --test raea_render_test -- --ignored"]
fn raea_motion_frames_0_and_10_differ() {
    let (path, _) = render("film.st", 10, "motion");
    assert_ne!(
        rgb_of(&path, 0),
        rgb_of(&path, 10),
        "a moving score rendered by raea must differ between frame 0 and frame 10 — identical frames \
         mean the bridge never advanced ST._clock"
    );
    std::fs::remove_file(&path).ok();
}

// 3. DETERMINISM
#[test]
#[ignore = "needs the raea binary and a Vulkan GPU: cargo test --test raea_render_test -- --ignored"]
fn raea_two_renders_are_identical_per_frame() {
    let (a, _) = render("film.st", 10, "detA");
    let (b, _) = render("film.st", 10, "detB");
    let (fa, _) = ffprobe_frames(&a);
    assert_eq!(fa, ffprobe_frames(&b).0, "both renders must have the same frame count");
    for i in 0..fa {
        assert!(rgb_of(&a, i) == rgb_of(&b, i), "frame {i} differs between two raea renders of the same page");
    }
    std::fs::remove_file(&a).ok();
    std::fs::remove_file(&b).ok();
}

// 4. DISCRIMINATION — the motion gate must fail on a page nobody animates.
#[test]
#[ignore = "needs the raea binary and a Vulkan GPU: cargo test --test raea_render_test -- --ignored"]
fn raea_motion_gate_fails_on_a_static_page() {
    let (path, _) = render("static.st", 10, "disc");
    assert!(
        rgb_of(&path, 0) == rgb_of(&path, 10),
        "a page whose score nothing consumes must render frozen under raea too — if frames differ, the \
         motion gate is satisfied by something other than the score"
    );
    std::fs::remove_file(&path).ok();
}

// 5. SHOT — markup stamped through <template> (raea's innerHTML/template.content).
#[test]
#[ignore = "needs the raea binary and a Vulkan GPU: cargo test --test raea_render_test -- --ignored"]
fn raea_draws_a_template_stamped_shot() {
    let shots = stills("raea_shot.st", &[0.9], "shot");
    let red = red_pixels(&shots[0]);
    // the 160px square at 480×270: ~25,600 px; unpatched, the shot never mounted (0)
    assert!(red > 15_000, "the stamped shot's red square must be on screen by 0.9s, found {red} red px");
}

// 6. CLIP TEXT — the background shows only through the glyphs.
#[test]
#[ignore = "needs the raea binary and a Vulkan GPU: cargo test --test raea_render_test -- --ignored"]
fn raea_clips_a_background_to_its_text() {
    let shots = stills("raea_clip_text.st", &[0.95], "cliptext");
    let red = red_pixels(&shots[0]);
    let total = shots[0].len() / 3;
    // unpatched: the whole h1 box is red (~60k px of 129.6k); the four I's are a fraction of that
    assert!(red > 500, "the letters must be painted with the clipped background, found {red} red px");
    assert!(
        red < total / 5,
        "`background-clip: text` must clip to the glyphs — {red} of {total} px are red, which is a slab, not text"
    );
}

// 7. MASK VAR — a mask whose stop reads an animated custom property grows.
#[test]
#[ignore = "needs the raea binary and a Vulkan GPU: cargo test --test raea_render_test -- --ignored"]
fn raea_repaints_a_mask_driven_by_a_custom_property() {
    let shots = stills("raea_mask_var.st", &[0.2, 0.9], "maskvar");
    let (early, late) = (red_pixels(&shots[0]), red_pixels(&shots[1]));
    assert!(
        late > early * 4 && late > 10_000,
        "the disc must grow as --reveal animates (0.2s: {early} red px, 0.9s: {late}) — a frozen mask means \
         the var()-driven mask-image change was never repainted"
    );
}

// 8. COUNTER — `content: counter(fr)` reset from a score-driven `--fr` ticks.
#[test]
#[ignore = "needs the raea binary and a Vulkan GPU: cargo test --test raea_render_test -- --ignored"]
fn raea_draws_a_css_counter_driven_by_the_score() {
    let shots = stills("raea_counter.st", &[0.1, 0.9], "counter");
    let (early, late) = (red_pixels(&shots[0]), red_pixels(&shots[1]));
    // 0.1s shows "90" (or so), 0.9s "810": both drawn, and not the same picture
    assert!(early > 200 && late > 200, "the counter's digits must be drawn (0.1s: {early} red px, 0.9s: {late})");
    assert!(shots[0] != shots[1], "the counter must change as the score drives --fr — identical frames mean it is frozen");
}

// 9. STILLS — one PNG per timestamp, named as the Chromium backend names them.
#[test]
#[ignore = "needs the raea binary and a Vulkan GPU: cargo test --test raea_render_test -- --ignored"]
fn raea_stills_are_written_per_timestamp() {
    let out_dir = tmp("stillnames", "d");
    let sopts = StillsOptions { times_s: vec![1.5, 0.5], out_dir: out_dir.clone(), width: 320, height: 180, dpr: 1.0 };
    let paths = render_stills(&fixture("film.st"), &sopts, 10).unwrap_or_else(|e| panic!("{NEEDS}: {e}"));
    let names: Vec<String> = paths.iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
    assert_eq!(names, ["still-500ms.png", "still-1500ms.png"], "stills are sorted by time and named by ms");
    assert!(rgb_of(&paths[0], 0) != rgb_of(&paths[1], 0), "0.5s and 1.5s of a moving score must differ");
    std::fs::remove_dir_all(&out_dir).ok();
}
