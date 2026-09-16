/// Headless Tests: Component Model — Reactive Props (CMHT-002)
///
/// Tests for class toggle (.class: $var;) and @on event handlers.
/// Verifies that class names are added/removed reactively when state
/// changes, and that @on click correctly mutates state.

@import "stdlib/testing/test"

// =============================================================================
// Class Toggle — Basic Add
// =============================================================================

@test "class toggle adds class when state is true" {
  @fixture { <div class="toggle-target"></div> }
  @given .toggle-target {
    $open bool: false;
    .is-open: $open;
  }
  @eval (ST.set(document.querySelector('.toggle-target'), 'open', true))
  @wait 50ms
  @then .toggle-target should have_class "is-open"
}

// =============================================================================
// Class Toggle — Remove
// =============================================================================

@test "class toggle removes class when state is false" {
  @fixture { <div class="toggle-off is-active"></div> }
  @given .toggle-off {
    $active bool: true;
    .is-active: $active;
  }
  @eval (ST.set(document.querySelector('.toggle-off'), 'active', false))
  @wait 50ms
  @then .toggle-off should not_have_class "is-active"
}

// =============================================================================
// @on click Toggle
// =============================================================================

@test "@on click toggles class via state" {
  @fixture { <button class="click-toggle">Toggle</button> }
  @given .click-toggle {
    $on bool: false;
    .is-on: $on;
    @on &.click { $on <- !$on; }
  }
  @when .click-toggle click
  @wait 50ms
  @then .click-toggle should have_class "is-on"
}

// =============================================================================
// Multiple Class Toggles
// =============================================================================

@test "multiple class toggles work independently" {
  @fixture { <div class="multi-toggle"></div> }
  @given .multi-toggle {
    $a bool: false;
    $b bool: true;
    .class-a: $a;
    .class-b: $b;
  }
  @wait 50ms
  @then .multi-toggle should not_have_class "class-a"
  @then .multi-toggle should have_class "class-b"
}

// =============================================================================
// Double Click Returns to Original
// =============================================================================

@test "double click returns to original state" {
  @fixture { <button class="dbl-click">Toggle</button> }
  @given .dbl-click {
    $on bool: false;
    .is-on: $on;
    @on &.click { $on <- !$on; }
  }
  @when .dbl-click click
  @wait 50ms
  @when .dbl-click click
  @wait 50ms
  @then .dbl-click should not_have_class "is-on"
}
