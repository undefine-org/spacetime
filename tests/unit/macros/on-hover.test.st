/// On Hover/Visible Animation Tests
///
/// Tests for the @on visibility-triggered and hover-triggered animation macros.
/// Verifies timeline registration, animation behavior, and multi-property support.
///
/// SHAPE (BUG-194/BUG-200): drivers at file scope in selector blocks;
/// @fixture markup only; @wait for the async selector-init scan before
/// timeline assertions. The fixture-nested form was silently dropped.

@import "stdlib/testing/test"

.reveal-test {
  @on &.visible(800ms) as $test-reveal {
    opacity: 0 -> 1;
  }
}

.timed-reveal {
  @on &.visible(2000ms) as $slow-reveal {
    opacity: 0 -> 1;
  }
}

.complete-test {
  @on &.visible(400ms) as $appear {
    opacity: 0 -> 1;
  }
}

.hover-target {
  @on &.hover(300ms) as $lift {
    transform: translateY(0) -> translateY(-10px);
  }
}

.hover-reverse {
  @on &.hover(200ms) as $scale-up {
    transform: scale(1) -> scale(1.1);
  }
}

.multi-hover {
  @on &.hover(150ms) as $glow {
    opacity: 0.8 -> 1;
    box-shadow: none -> 0 0 20px rgba(0,0,0,0.2);
  }
}

// =============================================================================
// Basic Visibility Trigger
// =============================================================================

@test "@on visible triggers animation on viewport entry" {
  @fixture {
    <div class="reveal-test" style="opacity: 0"></div>
  }
  @wait 100ms
  @then .reveal-test should exist
  @then .reveal-test should have_timeline "test-reveal"
}

@test "@on visible applies duration parameter" {
  @fixture {
    <div class="timed-reveal"></div>
  }
  @wait 100ms
  @then .timed-reveal should exist
  @then .timed-reveal should have_timeline "slow-reveal"
}

@test "@on visible completes timeline" {
  @fixture {
    <div class="complete-test"></div>
  }
  @wait 100ms
  @then .complete-test should have_timeline "appear"
  @wait_for_timeline "appear"
  @then .complete-test should exist
}

// =============================================================================
// Hover Trigger
// =============================================================================

@test "@on hover triggers animation on mouseenter" {
  @fixture {
    <div class="hover-target"></div>
  }
  @wait 100ms
  @when .hover-target mouseenter
  @wait 200ms
  @then .hover-target should exist
  @then .hover-target should have_timeline "lift"
}

@test "@on hover reverses on mouseleave" {
  @fixture {
    <div class="hover-reverse"></div>
  }
  @wait 100ms
  @when .hover-reverse mouseenter
  @wait 200ms
  @when .hover-reverse mouseleave
  @then .hover-reverse should exist
}

// =============================================================================
// Multiple Properties on Hover
// =============================================================================

@test "@on hover animates multiple properties" {
  @fixture {
    <div class="multi-hover"></div>
  }
  @wait 100ms
  @then .multi-hover should exist
  @then .multi-hover should have_timeline "glow"
}
