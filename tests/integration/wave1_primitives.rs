//! Wave 1 integration tests: @smooth-scroll, @cursor, @magnetic, @reveal, velocity()
//!
//! Verifies that all Wave 1 primitives compile successfully and produce
//! expected JS output patterns. These primitives are defined in stdlib/primitives/
//! and wrapped by stdlib/macros/.

use spacetime::{CompileOptions, compile, parse};

/// Helper to compile spacetime code and return the compiled JS string
fn compile_js(source: &str) -> String {
    let ast = parse(source).expect("Failed to parse");
    let result = compile(&ast, CompileOptions::default());
    assert!(
        result.pipeline_errors.is_empty(),
        "Compilation produced errors: {:?}",
        result.pipeline_errors
    );
    result.js
}

// =============================================================================
// @smooth-scroll
// =============================================================================

#[test]
fn smooth_scroll_default_params() {
    let js = compile_js(
        r#"
body {
    @smooth-scroll()
}
"#,
    );
    // Should produce JS with smooth scroll infrastructure
    assert!(!js.is_empty(), "Should produce non-empty JS");
}

#[test]
fn smooth_scroll_with_custom_lerp() {
    let js = compile_js(
        r#"
body {
    @smooth-scroll(lerp: 0.06)
}
"#,
    );
    assert!(
        js.contains("0.06"),
        "Custom lerp value should appear in JS output"
    );
}

#[test]
fn smooth_scroll_with_duration() {
    let js = compile_js(
        r#"
body {
    @smooth-scroll(duration: 1.2)
}
"#,
    );
    assert!(
        js.contains("1.2"),
        "Custom duration should appear in JS output"
    );
}

// =============================================================================
// @cursor
// =============================================================================

#[test]
fn cursor_default_params() {
    let js = compile_js(
        r#"
.container {
    @cursor()
}
"#,
    );
    assert!(
        js.contains("st-cursor"),
        "Should create cursor element with st-cursor class"
    );
    assert!(
        js.contains("pointermove"),
        "Should listen for pointermove events"
    );
}

#[test]
fn cursor_custom_size_and_color() {
    let js = compile_js(
        r##"
.container {
    @cursor(size: "32px", color: "#00ff00")
}
"##,
    );
    assert!(js.contains("32px"), "Custom size should appear in JS");
    assert!(js.contains("#00ff00"), "Custom color should appear in JS");
}

#[test]
fn cursor_hover_elements_preserves_commas() {
    // This was a previously-reported bug — default "a, button, [data-cursor]"
    // must not be truncated by comma splitting
    let js = compile_js(
        r#"
.container {
    @cursor()
}
"#,
    );
    assert!(
        js.contains("a, button, [data-cursor]"),
        "Default hoverElements should preserve full comma-separated selector; got:\n{}",
        &js[..js.len().min(2000)]
    );
}

#[test]
fn cursor_custom_blend_mode() {
    let js = compile_js(
        r#"
.container {
    @cursor(blend: "exclusion")
}
"#,
    );
    assert!(
        js.contains("exclusion"),
        "Custom blend mode should appear in JS"
    );
}

// =============================================================================
// @magnetic
// =============================================================================

#[test]
fn magnetic_default_params() {
    let js = compile_js(
        r#"
.btn {
    @magnetic()
}
"#,
    );
    assert!(!js.is_empty(), "Should produce non-empty JS for @magnetic");
}

#[test]
fn magnetic_custom_strength() {
    let js = compile_js(
        r#"
.btn {
    @magnetic(strength: 0.3)
}
"#,
    );
    assert!(
        js.contains("0.3"),
        "Custom strength should appear in JS output"
    );
}

#[test]
fn magnetic_custom_radius() {
    let js = compile_js(
        r#"
.btn {
    @magnetic(radius: 200)
}
"#,
    );
    assert!(
        js.contains("200"),
        "Custom radius should appear in JS output"
    );
}

// =============================================================================
// @reveal
// =============================================================================

#[test]
fn reveal_default_type() {
    let js = compile_js(
        r#"
.title {
    @reveal()
}
"#,
    );
    assert!(!js.is_empty(), "Should produce non-empty JS for @reveal");
}

#[test]
fn reveal_words_type() {
    let js = compile_js(
        r#"
.title {
    @reveal(split: "words")
}
"#,
    );
    assert!(
        !js.is_empty(),
        "Should produce non-empty JS for @reveal with words type"
    );
}

#[test]
fn reveal_chars_type() {
    let js = compile_js(
        r#"
.title {
    @reveal(split: "chars")
}
"#,
    );
    assert!(
        !js.is_empty(),
        "Should produce non-empty JS for @reveal with chars type"
    );
}

#[test]
fn reveal_lines_type() {
    let js = compile_js(
        r#"
.title {
    @reveal(split: "lines")
}
"#,
    );
    assert!(
        !js.is_empty(),
        "Should produce non-empty JS for @reveal with lines type"
    );
}

// =============================================================================
// @scroll (existing, verify no regression)
// =============================================================================

#[test]
fn scroll_animation_compiles() {
    let js = compile_js(
        r#"
.hero {
    @on &.scroll(name: hero-reveal) {
        opacity: 0 -> 1;
        translate-y: 40px -> 0;
    }
}
"#,
    );
    assert!(
        !js.is_empty(),
        "Should produce non-empty JS for @scroll animation"
    );
}

// =============================================================================
// velocity() value function
// =============================================================================

#[test]
fn velocity_function_parses() {
    let input = r#"
.element {
    @on &.scroll(name: hero-scroll) {
        & {
            translate-y: 0px -> velocity(0.5);
        }
    }
}
"#;
    parse(input).expect("velocity() should parse successfully");
}

#[test]
fn velocity_function_compiles() {
    let js = compile_js(
        r#"
.element {
    @on &.scroll(name: hero-scroll) {
        & {
            translate-y: 0px -> velocity(0.5);
        }
    }
}
"#,
    );
    // Should contain the velocity function marker or reference
    assert!(
        js.contains("velocity") || js.contains("stVelocity"),
        "Compiled JS should reference velocity function; got:\n{}",
        &js[..js.len().min(2000)]
    );
}

#[test]
fn velocity_function_default_scale() {
    let input = r#"
.element {
    @scroll hero-scroll {
        & {
            translate-y: 0px -> velocity();
        }
    }
}
"#;
    parse(input).expect("velocity() with no args should parse (default scale=1)");
}

#[test]
fn velocity_runtime_function_in_output() {
    let js = compile_js(
        r#"
.element {
    @on &.scroll(name: hero-scroll) {
        & {
            translate-y: 0px -> velocity(2);
        }
    }
}
"#,
    );
    // The runtime value-functions.js should be bundled with stVelocity
    assert!(
        js.contains("stVelocity"),
        "Runtime should include stVelocity function; got:\n{}",
        &js[..js.len().min(3000)]
    );
}
