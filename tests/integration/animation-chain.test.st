/// Animation Chaining & Composition Tests
///
/// Tests signal-driven animation chaining and multi-driver composition.

@import "stdlib/testing/test"

// =============================================================================
// Signal-driven chaining
// =============================================================================

.chain-first {
  @on &.load(duration: 300ms) as $intro {
    opacity: 0 -> 1;
  }
}

@form motion --followup($duration = 300ms) {
  opacity: 0 -> 1;
}

.chain-second {
  @on $intro.done {
    true => --followup(300ms);
  }
}

@test "followup runs after intro signal completes" {
  @fixture {
    <div>
      <div class="chain-first"></div>
      <div class="chain-second"></div>
    </div>
  }
  @then .chain-first should exist
  @then .chain-second should exist
}

@test "followup motion form accepts duration" {
  @fixture {
    <div class="delayed-chain-a">
      @on &.load(duration: 400ms) as $stepOne {
        opacity: 0 -> 1;
      }
    </div>
    <div class="delayed-chain-b">
      @on $stepOne.done {
        true => --followup(300ms);
      }
    </div>
  }
  @then .delayed-chain-a should exist
  @then .delayed-chain-b should exist
}

// =============================================================================
// Staggered Animations via @on visible
// =============================================================================

@test "@on visible with stagger animates children" {
  @fixture {
    <div class="stagger-container">
      @on &.visible reveal(600ms) {
        opacity: 0 -> 1;
        translate-y: 20px -> 0;
      }
    </div>
  }
  @then .stagger-container should exist
}

// =============================================================================
// Multi-Driver Composition
// =============================================================================

@test "scope with @scroll and @on &.hover" {
  @fixture {
    <div class="composed">
      @scroll fade(start: 0, end: 1) {
        opacity: 0 -> 1;
      }
      @on &.hover lift(200ms) {
        translate-y: 0 -> -5px;
      }
    </div>
  }
  @then .composed should exist
}

@test "scope with @load and @on &.click" {
  @fixture {
    <button class="interactive">
      @on &.load(duration: 500ms) as $appear {
        opacity: 0 -> 1;
      }
      @on &.click press(150ms) {
        scale: 1 -> 0.97;
      }
    </button>
  }
  @then .interactive should exist
}
