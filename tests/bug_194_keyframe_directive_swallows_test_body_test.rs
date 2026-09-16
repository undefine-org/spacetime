//! BUG-194 regression: an animation-driver directive whose body is a
//! KEYFRAME list (`$body:keyframes` capture -- @scroll, @mouse, @load, @on
//! hover/click/visible/focus with a `prop: a -> b;` body) placed directly as
//! a step inside a `@test { ... }` body compiles the ENTIRE ENCLOSING test to
//! an EMPTY async function -- no error, no warning, every step after (and
//! including) the animation directive is silently dropped from the compiled
//! output. `window.__spacetime_register_test(name, async () => {})` is
//! registered with a completely empty body, so the test ALWAYS reports a
//! green PASS regardless of what assertions were written.
//!
//! Verified via the compiler library directly (not the runtime), because the
//! bug itself would make a runtime-level regression test for this ALSO
//! falsely pass (any `@assert` placed after the triggering directive never
//! runs either) -- this must be pinned at the emitted-JS level.
//!
//! Confirmed this session that the SAME shape already exists, unnoticed,
//! throughout the real test suite -- e.g. every test in
//! `tests/unit/animations/mouse-tilt.test.st` (`@given .parent { @mouse
//! driver(...) { ... } }` followed by assertions) compiles to an empty body
//! and has been silently reporting false-green passes.
//!
//! CONTRAST: the mutation-form `@on <event> name { $sig <- value; }` (no
//! keyframes, a `$body:block` capture) does NOT trigger this and compiles
//! correctly -- isolating the defect to the keyframes-capture path
//! specifically, not animation directives in general.

use spacetime::compiler::Compiler;

/// Compile inline `.st` source the same way `Compiler::from_file` would, by
/// writing it to a temp file first (the compiler's public API is file-based).
fn compile_source(source: &str) -> String {
    let temp = tempfile::tempdir().expect("tempdir");
    let file = temp.path().join("probe.st");
    std::fs::write(&file, source).expect("write probe file");
    let compiled = Compiler::from_file(&file, temp.path())
        .expect("compile should succeed (no parse errors)")
        .compile();
    // E0909 (`<-` outside a @template component body) is downgraded to a
    // compile WARNING by the real test runner (the headless harness reports
    // it under "Compile warnings" and still runs the tests), so tolerate it
    // here too -- the control case below intentionally uses a `<-` mutation
    // to pin the non-keyframe contrast path. Any OTHER pipeline error means
    // the probe source itself is broken and the test result is meaningless.
    let hard_errors: Vec<_> = compiled
        .pipeline_errors
        .iter()
        .filter(|e| e.code != "E0909")
        .collect();
    assert!(
        hard_errors.is_empty(),
        "probe source must compile without pipeline errors (E0909 tolerated, \
         matches test-runner behaviour): {:?}",
        hard_errors
    );
    compiled.js.clone()
}

/// Extract the body of a registered test by name from the compiled JS. Looks
/// for `window.__spacetime_register_test("<name>", async () => {` and returns
/// the text up to the FIRST occurrence of the isolation-cleanup comment that
/// every compiled test body contains (a stable, distinctive anchor emitted by
/// the `@test` macro's own `%emit js` template) -- so we can tell a genuinely
/// empty body apart from one that has real statements before that point.
fn extract_test_body_prefix(js: &str, test_name: &str) -> String {
    let marker = format!("window.__spacetime_register_test(\"{test_name}\"");
    let start = js
        .find(&marker)
        .unwrap_or_else(|| panic!("test '{test_name}' not found in compiled JS"));
    let after_start = &js[start..];
    // The body runs from the opening `try {` to the `} finally {` that every
    // compiled @test wraps its steps in (from stdlib/testing/test.st's own
    // %emit js template for @test) -- slice between those two anchors.
    let try_pos = after_start
        .find("try {")
        .unwrap_or_else(|| panic!("'try {{' not found for test '{test_name}'"));
    let finally_pos = after_start[try_pos..]
        .find("} finally {")
        .unwrap_or_else(|| panic!("'}} finally {{' not found for test '{test_name}'"));
    after_start[try_pos + "try {".len()..try_pos + finally_pos].to_string()
}

#[test]
fn bare_scroll_keyframe_directive_does_not_swallow_the_test_body() {
    let js = compile_source(
        r#"@import "stdlib/testing/test"

@test "probe scroll" {
    @eval (window.__probeMarker = 1)
    @scroll probe-driver(start: 0, end: 1) {
        opacity: 0 -> 1;
    }
    @eval (window.__probeMarker = 2)
}
"#,
    );

    let body = extract_test_body_prefix(&js, "probe scroll");

    assert!(
        body.contains("__probeMarker"),
        "BUG-194: a @scroll keyframe directive inside @test must NOT swallow \
         the surrounding test body -- the compiled body between try{{}} and \
         finally{{}} is missing its @eval steps entirely.\n\nCompiled body was:\n{body}"
    );
}

#[test]
fn bare_on_hover_keyframe_directive_does_not_swallow_the_test_body() {
    let js = compile_source(
        r#"@import "stdlib/testing/test"

@test "probe hover" {
    @eval (window.__probeMarker2 = 1)
    @on hover probe-hover-driver(100ms) {
        opacity: 0 -> 1;
    }
    @eval (window.__probeMarker2 = 2)
}
"#,
    );

    let body = extract_test_body_prefix(&js, "probe hover");

    assert!(
        body.contains("__probeMarker2"),
        "BUG-194: an @on hover keyframe directive inside @test must NOT \
         swallow the surrounding test body.\n\nCompiled body was:\n{body}"
    );
}

#[test]
fn control_mutation_form_on_click_does_not_swallow_the_test_body() {
    // Control: the MUTATION form ($body:block, no keyframes) must keep
    // working -- this pins that the fix (once applied) does not regress the
    // already-correct path, and that this control ITSELF is not also broken
    // today (so a naive "both always pass" fix wouldn't hide behind it).
    let js = compile_source(
        r#"@import "stdlib/testing/test"

@test "probe mutation control" {
    @eval (window.__probeMarker3 = 1)
    @on click probe-mutation-driver {
        $probeMutationSignal <- true;
    }
    @eval (window.__probeMarker3 = 2)
}
"#,
    );

    let body = extract_test_body_prefix(&js, "probe mutation control");

    assert!(
        body.contains("__probeMarker3"),
        "control: a mutation-form @on click directive must not swallow the \
         test body either.\n\nCompiled body was:\n{body}"
    );
}

#[test]
fn given_plus_mouse_keyframe_directive_matches_real_mouse_tilt_suite_shape() {
    // Mirrors tests/unit/animations/mouse-tilt.test.st's actual, real,
    // currently-green shape -- @fixture + @given + @mouse keyframes,
    // confirmed this session to compile to an empty body in that real file.
    let js = compile_source(
        r#"@import "stdlib/testing/test"

@test "probe given mouse" {
    @fixture {
        <div class="probe-given-target" style="position:fixed;top:0;left:0;width:200px;height:200px;"></div>
    }
    @given .probe-given-target {
        @mouse probe-given-driver(axis: "x") {
            translate-x: 0 -> 100px;
        }
    }
    @eval (window.__probeMarker4 = 42)
}
"#,
    );

    let body = extract_test_body_prefix(&js, "probe given mouse");

    assert!(
        body.contains("__probeMarker4"),
        "BUG-194: the exact @fixture + @given + @mouse-keyframes shape used \
         throughout tests/unit/animations/mouse-tilt.test.st must not swallow \
         the surrounding test body.\n\nCompiled body was:\n{body}"
    );
}
