/// Scroll Velocity Tests
///
/// Tests for scroll velocity signal exposure in the scroll primitive.
/// Verifies velocity is accessible and works with scroll timelines.
///
/// SHAPE (BUG-194/BUG-200): drivers are declared at FILE SCOPE in a selector
/// block (`.cls { @scroll ... }`) and @fixture carries markup only. The
/// previous shape (directive nested inside fixture markup) compiled the test
/// body empty, so these tests were vacuous until 2026-07-18.

@version 2026-07-26;

@import "stdlib/testing/test"

.velocity-test {
  @on &.scroll(name: vel-basic, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1;
      }
}

.vel-vertical {
  @on &.scroll(name: vel-y, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1;
        translate-y: 50px -> 0;
      }
}

.vel-scrub {
  @on &.scroll(name: vel-scrub, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1;
      }
}

.vel-scope {
  @on &.scroll(name: vel-scoped, scope: entry, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1;
      }
}

.vel-update {
  @on &.scroll(name: vel-update, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1;
        skew-y: 0deg -> 5deg;
      }
}

// =============================================================================
// Velocity Signal Exposure
// =============================================================================

@test "scroll primitive exposes velocity signal" {
  @fixture {
    <div class="velocity-test" style="height: 3000px"></div>
  }
  @wait 100ms
  @then .velocity-test should exist
  @then .velocity-test should have_timeline "vel-basic"
}

@test "scroll velocity works with vertical axis" {
  @fixture {
    <div class="vel-vertical" style="height: 2000px"></div>
  }
  @wait 100ms
  @then .vel-vertical should exist
  @then .vel-vertical should have_timeline "vel-y"
}

@test "scroll velocity with scrub timeline" {
  @fixture {
    <div class="vel-scrub" style="height: 4000px"></div>
  }
  @wait 100ms
  @then .vel-scrub should exist
  @then .vel-scrub should have_timeline "vel-scrub"
}

@test "scroll velocity with scope parameter" {
  @fixture {
    <div class="vel-scope"></div>
  }
  @wait 100ms
  @then .vel-scope should exist
  @then .vel-scope should have_timeline "vel-scoped"
}

@test "scroll velocity updates on scroll event" {
  @fixture {
    <div class="vel-update" style="height: 2000px"></div>
  }
  @wait 100ms
  @then .vel-update should have_timeline "vel-update"
  @when .vel-update scroll 500
  @then .vel-update should exist
}
