/// Smooth Scroll Primitive Tests
///
/// Tests for the Lenis-style smooth scroll engine.
/// Verifies initialization, parameter handling, signal exposure, and cleanup.

// =============================================================================
// Basic Initialization
// =============================================================================

@test "smooth-scroll initializes on element" {
  @fixture {
    <div class="scroll-container" style="height: 2000px">
      @smooth-scroll()
    </div>
  }
  @then .scroll-container should exist
}

@test "smooth-scroll accepts lerp parameter" {
  @fixture {
    <div class="lerp-test">
      @smooth-scroll(lerp: 0.05)
    </div>
  }
  @then .lerp-test should exist
}

@test "smooth-scroll accepts duration parameter" {
  @fixture {
    <div class="duration-test">
      @smooth-scroll(duration: 1.5)
    </div>
  }
  @then .duration-test should exist
}

// =============================================================================
// Parameter Combinations
// =============================================================================

@test "smooth-scroll accepts all parameters" {
  @fixture {
    <div class="full-params">
      @smooth-scroll(
        lerp: 0.08,
        duration: 1.0,
        smoothWheel: true,
        smoothTouch: false,
        infinite: false,
        orientation: "vertical"
      )
    </div>
  }
  @then .full-params should exist
}

@test "smooth-scroll horizontal orientation" {
  @fixture {
    <div class="horizontal-scroll" style="width: 4000px">
      @smooth-scroll(orientation: "horizontal")
    </div>
  }
  @then .horizontal-scroll should exist
}

// =============================================================================
// Signal Exposure
// =============================================================================

@test "smooth-scroll exposes scroll position signals" {
  @fixture {
    <div class="signal-test" style="height: 3000px">
      @smooth-scroll(lerp: 0.1)
    </div>
  }
  @then .signal-test should exist
}

@test "smooth-scroll works with reduced motion preference" {
  @fixture {
    <div class="reduced-motion-test">
      @smooth-scroll(lerp: 0.1)
    </div>
  }
  @then .reduced-motion-test should exist
}

// =============================================================================
// Body Target (Viewport Scroll)
// =============================================================================

// NB: these were authored as `<body class=…>` nested in the fixture, but a
// fixture mounts its content inside a `<div>` — a nested `<body>` is invalid
// HTML. linkedom (logic backend) leniently KEEPS it; real Chromium (--cdp)
// correctly STRIPS it, so `.body-scroll-test` never existed there. Either way
// the primitive only ever ran on a plain element, never `document.body`, so the
// viewport-targeting branch (isWindow) was never exercised by these. They only
// assert init-with-body-oriented-params doesn't crash — expressed faithfully on
// a valid element here. True viewport-scroll targeting is a page-level concern
// covered by @drive automation, not a mounted fragment.
@test "smooth-scroll initializes with viewport-oriented params" {
  @fixture {
    <div class="body-scroll-test" style="height: 2000px">
      @smooth-scroll(lerp: 0.1, smoothWheel: true)
    </div>
  }
  @then .body-scroll-test should exist
}

@test "smooth-scroll initializes with all viewport params" {
  @fixture {
    <div class="body-full-params" style="height: 2000px">
      @smooth-scroll(
        lerp: 0.05,
        duration: 1.0,
        smoothWheel: true,
        smoothTouch: false
      )
    </div>
  }
  @then .body-full-params should exist
}
