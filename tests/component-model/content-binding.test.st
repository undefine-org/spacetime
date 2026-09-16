/// Headless Tests: Component Model — Content Binding (CMHT-003)
///
/// Tests for text content binding (text <- $var;).
/// Verifies that state values render as text content, update reactively
/// when state changes, and multiple bindings are independent.

@import "stdlib/testing/test"

// =============================================================================
// Text Content Binding — Initial Render
// =============================================================================

@test "text content binding displays state value" {
  @fixture {
    <div class="text-host">
      <span class="text-display"></span>
    </div>
  }
  @given .text-host {
    $label string: "hello";
    .text-display { text <- $label; }
  }
  @wait 50ms
  @then .text-display should have_text "hello"
}

// =============================================================================
// Text Binding — Reactivity
// =============================================================================

@test "text binding updates when state changes" {
  @fixture {
    <div class="reactive-host">
      <span class="reactive-display"></span>
      <button class="reactive-btn">Change</button>
    </div>
  }
  @given .reactive-host {
    $msg string: "before";
    .reactive-display { text <- $msg; }
    .reactive-btn { @on &.click { $msg <- "after"; } }
  }
  @wait 50ms
  @then .reactive-display should have_text "before"
  @when .reactive-btn click
  @wait 50ms
  @then .reactive-display should have_text "after"
}

// =============================================================================
// Multiple Content Bindings — Independence
// =============================================================================

@test "multiple content bindings are independent" {
  @fixture {
    <div class="multi-bind">
      <span class="bind-a"></span>
      <span class="bind-b"></span>
    </div>
  }
  @given .multi-bind {
    $x string: "foo";
    $y string: "bar";
    .bind-a { text <- $x; }
    .bind-b { text <- $y; }
  }
  @wait 50ms
  @then .bind-a should have_text "foo"
  @then .bind-b should have_text "bar"
}
