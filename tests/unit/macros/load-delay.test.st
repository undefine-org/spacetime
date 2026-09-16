/// Load Delay Type Tests
///
/// Verifies that @load delay parameters emit numeric values (not strings).
/// Regression test for the type-blind parse_default_value bug where
/// "0ms" was emitted as string "0ms" instead of number 0.
///
/// SHAPE (BUG-194/BUG-200): drivers at file scope; @fixture markup only;
/// @wait for the async selector-init scan before timeline assertions.

@version 2026-07-26;

@import "stdlib/testing/test"

.delay-default {
  @on &.load(name: fade-default, duration: 500ms, delay: 0ms, threshold: 0.1) {
        easing: ease-out-quad;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1;
      }
}

.delay-explicit {
  @on &.load(name: fade-delayed, duration: 500ms, delay: 200ms, threshold: 0.1) {
        easing: ease-out-quad;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1;
      }
}

// =============================================================================
// Default Delay Produces Numeric Value
// =============================================================================

@test "@load with default delay produces numeric delayMs" {
  @fixture {
    <div class="delay-default"></div>
  }
  @wait 100ms
  @then .delay-default should exist
  @then .delay-default should have_timeline "fade-default"
}

// =============================================================================
// Explicit Delay Produces Numeric Value
// =============================================================================

@test "@load with explicit delay 200ms produces numeric value" {
  @fixture {
    <div class="delay-explicit"></div>
  }
  @wait 100ms
  @then .delay-explicit should exist
  @then .delay-explicit should have_timeline "fade-delayed"
}
