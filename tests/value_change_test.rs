//! Integration tests for @value-change timeline feature
//!
//! Note: These tests use the new macro-based syntax:
//! @value-change(duration: Xms, stagger: Xms, from: "start"/"end"/"center") {
//!     :entering { ... }
//!     :exiting { ... }
//! }
//!
//! Parse tests are active. Codegen tests are ignored pending stdlib %emit pipeline
//! integration (value-change-driver primitive at stdlib/primitives/animation/drivers.st).

use spacetime::{compile, compiler::CompileOptions, parse};

// =============================================================================
// PARSE VERIFICATION TESTS (active)
// =============================================================================

#[test]
fn test_value_change_parses_basic() {
    let input = r#"
.price {
    @value-change(duration: 400ms, stagger: 50ms, from: "end") {
        :entering {
            translate-y: -100% -> 0
            opacity: 0 -> 1
        }
        :exiting {
            translate-y: 0 -> 100%
            opacity: 1 -> 0
        }
    }
}
"#;
    let ast = parse(input).unwrap();
    assert!(!ast.scopes.is_empty() || !ast.matches.is_empty());
}

#[test]
fn test_value_change_parses_stagger_direction() {
    let input = r#"
.qty {
    @value-change(duration: 250ms, stagger: 30ms, from: "end") {
        :entering {
            opacity: 0 -> 1
        }
        :exiting {
            opacity: 1 -> 0
        }
    }
}
"#;
    let ast = parse(input).unwrap();
    assert!(!ast.scopes.is_empty() || !ast.matches.is_empty());
}

#[test]
fn test_value_change_parses_multi_property() {
    let input = r#"
.counter {
    @value-change(duration: 300ms, stagger: 40ms) {
        :entering {
            opacity: 0 -> 1
            translate-y: -100% -> 0
        }
        :exiting {
            opacity: 1 -> 0
            translate-y: 0 -> 100%
        }
    }
}
"#;
    let ast = parse(input).unwrap();
    assert!(!ast.scopes.is_empty() || !ast.matches.is_empty());
}

#[test]
fn test_value_change_parses_with_easing() {
    let input = r#"
@preset easing &smooth: cubic-bezier(0.25, 0.1, 0.25, 1);

.counter {
    @value-change(duration: 300ms) {
        :entering {
            opacity: 0 -> 1
            easing: &smooth
        }
        :exiting {
            opacity: 1 -> 0
            easing: &smooth
        }
    }
}
"#;
    let ast = parse(input).unwrap();
    assert!(!ast.scopes.is_empty() || !ast.matches.is_empty());
}

// =============================================================================
// FORM MATCHING CAPTURE TESTS (active — verifies PseudoSelector extraction)
// =============================================================================

/// Helper to find a FormMatch by macro_name in the parse output.
fn find_match_recursive<'a>(
    ast: &'a spacetime::parser::ast::StFile,
    macro_name: &str,
) -> Option<&'a spacetime::syntax::FormMatch> {
    // Check top-level matches
    for m in &ast.matches {
        if m.macro_name == macro_name {
            return Some(m);
        }
    }
    // Check matches inside scopes
    for scope in &ast.scopes {
        for m in &scope.matches {
            if m.macro_name == macro_name {
                return Some(m);
            }
        }
    }
    None
}

#[test]
fn test_value_change_captures_enter_exit() {
    let input = r#"
.price {
    @value-change(duration: 400ms) {
        :entering {
            opacity: 0 -> 1
        }
        :exiting {
            opacity: 1 -> 0
        }
    }
}
"#;
    let ast = parse(input).unwrap();
    let fm = find_match_recursive(&ast, "value-change")
        .expect("Should produce a value-change FormMatch");

    assert!(
        fm.captures.contains_key("enterAnim"),
        "Should capture enterAnim from :entering block. Captures: {:?}",
        fm.captures.keys().collect::<Vec<_>>()
    );
    assert!(
        fm.captures.contains_key("exitAnim"),
        "Should capture exitAnim from :exiting block. Captures: {:?}",
        fm.captures.keys().collect::<Vec<_>>()
    );
}

#[test]
fn test_value_change_optional_pseudo_selector() {
    // Only :entering, no :exiting — should succeed since both are optional
    let input = r#"
.price {
    @value-change(duration: 400ms) {
        :entering {
            opacity: 0 -> 1
        }
    }
}
"#;
    let ast = parse(input).unwrap();
    let fm = find_match_recursive(&ast, "value-change")
        .expect("Should produce a value-change FormMatch");

    assert!(
        fm.captures.contains_key("enterAnim"),
        "Should capture enterAnim"
    );
    // exitAnim should NOT be present since :exiting block is absent
    assert!(
        !fm.captures.contains_key("exitAnim"),
        "Should not have exitAnim when :exiting is absent"
    );
}

#[test]
fn test_value_change_compile_no_missing_param() {
    let input = r#"
.price {
    @value-change(duration: 400ms) {
        :entering { opacity: 0 -> 1 }
        :exiting { opacity: 1 -> 0 }
    }
}
"#;
    let ast = parse(input).unwrap();
    let compiled = compile(&ast, CompileOptions::default());

    // Should NOT have E0804 (missing parameter) errors for enterAnim/exitAnim
    let e0804_errors: Vec<_> = compiled
        .pipeline_errors
        .iter()
        .filter(|e| {
            format!("{:?}", e).contains("MissingParameter")
                && (format!("{:?}", e).contains("enterAnim")
                    || format!("{:?}", e).contains("exitAnim"))
        })
        .collect();

    assert!(
        e0804_errors.is_empty(),
        "Should not have E0804 errors for enterAnim/exitAnim: {:?}",
        e0804_errors
    );
}

// =============================================================================
// CODEGEN TESTS
// =============================================================================

#[test]
fn test_compile_value_change_generates_runtime() {
    let input = r#"
.price {
    @value-change(duration: 400ms, stagger: 50ms, from: "end") {
        :entering {
            translate-y: -100% -> 0
            opacity: 0 -> 1
        }
        :exiting {
            translate-y: 0 -> 100%
            opacity: 1 -> 0
        }
    }
}
"#;
    let ast = parse(input).unwrap();
    let compiled = compile(&ast, CompileOptions::default());
    // Expected from value-change-driver primitive %emit block
    assert!(
        compiled.js.contains("MutationObserver"),
        "JS should contain MutationObserver from value-change-driver primitive"
    );
}

#[test]
fn test_compile_value_change_has_animations() {
    let input = r#"
.counter {
    @value-change(duration: 300ms) {
        :entering {
            translate-y: -100% -> 0
            opacity: 0 -> 1
        }
        :exiting {
            translate-y: 0 -> 100%
            opacity: 1 -> 0
        }
    }
}
"#;
    let ast = parse(input).unwrap();
    let compiled = compile(&ast, CompileOptions::default());
    assert!(
        compiled.js.contains("animate"),
        "JS should contain Web Animations API call from primitive"
    );
}

#[test]
fn test_compile_value_change_produces_js() {
    let input = r#"
.counter {
    @value-change(duration: 300ms) {
        :entering { opacity: 0 -> 1 }
        :exiting { opacity: 1 -> 0 }
    }
}
"#;
    let ast = parse(input).unwrap();
    let compiled = compile(&ast, CompileOptions::default());
    assert!(
        !compiled.js.is_empty(),
        "Compiled JS should not be empty when primitives are wired"
    );
}

#[test]
fn test_compile_value_change_with_easing_produces_js() {
    let input = r#"
@preset easing &smooth: cubic-bezier(0.25, 0.1, 0.25, 1);

.counter {
    @value-change(duration: 300ms) {
        :entering {
            opacity: 0 -> 1
            easing: &smooth
        }
        :exiting {
            opacity: 1 -> 0
            easing: &smooth
        }
    }
}
"#;
    let ast = parse(input).unwrap();
    let compiled = compile(&ast, CompileOptions::default());
    assert!(
        !compiled.js.is_empty(),
        "Compiled JS should not be empty when primitives are wired"
    );
}

// =============================================================================
// RUNTIME FORMAT VERIFICATION TESTS
// =============================================================================

#[test]
fn test_compile_value_change_contains_waapi_converter() {
    let input = r#"
.price {
    @value-change(duration: 400ms, stagger: 50ms, from: "end") {
        :entering {
            translate-y: -100% -> 0
            opacity: 0 -> 1
        }
        :exiting {
            translate-y: 0 -> 100%
            opacity: 1 -> 0
        }
    }
}
"#;
    let ast = parse(input).unwrap();
    let compiled = compile(&ast, CompileOptions::default());

    // The emitted JS must include the IR→WAAPI converter
    assert!(
        compiled.js.contains("irToWaapi"),
        "JS should contain irToWaapi converter for WAAPI keyframe format.\nJS output:\n{}",
        &compiled.js[..compiled.js.len().min(2000)]
    );

    // Should NOT pass raw IR params directly to animate()
    assert!(
        !compiled.js.contains("animate(%enterKeyframes")
            && !compiled.js.contains("animate(%exitKeyframes"),
        "JS should not pass raw IR params directly to animate()"
    );
}

// =============================================================================
// PRETEXT-BASED MEASUREMENT TESTS
// =============================================================================

#[test]
fn test_compile_value_change_uses_pretext_measurement() {
    let input = r#"
.price {
    @value-change(duration: 400ms, stagger: 50ms, from: "end") {
        :entering {
            translate-y: -100% -> 0
            opacity: 0 -> 1
        }
        :exiting {
            translate-y: 0 -> 100%
            opacity: 1 -> 0
        }
    }
}
"#;
    let ast = parse(input).unwrap();
    let compiled = compile(&ast, CompileOptions::default());
    // Superseded by the line-aware rewrite (PLAN-072): value-change-driver no
    // longer embeds a private ST.text namespace wrapping a stale inlined
    // pretext copy — it references the shared, current `pretext` global from
    // `stdlib/text` (demand-emitted via `stdlib/macros/timeline.st`'s
    // `@import "stdlib/text"`) directly.
    assert!(
        compiled.js.contains("pretext.prepareWithSegments")
            || compiled.js.contains("pretext.layoutWithLines"),
        "JS should contain pretext.prepareWithSegments or pretext.layoutWithLines for pretext-based line-layout measurement.\nJS output:\n{}",
        &compiled.js[..compiled.js.len().min(3000)]
    );
}

#[test]
fn test_compile_value_change_concurrent_height() {
    let input = r#"
.price {
    @value-change(duration: 400ms, stagger: 50ms, from: "end") {
        :entering {
            translate-y: -100% -> 0
            opacity: 0 -> 1
        }
        :exiting {
            translate-y: 0 -> 100%
            opacity: 1 -> 0
        }
    }
}
"#;
    let ast = parse(input).unwrap();
    let compiled = compile(&ast, CompileOptions::default());
    assert!(
        compiled.js.contains("targetHeight") && compiled.js.contains("totalAnimDur"),
        "JS should contain targetHeight and totalAnimDur for concurrent height animation.\nJS output:\n{}",
        &compiled.js[..compiled.js.len().min(2000)]
    );
}

#[test]
fn test_compile_value_change_no_sequential_height() {
    let input = r#"
.price {
    @value-change(duration: 400ms, stagger: 50ms, from: "end") {
        :entering {
            translate-y: -100% -> 0
            opacity: 0 -> 1
        }
        :exiting {
            translate-y: 0 -> 100%
            opacity: 1 -> 0
        }
    }
}
"#;
    let ast = parse(input).unwrap();
    let compiled = compile(&ast, CompileOptions::default());
    assert!(
        !compiled.js.contains("naturalHeight"),
        "JS should NOT contain naturalHeight — old sequential height pattern should be removed.\nJS output:\n{}",
        &compiled.js[..compiled.js.len().min(2000)]
    );
}

#[test]
fn test_compile_value_change_uses_clippath() {
    let input = r#"
.price {
    @value-change(duration: 400ms) {
        :entering { opacity: 0 -> 1 }
        :exiting { opacity: 1 -> 0 }
    }
}
"#;
    let ast = parse(input).unwrap();
    let compiled = compile(&ast, CompileOptions::default());
    assert!(
        compiled.js.contains("clipPath"),
        "JS should contain clipPath for overflow handling instead of paddingTop/marginTop.\nJS output:\n{}",
        &compiled.js[..compiled.js.len().min(2000)]
    );
}

#[test]
fn test_compile_value_change_uses_grapheme_segmenter() {
    let input = r#"
.price {
    @value-change(duration: 400ms) {
        :entering { opacity: 0 -> 1 }
        :exiting { opacity: 1 -> 0 }
    }
}
"#;
    let ast = parse(input).unwrap();
    let compiled = compile(&ast, CompileOptions::default());
    assert!(
        compiled.js.contains("Segmenter"),
        "JS should contain Intl.Segmenter for grapheme-correct character splitting.\nJS output:\n{}",
        &compiled.js[..compiled.js.len().min(2000)]
    );
}

#[test]
fn test_compile_value_change_no_font_kerning_hack() {
    let input = r#"
.price {
    @value-change(duration: 400ms) {
        :entering { opacity: 0 -> 1 }
        :exiting { opacity: 1 -> 0 }
    }
}
"#;
    let ast = parse(input).unwrap();
    let compiled = compile(&ast, CompileOptions::default());
    // fontKerning may appear in comments but should not be set as a style property
    assert!(
        !compiled.js.contains(".fontKerning"),
        "JS should NOT contain .fontKerning style manipulation — hack removed.\nJS output:\n{}",
        &compiled.js[..compiled.js.len().min(3000)]
    );
}

#[test]
fn test_compile_value_change_has_reduced_motion() {
    let input = r#"
.price {
    @value-change(duration: 400ms) {
        :entering { opacity: 0 -> 1 }
        :exiting { opacity: 1 -> 0 }
    }
}
"#;
    let ast = parse(input).unwrap();
    let compiled = compile(&ast, CompileOptions::default());
    assert!(
        compiled.js.contains("prefers-reduced-motion"),
        "JS should contain prefers-reduced-motion check for accessibility.\nJS output:\n{}",
        &compiled.js[..compiled.js.len().min(3000)]
    );
}

#[test]
fn test_compile_value_change_no_box_model_mutation() {
    let input = r#"
.price {
    @value-change(duration: 400ms) {
        :entering { opacity: 0 -> 1 }
        :exiting { opacity: 1 -> 0 }
    }
}
"#;
    let ast = parse(input).unwrap();
    let compiled = compile(&ast, CompileOptions::default());
    assert!(
        !compiled.js.contains(".paddingTop"),
        "JS should NOT contain .paddingTop — box-model overflow hack removed.\nJS output:\n{}",
        &compiled.js[..compiled.js.len().min(3000)]
    );
    assert!(
        !compiled.js.contains(".marginTop"),
        "JS should NOT contain .marginTop — box-model overflow hack removed.\nJS output:\n{}",
        &compiled.js[..compiled.js.len().min(3000)]
    );
}
