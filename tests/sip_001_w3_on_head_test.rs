//! PLAN-124 W3.2 — `@on <driver> --form;`: the driver-projection head.
//!
//! The head resolves through TWO registries as data: the driver registry
//! (member → primitive, drivers.st) and the form registry (form name →
//! declared keyframes, form.st). The `drive` pseudo-primitive in the macro's
//! `%binds` is expanded between Evaluate and Resolve (src/pipeline/drivers.rs)
//! into the entry's real primitive + apply-animations. Every failure is loud:
//! E0948 (unknown/unbound driver, undeclared form) and E0949 (kind mismatch).

use std::process::Command;

fn write_temp(source: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "w32-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let file = dir.join("index.st");
    std::fs::write(&file, source).expect("write source");
    file
}

fn run_cli(source: &str, args: &[&str]) -> (bool, String) {
    let file = write_temp(source);
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_spacetime"));
    cmd.arg(args[0]).arg(&file).args(&args[1..]);
    let out = cmd.output().expect("run spacetime CLI");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
    (out.status.success(), combined)
}

fn build_js(source: &str) -> String {
    let file = write_temp(source);
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("build")
        .arg(&file)
        .output()
        .expect("run spacetime build");
    assert!(
        out.status.success(),
        "build failed:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let js = std::fs::read_to_string(file.parent().unwrap().join("spacetime.js"))
        .expect("read spacetime.js");
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
    js
}

const RISE: &str = "@form motion --rise($distance = 24px, $duration = 700ms) {\n  opacity: 0 -> 1;\n  translate-y: $distance -> 0;\n}\n";

/// THE GATE — the launch article's first @on fence compiles AND emits the
/// registry-driven runtime: the entry's primitive (visible → intersection),
/// the resolved form's keyframes, and the form's param DEFAULT substituted
/// ($distance → 24px). Asserted on emitted JS, never on inspect output.
#[test]
fn the_article_fence_emits_the_resolved_runtime() {
    let src = format!("{RISE}\n.header {{\n  @on &.visible: --rise;\n}}\n");
    let js = build_js(&src);
    assert!(
        js.contains("IntersectionObserver"),
        "the visible driver's registry entry names `intersection` — its \
         primitive must emit:\n{js}"
    );
    assert!(
        js.contains("__drive_visible_"),
        "the driver timeline gets a synthetic name:\n{js}"
    );
    assert!(
        js.contains("\"translate-y\""),
        "the DECLARED form's keyframes drive the animation (not an inline body):\n{js}"
    );
    assert!(
        js.contains("\"24px\""),
        "the form's param DEFAULT substitutes into the keyframes:\n{js}"
    );
    assert!(
        !js.contains("$distance"),
        "no unsubstituted param may leak into the runtime:\n{js}"
    );
}

#[test]
fn unknown_driver_member_names_the_registry() {
    let src = format!("{RISE}\n.a {{\n  @on &.typo: --rise;\n}}\n");
    let (passed, out) = run_cli(&src, &["check"]);
    assert!(!passed, "an unknown driver must fail:\n{out}");
    assert!(out.contains("E0948"), "must be E0948:\n{out}");
    assert!(
        out.contains("visible") && out.contains("%registers driver"),
        "the hint must name the registered drivers and how to add one:\n{out}"
    );
}

#[test]
fn a_planned_driver_is_a_loud_refusal() {
    let src = format!("{RISE}\n.a {{\n  @on &.steps: --rise;\n}}\n");
    let (passed, out) = run_cli(&src, &["check"]);
    assert!(
        !passed,
        "a planned driver must fail, never silently no-op:\n{out}"
    );
    assert!(
        out.contains("E0948") && out.contains("PLANNED"),
        "the error must say the word is known but unbound:\n{out}"
    );
}

#[test]
fn a_non_motion_form_is_a_kind_error() {
    let src = "@form style --card-surface { background: #121722; }\n\n.a {\n  @on &.visible: --card-surface;\n}\n";
    let (passed, out) = run_cli(src, &["check"]);
    assert!(!passed, "a style form in the form slot must fail:\n{out}");
    assert!(
        out.contains("E0949") && out.contains("motion"),
        "must be the KIND error naming what the slot takes:\n{out}"
    );
}

#[test]
fn an_undeclared_form_is_an_error() {
    let src = ".a {\n  @on &.visible: --ghost;\n}\n";
    let (passed, out) = run_cli(src, &["check"]);
    assert!(!passed, "an undeclared form must fail:\n{out}");
    assert!(
        out.contains("E0948") && out.contains("--ghost"),
        "must name the form (the @on form slot is a capture, so BUG-241's \
         statement-position validation cannot see it — the drive expansion \
         owns this error):\n{out}"
    );
}

/// The OLD dispatcher surface (`@on hover lift(300ms) { … }`) coexists until
/// the W3.4 cutover: sibling forms, discriminated by the first token.
#[test]
fn the_old_surface_still_works() {
    let src = ".box {\n  @on hover lift(300ms) { translate-y: 0 -> -4px; }\n}\n";
    let js = build_js(src);
    assert!(
        js.contains("hover") && js.contains("translate-y"),
        "the old @on surface must keep emitting through the cutover:\n{js}"
    );
}

/// W3 review P2: statement-site call args are read (was silently the default
/// while form_application captured `args` and the expansion read `form_params`).
#[test]
fn statement_site_call_args_override_the_default() {
    let src = format!("{RISE}\n.a {{\n  @on &.visible: --rise(distance: 12px);\n}}\n");
    let js = build_js(&src);
    assert!(
        js.contains("\"12px\""),
        "the statement-site arg reaches the keyframes:\n{js}"
    );
    assert!(
        !js.contains("\"24px\""),
        "the default must NOT survive the override:\n{js}"
    );
}

/// A SUBJECT (`@on &hero.visible …`) is the OBSERVED element — resolved by
/// name through the `&name` rail (`ST.ref("hero")`), never a CSS descendant
/// selector. The ANIMATED element stays the scope owner (self). Before the
/// fix the subject was silently dropped: the driver observed the card
/// itself (measured).
#[test]
fn a_subject_routes_the_driver_to_the_referenced_element() {
    // `&hero .hero;` declares an ELEMENT ref (the ST.ref rail); `&hero { }`
    // would be an entity (st-world) — a different registry the subject
    // route does not read.
    let src = format!("{RISE}\n&hero .hero;\n.card {{\n  @on &hero.visible: --rise;\n}}\n");
    let js = build_js(&src);
    assert!(
        js.contains("ST.ref(\"hero\")"),
        "the driver must observe the referenced element:\n{js}"
    );
}

/// The animated element is the SCOPE OWNER even with a subject: hero's
/// visibility drives the CARD's animation (elements depend on each other).
#[test]
fn the_animation_stays_on_the_scope_owner() {
    let src = format!("{RISE}\n&hero .hero;\n.card {{\n  @on &hero.visible: --rise;\n}}\n");
    let js = build_js(&src);
    // The block that carries the keyframes is apply-animations' — it must
    // NOT reference the subject. (The earlier disjunction passed even when
    // BOTH blocks used ST.ref("hero") — the exact regression this gates.)
    assert!(
        js.contains("ST.ref(\"hero\")"),
        "the DRIVER still observes the subject:\n{js}"
    );
    // Every ST.ref("hero") must live in DRIVER code (an observer block),
    // never in an apply-animations block.
    for (pos, _) in js.match_indices("ST.ref(\"hero\")") {
        let block_start = js[..pos].rfind("const init = function(el)").unwrap_or(0);
        let block = &js[block_start..pos];
        assert!(
            block.contains("IntersectionObserver"),
            "apply-animations must target the scope owner, never the subject:\n{}",
            &js[block_start..(pos + 40).min(js.len())]
        );
    }
}
