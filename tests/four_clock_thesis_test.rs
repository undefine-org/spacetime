//! W5 — P10 (SIP-001b): the four-clock thesis gate.
//!
//! The family's bet (SIP-001-FAMILY-README §"The bet"): *a website and a video
//! differ only in which signal drives the timeline.* That claim is falsifiable
//! in the strongest form the compiler can offer at compile time: ONE `@form
//! score` body, spliced unchanged under all four clocks, must compile.
//!
//! ```st
//! @form score --story { &title for 50%; }
//! .film  { @score &.time(12s)        { --story; } }   // a film
//! .story { @score &.scroll(cover)    { --story; } }   // scrollytelling
//! .deck  { @score &.steps(advance: &.click) { --story; } } // slide deck
//! .mv    { @score &.playback         { --story; } }   // a media playhead
//! ```
//!
//! The fragment names no driver — the driver is supplied at the `@score` head.
//! That is exactly what R4 predicted: fragments are projection-agnostic, so the
//! four poles need ONE body each, not four. If any clock needed the fragment
//! rewritten, SIP-001b §5's falsifiable test would have failed and the family
//! would need reopening, not patching.

use spacetime::compiler::Compiler;

fn compile_js(source: &str) -> String {
    let dir = tempfile::tempdir().expect("tempdir");
    let entry = dir.path().join("index.st");
    std::fs::write(&entry, source).expect("write");
    Compiler::from_file(&entry, dir.path())
        .unwrap_or_else(|e| panic!("compile failed: {e}"))
        .compile()
        .js
}

fn check_ok(source: &str) -> (bool, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let entry = dir.path().join("index.st");
    std::fs::write(&entry, source).expect("write");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("check")
        .arg(&entry)
        .output()
        .expect("run check");
    (
        out.status.success(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

const PAGE: &str = r#"@version 2026-06-09;


"#;

/// THE FOUR-CLOCK GATE. The SAME `@form score --story` body, spliced under
/// `.time`, `.scroll`, `.steps` and `.playback`, with ZERO fragment changes.
///
/// This is the extension of `the_same_fragment_runs_under_both_poles`
/// (clip_driver_test) from two clocks to four — the two new clocks this wave
/// makes real (`.steps`, `.playback`).
#[test]
fn the_same_fragment_runs_under_all_four_clocks() {
    let source = format!(
        r#"{PAGE}
@form score --story {{
  &title for 50%;
}}

.film  {{ @score &.time(12s)        {{ --story; }} }}
.story {{ @score &.scroll(cover)    {{ --story; }} }}
.deck  {{ @score &.steps(advance: &.click) {{ --story; }} }}
.mv    {{ @score &.playback         {{ --story; }} }}
"#
    );

    let (ok, output) = check_ok(&source);
    assert!(
        ok,
        "W5/P10: the SAME `@form score` body must compile under &.time (a film), \
         &.scroll (scrollytelling), &.steps (a slide deck) and &.playback (a media \
         playhead). If the four poles needed different bodies, SIP-001b §5's \
         falsifiable test would have failed — the family would need reopening, not \
         patching.\nOutput:\n{output}"
    );
}

/// The NEGATIVE half: the same fragment must COMPILE under all four, but the
/// four clocks are NOT interchangeable — a placement that names a position in
/// one domain must be refused in another. `.time` refuses ratios, `.scroll`
/// refuses times. This asserts the check still discriminates: the gate isn't
/// green because placements stopped being validated.
#[test]
fn the_four_clock_fragment_is_not_the_only_green_path() {
    // `at 2s` is a time placement — legal under .time/.playback, refused under
    // .scroll (normalized).
    let scroll_rejects_time = format!(
        r#"{PAGE}
.story {{ @score &.scroll(cover) {{ &title at 2s for 3s; }} }}
"#
    );
    let (ok, out) = check_ok(&scroll_rejects_time);
    assert!(
        !ok && out.contains("E0951"),
        ".scroll is a normalized domain; `at 2s` names a TIME, which must be \
         refused (E0951), not silently treated as a meaningless number.\nOutput:\n{out}"
    );

    // `for 30%` is a ratio — legal under .scroll/.steps, refused under .time.
    let time_rejects_ratio = format!(
        r#"{PAGE}
.film {{ @score &.time(12s) {{ &title for 30%; }} }}
"#
    );
    let (ok, out) = check_ok(&time_rejects_ratio);
    assert!(
        !ok && out.contains("E0951"),
        ".time is a declared time domain; `for 30%` names a ratio, which must be \
         refused (E0951).\nOutput:\n{out}"
    );
}

/// The two new clocks must actually EMIT their driver primitives and a real
/// window map — a fragment that compiles but lowers to nothing is the banned
/// silent class (a directive that compiles to silence is indistinguishable
/// from one never written).
#[test]
fn steps_and_playback_lower_to_real_driver_binds() {
    let source = format!(
        r#"{PAGE}
.deck {{ @score &.steps(advance: &.click) {{ &a for 30%; &b for 30%; }} }}
.mv   {{ @score &.playback              {{ &bar for 50%; }} }}
"#
    );

    let js = compile_js(&source);

    assert!(
        js.contains("steps-driver") || js.contains("var stepCount"),
        "`.steps` must emit a real steps-driver bind (with a step count), not \
         compile to nothing.\n(bundle length {})",
        js.len()
    );
    assert!(
        js.contains("advanceEvents") && js.contains("addEventListener"),
        "`.steps` advance must attach real event listeners."
    );
    assert!(
        js.contains("timeupdate") || js.contains("currentTime"),
        "`.playback` must bind the media playhead (`timeupdate`/currentTime), not \
         compile to nothing."
    );
    assert!(
        js.contains("_swTotal") && js.contains("_swMap"),
        "both new clocks must flow through the SAME window-map lowering as \
         .time/.scroll."
    );
}

/// Evaluate the FIRST emitted `_swMap` in a compiled playback source at the
/// given parent-progress points, with the media duration STUBBED to
/// `stub_duration_ms`.
///
/// Unlike a compile-time-total domain (`.time`/`.clip`), `.playback` emits
/// `_swTotal` as a SIGNAL NAME the primitive resolves per sample — so the map
/// block is NOT bare-runnable in Node. It is run against a stub `ST` whose
/// `resolve` returns the stubbed duration whenever the requested signal is the
/// window's own total, which is exactly what `ST.resolve` does on a real
/// element once the media metadata has loaded. Sampling the real map (D3), not
/// grepping emitted constants, is the only way to see a 0.5ms window.
fn sample_playback_map(source: &str, points: &[f64], stub_duration_ms: u32) -> Vec<f64> {
    let js = compile_js(source);
    let start = js.find("var _swName =").expect("emitted score window");
    let end = start
        + js[start..]
            .find("var progress =")
            .expect("emitted score map end");
    let point_list = points
        .iter()
        .map(f64::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    let program = format!(
        "var ST = {{ resolve: function (el, name) {{ return name === _swTotal ? {stub} : 0; }} }};\nvar el = {{}};\n{block}\nconsole.log(JSON.stringify([{points}].map(_swMap)));",
        block = &js[start..end],
        points = point_list,
        stub = stub_duration_ms
    );
    let output = std::process::Command::new("node")
        .arg("-e")
        .arg(program)
        .output()
        .expect("run node to sample emitted playback map");
    assert!(
        output.status.success(),
        "Node could not evaluate emitted playback score map: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("Node emitted sample array")
}

/// THE PLAYBACK ARM, SAMPLED (BUG-270). The four-clock thesis gate above
/// asserts the fragment COMPILES under `&.playback`; this one asserts it
/// BEHAVES — the same `for 50%` must be a real half-of-the-media window, not
/// a compile-time 0.5 constant against a runtime total.
///
/// `.playback`'s total is the media duration, unknown until metadata loads, so
/// it is emitted as a SIGNAL and resolved per sample. A `for 50%` clip must
/// therefore be scaled by that RESOLVED total: against a stubbed 10000ms
/// duration it is a 5000ms window, so at parent progress 0.25 the playhead is
/// at 2500ms = 50% through the clip, and it completes at 0.5 (5000ms).
///
/// Before the fix the compiler folded the raw 0.5 into a constant span against
/// `total = 1.0` (the compile-time placeholder), so the emitted map was a
/// 0.5-MILLISECOND window: `local = (p * 10000) / 0.5`, clamping to 1.0 at
/// parent progress 0.00005. Sampled over [0, .25, .5, .75, 1] the broken map
/// reads [0, 1, 1, 1, 1] — the clip jumps to its end at a quarter of the way
/// through, which is the exact defect this gate exists to catch.
#[test]
fn the_playback_arm_samples_a_real_half_of_the_media_window() {
    let source = format!(
        r#"{PAGE}
@form score --story {{
  &title for 50%;
}}

.mv {{ @score &.playback {{ --story; }} }}
"#
    );

    let samples = sample_playback_map(&source, &[0.0, 0.25, 0.5, 0.75, 1.0], 10000);
    let rounded: Vec<f64> = samples
        .iter()
        .map(|x| (x * 1000.0).round() / 1000.0)
        .collect();
    assert_eq!(
        rounded,
        vec![0.0, 0.5, 1.0, 1.0, 1.0],
        "BUG-270: a `for 50%` clip under &.playback is half the MEDIA duration. \
         Against a 10000ms stub its window is [0, 5000ms), so the playhead sits \
         at 2500ms (50% through the clip) at parent progress 0.25 and completes \
         at 0.5. A clip that jumps to 1.0 by 0.25 is a compile-time 0.5-\
         millisecond span leaking against the runtime total."
    );
}


