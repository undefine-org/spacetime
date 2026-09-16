//! Playwright-based integration tests for .test.st files.
//!
//! Runs every .test.st file in a real Chromium browser via Playwright,
//! giving access to real DOM, CSS, rAF, IntersectionObserver, etc.
//!
//! Usage:
//!   cargo test --test playwright_st_test
//!
//! Each #[test] compiles one .test.st file to HTML, runs it in a warm Chromium
//! tab over CDP (PLAN-027 W2), and asserts all JS-level tests pass.
//!
//! Requires `--features cdp`:  cargo test --features cdp --test playwright_st_test
#![cfg(feature = "cdp")]

mod playwright_st;
use playwright_st::run_st_tests;

// =============================================================================
// Unit — assertions
// =============================================================================

#[test]
fn unit_assertions_binding() {
    run_st_tests(&["tests/unit/assertions/binding.test.st"]);
}

#[test]
fn unit_assertions_state() {
    run_st_tests(&["tests/unit/assertions/state.test.st"]);
}

#[test]
fn unit_assertions_timeline() {
    run_st_tests(&["tests/unit/assertions/timeline.test.st"]);
}

// =============================================================================
// Unit — animations
// =============================================================================

#[test]
fn unit_animations_value_change() {
    run_st_tests(&["tests/unit/animations/value-change.test.st"]);
}

#[test]
fn unit_animations_on_visible_mutation() {
    run_st_tests(&["tests/unit/animations/on-visible-mutation.test.st"]);
}

/// BUG-265: a mutation body under `@on $sig.change` is the SIGNAL rail
/// (change-driver), never the DOM `change` event; `&.change` on an element is
/// the DOM event; immediate:/debounce: seed/coalesce on the change-driver.
#[test]
fn unit_animations_on_sig_change_mutation() {
    run_st_tests(&["tests/unit/animations/on-sig-change-mutation.test.st"]);
}

// =============================================================================
// Unit — macros
// =============================================================================

#[test]
fn unit_macros_cycle() {
    run_st_tests(&["tests/unit/macros/cycle.test.st"]);
}

#[test]
fn unit_macros_edit() {
    run_st_tests(&["tests/unit/macros/edit.test.st"]);
}

#[test]
fn unit_macros_load_delay() {
    run_st_tests(&["tests/unit/macros/load-delay.test.st"]);
}

#[test]
fn unit_macros_loop() {
    run_st_tests(&["tests/unit/macros/loop.test.st"]);
}

#[test]
fn unit_macros_on_click() {
    run_st_tests(&["tests/unit/macros/on-click.test.st"]);
}

#[test]
fn unit_macros_on_hover() {
    run_st_tests(&["tests/unit/macros/on-hover.test.st"]);
}

#[test]
fn unit_macros_on_load() {
    run_st_tests(&["tests/unit/macros/on-load.test.st"]);
}

#[test]
fn unit_macros_on_visible_immediate() {
    run_st_tests(&["tests/unit/macros/on-visible-immediate.test.st"]);
}

#[test]
fn unit_macros_state_machine() {
    run_st_tests(&["tests/unit/macros/state-machine.test.st"]);
}

// =============================================================================
// Unit — primitives
// =============================================================================

#[test]
fn unit_primitives_scroll_driver() {
    run_st_tests(&["tests/unit/primitives/scroll-driver.test.st"]);
}

#[test]
fn unit_primitives_scroll_scope() {
    run_st_tests(&["tests/unit/primitives/scroll-scope.test.st"]);
}

// =============================================================================
// Unit — runtime
// =============================================================================

#[test]
fn unit_runtime_color() {
    run_st_tests(&["tests/unit/runtime/color.test.st"]);
}

#[test]
fn unit_runtime_core() {
    run_st_tests(&["tests/unit/runtime/core.test.st"]);
}

#[test]
fn unit_runtime_easing() {
    run_st_tests(&["tests/unit/runtime/easing.test.st"]);
}

#[test]
fn unit_runtime_interpolate() {
    run_st_tests(&["tests/unit/runtime/interpolate.test.st"]);
}

#[test]
fn unit_runtime_stagger() {
    run_st_tests(&["tests/unit/runtime/stagger.test.st"]);
}

#[test]
fn unit_runtime_timeline() {
    run_st_tests(&["tests/unit/runtime/timeline.test.st"]);
}

#[test]
fn unit_runtime_value_functions() {
    run_st_tests(&["tests/unit/runtime/value-functions.test.st"]);
}

// NOTE: stdlib/text/tests/*.test.st are NOT registered here yet — .test.st
// assertions are vacuous until BUG-051 / PLAN-026 lands (the :block body capture
// emits the test body as a dead string literal). The pretext fusion is verified
// meanwhile by tests/text_module_test.rs at the emit level. Re-add these once
// BUG-051 is fixed:
//   unit_text_measure        -> stdlib/text/tests/measure.test.st
//   unit_text_reveal_lines   -> stdlib/text/tests/reveal-lines.test.st

// =============================================================================
// Integration
// =============================================================================

#[test]
fn integration_animation_chain() {
    run_st_tests(&["tests/integration/animation-chain.test.st"]);
}

#[test]
fn integration_apply_animations() {
    run_st_tests(&["tests/integration/apply-animations.test.st"]);
}

#[test]
fn integration_counter() {
    run_st_tests(&["tests/integration/counter.test.st"]);
}

#[test]
fn integration_form() {
    run_st_tests(&["tests/integration/form.test.st"]);
}

#[test]
fn integration_modal() {
    run_st_tests(&["tests/integration/modal.test.st"]);
}

#[test]
fn integration_toggle() {
    run_st_tests(&["tests/integration/toggle.test.st"]);
}

// =============================================================================
// E2E
// =============================================================================

#[test]
fn e2e_page_load() {
    run_st_tests(&["tests/e2e/page-load.test.st"]);
}
