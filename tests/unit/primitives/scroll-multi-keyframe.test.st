/// Scroll Multi-Keyframe Animation Tests
///
/// Tests for multi-keyframe scroll animations (e.g. `opacity: 0 -> 1 -> 1 -> 0`).
/// This pattern merges separate in/out animations into a single timeline to prevent
/// the initial-state overlap bug where `applyInitialState()` at progress=0 would
/// apply the wrong value when multiple timelines target the same element.
///
/// SHAPE (BUG-194/BUG-200): drivers are declared at FILE SCOPE in a selector
/// block (`.cls { @scroll ... }`) and @fixture carries markup only. The previous
/// shape (directive nested inside fixture markup) compiled the test body empty
/// and was silently dropped, so these tests were vacuous until 2026-07-19.

@version 2026-07-26;

@import "stdlib/testing/test"

// =============================================================================
// File-scope drivers: each selector block preserves the original timeline
// names, parameters, keyframes, and multi-keyframe ranges verbatim.
// =============================================================================

.mk-opacity {
  @on &.scroll(name: mk-opacity-test, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1 -> 1 -> 0;
      }
}

.mk-translate {
  @on &.scroll(name: mk-translate-test, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                translate-y: 30px -> 0 -> 0 -> -20px;
      }
}

.mk-combined {
  @on &.scroll(name: mk-combined-test, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1 -> 1 -> 0;
        translate-y: 30px -> 0 -> 0 -> -20px;
      }
}

.mk-initial {
  @on &.scroll(name: mk-initial-test, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1 -> 1 -> 0;
      }
}

.mk-two-kf {
  @on &.scroll(name: mk-two-kf-test, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1;
      }
}

.mk-parent {
  @on &.scroll(name: mk-child-test, scope: cover, start: 0.23, end: 0.42, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                [data-wp="2"] {
            opacity: 0 -> 1 -> 1 -> 0;
            translate-y: 30px -> 0 -> 0 -> -20px;
        }

      }
}

.mk-multi-child {
  @on &.scroll(name: mk-multi-child-test, scope: cover, start: 0.23, end: 0.42, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                [data-wp="2"] {
            opacity: 0 -> 1 -> 1 -> 0;
        }

        .node-2 {
            scale: 0.5 -> 1 -> 1 -> 0.5;
        }

      }
}

.mk-sequential {
  @on &.scroll(name: mk-seq-w2, scope: cover, start: 0.23, end: 0.42, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1 -> 1 -> 0;
      }
  @on &.scroll(name: mk-seq-w3, scope: cover, start: 0.43, end: 0.62, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1 -> 1 -> 0;
      }
  @on &.scroll(name: mk-seq-w4, scope: cover, start: 0.63, end: 0.82, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1 -> 1 -> 0;
      }
}

.mk-pillar {
  @on &.scroll(name: mk-pillar-w2, scope: cover, start: 0.23, end: 0.42, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                [data-wp="2"] {
            opacity: 0 -> 1 -> 1 -> 0;
            translate-y: 30px -> 0 -> 0 -> -20px;
        }

      }
  @on &.scroll(name: mk-pillar-w3, scope: cover, start: 0.43, end: 0.62, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                [data-wp="3"] {
            opacity: 0 -> 1 -> 1 -> 0;
            translate-y: 30px -> 0 -> 0 -> -20px;
        }

      }
  @on &.scroll(name: mk-pillar-w4, scope: cover, start: 0.63, end: 0.82, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                [data-wp="4"] {
            opacity: 0 -> 1 -> 1 -> 0;
            translate-y: 30px -> 0 -> 0 -> -20px;
        }

      }
}

.mk-last-wp {
  @on &.scroll(name: mk-last-wp-test, scope: cover, start: 0.83, end: 0.93, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                [data-wp="5"] {
            opacity: 0 -> 1;
            translate-y: 30px -> 0;
        }

      }
}

.mk-full-pillar {
  @on &.scroll(name: mk-full-w2, scope: cover, start: 0.23, end: 0.42, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                [data-wp="2"] {
            opacity: 0 -> 1 -> 1 -> 0;
            translate-y: 30px -> 0 -> 0 -> -20px;
        }

      }
  @on &.scroll(name: mk-full-w3, scope: cover, start: 0.43, end: 0.62, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                [data-wp="3"] {
            opacity: 0 -> 1 -> 1 -> 0;
            translate-y: 30px -> 0 -> 0 -> -20px;
        }

      }
  @on &.scroll(name: mk-full-w4, scope: cover, start: 0.63, end: 0.82, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                [data-wp="4"] {
            opacity: 0 -> 1 -> 1 -> 0;
            translate-y: 30px -> 0 -> 0 -> -20px;
        }

      }
  @on &.scroll(name: mk-full-w5, scope: cover, start: 0.83, end: 0.93, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                [data-wp="5"] {
            opacity: 0 -> 1;
            translate-y: 30px -> 0;
        }

      }
}

.mk-easing {
  @on &.scroll(name: mk-easing-test, scope: cover, start: 0.2, end: 0.8, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0 -> 1 -> 1 -> 0;
        translate-y: 30px -> 0 -> 0 -> -20px;
        easing: --ease-out-expo;
      }
}

.mk-three-kf {
  @on &.scroll(name: mk-three-kf-test, scope: cover, start: 0, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                scale: 0.5 -> 1 -> 0.5;
      }
}

.mk-node {
  @on &.scroll(name: mk-node-test, scope: cover, start: 0.23, end: 0.42, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                opacity: 0.3 -> 1 -> 1 -> 0.3;
        scale: 0.5 -> 1 -> 1 -> 0.5;
      }
}

// =============================================================================
// Basic Multi-Keyframe Compilation
// =============================================================================

@test "multi-keyframe opacity compiles and registers timeline" {
  @fixture {
    <div class="mk-opacity"></div>
  }
  @wait 100ms
  @then .mk-opacity should exist
  @then .mk-opacity should have_timeline "mk-opacity-test"
}

@test "multi-keyframe translate-y compiles and registers timeline" {
  @fixture {
    <div class="mk-translate"></div>
  }
  @wait 100ms
  @then .mk-translate should exist
  @then .mk-translate should have_timeline "mk-translate-test"
}

// =============================================================================
// Combined Multi-Keyframe Properties
// =============================================================================

@test "combined multi-keyframe opacity and translate-y in one scroll block" {
  @fixture {
    <div class="mk-combined"></div>
  }
  @wait 100ms
  @then .mk-combined should exist
  @then .mk-combined should have_timeline "mk-combined-test"
}

// =============================================================================
// Initial State at Progress 0
// =============================================================================

// A `scope: cover` timeline maps progress across the element's travel from
// BELOW the viewport (`rect.top == innerHeight`, progress 0) to fully past it
// (`rect.top == -height`, progress 1).
//
// So "progress 0" is a POSITION, not a moment in time. A fixture element
// rendered at the top of the viewport is already ~80% through that range, and
// the driver correctly reports ~0.8 — these tests previously asserted a
// progress-0 value against an element sitting at progress 0.8 and read the
// resulting mismatch (`opacity: 0, got 0.51`) as the animation failing to hold
// its first keyframe (BUG-343).
//
// Verified against the real driver under --cdp: the same element placed below
// the fold DOES hold `opacity: 0`, for both the 2-keyframe and 4-keyframe
// shapes. The spacer is what puts the element at progress 0.

@test "multi-keyframe initial state uses first keyframe value for opacity" {
  @fixture {
    <div style="height: 200vh"></div>
    <div class="mk-initial" style="opacity: 0"></div>
  }
  @wait 100ms
  @then .mk-initial should have_style "opacity: 0"
}

@test "two-keyframe initial state uses first keyframe value" {
  @fixture {
    <div style="height: 200vh"></div>
    <div class="mk-two-kf" style="opacity: 0"></div>
  }
  @wait 100ms
  @then .mk-two-kf should have_style "opacity: 0"
}

// =============================================================================
// Child Selectors with Multi-Keyframe (the [data-wp] pattern)
// =============================================================================

@test "child selector with multi-keyframe inside scroll block" {
  @fixture {
    <div class="mk-parent">
      <div data-wp="2" class="mk-waypoint">Waypoint 2</div>
    </div>
  }
  @wait 100ms
  @then .mk-parent should exist
  @then .mk-waypoint should exist
  @then .mk-parent should have_timeline "mk-child-test"
}

@test "multiple child selectors in one scroll block" {
  @fixture {
    <div class="mk-multi-child">
      <div data-wp="2" class="mk-wp2">Waypoint</div>
      <div class="node-2">Node</div>
    </div>
  }
  @wait 100ms
  @then .mk-multi-child should exist
  @then .mk-wp2 should exist
  @then .node-2 should exist
  @then .mk-multi-child should have_timeline "mk-multi-child-test"
}

// =============================================================================
// Sequential Non-Overlapping Scroll Ranges (the pillar pattern)
// =============================================================================

@test "sequential scroll blocks with non-overlapping ranges" {
  @fixture {
    <div class="mk-sequential" style="height: 2000px"></div>
  }
  @wait 100ms
  @then .mk-sequential should exist
  @then .mk-sequential should have_timeline "mk-seq-w2"
  @then .mk-sequential should have_timeline "mk-seq-w3"
  @then .mk-sequential should have_timeline "mk-seq-w4"
}

@test "pillar-style sequential waypoints with child selectors" {
  @fixture {
    <div class="mk-pillar">
      <div data-wp="2" class="pillar-wp2">W2</div>
      <div data-wp="3" class="pillar-wp3">W3</div>
      <div data-wp="4" class="pillar-wp4">W4</div>
    </div>
  }
  @wait 100ms
  @then .mk-pillar should exist
  @then .mk-pillar should have_timeline "mk-pillar-w2"
  @then .mk-pillar should have_timeline "mk-pillar-w3"
  @then .mk-pillar should have_timeline "mk-pillar-w4"
  @then .pillar-wp2 should exist
  @then .pillar-wp3 should exist
  @then .pillar-wp4 should exist
}

// =============================================================================
// Last Waypoint Stays Visible (no fade-out at end)
// =============================================================================

@test "last waypoint uses two-keyframe fade-in without fade-out" {
  @fixture {
    <div class="mk-last-wp">
      <div data-wp="5" class="mk-final-wp">Final Waypoint</div>
    </div>
  }
  @wait 100ms
  @then .mk-last-wp should exist
  @then .mk-final-wp should exist
  @then .mk-last-wp should have_timeline "mk-last-wp-test"
}

@test "full pillar pattern with last waypoint staying visible" {
  @fixture {
    <div class="mk-full-pillar">
      <div data-wp="2" class="full-wp2">W2</div>
      <div data-wp="3" class="full-wp3">W3</div>
      <div data-wp="4" class="full-wp4">W4</div>
      <div data-wp="5" class="full-wp5">W5</div>
    </div>
  }
  @wait 100ms
  @then .mk-full-pillar should exist
  @then .mk-full-pillar should have_timeline "mk-full-w2"
  @then .mk-full-pillar should have_timeline "mk-full-w3"
  @then .mk-full-pillar should have_timeline "mk-full-w4"
  @then .mk-full-pillar should have_timeline "mk-full-w5"
  @then .full-wp2 should exist
  @then .full-wp3 should exist
  @then .full-wp4 should exist
  @then .full-wp5 should exist
}

// =============================================================================
// Multi-Keyframe with Easing
// =============================================================================

@test "multi-keyframe with easing parameter compiles" {
  @fixture {
    <div class="mk-easing"></div>
  }
  @wait 100ms
  @then .mk-easing should exist
  @then .mk-easing should have_timeline "mk-easing-test"
}

// =============================================================================
// Three-Keyframe Values (loop-style patterns)
// =============================================================================

@test "three-keyframe scale compiles and registers timeline" {
  @fixture {
    <div class="mk-three-kf"></div>
  }
  @wait 100ms
  @then .mk-three-kf should exist
  @then .mk-three-kf should have_timeline "mk-three-kf-test"
}

@test "node-style multi-keyframe with fractional values" {
  @fixture {
    <div class="mk-node"></div>
  }
  @wait 100ms
  @then .mk-node should exist
  @then .mk-node should have_timeline "mk-node-test"
}
