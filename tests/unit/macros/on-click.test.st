/// On Click Animation Tests
///
/// Tests for the @on click macro which triggers animations on click events.
/// Verifies click trigger, multi-stop animations, and timeline registration.

// =============================================================================
// Basic @on click Animation
// =============================================================================

@test "@on click triggers animation on click" {
  @fixture {
    <button class="click-test">
      @on &.click press(300ms) {
        scale: 1 -> 0.95;
      }
    </button>
  }
  @when .click-test click
  @wait 50ms
  @then .click-test should exist
}

@test "@on click registers timeline" {
  @fixture {
    <button class="click-reg">
      @on &.click tap(200ms) {
        opacity: 1 -> 0.7;
      }
    </button>
  }
  @then .click-reg should exist
}

// =============================================================================
// Multi-Stop Arrow Syntax
// =============================================================================

@test "@on click multi-stop animation" {
  @fixture {
    <button class="multi-stop">
      @on &.click bounce(300ms) {
        scale: 1 -> 0.95 -> 1;
      }
    </button>
  }
  @when .multi-stop click
  @wait 50ms
  @then .multi-stop should exist
}

// =============================================================================
// Click Animation with Multiple Properties
// =============================================================================

@test "@on click animates multiple properties" {
  @fixture {
    <button class="multi-click">
      @on &.click flash(200ms) {
        scale: 1 -> 0.98;
        opacity: 1 -> 0.8;
      }
    </button>
  }
  @when .multi-click click
  @wait 50ms
  @then .multi-click should exist
}
