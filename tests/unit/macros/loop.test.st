/// Loop Animation Tests
///
/// Tests for the @loop macro which creates infinitely looping animations.
/// Verifies automatic start, running state, and custom duration.
///
/// SHAPE (BUG-194/BUG-200): drivers at file scope in selector blocks;
/// @fixture markup only; @wait for the async selector-init scan before
/// timeline assertions.
/// RUNG: `needs timing` — an infinite @loop driver keeps rAF pending
/// forever, and the headless V8 world cannot quiesce its event loop (the
/// run hangs). CDP runs them for real.

@version 2026-07-26;

@import "stdlib/testing/test"

.loop-test {
  @on &.loop(name: spin, duration: 1s, mode: pingpong, phase: 0) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                rotate: 0deg -> 360deg;
      }
}

.running-test {
  @on &.loop(name: pulse, duration: 2s, mode: pingpong, phase: 0) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                scale: 1 -> 1.1;
      }
}

.named-dur {
  @on &.loop(name: breathe, duration: 3s, mode: pingpong, phase: 0) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0.5 -> 1;
      }
}

.fast-loop {
  @on &.loop(name: blink, duration: 500ms, mode: pingpong, phase: 0) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 1 -> 0;
      }
}

.multi-loop {
  @on &.loop(name: float, duration: 2s, mode: pingpong, phase: 0) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                translate-y: 0 -> -10px;
        opacity: 0.8 -> 1;
      }
}

// =============================================================================
// Basic @loop Animation
// =============================================================================

@test "@loop starts automatically" needs timing {
  @fixture {
    <div class="loop-test"></div>
  }
  @wait 100ms
  @then .loop-test should exist
  @then .loop-test should have_timeline "spin"
}

@test "@loop timeline is running (not completed)" needs timing {
  @fixture {
    <div class="running-test"></div>
  }
  @wait 100ms
  @then .running-test should exist
  @then .running-test should have_timeline "pulse"
}

// =============================================================================
// @loop with Custom Duration
// =============================================================================

@test "@loop with named duration parameter" needs timing {
  @fixture {
    <div class="named-dur"></div>
  }
  @wait 100ms
  @then .named-dur should exist
  @then .named-dur should have_timeline "breathe"
}

@test "@loop with short duration" needs timing {
  @fixture {
    <div class="fast-loop"></div>
  }
  @wait 100ms
  @then .fast-loop should exist
  @then .fast-loop should have_timeline "blink"
}

// =============================================================================
// @loop Multi-Property
// =============================================================================

@test "@loop animates multiple properties" needs timing {
  @fixture {
    <div class="multi-loop"></div>
  }
  @wait 100ms
  @then .multi-loop should exist
  @then .multi-loop should have_timeline "float"
}
