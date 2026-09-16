//! PLAN-126 (SIP-001 W3.6) — signal-subject facets, colon statements, `as`
//! publication, arm guards. Gates assert on OBSERVABLE emitted output
//! (never inspect output, never check-green alone — W0714 makes unmatched
//! directives warnings) and on the exact diagnostic codes the showcase
//! (docs/specs/SIP-001-W36-SYNTAX-SHOWCASE.org) promises.

use std::process::Command;

fn build_src(src: &str) -> (bool, String, String) {
    let dir = std::env::temp_dir().join(format!(
        "w36-gate-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let file = dir.join("index.st");
    std::fs::write(&file, src).expect("write fixture");
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("build")
        .arg(&file)
        .output()
        .expect("run spacetime build");
    let js = std::fs::read_to_string(dir.join("spacetime.js")).unwrap_or_default();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let _ = std::fs::remove_dir_all(&dir);
    (out.status.success(), js, stderr)
}

const RISE: &str =
    "@form motion --rise($distance = 24px) { opacity: 0 -> 1; translateY: $distance -> 0; }\n";

// ---------- happy paths: the emitted JS proves the mechanism ----------

#[test]
fn colon_statement_emits_driver_and_form() {
    let src =
        format!("{RISE}\n.card {{\n  @on &.visible(threshold: 0.2): --rise(distance: 8px);\n}}\n");
    let (ok, js, err) = build_src(&src);
    assert!(ok, "build failed: {err}");
    assert!(
        js.contains("IntersectionObserver"),
        "the visible driver: {js}"
    );
    assert!(js.contains("0.2"), "the driver param: {js}");
    assert!(js.contains("8px"), "the form arg (not the default): {js}");
}

#[test]
fn mutation_one_liner_emits_mutation_handler() {
    let src = ".menu {\n  @on &.click: $open <- !$open;\n}\n".to_string();
    let (ok, js, err) = build_src(&src);
    assert!(ok, "build failed: {err}");
    assert!(
        js.contains("open <- !$open") || js.contains("open <- !"),
        "the mutation action reaches the handler: {js}"
    );
    assert!(js.contains("click"), "the event name: {js}");
}

#[test]
fn signal_driver_body_emits_change_driver() {
    let src = ".badge {\n  @on $count.change(duration: 500ms) { font-size: 14px -> 24px; }\n}\n"
        .to_string();
    let (ok, js, err) = build_src(&src);
    assert!(ok, "build failed: {err}");
    assert!(
        js.contains("\"count\""),
        "the signal name rides the driver: {js}"
    );
    assert!(js.contains("500"), "the typed duration (ms): {js}");
    assert!(js.contains("font-size"), "the body keyframes: {js}");
}

#[test]
fn as_clause_names_the_publication() {
    let src =
        ".hero {\n  @on &.scroll(start: 0.2) as $reveal { opacity: 0 -> 1; }\n}\n".to_string();
    let (ok, js, err) = build_src(&src);
    assert!(ok, "build failed: {err}");
    assert!(
        js.contains("\"reveal\""),
        "the driver publishes progress under the as-name: {js}"
    );
    assert!(
        !js.contains("__drive_scroll"),
        "no synthetic name when `as` is given: {js}"
    );
}

#[test]
fn facet_arms_lower_done_to_a_derived_signal() {
    let src = format!(
        "{RISE}\n.hero {{\n  @on &.load(duration: 100ms) as $reveal {{ opacity: 0 -> 1; }}\n}}\n.subtitle {{\n  @on $reveal.done {{ true => --rise; }}\n}}\n"
    );
    let (ok, js, err) = build_src(&src);
    assert!(ok, "build failed: {err}");
    assert!(
        js.contains("$reveal >= 1"),
        "the done facet lowers to the derived expression: {js}"
    );
    assert!(
        js.contains("__facet_done_"),
        "the derived signal is named: {js}"
    );
    assert!(
        js.contains("match: \"true\""),
        "the true arm watches the lowered signal: {js}"
    );
}

#[test]
fn bool_sugar_desugars_to_a_true_arm() {
    // `@on $reveal.done: --rise;` must emit EXACTLY the braced shape's
    // machinery: derived-signal + one true arm (D30: one mechanism).
    let sugar = format!(
        "{RISE}\n.a {{\n  @on &.load(duration: 100ms) as $reveal {{ opacity: 0 -> 1; }}\n}}\n.x {{\n  @on $reveal.done: --rise;\n}}\n"
    );
    let (ok, js, err) = build_src(&sugar);
    assert!(ok, "build failed: {err}");
    assert!(js.contains("$reveal >= 1"), "the same lowering: {js}");
    assert!(
        js.contains("match: \"true\""),
        "the sugar IS a true arm, not a second dispatcher: {js}"
    );
}

#[test]
fn arm_guards_emit_transpiled_match_time_expressions() {
    let src = format!(
        "{RISE}\n.stock {{\n  @on $price {{\n    $old -> $new where $new > $old => --rise;\n    _ => --rise;\n  }}\n}}\n"
    );
    let (ok, js, err) = build_src(&src);
    assert!(ok, "build failed: {err}");
    assert!(
        js.contains("guard: \"__new > __old\""),
        "the guard transpiles bindings to the runtime's __old/__new: {js}"
    );
    assert!(
        js.contains("match: \"*\""),
        "a binding pattern matches any transition; the guard filters: {js}"
    );
}

#[test]
fn prev_facet_arms_emit_prev_signal() {
    let src = format!(
        "{RISE}\n.x {{\n  $price number: 100;\n  @on $price.prev {{ true => --rise; }}\n}}\n"
    );
    let (ok, js, err) = build_src(&src);
    assert!(ok, "build failed: {err}");
    assert!(
        js.contains("__facet_prev_"),
        "the prev facet lowers through prev-signal: {js}"
    );
}

#[test]
fn space_form_lowers_like_the_colon_form() {
    // PLAN-150 W3-arc: the colon-free `@on <driver> --form;` is the film
    // surface's natural spelling (`@on &.clip --crane;`). It now LOWERS
    // identically to the colon form (both emit the driver + form) rather than
    // erroring — the two are unambiguous (a trailing `;` after a form
    // application) and the golden film contract uses the colon-free spelling.
    let src = format!("{RISE}\n.card {{\n  @on &.visible(threshold: 0.2) --rise(distance: 8px);\n}}\n");
    let (ok, js, err) = build_src(&src);
    assert!(ok, "the colon-free space form must compile now: {err}");
    assert!(js.contains("IntersectionObserver"), "the visible driver: {js}");
    assert!(js.contains("8px"), "the form arg (not the default): {js}");
}

// ---------- admissibility: the exact diagnostics the showcase promises ----------

fn check_fails(src: &str, code: &str, needle: &str) {
    let dir = std::env::temp_dir().join(format!(
        "w36-err-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let file = dir.join("index.st");
    std::fs::write(&file, src).expect("write fixture");
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("check")
        .arg(&file)
        .output()
        .expect("run spacetime check");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        !out.status.success(),
        "expected {code}, but the build PASSED:\n{stdout}{stderr}"
    );
    let both = format!("{stdout}{stderr}");
    assert!(both.contains(code), "expected {code}, got:\n{both}");
    assert!(both.contains(needle), "expected `{needle}` in:\n{both}");
}

#[test]
fn elem_subject_on_a_signal_facet_is_e0949() {
    check_fails(
        ".a {\n  @on &hero.change { opacity: 0 -> 1; }\n}\n",
        "E0949",
        "signal",
    );
}

#[test]
fn signal_subject_on_an_elem_facet_is_e0949() {
    check_fails(
        ".a {\n  @on $count.visible { opacity: 0 -> 1; }\n}\n",
        "E0949",
        "elem",
    );
}

#[test]
fn value_facet_in_body_position_is_e0950() {
    check_fails(
        ".a {\n  @on $reveal.done { opacity: 0 -> 1; }\n}\n",
        "E0950",
        "bodies consume DRIVERS",
    );
}

#[test]
fn mutation_on_a_signal_subject_is_e0949() {
    check_fails(
        ".a {\n  @on $count.change: $x <- 1;\n}\n",
        "E0949",
        "DOM event listeners",
    );
}

#[test]
fn two_forms_after_the_driver_fail_loudly() {
    // Two forms after the driver is still a hard error (PLAN-150 W3-arc kept the
    // loud refusal): the colon-free statement takes exactly ONE consequence, so
    // `--a --b;` matches no grammar and fails E0946. The message changed with
    // the bare form's landing (it no longer names the missing `:`), but the
    // refusal is intact — never a silent drop.
    check_fails(
        "@form motion --a { opacity: 0 -> 1; }\n@form motion --b { opacity: 0 -> 1; }\n.a {\n  @on &.visible --a --b;\n}\n",
        "E0946",
        "does not match its declared grammar",
    );
}

#[test]
fn planned_running_refuses_loudly() {
    check_fails(
        "@form motion --x { opacity: 0 -> 1; }\n.a {\n  @on $reveal.running { true => --x; }\n}\n",
        "E0948",
        "PLANNED",
    );
}

// ---------- text-change: the two-slot body (showcase §7) ----------

const TWO_SLOT: &str = ".qty {\n  @on &.text-change(duration: 250ms, stagger: 30ms, from: \"center\") {\n    :entering { translate-y: -50% -> 0; opacity: 0 -> 1; }\n    :exiting { translate-y: 0 -> 50%; opacity: 1 -> 0; }\n  }\n}\n";

#[test]
fn two_slot_body_emits_the_text_change_runtime() {
    let (ok, js, err) = build_src(TWO_SLOT);
    assert!(ok, "build failed: {err}");
    assert!(
        js.contains("MutationObserver"),
        "the observer runtime: {js}"
    );
    assert!(js.contains("dur = 250"), "duration param: {js}");
    assert!(
        js.contains("stagger = 30"),
        "stagger -> charStagger via the registry param_map: {js}"
    );
    assert!(
        js.contains("staggerDir = 'center'"),
        "from -> staggerFrom via the registry param_map: {js}"
    );
    assert!(
        js.contains("\"-50%\"") && js.contains("\"50%\""),
        "the SLOT keyframes, not the defaults: {js}"
    );
}

#[test]
fn slots_on_a_plain_driver_are_e0950() {
    check_fails(
        ".a {\n  @on &.visible { :entering { opacity: 0 -> 1; } }\n}\n",
        "E0950",
        "entering-exiting",
    );
}

#[test]
fn plain_lines_on_text_change_are_e0950() {
    check_fails(
        ".a {\n  @on &.text-change { opacity: 0 -> 1; }\n}\n",
        "E0950",
        "ONLY",
    );
}

#[test]
fn unknown_slot_names_are_e0950() {
    check_fails(
        ".a {\n  @on &.text-change { :enterin { opacity: 0 -> 1; } }\n}\n",
        "E0950",
        "entering` and `exiting",
    );
}

// ---------- review-swarm regression gates (RevPipeline/RevMatcher/RevDeletions) ----------

#[test]
fn as_publication_emits_a_named_progress_write() {
    // Review A: `as` must publish progress under the given name (the
    // pre-fix emission wrote to "" — the gate must see the actual write).
    let src =
        ".hero {\n  @on &.scroll(start: 0.2) as $reveal { opacity: 0 -> 1; }\n}\n".to_string();
    let (ok, js, err) = build_src(&src);
    assert!(ok, "build failed: {err}");
    assert!(
        js.contains("ST.set(el, \"reveal\""),
        "progress publishes under the as-name: {js}"
    );
    assert!(
        !js.contains("ST.set(el, \"\""),
        "never the empty-name write (the pre-fix shape): {js}"
    );
}

#[test]
fn nameless_driver_publishes_under_its_synthetic_name() {
    // Review A generalized: since W3 the nameless case published "".
    let src = ".card {\n  @on &.visible { opacity: 0 -> 1; }\n}\n".to_string();
    let (ok, js, err) = build_src(&src);
    assert!(ok, "build failed: {err}");
    assert!(
        js.contains("ST.set(el, \"__drive_visible_"),
        "the synthetic name is published, so consumers can watch it: {js}"
    );
}

#[test]
fn prev_in_statement_position_is_e0950() {
    check_fails(
        "@form motion --x { opacity: 0 -> 1; }\n.a {\n  @on $price.prev: --x;\n}\n",
        "E0950",
        "expression",
    );
}

#[test]
fn done_on_an_unbound_subject_is_e0950() {
    // Review B: `.done` needs an `as`-bound progress publication in the file.
    check_fails(
        "@form motion --x { opacity: 0 -> 1; }\n.a {\n  @on $reveal.done { true => --x; }\n}\n",
        "E0950",
        "uninferrable",
    );
}

#[test]
fn done_on_an_as_bound_subject_compiles() {
    let src = "@form motion --x { opacity: 0 -> 1; }\n.a {\n  @on &.load(duration: 100ms) as $reveal { opacity: 0 -> 1; }\n}\n.b {\n  @on $reveal.done { true => --x; }\n}\n".to_string();
    let (ok, js, err) = build_src(&src);
    assert!(ok, "as-bound done must compile: {err}");
    assert!(js.contains("$reveal >= 1"), "the done lowering: {js}");
}

#[test]
fn prev_expression_lowering_emits_the_helper_not_member_access() {
    // Review MA: pure-expression facets must not fall to `.prev` member access.
    let src =
        ".stock {\n  $price number: 100;\n  color: $price > $price.prev ? #16a34a : #dc2626;\n}\n"
            .to_string();
    let (ok, js, err) = build_src(&src);
    assert!(ok, "build failed: {err}");
    assert!(
        js.contains("ST.prev(__node, 'price')"),
        "the prev helper lowering: {js}"
    );
    assert!(
        !js.contains("ST.resolve(__node, 'price').prev"),
        "never the bare member-access lowering (review MA): {js}"
    );
}

#[test]
fn entering_only_two_slot_uses_default_exit() {
    // Review H/DB: an entering-only body emits enter keyframes and NO exit
    // arg, so the runtime default fires (the retired form's behavior).
    let src = ".qty {\n  @on &.text-change(duration: 200ms) {\n    :entering { translate-y: -50% -> 0; }\n  }\n}\n".to_string();
    let (ok, js, err) = build_src(&src);
    assert!(ok, "build failed: {err}");
    assert!(js.contains("MutationObserver"), "{js}");
    assert!(js.contains("\"-50%\""), "the enter slot: {js}");
    assert!(
        !js.contains("exitKeyframes"),
        "no exit arg when the slot is absent — the default fires: {js}"
    );
}

#[test]
fn duplicate_slots_are_e0950() {
    check_fails(
        ".a {\n  @on &.text-change { :entering { opacity: 0 -> 1; } :entering { opacity: 0 -> 0.5; } }\n}\n",
        "E0950",
        "twice",
    );
}

#[test]
fn as_on_text_change_is_e0950() {
    check_fails(
        ".a {\n  @on &.text-change as $x { :entering { opacity: 0 -> 1; } }\n}\n",
        "E0950",
        "no progress to publish",
    );
}

#[test]
fn two_slot_honors_the_subject() {
    // Review E: `@on &hero.text-change` watches hero's text, not self.
    let src =
        "&hero .hero;\n.card {\n  @on &hero.text-change { :entering { opacity: 0 -> 1; } }\n}\n"
            .to_string();
    let (ok, js, err) = build_src(&src);
    assert!(ok, "build failed: {err}");
    assert!(
        js.contains("ST.ref(\"hero\")"),
        "the observer attaches to the subject: {js}"
    );
}
