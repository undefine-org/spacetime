//! PLAN-124 W3.3b — `@on <driver> { … }`: inline choreography bodies.
//!
//! A body mixes: splices of declared motion forms (`--rise(distance: 12px)`),
//! keyframe lines (`translate-x: 0 -> 4px;`), settings (`stagger: 70ms;` —
//! routed to apply-animations args via the primitive's own signature), static
//! properties (`border-left-color: #8ab4ff;`), and mutations
//! (`$open <- !$open;` — routed to on-mutation-handler, EVENT drivers only).

use std::process::Command;

fn write_temp(source: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "w33b-{}-{}",
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

const RISE: &str = "@form motion --rise($distance = 24px, $duration = 700ms) {\n  opacity: 0 -> 1;\n  translate-y: $distance -> 0;\n  easing: --ease-out-expo;\n}\n";

/// THE GATE — the article's `.header` body fence: a splice WITH call-arg
/// overrides plus a setting, driven by the visible primitive.
#[test]
fn the_header_body_fence_emits_choreography() {
    let src = format!(
        "{RISE}\n.header {{\n  @on &.visible {{\n    --rise(distance: 12px, duration: 420ms);\n    stagger: 70ms;\n  }}\n}}\n"
    );
    let js = build_js(&src);
    assert!(
        js.contains("IntersectionObserver"),
        "visible → intersection:\n{js}"
    );
    assert!(
        js.contains("\"12px\""),
        "the NAMED call arg overrides the form's declared default (24px):\n{js}"
    );
    assert!(
        !js.contains("\"24px\""),
        "the default must NOT survive the override:\n{js}"
    );
    assert!(
        js.contains("defaultStaggerDelay = 70"),
        "the setting reaches the runtime as the NUMBER its signature declares \
         (the earlier string `\"70ms\"` computed NaN — found by the W3 review):\n{js}"
    );
    assert!(
        !js.contains("defaultStaggerDelay = \"70ms\"")
            && !js.contains("defaultStaggerDelay = '70ms'"),
        "never as a string:\n{js}"
    );
}

/// The hover body fence: a static property and a keyframe line.
#[test]
fn the_hover_body_fence_emits_static_and_keyframes() {
    let src = ".hoverable {\n  @on &.hover {\n    border-left-color: #8ab4ff;\n    translate-x: 0 -> 4px;\n  }\n}\n";
    let js = build_js(&src);
    assert!(
        js.contains("#8ab4ff"),
        "the no-arrow line is a static property:\n{js}"
    );
    assert!(
        js.contains("translate-x") && js.contains("4px"),
        "the keyframe line animates:\n{js}"
    );
}

/// The article's `.toggle` fence: a mutation body on an event driver routes
/// to the mutation handler, NOT the animation path.
#[test]
fn a_mutation_body_routes_to_the_mutation_handler() {
    let src = ".toggle {\n  $open bool: false;\n  @on &.click { $open <- !$open; }\n}\n";
    let js = build_js(&src);
    assert!(
        js.contains("$open <- !$open"),
        "the mutation reaches the runtime verbatim:\n{js}"
    );
}

/// A mutation on a PROGRESS driver is a kind error: progress drives, it
/// cannot fire.
#[test]
fn a_mutation_on_a_progress_driver_is_a_kind_error() {
    let src = ".a {\n  $x number: 0;\n  @on &.scroll { $x <- $x + 1; }\n}\n";
    let (passed, out) = run_cli(src, &["check"]);
    assert!(!passed, "a mutation on a progress driver must fail:\n{out}");
    assert!(
        out.contains("E0949") && out.contains("progress"),
        "must name the driver value type:\n{out}"
    );
}

/// A subject with an inline body: the driver observes the referenced
/// element, the body's keyframes play on the scope owner.
#[test]
fn a_subject_with_an_inline_body_observes_the_reference() {
    let src = "&sidebar .sidebar;\n.list {\n  @on &sidebar.scroll { translate-y: 0 -> 20px; }\n}\n";
    let js = build_js(src);
    assert!(
        js.contains("ST.ref(\"sidebar\")"),
        "the scroll driver must attach to the sidebar:\n{js}"
    );
    assert!(
        js.contains("\"translate-y\""),
        "the inline body's keyframes still emit:\n{js}"
    );
}

/// A DRIVER-ONLY body (`@on &.scroll(name: page) { }`) is legal and emits
/// the driver bind with NO apply-animations — the progress is consumable as
/// a binding (the old surface compiled empty bodies the same way).
#[test]
fn an_empty_body_emits_the_driver_only() {
    let src = ".fill {\n  @on &.scroll(name: page) { }\n  scale-x: $page;\n}\n";
    let js = build_js(src);
    assert!(
        js.contains("getBoundingClientRect"),
        "the scroll driver itself must emit:\n{js}"
    );
    assert!(
        js.contains("\"page\""),
        "the named driver publishes its progress under the name:\n{js}"
    );
}
