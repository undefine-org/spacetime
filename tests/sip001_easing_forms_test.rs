//! SIP-001c / BUG-263 — easing curves are `@form easing --name` FORMS, not `~`
//! preset entities.
//!
//! The deletion gate (`sip001_deletion_gate_test`) proves the STRUCTURE (28
//! `@form easing --ease-*` declarations, the `~` PRESET_REF token kind gone).
//! This test proves the BEHAVIOUR at the emission boundary:
//!
//!   * a page using `--ease-out-expo` (the form sigil) compiles AND the bare
//!     name `ease-out-expo` reaches the emitted JS, where the runtime easing
//!     map (keyed BARE, D12) resolves it — never a silent fallback;
//!   * a page using the OLD spelling `~ease-out-expo` (the retired `~` preset
//!     reference) FAILS LOUDLY with the retirement diagnostic — never a silent
//!     degrade to the default easing (the arc's banned silent-acceptance class).

use std::process::Command;

fn write_temp(source: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "sip001ease-{}-{}",
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

/// Compile a page and return (success, combined stdout+stderr, emitted JS text).
fn compile(source: &str) -> (bool, String, String) {
    let file = write_temp(source);
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("build")
        .arg(&file)
        .output()
        .expect("run spacetime build");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let js = std::fs::read_to_string(file.parent().unwrap().join("spacetime.js"))
        .unwrap_or_default();
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
    (out.status.success(), combined, js)
}

/// The article's canonical easing form — referenced WITHOUT re-declaring it, so
/// it resolves from the stdlib `@form easing --ease-out-expo` curve library.
const EASE_SOURCE: &str = ".card {\n  @on &.scroll(name: r, start: 0, end: 1) {\n    opacity: 0 -> 1;\n    easing: --ease-out-expo;\n  }\n}\n";

/// The form-sigil easing reaches the emitted JS as its BARE name (D12 strips
/// `--`), where the runtime easing map resolves it.
#[test]
fn form_easing_reaches_emitted_js() {
    let (ok, out, js) = compile(EASE_SOURCE);
    assert!(ok, "a page using `--ease-out-expo` must compile:\n{out}");
    assert!(
        js.contains("defaultEasing = \"ease-out-expo\"")
            || js.contains("defaultEasing = 'ease-out-expo'"),
        "the eased name must reach the runtime easing slot stripped of `--`:\n{js}"
    );
    assert!(
        !js.contains("defaultEasing = \"~ease-out-expo\""),
        "the retired `~` sigil must never leak into the emitted JS:\n{js}"
    );
}

/// A page-declared custom easing form also resolves.
#[test]
fn custom_form_easing_reaches_emitted_js() {
    let src = "@form easing --custom-smooth { cubic-bezier(0.4, 0, 0.2, 1) }\n"
        .to_string()
        + ".card {\n  @on &.scroll(name: r, start: 0, end: 1) {\n    opacity: 0 -> 1;\n    easing: --custom-smooth;\n  }\n}\n";
    let (ok, out, js) = compile(&src);
    assert!(ok, "a custom easing form must compile:\n{out}");
    assert!(
        js.contains("defaultEasing = \"custom-smooth\"")
            || js.contains("defaultEasing = 'custom-smooth'"),
        "the custom eased name must reach the runtime easing slot:\n{js}"
    );
}

/// THE DISCRIMINATION — the retired `~` spelling must now error LOUDLY, never
/// silently degrade to the default easing.
#[test]
fn tilde_easing_errors_loudly() {
    let old = ".card {\n  @on &.scroll(name: r, start: 0, end: 1) {\n    opacity: 0 -> 1;\n    easing: ~ease-out-expo;\n  }\n}\n";
    let (ok, out, _js) = compile(old);
    assert!(!ok, "the retired `~ease-out-expo` spelling must NOT compile");
    assert!(
        out.contains("preset references are retired"),
        "the failure must be the loud retirement diagnostic, got:\n{out}"
    );
}
