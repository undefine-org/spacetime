/// Scroll Composition Tests
///
/// Tests for animation composition when multiple @scroll blocks target
/// the same CSS property on the same element (the in/out pattern).
/// Verifies ownership tracking prevents clobbering at init and during scroll.
///
/// SHAPE (BUG-194/BUG-200): drivers are declared at FILE SCOPE in a selector
/// block (`.cls { @scroll ... }`) and @fixture carries markup only. The
/// previous shape (directive nested inside fixture markup) compiled the whole
/// test empty (BUG-194) and the driver was silently dropped (BUG-200), so
/// these tests were vacuous until 2026-07-18.

@version 2026-07-26;

@import "stdlib/testing/test"

.comp-init {
  @on &.scroll(name: comp-in, scope: cover, start: 0.05, end: 0.25, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                .comp-target {
            opacity: 0 -> 1;
        }

      }
  @on &.scroll(name: comp-out, scope: cover, start: 0.45, end: 0.55, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                .comp-target {
            opacity: 1 -> 0;
        }

      }
}

.comp-dual {
  @on &.scroll(name: dual-in, scope: cover, start: 0, end: 0.3, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                .dual-target {
            opacity: 0 -> 1;
            translate-y: 30px -> 0;
        }

      }
  @on &.scroll(name: dual-out, scope: cover, start: 0.7, end: 1, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                .dual-target {
            opacity: 1 -> 0;
            translate-y: 0 -> -30px;
        }

      }
}

.comp-multi {
  @on &.scroll(name: multi-in, scope: cover, start: 0.05, end: 0.3, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                .multi-target {
            opacity: 0 -> 1;
            translate-y: 40px -> 0;
        }

      }
  @on &.scroll(name: multi-out, scope: cover, start: 0.6, end: 0.85, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                .multi-target {
            opacity: 1 -> 0;
            translate-y: 0 -> -40px;
        }

      }
}

.comp-single {
  @on &.scroll(name: single-fade, scope: cover, start: 0, end: 0.5, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                .single-target {
            opacity: 0 -> 1;
        }

      }
}

.comp-single-multi {
  @on &.scroll(name: single-multi, scope: cover, start: 0, end: 0.5, scrub: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                .sm-target {
            opacity: 0 -> 1;
            translate-y: 20px -> 0;
        }

      }
}

// =============================================================================
// Init-time Composition (first-writer-wins)
// =============================================================================

@test "in/out scroll animations: initial state uses first animation value" {
  @fixture {
    <div style="height: 200vh"></div>
    <div class="comp-init" style="height: 3000px">
      <div class="comp-target">Target</div>
    </div>
  }
  @wait 100ms
  @then .comp-target should have_style "opacity: 0"
}

@test "in/out scroll animations on same element both register timelines" {
  @fixture {
    <div class="comp-dual" style="height: 3000px">
      <div class="dual-target">Target</div>
    </div>
  }
  @wait 100ms
  @then .comp-dual should have_timeline "dual-in"
  @then .comp-dual should have_timeline "dual-out"
}

// =============================================================================
// Multiple Properties Composition
// =============================================================================

@test "in/out composition with multiple properties preserves initial state" {
  @fixture {
    <div style="height: 200vh"></div>
    <div class="comp-multi" style="height: 3000px">
      <div class="multi-target">Target</div>
    </div>
  }
  @wait 100ms
  @then .multi-target should have_style "opacity: 0"
}

// =============================================================================
// Single Animation (no composition - regression guard)
// =============================================================================

@test "single scroll animation applies initial state normally" {
  @fixture {
    <div style="height: 200vh"></div>
    <div class="comp-single" style="height: 3000px">
      <div class="single-target">Target</div>
    </div>
  }
  @wait 100ms
  @then .single-target should have_style "opacity: 0"
}

@test "single scroll animation with multiple properties" {
  @fixture {
    <div style="height: 200vh"></div>
    <div class="comp-single-multi" style="height: 3000px">
      <div class="sm-target">Target</div>
    </div>
  }
  @wait 100ms
  @then .sm-target should have_style "opacity: 0"
}
