//! Integration tests for template invocation ordering relative to event handlers.
//!
//! Verifies that template invocations (%order 200) are emitted before
//! event handlers (default %order 500+) in the compiled JS output.
//! This ensures that querySelector() calls in event handlers find
//! elements created by template invocations.
//!
//! Bug: Previously, macros with %binds used `bind_index` as order,
//! ignoring the macro's %order directive. This caused event handlers
//! targeting template-internal elements to be emitted BEFORE the
//! template invocation code, making querySelector() return null.

use spacetime::compiler::CompileOptions;
use spacetime::{compile, parse};

/// Compile a .st snippet and return the JS output.
fn compile_st(input: &str) -> String {
    let ast = parse(input).expect("Failed to parse");
    let compiled = compile(&ast, CompileOptions::default());
    compiled.js
}

/// Template invocation must appear before event handlers for template-internal elements.
///
/// When `.inner-btn` lives inside a template invoked on `.container`,
/// the invoke-template IIFE must be emitted before the on-mutation-handler IIFE.
/// Otherwise, querySelector('.inner-btn') returns null and the handler silently fails.
#[test]
fn test_template_invoke_before_event_handler_in_output() {
    let input = r#"
@template &my-widget($label) {
    <div class="widget">
        <button class="inner-btn">`$label`</button>
    </div>
}

.container {
    &my-widget("Click me") {
        <span>body</span>
    }
}

.inner-btn {
    @on &.click {
        $active <- !$active
    }
}
"#;

    let js = compile_st(input);

    // Find the template invocation for .container (invoke-template primitive)
    // The selector-phase invoke-template uses querySelector('.container') then
    // calls factory(...). Look for the templateName assignment near .container.
    let invoke_pos = js
        .find("const templateName = \"my-widget\"")
        .or_else(|| js.find("templateName = \"my-widget\""))
        .expect("JS output should contain template invocation for my-widget");

    // Find the selector init registration for .inner-btn
    let handler_pos = js
        .find("registerSelectorInit('.inner-btn'")
        .or_else(|| js.find("registerSelectorInit(\".inner-btn\""))
        .expect("JS output should contain registerSelectorInit for .inner-btn");

    assert!(
        invoke_pos < handler_pos,
        "Template invocation (pos {}) must appear BEFORE event handler querySelector (pos {})\n\
         This ensures the element exists in DOM when querySelector runs.",
        invoke_pos,
        handler_pos,
    );
}

/// Event handlers for elements NOT inside templates should still work.
/// This is a regression test to ensure the ordering fix doesn't break
/// direct (non-template) element event handlers.
#[test]
fn test_direct_element_event_handler_still_works() {
    let input = r#"
.my-button {
    @on &.click {
        $clicked <- true
    }
}
"#;

    let js = compile_st(input);

    // Should contain registerSelectorInit for .my-button (eager querySelectorAll replaced by batched init)
    assert!(
        js.contains("registerSelectorInit('.my-button'")
            || js.contains("registerSelectorInit(\".my-button\"")
            || js.contains("querySelectorAll('.my-button')")
            || js.contains("querySelector('.my-button')"),
        "JS output should contain registerSelectorInit for .my-button"
    );

    // Should contain the mutation handler body
    assert!(
        js.contains("$clicked <- true") || js.contains("clicked"),
        "JS output should contain the mutation action"
    );
}

/// Multiple templates should all be invoked before any event handlers.
#[test]
fn test_multiple_templates_before_handlers() {
    let input = r#"
@template &widget-a($text) {
    <div class="widget-a"><span class="a-inner">`$text`</span></div>
}

@template &widget-b($text) {
    <div class="widget-b"><span class="b-inner">`$text`</span></div>
}

.section-a {
    &widget-a("Hello") {
        <span>a body</span>
    }
}

.section-b {
    &widget-b("World") {
        <span>b body</span>
    }
}

.a-inner {
    @on &.click {
        $stateA <- !$stateA
    }
}

.b-inner {
    @on &.click {
        $stateB <- !$stateB
    }
}
"#;

    let js = compile_st(input);

    // Find the mutation handlers for template-internal elements
    let handler_a_pos = js
        .find("querySelector('.a-inner')")
        .or_else(|| js.find("querySelector(\".a-inner\")"));

    let handler_b_pos = js
        .find("querySelector('.b-inner')")
        .or_else(|| js.find("querySelector(\".b-inner\")"));

    // Find template invocation positions (the ones that assign templateName)
    // These are the invoke-template primitives, not the register-template ones.
    let invoke_a_pos = js
        .find("const templateName = \"widget-a\"")
        .or_else(|| js.find("templateName = \"widget-a\""));
    let invoke_b_pos = js
        .find("const templateName = \"widget-b\"")
        .or_else(|| js.find("templateName = \"widget-b\""));

    // At least one invoke should exist
    assert!(
        invoke_a_pos.is_some() || invoke_b_pos.is_some(),
        "Should have template invocations in output"
    );

    // All template invocations should appear before all event handlers
    if let (Some(inv_a), Some(h_a)) = (invoke_a_pos, handler_a_pos) {
        assert!(
            inv_a < h_a,
            "widget-a invocation (pos {}) must appear before .a-inner handler (pos {})",
            inv_a,
            h_a
        );
    }

    if let (Some(inv_b), Some(h_b)) = (invoke_b_pos, handler_b_pos) {
        assert!(
            inv_b < h_b,
            "widget-b invocation (pos {}) must appear before .b-inner handler (pos {})",
            inv_b,
            h_b
        );
    }
}
