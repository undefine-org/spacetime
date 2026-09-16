//! PLAN-124 W3.5 — THE ARTICLE GATE: the launch article is the acceptance
//! test. Every runnable fence class in
//! docs/specs/SIP-001-LAUNCH-ARTICLE.st.md must compile AND emit the
//! runtime the prose promises. Asserted on emitted JS (observable compile
//! effects), never on inspect output, never on check-green alone (W0714
//! makes an unmatched directive a warning, so green ≠ matched).
//!
//! The article is copied to a temp dir and BUILT; the emitted spacetime.js
//! is the evidence.

use std::process::Command;

fn build_article_js() -> String {
    let dir = std::env::temp_dir().join(format!(
        "w35-article-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let src = std::fs::read_to_string("docs/specs/SIP-001-LAUNCH-ARTICLE.st.md")
        .expect("the launch article exists");
    let file = dir.join("index.st.md");
    std::fs::write(&file, src).expect("write article copy");
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("build")
        .arg(&file)
        .output()
        .expect("run spacetime build");
    assert!(
        out.status.success(),
        "the launch article must BUILD (its fences are promises):\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let js = std::fs::read_to_string(dir.join("spacetime.js")).expect("read spacetime.js");
    let _ = std::fs::remove_dir_all(&dir);
    js
}

/// Fence 1: `@on &.visible --rise;` — the driver registry's `intersection`
/// primitive and the form's param DEFAULT (`24px`) substituted.
#[test]
fn the_visible_splice_emits_observer_and_default() {
    let js = build_article_js();
    assert!(
        js.contains("IntersectionObserver"),
        "the visible driver's registry entry names `intersection`:\n{js}"
    );
    assert!(
        js.contains("\"24px\""),
        "the form's default substitutes into the keyframes:\n{js}"
    );
    assert!(
        !js.contains("$distance"),
        "no unsubstituted param may leak:\n{js}"
    );
}

/// Fence (footer): `@on &.visible(threshold: 0.2) --rise(distance: 8px);`
/// — statement-site call args override the defaults, and head params reach
/// the driver. (An earlier fence passed `duration:` to the SPLICE — a dead
/// arg: driver splices route form params to keyframes only, and `visible`
/// is progress-triggered with no duration. The gate caught it.)
#[test]
fn the_splice_call_args_override() {
    let js = build_article_js();
    assert!(
        js.contains("\"8px\""),
        "the call arg reaches keyframes:\n{js}"
    );
    assert!(
        js.contains("threshold: 0.2"),
        "the head's driver param reaches the observer init:\n{js}"
    );
}

/// Fence (drawer): `@on $open { true => --slide-open(280ms); false => … }`
/// — signal arms resolve at COMPILE time: the declared forms' keyframes
/// (height 0 -> 4.5rem) and the arm durations ride the dispatch table.
#[test]
fn the_signal_arms_resolve_at_compile_time() {
    let js = build_article_js();
    assert!(
        js.contains("4.5rem"),
        "the slide-open form's keyframes are inlined into the arms:\n{js}"
    );
    // Both dispatch records, exactly — a dropped or mis-lowered arm must
    // fail the gate (the earlier `contains("280")` passed on either arm).
    assert!(
        js.contains("match: \"true\""),
        "the true arm's match record reaches the dispatch table:\n{js}"
    );
    assert!(
        js.contains("match: \"false\""),
        "the false arm's match record reaches the dispatch table:\n{js}"
    );
    assert!(
        js.contains("duration: 280") && js.contains("duration: 200"),
        "both arm durations ride the dispatch table:\n{js}"
    );
    assert!(
        !js.contains("--slide-open"),
        "the form SIGIL never reaches the runtime (resolved at compile):\n{js}"
    );
}

/// Fence (drawer): `@on &.click { $open <- !$open; }` — an event driver
/// with a mutation body reaches the runtime verbatim.
#[test]
fn the_event_mutation_reaches_the_runtime() {
    let js = build_article_js();
    assert!(
        js.contains("$open <- !$open"),
        "the mutation body reaches the runtime:\n{js}"
    );
}

/// Fence (bodies): `@on &.visible(threshold: 0.5) { … stagger: 80ms; }` —
/// driver params reach the observer init; inline-body settings reach
/// apply-animations.
#[test]
fn the_driver_params_and_settings_emit() {
    let js = build_article_js();
    assert!(
        js.contains("threshold: 0.5"),
        "the driver param reaches the IntersectionObserver init:\n{js}"
    );
    assert!(
        js.contains("defaultStaggerDelay = 80"),
        "the inline-body stagger setting reaches apply-animations:\n{js}"
    );
}

/// Fence (bodies): `@on &.scroll(name: page) { }` + `scale-x: $page;` —
/// a driver-only body emits the scroll driver (no keyframes needed), and
/// the published progress is consumable as a reactive binding.
#[test]
fn the_driver_only_body_publishes_progress() {
    let js = build_article_js();
    assert!(
        js.contains("getBoundingClientRect"),
        "the scroll driver emits without any animations:\n{js}"
    );
    assert!(
        js.contains("--fill"),
        "the consumed progress publishes a custom property:\n{js}"
    );
}

/// Fence (references): `@on &hero.visible --rise;` — the subject is the
/// OBSERVED element, resolved through the `&name` rail; the scope owner is
/// animated. The subject must never be silently dropped (it was, once:
/// the driver observed the card itself).
#[test]
fn the_subject_is_observed_via_st_ref() {
    let js = build_article_js();
    assert!(
        js.contains("ST.ref(\"hero\")"),
        "the driver observes the referenced hero element:\n{js}"
    );
    // The article promises the CARD is animated: every ST.ref("hero") must
    // live in DRIVER code (an IntersectionObserver block), never in an
    // apply-animations block. (ST.ref alone would also pass if
    // apply-animations were retargeted — the exact regression.)
    for (pos, _) in js.match_indices("ST.ref(\"hero\")") {
        let block_start = js[..pos].rfind("const init = function(el)").unwrap_or(0);
        let block = &js[block_start..pos];
        assert!(
            block.contains("IntersectionObserver"),
            "ST.ref(\"hero\") outside driver code — apply-animations retargeted?:\n{}",
            &js[block_start..(pos + 40).min(js.len())]
        );
    }
}

/// BUG-250 GATE: the article's emitted JS must PARSE. Content assertions
/// (every test above) passed while the page's runtime died at load with
/// "identifier starts immediately after numeric literal" — a mixed
/// literal+signal CSS value emitted raw tokens. `node --check` when node is
/// available; the test SKIPS (with a note) when it is not.
#[test]
fn the_emitted_js_parses() {
    let dir = std::env::temp_dir().join(format!(
        "w35-syntax-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let src = std::fs::read_to_string("docs/specs/SIP-001-LAUNCH-ARTICLE.st.md")
        .expect("the launch article exists");
    let file = dir.join("index.st.md");
    std::fs::write(&file, src).expect("write article copy");
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("build")
        .arg(&file)
        .output()
        .expect("run spacetime build");
    assert!(out.status.success(), "article build failed");
    let js_path = dir.join("spacetime.js");
    match Command::new("node").arg("--check").arg(&js_path).output() {
        Ok(check) => {
            assert!(
                check.status.success(),
                "the article's emitted JS must parse (BUG-250 class):\n{}",
                String::from_utf8_lossy(&check.stderr)
            );
        }
        Err(_) => eprintln!("node unavailable — JS-syntax gate skipped"),
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// PLAN-126 (W3.6): the article's NEW sections — names/publication, signals,
/// guards — must emit their runtime, not just parse (the article is the
/// acceptance test; every fence is a promise).
#[test]
fn the_new_sections_emit_their_runtimes() {
    let js = build_article_js();
    // as $reveal publishes under the given name (never a synthetic one).
    assert!(js.contains("\"reveal\""), "the as-publication: {js}");
    // .done lowers to a derived signal from the published progress.
    assert!(js.contains("$reveal >= 1"), "the done facet lowering: {js}");
    // .change rides the change-driver with the signal name.
    assert!(js.contains("\"count\""), "the change-driver signal: {js}");
    // text-change emits the MutationObserver char choreography.
    assert!(
        js.contains("MutationObserver"),
        "the text-change runtime: {js}"
    );
    // Guards transpile to the runtime's __old/__new with match-time eval.
    assert!(
        js.contains("__new > __old") && js.contains("__new < __old"),
        "the guard arms: {js}"
    );
    // .prev in expression position lowers to the ST.prev helper, which the
    // runtime ships.
    assert!(
        js.contains("ST.prev("),
        "the prev expression lowering: {js}"
    );
    assert!(
        js.contains("_prevTrackers"),
        "the runtime prev helper: {js}"
    );
}
