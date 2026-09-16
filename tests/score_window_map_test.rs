//! W4/R3 — a score lowers to a WINDOW MAP, not to a scheduler.
//!
//! # The ruling being gated
//!
//! `@score` adds no runtime concept (SIP-001 §Proposal/4, ruling R3). Every
//! driver already publishes exactly one 0..1 progress signal, so a score is a
//! pure derivation over that signal: each clip's progress is a function of its
//! parent's progress.
//!
//! ```st
//! &lower-third at 2s for 3s          // inside @score &.time(60s)
//! ```
//! ```js
//! $lower_third_progress <- clamp01(($film_progress * 60000 - 2000) / 3000)
//! ```
//!
//! Everything in a score body is a shape of that one map: `at/for` is an affine
//! window; `during A to B` resolves a window from two named spans; a transition
//! form applies over the overlap; and `rate:` is the point where the map stops
//! being affine (which is why ramp/freeze/reverse are one concept and not
//! three). Because the map is PURE, the score is seekable by construction —
//! pressure test C4 (render determinism) falls out instead of being enforced.
//!
//! # Why this is asserted on emitted JS
//!
//! The distinction between "a window map" and "a scheduler that happens to work
//! right now" is invisible to a check-green or an `inspect` dump. It is visible
//! in what the compiler EMITS. If these assertions are ever satisfied by a tick
//! loop walking a clip list, the design has been lost and live/render will
//! diverge (SIP-001b §8.6) — so the shape of the output is the thing under test,
//! not merely its behavior.
//!
//! # Status: RED by design
//!
//! Written before `@score` exists, per PLAN-128 t1.

use spacetime::compiler::Compiler;

/// Compile a page and return its emitted JS.
///
/// NB a fixture that is not a PAGE compiles to nothing while still reporting
/// success, so every source here carries real markup and is compiled through
/// the same API the CLI uses.
fn compile_js(source: &str) -> String {
    let dir = tempfile::tempdir().expect("tempdir");
    let entry = dir.path().join("index.st");
    std::fs::write(&entry, source).expect("write");
    Compiler::from_file(&entry, dir.path())
        .unwrap_or_else(|e| panic!("compile failed: {e}"))
        .compile()
        .js
}

const PAGE: &str = r#"@version 2026-06-09;

<div class="stage">
  <div class="title">Title</div>
  <div class="lower-third">Ada</div>
</div>
"#;

/// A baseline: the same page with no score at all. Every assertion below is a
/// claim that the score ADDED something, and that claim is only meaningful
/// against a control — a bundle already containing the runtime will match all
/// sorts of substrings by accident.
fn baseline() -> String {
    compile_js(PAGE)
}

/// Evaluate the FIRST emitted `_swMap` in a compiled source at the given
/// parent-progress points. Constants reaching the bundle are not behavior:
/// this executes the same pure map the browser receives (D3 — SAMPLE the map,
/// don't grep the emitted constants). The extracted block references only the
/// map itself; a compile-time constant `total` (every domain except `.playback`)
/// keeps `ST.resolve` from ever evaluating, so the block runs bare in Node.
fn sample_map(source: &str, points: &[f64]) -> Vec<f64> {
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
        "{}\nconsole.log(JSON.stringify([{point_list}].map(_swMap)));",
        &js[start..end]
    );
    let output = std::process::Command::new("node")
        .arg("-e")
        .arg(program)
        .output()
        .expect("run node to sample emitted score map");
    assert!(
        output.status.success(),
        "Node could not evaluate emitted score map: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("Node emitted sample array")
}

/// Compile a fragment under a given score head (`&.time(12s)` or `&.clip`) and
/// sample the emitted window map. The `--story;` bare-splice form is used
/// because it is legal under BOTH poles — an inline `for 50%` under a `.time`
/// clock is E0951, so the "same body, both projections" thesis can only be
/// stated through a fragment.
fn sample_fragment_map(domain: &str, points: &[f64]) -> Vec<f64> {
    let source = format!(
        r#"{PAGE}
@form score --story {{
  &title for 50%;
}}

.stage {{
  @score {domain} {{ --story; }}
}}
"#
    );
    sample_map(&source, points)
}

/// The core of R3: a placement becomes an arithmetic derivation of the parent's
/// progress signal.
///
/// The exact spelling of the emitted expression is the implementer's to choose;
/// what is asserted is that the child's progress is COMPUTED FROM the parent's,
/// which is what makes the score a pure function and therefore seekable.
#[test]
fn a_placement_lowers_to_a_derivation_of_parent_progress() {
    let source = format!(
        r#"{PAGE}
.stage {{
  @score &.time(60s) as &film {{
    &lower-third at 2s for 3s;
  }}
}}
"#
    );

    let js = compile_js(&source);
    let base = baseline();
    assert!(
        js.len() > base.len(),
        "W4/R3: the score emitted NOTHING — the bundle is no larger than a page \
         with no score at all. A directive that compiles to silence is \
         indistinguishable from one that was never written (BUG-216 class).\n\
         score={} baseline={}",
        js.len(),
        base.len()
    );

    // The window's own numbers must reach the output. `at 2s for 3s` inside a
    // 60s score is the window [2000, 5000) — if neither bound appears, the
    // placement was parsed and then discarded.
    let has_offset = js.contains("2000") || js.contains("2e3");
    let has_span = js.contains("3000") || js.contains("3e3");
    assert!(
        has_offset && has_span,
        "W4/R3: `at 2s for 3s` must lower to a window carrying its own bounds \
         (offset 2000ms, span 3000ms). Neither reached the bundle, so the \
         placement was accepted and dropped.\noffset={has_offset} span={has_span}"
    );
}

/// R3's payoff, and the reason the IR is a map rather than a schedule: `rate` is
/// the DERIVATIVE of the window map. A constant rate of 0 is a freeze; a
/// negative rate is a reversal. They are one mechanism, so they must lower
/// through one path.
/// NB the surface is a TRAILING WORD (`rate 1 -> 2`), not a braced settings
/// block. A braced block was tried first: the score body is captured as
/// `{ $entries:score_entry+ }` and that capture ends at the first `}`
/// regardless of nesting, so an inner brace closes the score (loudly — E0946,
/// BUG-229's fix working). The trailing form is the better surface anyway, per
/// P4: `rate` modifies WHEN the clip's progress advances, which is placement
/// vocabulary, the same category as `at` and `for`.
#[test]
fn rate_lowers_through_the_same_window_map() {
    let ramp = compile_js(&format!(
        r#"{PAGE}
.stage {{
  @score &.time(60s) {{
    &lower-third at 0s for 10s rate 1 -> 2;
  }}
}}
"#
    ));
    let freeze = compile_js(&format!(
        r#"{PAGE}
.stage {{
  @score &.time(60s) {{
    &lower-third at 0s for 10s rate 0;
  }}
}}
"#
    ));

    assert_ne!(
        ramp, freeze,
        "W4/R3: `rate: 1 -> 2` (a ramp) and `rate: 0` (a freeze) must produce \
         DIFFERENT window maps. Identical output means rate was parsed and \
         ignored — the same defect class as BUG-253, where five driver rows \
         compiled to byte-identical logic."
    );
}

/// A score under a `&.clip` driver must produce the SAME derivation shape as one
/// under `&.time`, because R1 says the driver supplies the domain and R3 says
/// the body is a pure map either way.
///
/// This is SIP-001b §5's falsifiable thesis reduced to a compile-time claim —
/// but asserted by SAMPLING the window map (D3's discipline), not by comparing
/// bundle lengths (the old `len > baseline` check, which a score that emitted
/// garbage-but-plausible code satisfied). The `--story;` bare-splice fragment is
/// the same body under both poles — an inline `for 50%` under a `.time` clock
/// is E0951, so the thesis can only be stated through a fragment.
///
/// `&.time(12s)` gives `total = 12000`, so `for 50%` → a 6000ms span; `&.clip`
/// gives `total = 1.0`, so `for 50%` → 0.5 of the inherited window. BOTH are
/// "half the parent", so the maps must sample IDENTICALLY.
///
/// The sample at 0.25 is also the BUG-267 discriminator: a bare `for 50%` that
/// leaked the raw 0.5 against `total = 12000` (a half-millisecond window) would
/// clamp to 1.0 at parent progress 0.25 instead of 0.5. The old `len > baseline`
/// assertion was structurally blind to exactly this.
#[test]
fn the_same_body_samples_the_same_map_under_both_projections() {
    let pts = [0.0, 0.25, 0.5, 0.75, 1.0];
    let by_time = sample_fragment_map("&.time(12s)", &pts);
    let by_clip = sample_fragment_map("&.clip", &pts);

    let expected = vec![0.0, 0.5, 1.0, 1.0, 1.0];
    let round = |v: &Vec<f64>| {
        v.iter()
            .map(|x| (x * 1000.0).round() / 1000.0)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        round(&by_time),
        expected,
        "W4/R3: `&title for 50%` under a bare `--story;` splice in a 12s score \
         is a 6000ms window, so its map must reach the end at half the parent's \
         progress. A raw 0.5 reaching the window map (span against total 12000) \
         would clamp to 1.0 at 0.25 — BUG-267's half-millisecond window."
    );
    assert_eq!(
        round(&by_clip),
        expected,
        "W4/R3: under `&.clip` the same body is a 0.5 fraction of the inherited \
         window, which must sample identically to the time pole — that identity \
         IS the thesis (SIP-001b §5). If the two poles needed different \
         mechanisms, these maps would diverge."
    );
    assert_eq!(
        round(&by_time),
        round(&by_clip),
        "W4/R3: the two poles must sample identically for the SAME body. A \
         divergence here means the driver domain changed the shape of the \
         window map, not just its scale."
    );
}

/// The three rate shapes must be three DIFFERENT maps.
///
/// R3's claim is that ramp, freeze and reverse are one mechanism — the
/// derivative of the window map — rather than three features. That claim only
/// holds if they are genuinely distinguishable in the output: one expression
/// producing three behaviors, not one expression producing one behavior three
/// times.
///
/// This is the BUG-253 lesson applied to arithmetic. There, five registry rows
/// compiled to byte-identical logic and nobody noticed for months because
/// every gate was positive-only. Here, `rate 0` and `rate -1` collapsing into
/// the same emitted map would be the same defect wearing different clothes.
#[test]
fn ramp_freeze_and_reverse_are_three_distinct_maps() {
    let of = |rate: &str| {
        compile_js(&format!(
            r#"{PAGE}
.stage {{
  @score &.time(10s) {{
    &lower-third at 0s for 10s rate {rate};
  }}
}}
"#
        ))
    };

    let plain = of("1");
    let ramp = of("1 -> 2");
    let freeze = of("0");
    let reverse = of("-1");

    let cases = [
        ("rate 1 (the affine default)", &plain),
        ("rate 1 -> 2 (speed ramp)", &ramp),
        ("rate 0 (freeze)", &freeze),
        ("rate -1 (reverse)", &reverse),
    ];

    for (i, (name_a, a)) in cases.iter().enumerate() {
        for (name_b, b) in cases.iter().skip(i + 1) {
            assert_ne!(
                a, b,
                "W4/R3: `{name_a}` and `{name_b}` emit IDENTICAL output. Rate is \
                 the derivative of the window map, so these must produce \
                 different maps — identical output means the rate was parsed \
                 and then ignored, which is exactly BUG-253's signature (two \
                 inputs, byte-identical logic)."
            );
        }
    }

    // And the default must stay CLEAN: the common case emits no rate
    // machinery, so reading the generated code for an ordinary clip is not
    // taxed by a feature it does not use.
    assert!(
        plain.contains("_swRateFrom = 1") && !plain.contains("_swRateFrom = 0"),
        "W4/R3: rate 1 is the identity and should lower as the plain affine \
         window."
    );
}

/// The rate map must BEHAVE, not merely compile differently.
///
/// This test exists because the sibling `ramp_freeze_and_reverse_are_three_
/// distinct_maps` passed while the implementation was WRONG. It compared
/// emitted bundles, and the bundles did differ — `_swRateFrom = -1` versus
/// `_swRateFrom = 2` — while the resulting behavior was identical to `rate 1`
/// in both cases, because the integral was being normalized by its own total
/// and the normalization cancelled every constant rate.
///
/// The lesson generalizes past this feature: asserting that two compilations
/// DIFFER proves the input reached the output, not that it MEANS anything.
/// Where a feature is arithmetic, evaluate the arithmetic.
///
/// The expectations below are the semantics an editor's speed control has:
/// rate 2 finishes early and holds; rate 0.5 never finishes; rate -1 walks
/// backward from 1 to 0; rate 0 never moves.
#[test]
fn the_rate_map_evaluates_correctly_over_its_window() {
    // Transcribed from score-window.st. Negative rates select their reversed
    // origin once at t=0; no sample-dependent correction is allowed.
    fn map(r0: f64, r1: f64, local: f64) -> f64 {
        if r0 == 1.0 && r1 == 1.0 {
            return local;
        }
        let area = r0 * local + (r1 - r0) * local * local / 2.0;
        let origin = if r0 < 0.0 { 1.0 } else { 0.0 };
        (origin + area).clamp(0.0, 1.0)
    }

    let at = |r0: f64, r1: f64| -> Vec<f64> {
        [0.0, 0.25, 0.5, 0.75, 1.0]
            .iter()
            .map(|t| (map(r0, r1, *t) * 1000.0).round() / 1000.0)
            .collect()
    };

    for (rate, transcribed, expected) in [
        ("1", at(1.0, 1.0), vec![0.0, 0.25, 0.5, 0.75, 1.0]),
        ("1 -> 2", at(1.0, 2.0), vec![0.0, 0.281, 0.625, 1.0, 1.0]),
        ("2", at(2.0, 2.0), vec![0.0, 0.5, 1.0, 1.0, 1.0]),
        ("0.5", at(0.5, 0.5), vec![0.0, 0.125, 0.25, 0.375, 0.5]),
        ("0", at(0.0, 0.0), vec![0.0, 0.0, 0.0, 0.0, 0.0]),
        ("-1", at(-1.0, -1.0), vec![1.0, 0.75, 0.5, 0.25, 0.0]),
        ("-2", at(-2.0, -2.0), vec![1.0, 0.5, 0.0, 0.0, 0.0]),
        ("2 -> 0.5", at(2.0, 0.5), vec![0.0, 0.453, 0.813, 1.0, 1.0]),
        ("10", at(10.0, 10.0), vec![0.0, 1.0, 1.0, 1.0, 1.0]),
    ] {
        let emitted = sample_map(
            &format!(
                r#"{PAGE}
.stage {{
  @score &.time(10s) {{
    &lower-third at 0s for 10s rate {rate};
  }}
}}
"#
            ),
            &[0.0, 0.25, 0.5, 0.75, 1.0],
        )
        .into_iter()
        .map(|sample| (sample * 1000.0).round() / 1000.0)
        .collect::<Vec<_>>();
        assert_eq!(
            transcribed, expected,
            "transcription must specify rate {rate}"
        );
        assert_eq!(
            emitted, expected,
            "emitted rate {rate} must sample to its declared window map"
        );
    }

    // Negative-rate origin is fixed at the window start. This negative catches
    // the old per-sample `if area < 0 { 1 + area }` correction: it produced 0
    // at t=0 for rate -2 and silently made its first half look like a forward map.
    assert_ne!(
        at(-2.0, -2.0)[0],
        0.0,
        "rate -2 starts at the reversed origin"
    );
}

/// Guards the transcription above against drift.
///
/// A hand-copied formula in a test is a second implementation, and two
/// implementations disagree eventually. This asserts the primitive still
/// contains the expression the test models, so a change to one without the
/// other fails loudly rather than leaving the test quietly checking a formula
/// nothing runs.
#[test]
fn the_transcribed_rate_map_matches_the_primitive() {
    let src = std::fs::read_to_string("stdlib/primitives/animation/score-window.st")
        .expect("read score-window.st");
    for fragment in [
        "_swRateFrom * local",
        "(_swRateTo - _swRateFrom) * local * local / 2",
        "var origin = _swRateFrom < 0 ? 1 : 0;",
        "local = origin + area;",
    ] {
        assert!(
            src.contains(fragment),
            "the rate map in score-window.st no longer contains `{fragment}`, so \
             the formula transcribed into `the_rate_map_evaluates_correctly_over_\
             its_window` is modelling code that no longer exists. Update both."
        );
    }
    assert!(
        !src.contains("if (area < 0) area = 1 + area;"),
        "reverse origin must be selected once at t=0, never patched per sample"
    );
    assert!(
        !src.contains("area / whole"),
        "the integral is being normalized by its own total again — that makes \
         EVERY constant rate identical to rate 1 (rate 2 and rate -1 both \
         collapse into the identity map), which is the bug this pair of tests \
         was written to catch."
    );
}

/// A rate ramp cannot change which endpoint is its origin midway through.
///
/// The negative assertion matters: accepting the source while dropping its
/// window would still produce a superficially green compile.
#[test]
fn sign_crossing_rate_ramp_is_refused() {
    let source = format!(
        r#"{PAGE}
.stage {{
  @score &.time(2s) {{
    &title for 2s rate 1 -> -1;
  }}
}}
"#
    );
    let dir = tempfile::tempdir().expect("tempdir");
    let entry = dir.path().join("index.st");
    std::fs::write(&entry, source).expect("write");
    let compiled = Compiler::from_file(&entry, dir.path())
        .expect("compile setup")
        .compile();

    assert!(
        compiled
            .pipeline_errors
            .iter()
            .any(|error| error.code == "E0954"),
        "a sign-crossing rate must fail loudly rather than emit an arbitrary map: {:?}",
        compiled.pipeline_errors
    );
    assert!(
        !compiled
            .pipeline_errors
            .iter()
            .any(|error| error.code == "E0951"),
        "the crossing error must name the rate problem, not disguise it as an empty score"
    );
}

/// A dead rate window is valid but suspicious; intentional freeze and normal
/// speed remain quiet. Both positive and negative assertions prevent warning
/// noise from becoming an accepted baseline.
#[test]
fn dead_rate_windows_warn_but_freeze_and_identity_do_not() {
    let compile = |rate: &str| {
        let source = format!(
            r#"{PAGE}
.stage {{
  @score &.time(2s) {{
    &title for 2s rate {rate};
  }}
}}
"#
        );
        let dir = tempfile::tempdir().expect("tempdir");
        let entry = dir.path().join("index.st");
        std::fs::write(&entry, source).expect("write");
        Compiler::from_file(&entry, dir.path())
            .expect("compile setup")
            .compile()
    };

    let fast = compile("10");
    assert!(
        fast.migration_warnings.iter().any(|warning| {
            warning.code == "W0718"
                && warning.message.contains("90%")
                && warning
                    .hint
                    .as_deref()
                    .is_some_and(|hint| hint.contains("for 200ms"))
        }),
        "rate 10 must warn that 90% is dead and offer its active duration: {:?}",
        fast.migration_warnings
    );

    let slow = compile("0.5");
    assert!(
        slow.migration_warnings
            .iter()
            .any(|warning| warning.code == "W0718" && warning.message.contains("50%")),
        "rate 0.5 must warn that half the clip is never reached: {:?}",
        slow.migration_warnings
    );

    for rate in ["0", "1"] {
        let compiled = compile(rate);
        assert!(
            !compiled
                .migration_warnings
                .iter()
                .any(|warning| warning.code == "W0718"),
            "rate {rate} must not produce dead-window noise: {:?}",
            compiled.migration_warnings
        );
    }
}

/// `gap` advances the playhead without placing anything.
///
/// A gap is an absence, and an absence is already expressible by where the
/// next clip starts — so it earns its place only by keeping a score RELATIVE.
/// A body written in `->` sequence and `gap` carries no absolute offsets, so
/// inserting a clip near the top does not renumber everything below it. That
/// is the difference between a score you can edit and one you must recompute.
#[test]
fn a_gap_moves_the_playhead_and_places_nothing() {
    let with_gap = compile_js(&format!(
        r#"{PAGE}
.stage {{
  @score &.time(10s) {{
    &title for 2s;
    gap 500ms;
    &lower-third for 2s;
  }}
}}
"#
    ));

    assert!(
        with_gap.contains("_swOffset = 2500"),
        "W4/t3b: the clip after `gap 500ms` must start at 2500 (2000 + 500). \
         The gap places nothing, but the playhead still moves — a gap that \
         does not advance the cursor is silently a no-op, which makes every \
         relative score subtly wrong."
    );

    // And it must not emit a window of its own: a gap has no subject, so
    // there is nothing to animate. Exactly two clips, two windows.
    assert_eq!(
        with_gap.matches("_swMap = function").count(),
        2,
        "W4/t3b: a gap must not emit a window map — it is an absence, not an \
         invisible clip with a progress signal."
    );
}

/// R2: grouping is a FRAGMENT, and a fragment must actually EXPAND.
///
/// `@form score --name` was already the settled answer to "who owns scene
/// grouping" — no `@scene` keyword — but a fragment that does not expand is
/// worse than no grouping at all: `--story` was compiling to an opaque subject
/// and emitting a window for a nonexistent element `__clip___story`, while
/// neither of the fragment's two clips existed. Green check, plausible bundle,
/// wrong page.
///
/// Two root causes, both worth remembering. The `@form score` declaration
/// stored its body as `keyframes` — form.st's own comment said "score lines
/// are NOT keyframes; the score grammar lands with `@score` itself", which had
/// been true and no longer was. And a declaration stores its name WITH the
/// `--` sigil while the splice's capture strips it.
#[test]
fn a_score_fragment_expands_into_its_own_steps() {
    let js = compile_js(&format!(
        r#"{PAGE}
@form score --story {{
  &title for 50%;
  &lower-third for 50%;
}}

.stage {{
  @score &.time(6s) {{ --story; }}
}}
"#
    ));

    assert!(
        js.contains("_swName = \"__clip_title\""),
        "W4/R2: the fragment's FIRST clip must exist as a real window."
    );
    assert!(
        js.contains("_swName = \"__clip_lower_third\"")
            || js.contains("_swName = \"__clip_lower_third_1\""),
        "W4/R2: the fragment's SECOND clip must exist too — expanding only the \
         head would be a subtler version of the same bug."
    );
    assert!(
        !js.contains("__clip___story"),
        "W4/R2: `--story` must not survive as a subject in its own right. A \
         window named after the FRAGMENT means the splice was treated as an \
         element, so the page animates something that does not exist."
    );

    // The second clip must be SEQUENCED after the first: a fragment is an
    // arrangement, so its internal order has to survive splicing.
    assert!(
        js.contains("_swOffset = 3000"),
        "W4/R2: the fragment's second clip must start at 3000ms (half the 6s \
         score, after the first's 50% span). Expanding the steps but losing \
         their sequence would stack every clip at 0. NB W5b/BUG-267: the bare \
         splice scales the raw 0.5 into the score's total, so this is 3000ms, \
         not `_swOffset = 0.5` — a half-millisecond offset."
    );
}

/// A self-splicing fragment must not CRASH the compiler.
///
/// `@form score --loopy { &a for 25%; --loopy; }` overflowed the stack and
/// aborted the process — not a diagnostic, not a nonzero exit, a `fatal
/// runtime error: stack overflow`. Found by the W4 review swarm.
///
/// A cycle is refused at the splice point and the step is left unexpanded, so
/// the entry survives for downstream diagnostics rather than the score
/// silently losing it. The compiler completing is the assertion here: this
/// test passing at all is the fix.
#[test]
fn a_self_splicing_fragment_does_not_crash_the_compiler() {
    let js = compile_js(&format!(
        r#"{PAGE}
@form score --loopy {{
  &title for 25%;
  --loopy;
}}

.stage {{
  @score &.time(8s) {{ --loopy; }}
}}
"#
    ));

    // The non-recursive part still expands.
    assert!(
        js.contains("_swName = \"__clip_title\""),
        "W4/R2: a cycle must stop the DESCENT, not discard the fragment's \
         well-formed steps."
    );
}

/// A mutual cycle across two fragments must not crash either.
///
/// The guard tracks the whole splice stack rather than just the immediate
/// parent, so `--a` -> `--b` -> `--a` is caught at the same point a direct
/// self-splice is. A guard that only compared against the immediate parent
/// would pass the test above and still abort here.
#[test]
fn a_mutual_fragment_cycle_does_not_crash_the_compiler() {
    let js = compile_js(&format!(
        r#"{PAGE}
@form score --ping {{
  &title for 25%;
  --pong;
}}
@form score --pong {{
  &lower-third for 25%;
  --ping;
}}

.stage {{
  @score &.time(8s) {{ --ping; }}
}}
"#
    ));

    assert!(
        js.contains("_swName = \"__clip_title\""),
        "W4/R2: a mutual cycle must terminate with the well-formed steps intact."
    );
}

/// A clip's signal suffix must count PRIOR PLACEMENTS OF THE SAME SUBJECT,
/// not the loop index.
///
/// Keyed on the index, the SECOND CLIP of any score got `_1` regardless of
/// subject: `&b at 3s` published `__clip_b_1` while `@on &.clip` on `.b`
/// watched `__clip_b`. Two names that never meet — the element sat frozen with
/// a green build, which is BUG-258's exact signature reappearing one clip
/// later.
///
/// The first clip always worked, so every single-clip test passed. That is why
/// this gate uses TWO DISTINCT subjects: the bug is invisible with one.
#[test]
fn distinct_subjects_are_never_suffixed() {
    let js = compile_js(&format!(
        r#"{PAGE}
.stage {{
  @score &.time(4s) {{
    &title at 0s for 1s;
    &lower-third at 3s for 1s;
  }}
}}
"#
    ));

    assert!(
        js.contains("_swName = \"__clip_title\""),
        "the first clip publishes under its bare subject name"
    );
    assert!(
        js.contains("_swName = \"__clip_lower_third\""),
        "a DIFFERENT subject must also publish bare — a `_1` here means the \
         suffix is counting clips instead of repeats, and the consumer \
         (`@on &.clip` on `.lower-third`, which derives `__clip_lower_third`) \
         watches a signal nobody publishes"
    );
    assert!(
        !js.contains("__clip_lower_third_1"),
        "no suffix may appear for a subject placed once"
    );
}

/// …and the suffix MUST still appear when one subject really is placed twice.
/// Without this, the fix above is satisfied by never suffixing at all, which
/// would make two windows collide on one name.
#[test]
fn a_repeated_subject_is_suffixed() {
    let js = compile_js(&format!(
        r#"{PAGE}
.stage {{
  @score &.time(4s) {{
    &title at 0s for 1s;
    &title at 3s for 1s;
  }}
}}
"#
    ));

    assert!(
        js.contains("_swName = \"__clip_title\""),
        "the first placement keeps the bare name"
    );
    assert!(
        js.contains("_swName = \"__clip_title_1\""),
        "the SECOND placement of the SAME subject must be suffixed, or the two \
         windows collide and the later one silently wins"
    );
}

/// W5a/D2: a fragment's RELATIVE measures scale to the frame it is spliced
/// into — the property that makes one fragment reusable at two scales, which
/// is R2's entire justification for grouping being a fragment rather than a
/// `@scene` keyword.
///
/// Before this, the splice offset a fragment's `at` and never scaled its
/// `span`, so `&head for 50%` inside a `.time` score lowered to `span = 0.5`
/// against `total = 6000` — a clip lasting HALF A MILLISECOND, from a page
/// that compiled clean and reported success.
#[test]
fn a_fragments_percentages_scale_to_its_frame() {
    let js = compile_js(&format!(
        r#"{PAGE}
@form score --story {{
  &title for 50%;
  &lower-third for 50%;
}}

.stage {{
  @score &.time(60s) {{ --story for 20s; }}
}}
"#
    ));

    assert!(
        js.contains("_swSpan = 10000"),
        "50% of a 20s frame is 10s. A `0.5` here is the raw fraction leaking \
         into a millisecond domain — the clip would last half a millisecond."
    );
    assert!(
        !js.contains("_swSpan = 0.5"),
        "a relative measure must never reach the window map unscaled"
    );
}

/// THE OTHER HALF: the same fragment at a different frame must produce
/// different numbers. Without this, the test above is satisfied by a
/// hard-coded 10000 that ignores the frame entirely.
#[test]
fn the_same_fragment_scales_differently_at_a_different_frame() {
    let js = compile_js(&format!(
        r#"{PAGE}
@form score --story {{
  &title for 50%;
  &lower-third for 50%;
}}

.stage {{
  @score &.time(60s) {{ --story for 4s; }}
}}
"#
    ));

    assert!(
        js.contains("_swSpan = 2000"),
        "50% of a 4s frame is 2s — the SAME fragment, a different scale. This \
         is the portability claim (R2) stated as a number."
    );
    assert!(
        js.contains("_swOffset = 2000"),
        "the second clip follows the first inside the scaled frame"
    );
}

/// THE NEGATIVE: an ABSOLUTE measure inside a fragment must NOT scale. It
/// already names a position in the score's own domain, and rescaling it would
/// silently move a clip the author placed exactly.
#[test]
fn a_fragments_absolute_times_do_not_scale() {
    let js = compile_js(&format!(
        r#"{PAGE}
@form score --story {{
  &title at 1s for 2s;
}}

.stage {{
  @score &.time(60s) {{ --story for 20s; }}
}}
"#
    ));

    assert!(
        js.contains("_swSpan = 2000"),
        "`for 2s` is absolute: it stays 2s regardless of the frame it is \
         spliced into. Scaling it to 40000 (2s x the 20s frame) would move a \
         clip the author placed exactly."
    );
}

/// W5b/BUG-267: a BARE splice (`--story;` — no `for` frame) under an
/// own-clock `.time`/`.loop` score must scale its relative measures to the
/// PARENT'S TOTAL, not leak the raw fraction. The D2 commit promised this —
/// "a splice with no frame keeps the parent's scale, so a percentage stays a
/// fraction of the whole score" — but only the FRAMED path implemented it;
/// the bare path left `for 50%` at span = 0.5 against total = 6000, a
/// half-millisecond window.
///
/// Sampled, not grep'd (D3): `&head for 50%` in a `.loop(6s)` score is a 3s
/// window, so its map reaches the end at HALF the parent's progress. A leaked
/// 0.5 would clamp to 1.0 by parent progress 0.125 — the jump visible in
/// demos/score-launch §3.
#[test]
fn a_bare_splice_scales_relative_measures_to_the_parents_total() {
    let js = compile_js(&format!(
        r#"{PAGE}
@form score --story {{
  &title for 50%;
  &lower-third for 50%;
}}

.stage {{
  @score &.loop(6s) {{ --story; }}
}}
"#
    ));
    assert!(
        js.contains("_swSpan = 3000") && js.contains("_swTotal = 6000"),
        "BUG-267: a bare `--story;` under `.loop(6s)` keeps the parent's scale, \
         so `for 50%` is half of 6s = 3000ms. `_swSpan = 0.5` against \
         `_swTotal = 6000` is the raw fraction leaking into a millisecond domain."
    );

    // The behavioral half: SAMPLE the emitted map. Half the 6s parent means
    // the clip is half-done at parent progress 0.25 and done at 0.5.
    let samples = sample_fragment_map("&.loop(6s)", &[0.0, 0.25, 0.5, 0.75, 1.0]);
    let rounded: Vec<f64> = samples
        .iter()
        .map(|x| (x * 1000.0).round() / 1000.0)
        .collect();
    assert_eq!(
        rounded,
        vec![0.0, 0.5, 1.0, 1.0, 1.0],
        "BUG-267: a bare `for 50%` under a 6s clock must sample to half the \
         parent's span (0.5 at parent progress 0.25, done by 0.5). A raw 0.5 \
         in the window map would be a half-millisecond window that clamps to \
         1.0 almost immediately."
    );
}
