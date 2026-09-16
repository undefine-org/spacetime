//! BUG-297 — a custom `@form easing --name { curve }` declaration ships its
//! CURVE to the runtime, and an unknown easing form is a hard error.
//!
//! Before this fix the form's body went nowhere: the emitted JS carried the
//! bare name (`defaultEasing = "custom-smooth"`), the runtime easing map had
//! no such entry, and `getEasing` silently fell back to `easings.linear` —
//! the author declared a curve and the page shipped a different one with
//! nothing complaining (the arc's banned silent-acceptance class; the old
//! gate only asserted the NAME reached JS — BUG-252's exact failure mode).
//!
//! The fix: easing form declarations in the merged AST (page + imports +
//! `_prelude.st`) emit a runtime REGISTRATION (`ST.registerEasing`) beside
//! the bundle, and a `--name` no easing form declares is E0962 at the D12
//! strip boundary. Behavior proof lives in
//! tests/easing/custom-form-curve.test.st (virtual-clock discrimination:
//! cubic-bezier(0.4, 0, 0.2, 1) at t=0.5 is NOT the linear 0.5).

use std::process::Command;

fn write_temp(files: &[(&str, &str)]) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "bug297-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    for (name, source) in files {
        std::fs::write(dir.join(name), source).expect("write source");
    }
    dir
}

/// Compile a page and return (success, combined stdout+stderr, emitted JS text).
fn compile(files: &[(&str, &str)], entry: &str) -> (bool, String, String) {
    let dir = write_temp(files);
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("build")
        .arg(dir.join(entry))
        .output()
        .expect("run spacetime build");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let js = std::fs::read_to_string(dir.join("spacetime.js")).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    (out.status.success(), combined, js)
}

const PAGE: &str = "@form easing --custom-smooth { cubic-bezier(0.4, 0, 0.2, 1) }\n\
                    .card {\n  @on &.scroll(name: r, start: 0, end: 1) {\n    opacity: 0 -> 1;\n    easing: --custom-smooth;\n  }\n}\n";

/// THE BUG: the declared curve must reach the runtime as a REGISTRATION, so
/// `ST.getEasing("custom-smooth")` resolves the author's curve — never the
/// silent linear fallback.
#[test]
fn a_custom_easing_curve_registers_with_the_runtime() {
    let (ok, out, js) = compile(&[("index.st", PAGE)], "index.st");
    assert!(ok, "a custom easing form must compile:\n{out}");
    assert!(
        js.contains("registerEasing(\"custom-smooth\""),
        "the emitted JS must REGISTER the custom curve:\n{js}"
    );
    assert!(
        js.contains("cubicBezier(0.4, 0, 0.2, 1)"),
        "the registration must carry the DECLARED curve, not a substitute:\n{js}"
    );
}

/// A spring body registers through the spring factory with its declared
/// parameters.
#[test]
fn a_spring_easing_registers_with_its_parameters() {
    let src = "@form easing --my-spring { spring(300, 15, 1) }\n\
               .card {\n  @on &.scroll(name: r, start: 0, end: 1) {\n    opacity: 0 -> 1;\n    easing: --my-spring;\n  }\n}\n";
    let (ok, out, js) = compile(&[("index.st", src)], "index.st");
    assert!(ok, "a spring easing form must compile:\n{out}");
    assert!(
        js.contains("registerEasing(\"my-spring\"")
            && js.contains("springCurve(300, 15, 1)"),
        "the spring's declared parameters must reach the runtime:\n{js}"
    );
}

/// An easing declared in an IMPORTED file registers too (the merged AST is
/// the declaration set).
#[test]
fn an_imported_easing_registers() {
    let brand = "@form easing --brand-smooth { cubic-bezier(0.19, 1, 0.22, 1) }\n";
    let page = "@import \"./brand.st\"\n\
                .card {\n  @on &.scroll(name: r, start: 0, end: 1) {\n    opacity: 0 -> 1;\n    easing: --brand-smooth;\n  }\n}\n";
    let (ok, out, js) = compile(&[("index.st", page), ("brand.st", brand)], "index.st");
    assert!(ok, "an imported easing form must compile:\n{out}");
    assert!(
        js.contains("registerEasing(\"brand-smooth\""),
        "the imported form's curve must register:\n{js}"
    );
}

/// An easing form name NOTHING declares is a hard E0962 — never a silent
/// linear fallback at runtime.
#[test]
fn an_unknown_easing_form_is_a_hard_error() {
    let src = ".card {\n  @on &.scroll(name: r, start: 0, end: 1) {\n    opacity: 0 -> 1;\n    easing: --custm-smooth;\n  }\n}\n";
    let (ok, out, _js) = compile(&[("index.st", src)], "index.st");
    assert!(!ok, "an unknown easing form must FAIL build (E0962)");
    assert!(out.contains("E0962"), "the error must be E0962:\n{out}");
    assert!(
        out.contains("--custm-smooth"),
        "the error must name the unknown form:\n{out}"
    );
}

/// REGRESSION PINS: the stdlib curve library (`--ease-out-expo`), CSS
/// keyword curves, and inline function easings all stay valid.
#[test]
fn stdlib_keywords_and_inline_easings_stay_valid() {
    let src = ".a {\n  @on &.scroll(name: r, start: 0, end: 1) {\n    opacity: 0 -> 1;\n    easing: --ease-out-expo;\n  }\n}\n\
               .b {\n  @on &.scroll(name: s, start: 0, end: 1) {\n    opacity: 0 -> 1;\n    easing: ease-in-out;\n  }\n}\n\
               .c {\n  @on &.scroll(name: t, start: 0, end: 1) {\n    opacity: 0 -> 1;\n    easing: cubic-bezier(0.4, 0, 0.2, 1);\n  }\n}\n";
    let (ok, out, _js) = compile(&[("index.st", src)], "index.st");
    assert!(
        ok,
        "stdlib forms, CSS keywords, and inline cubic-bezier must all compile:\n{out}"
    );
}
