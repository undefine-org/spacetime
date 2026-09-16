/// Scroll Driver Primitive Tests
///
/// Tests for the scroll-based animation driver.
/// Verifies timeline registration, scroll progress, and multi-property animations.
///
/// SHAPE (BUG-194/BUG-200): drivers are declared at FILE SCOPE in a selector
/// block (`.cls { @scroll ... }`) and @fixture carries markup only. The
/// previous shape (directive nested inside fixture markup, `<div> @scroll ...
/// </div>`) never bound a driver -- it was silently dropped (BUG-200) AND,
/// inside the @test body, compiled the whole test empty (BUG-194), so these
/// tests were vacuous until 2026-07-18.

@version 2026-07-26;

@import "stdlib/testing/test"

.scroll-animated {
  @on &.scroll(name: test-scroll, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1;
      }
}

.late-start {
  @on &.scroll(name: delayed, scope: cover, start: 0.5, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1;
      }
}

.early-end {
  @on &.scroll(name: quick, scope: cover, start: 0, end: 0.5, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1;
      }
}

.multi-prop {
  @on &.scroll(name: multi, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1;
        translate-y: 100px -> 0;
      }
}

.scroll-progress {
  @on &.scroll(name: progress-test, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1;
      }
}

.scope-cover {
  @on &.scroll(name: cover-test, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1;
      }
}

.scope-entry {
  @on &.scroll(name: entry-test, scope: entry, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1;
      }
}

.scope-exit {
  @on &.scroll(name: exit-test, scope: exit, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1;
      }
}

.scope-contain {
  @on &.scroll(name: contain-test, scope: contain, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1;
      }
}

.scope-subrange {
  @on &.scroll(name: subrange-test, scope: cover, start: 0.3, end: 0.7, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1;
        translate-y: 40px -> 0;
      }
}

// =============================================================================
// Basic Scroll Animation
// =============================================================================

@test "scroll driver initializes at 0 progress" {
  @fixture {
    <div class="scroll-test" style="opacity: 0">Scroll Me</div>
  }
  @wait 100ms
  @then .scroll-test should have_style "opacity: 0"
}

@test "scroll driver applies animation on scroll" {
  @fixture {
    <div class="scroll-animated"></div>
  }
  // Let the runtime's selector-init scan process the fixture insert
  // (it runs async) so the file-scope driver registers its timeline.
  @wait 100ms
  @then .scroll-animated should exist
  @then .scroll-animated should have_timeline "test-scroll"
}

// =============================================================================
// Scroll Start/End Parameters
// =============================================================================

@test "scroll animation respects start parameter" {
  @fixture {
    <!-- progress 0 is a POSITION: the element must sit BELOW the fold, where
         `rect.top == innerHeight`. Rendered at the top of the viewport it is
         already most of the way through a `cover` range, and the driver
         correctly reports ~0.66. -->
    <div style="height: 200vh"></div>
    <div class="late-start" style="opacity: 0"></div>
  }
  // The driver registers its timeline on the next animation frame (deliberately
  // deferred so watchers bind first and getBoundingClientRect does not force a
  // reflow per timeline). Asserting immediately races that frame — every other
  // test in this file waits, and this one alone did not.
  @wait 100ms
  // At 0% scroll, opacity should still be 0
  @then .late-start should have_style "opacity: 0"
  @then .late-start should have_timeline "delayed"
}

@test "scroll animation respects end parameter" {
  @fixture {
    <div class="early-end"></div>
  }
  @wait 100ms
  @then .early-end should exist
  @then .early-end should have_timeline "quick"
}

// =============================================================================
// Multiple Properties
// =============================================================================

@test "scroll driver animates multiple properties" {
  @fixture {
    <div class="multi-prop"></div>
  }
  @wait 100ms
  @then .multi-prop should exist
  @then .multi-prop should have_timeline "multi"
}

// =============================================================================
// Scroll Progress Updates
// =============================================================================

@test "scroll driver updates progress on scroll event" {
  @fixture {
    <div class="scroll-progress" style="height: 2000px"></div>
  }
  @wait 100ms
  @then .scroll-progress should have_timeline "progress-test"
  @when .scroll-progress scroll 500
  @then .scroll-progress should exist
}

// =============================================================================
// Scope Parameter
// =============================================================================

@test "scroll driver accepts scope: cover (default)" {
  @fixture {
    <div class="scope-cover"></div>
  }
  @wait 100ms
  @then .scope-cover should exist
  @then .scope-cover should have_timeline "cover-test"
}

@test "scroll driver accepts scope: entry" {
  @fixture {
    <div class="scope-entry"></div>
  }
  @wait 100ms
  @then .scope-entry should exist
  @then .scope-entry should have_timeline "entry-test"
}

@test "scroll driver accepts scope: exit" {
  @fixture {
    <div class="scope-exit"></div>
  }
  @wait 100ms
  @then .scope-exit should exist
  @then .scope-exit should have_timeline "exit-test"
}

@test "scroll driver accepts scope: contain" {
  @fixture {
    <div class="scope-contain"></div>
  }
  @wait 100ms
  @then .scope-contain should exist
  @then .scope-contain should have_timeline "contain-test"
}

@test "scroll driver with scope and sub-range" {
  @fixture {
    <div class="scope-subrange"></div>
  }
  @wait 100ms
  @then .scope-subrange should exist
  @then .scope-subrange should have_timeline "subrange-test"
}
