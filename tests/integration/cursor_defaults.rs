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

fn assert_contains_all(js: &str, expected: &[&str], test_name: &str) {
    for needle in expected {
        assert!(
            js.contains(needle),
            "{test_name} expected compiled JS to contain `{needle}`; got:\n{}",
            &js[..js.len().min(3000)]
        );
    }
}

#[test]
fn cursor_primary_pointer_first_and_touch_fallback_contract_is_emitted() {
    let source = r#"
.test {
    @cursor()
}
"#;

    let js = compile_js(source);

    assert_contains_all(
        &js,
        &[
            "(pointer: fine)",
            "(hover: hover)",
            "(pointer: coarse)",
            "(hover: none)",
            "const isPrimaryDesktop = pointerFineMql.matches && hoverHoverMql.matches",
            "const isTouchFallback =",
            "maxTouchPoints",
            "msMaxTouchPoints",
            "'ontouchstart' in window",
        ],
        "cursor_primary_pointer_first_and_touch_fallback_contract_is_emitted",
    );
}

#[test]
fn cursor_default_parameters_remain_stable() {
    let js = compile_js(
        r#"
.test {
    @cursor()
}
"#,
    );

    assert!(
        js.contains("a, button, [data-cursor]"),
        "Default hoverElements should keep full selector list; got:\n{}",
        &js[..js.len().min(2000)]
    );

    assert_contains_all(
        &js,
        &[
            "width: ' + \"24px\"",
            "targetScale = isHovering ? 1.5 : 1",
            "targetScale = 0.9",
            "mix-blend-mode: ' + \"difference\"",
            "background: ' + \"#ff6b47\"",
        ],
        "cursor_default_parameters_remain_stable",
    );
}

#[test]
fn magnetic_primary_pointer_first_and_touch_fallback_contract_is_emitted() {
    let js = compile_js(
        r#"
.test {
    @magnetic()
}
"#,
    );

    assert_contains_all(
        &js,
        &[
            "(pointer: fine)",
            "(hover: hover)",
            "(pointer: coarse)",
            "(hover: none)",
            "(prefers-reduced-motion: reduce)",
            "const isPrimaryDesktop =",
            "!reducedMotionMql.matches",
            "const isTouchFallback =",
            "hasTouchCapability()",
            "addMqlListener(reducedMotionMql, onModalityChange)",
        ],
        "magnetic_primary_pointer_first_and_touch_fallback_contract_is_emitted",
    );
}

#[test]
fn magnetic_default_parameters_remain_stable() {
    let js = compile_js(
        r#"
.test {
    @magnetic()
}
"#,
    );

    assert_contains_all(
        &js,
        &[
            "const radiusPx = 100",
            "const toleranceDistance = radiusPx * 0.5",
            "const factor = 0.3 * (1 - distance / radiusPx)",
            "currentX += (targetX - currentX) * 0.1",
            "switch (\"both\")",
        ],
        "magnetic_default_parameters_remain_stable",
    );
}

#[test]
fn magnetic_parent_trigger_uses_parent_anchor_contract() {
    let js = compile_js(
        r#"
.child {
    @magnetic(trigger: "parent")
}
"#,
    );

    assert_contains_all(
        &js,
        &[
            "const triggerEl = \"parent\" === 'parent' ? el.parentElement : el;",
            "const rect = (triggerEl || el).getBoundingClientRect();",
        ],
        "magnetic_parent_trigger_uses_parent_anchor_contract",
    );
}
