//! BUG-265: a mutation body under `@on $sig.change` routes to the SIGNAL rail
//! (the change-driver), NEVER to the DOM `change` event dispatcher.
//!
//! The subject's SIGIL decides the rail — this is the anti-silent-acceptance
//! contract:
//!
//! | written              | rail                    | evidence in emit         |
//! |----------------------|-------------------------|--------------------------|
//! | `@on $sig.change {}` | signal (change-driver)  | `ST.watchChanges`, no DOM listener |
//! | `@on &.change {}`    | DOM `change` event      | `eventType = "change"` + `addEventListener(eventType, handler)` |
//! | `@on &el.change {}`  | DOM `change` event      | same                       |
//!
//! String assertions are load-bearing HERE because they assert ABSENCE (a DOM
//! listener must not exist for the signal form) — the runtime always carries
//! some `addEventListener('change')` (asset-upload helper), so the negative is
//! scoped to the site-code marker the on-mutation-handler bind emits:
//! `const eventType = "change"` + `__stTargetEl.addEventListener(eventType, handler)`.

use std::process::Command;

fn build(src: &str) -> (bool, String, String) {
    let dir = std::env::temp_dir().join(format!(
        "st_sigchange_{}_{}",
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
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
    (out.status.success(), js, err)
}

const SIGNAL_MUTATION: &str = "@data inline $src number : 0;\n@data inline $dst number : 0;\n.a {\n  @on $src.change { $dst <- $src + 1; }\n}\n";
const ELEM_MUTATION: &str = "@data inline $src number : 0;\n@data inline $dst number : 0;\n.f {\n  @on &.change { $dst <- $src + 1; }\n}\n";

/// THE NEGATIVE (signal form): the emitted site code must NOT bind a DOM
/// `change` listener — the change-driver's `ST.watchChanges` is the only
/// subscription. This is the bug: before the fix the mutation fell to the DOM
/// dispatcher and watched a DOM event that never carries the signal.
#[test]
fn signal_subject_mutation_binds_no_dom_change_listener() {
    let (ok, js, err) = build(SIGNAL_MUTATION);
    assert!(ok, "build failed: {err}");
    assert!(
        // Assert the CALL, not the emitted variable's name. This pinned
        // `ST.watchChanges(el, sig` until the emit renamed its local to
        // `targetEl` — whereupon a correct rail failed a test that could not
        // have detected the rail actually breaking.
        js.contains("ST.watchChanges("),
        "the signal rail rides the change-driver's ST.watchChanges: {err}"
    );
    assert!(
        !js.contains("const eventType = \"change\""),
        "no on-mutation-handler DOM bind may exist for the signal form. Got eventType=change: {err}"
    );
    assert!(
        !js.contains("__stTargetEl.addEventListener(eventType, handler)"),
        "no DOM listener binding for the signal form. Got it: {err}"
    );
}

/// THE POSITIVE companion: the same negative would pass vacuously if the
/// signal body never emitted AT ALL — prove the mutation reaches the rail as
/// the change-driver's `actions` arg.
#[test]
fn signal_subject_mutation_reaches_the_change_driver_as_actions() {
    let (ok, js, err) = build(SIGNAL_MUTATION);
    assert!(ok, "build failed: {err}");
    assert!(
        js.contains("$dst <- $src + 1"),
        "the mutation action text rides the change-driver: {err}"
    );
    assert!(
        js.contains("const actions = \"$dst <- $src + 1\""),
        "the change-driver carries the actions arg: {err}"
    );
}

/// The element-subject form still binds the DOM `change` event — the real one,
/// for real form elements. Don't break it while fixing the signal rail.
#[test]
fn element_subject_change_still_binds_the_dom_event() {
    let (ok, js, err) = build(ELEM_MUTATION);
    assert!(ok, "build failed: {err}");
    assert!(
        js.contains("const eventType = \"change\"") && js.contains("__stTargetEl.addEventListener(eventType, handler)"),
        "`&.change` on an element must bind the DOM change event. Got: {err}"
    );
}

/// A named element subject routes the same way (`&el.change` → DOM).
#[test]
fn named_element_subject_change_binds_the_dom_event() {
    let src = "@data inline $src number : 0;\n@data inline $dst number : 0;\n<div id=\"i\"></div>\n.f {\n  @on &i.change { $dst <- $src + 1; }\n}\n";
    let (ok, js, err) = build(src);
    assert!(ok, "build failed: {err}");
    assert!(
        js.contains("const eventType = \"change\"") && js.contains("__stTargetEl.addEventListener(eventType, handler)"),
        "`&el.change` must bind the DOM change event. Got: {err}"
    );
}

/// A MOTION body under `$sig.change` still emits the change-driver (progress
/// animation) unchanged.
#[test]
fn signal_subject_motion_body_still_emits_change_driver() {
    let src = "@data inline $count number : 0;\n.badge {\n  @on $count.change(duration: 500ms) { font-size: 14px -> 24px; }\n}\n";
    let (ok, js, err) = build(src);
    assert!(ok, "build failed: {err}");
    assert!(
        js.contains("ST.watchChanges(") && js.contains("font-size"),
        "the motion body still rides the change-driver with its keyframes: {err}"
    );
    assert!(
        !js.contains("const eventType = \"change\""),
        "a motion body binds no DOM listener either: {err}"
    );
}

/// `immediate:` and `debounce:` reach the change-driver as registry data.
#[test]
fn immediate_and_debounce_flow_through_to_the_change_driver() {
    let src = "@data inline $src number : 0;\n@data inline $dst number : 0;\n.x {\n  @on $src.change(immediate: true, debounce: 200ms) { $dst <- $src + 2; }\n}\n";
    let (ok, js, err) = build(src);
    assert!(ok, "build failed: {err}");
    assert!(
        js.contains("const runImmediate = true"),
        "immediate: true must seed at mount. Got: {err}"
    );
    assert!(
        js.contains("const debounceMs = typeof 200 === 'string'")
            || js.contains(": (200 || 0)"),
        "debounce: 200ms must reach the driver as its ms value. Got: {err}"
    );
    assert!(
        !js.contains("const eventType = \"change\""),
        "the immediate/debounce form is still the signal rail, no DOM listener: {err}"
    );
}

/// A MIXED signal-head body (mutations AND keyframes) is ambiguous — which
/// rail does the consequence ride? Refuse loudly, never silently run one half.
#[test]
fn mixed_signal_head_body_is_a_compile_error() {
    let src = "@data inline $src number : 0;\n@data inline $dst number : 0;\n.x {\n  @on $src.change { $dst <- $src + 1; opacity: 0 -> 1; }\n}\n";
    let (ok, _js, err) = build(src);
    assert!(
        !ok && err.contains("E0949"),
        "a signal-head body is a MUTATION consequence OR a MOTION body, never both. Got: {err}"
    );
    assert!(
        err.contains("MUTATION consequence") && err.contains("MOTION body"),
        "the error must name both readings, never be silent: {err}"
    );
}

/// A MOTION body on an ELEMENT-subject `.change` stays refused (no element
/// motion rail for `.change`).
#[test]
fn element_subject_change_motion_is_a_compile_error() {
    let src = "@data inline $src number : 0;\n.x {\n  @on &.change { opacity: 0 -> 1; }\n}\n";
    let (ok, _js, err) = build(src);
    assert!(
        !ok && err.contains("E0949"),
        "an element-subject `.change` motion body has no element motion rail. Got: {err}"
    );
}

/// A DOM event mutation on a NON-change element driver still binds its event —
/// the `&.click` path is the "real one" that must keep working.
#[test]
fn element_click_mutation_still_binds_its_dom_event() {
    let src = "@data inline $src number : 0;\n@data inline $dst number : 0;\n.x {\n  @on &.click { $dst <- $src + 1; }\n}\n";
    let (ok, js, err) = build(src);
    assert!(ok, "build failed: {err}");
    assert!(
        js.contains("const eventType = \"click\""),
        "the element click mutation must still bind its DOM event. Got: {err}"
    );
}

/// BUG-346 — a CSS property is not a mutable cell, and the refusal must say so.
///
/// `<-` means different things in different scopes, and both are legitimate:
///
///   .a { text <- $n; }                reactive BINDING at selector scope — holds
///   .a { @on &.click { $n <- 1; } }   signal MUTATION in an @on body — fires once
///
/// A PROPERTY inside an `@on` body is neither. `%capture_type mutation` opens on
/// a literal `"$"`, so it only ever matches a signal, and `ST.runMutations` has
/// no style-write path at all — there is nothing for it to lower to.
///
/// The refusal is right; the MESSAGE was not. It fell through every
/// `on_motion_body` alternative and surfaced as E0946 "expected capture
/// `$body:body` but no tokens remaining" — a complaint about the BODY, for a
/// mistake on one LINE, naming a capture the author never wrote and offering no
/// way forward.
#[test]
fn a_property_mutation_in_an_on_body_is_refused_by_name() {
    let src = ".box {\n  @on &.click { opacity <- 0.8; }\n}\n";
    let (ok, _js, err) = build(src);
    assert!(!ok, "a property mutation must not compile. Got: {err}");
    assert!(
        err.contains("opacity"),
        "the refusal must name the PROPERTY the author wrote, not an internal \
         capture. Got: {err}"
    );
    assert!(
        err.contains("->") || err.contains("signal"),
        "the refusal must point at what DOES work — `opacity: 0 -> 1` for a \
         property, or a signal for a value that changes. Got: {err}"
    );
    assert!(
        !err.contains("no tokens remaining"),
        "E0946's body-level message is the bug: it describes the parser's \
         position, not the author's mistake. Got: {err}"
    );
}

/// The counter-case: a SIGNAL mutation in the same position must keep working.
/// The fix narrows a diagnostic, it must not narrow the language.
#[test]
fn a_signal_mutation_in_an_on_body_still_compiles() {
    let src = "@data inline $n number : 0;\n.box {\n  @on &.click { $n <- 1; }\n}\n";
    let (ok, js, err) = build(src);
    assert!(ok, "a signal mutation must still compile. Got: {err}");
    assert!(
        js.contains("const eventType = \"click\""),
        "and must still bind its DOM listener: {err}"
    );
}

/// The same refusal must hold in a NESTED motion scope.
///
/// `on_motion_body` is RECURSIVE (`$sel:selector "{" $scope:on_motion_body "}"`),
/// so the `$bad` alternative matches inside `.child { … }` and `& { … }` too.
/// The first cut of BUG-346 handled `bad` only in the TOP-LEVEL body loop, so a
/// nested property mutation parsed, matched, and was then dropped on the floor —
/// reintroducing the exact silent-drop the fix exists to remove, one level down.
#[test]
fn a_property_mutation_in_a_nested_motion_scope_is_also_refused() {
    for src in [
        ".a {\n  @on &.scroll { .child { opacity <- 0.8; } }\n}\n",
        ".a {\n  @on &.scroll { & { opacity <- 0.8; } }\n}\n",
        // NB: two levels of nesting (`.child { .grand { … } }`) is not tested
        // here because the grammar does not support it AT ALL — the valid
        // `opacity: 0 -> 1` is equally E0946 at that depth. That is a separate
        // pre-existing limit, not this refusal's business.
    ] {
        let (ok, _js, err) = build(src);
        assert!(
            !ok && err.contains("opacity"),
            "a nested property mutation must be refused by name, not dropped. \
             Source: {src:?} Got: {err}"
        );
    }
}
