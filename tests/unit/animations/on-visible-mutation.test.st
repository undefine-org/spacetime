/// On Visible Mutation Tests
///
/// Tests for @on visible { mutations } — the mutation (non-animation) form
/// of @on visible. Verifies IntersectionObserver is used instead of
/// addEventListener('visible'), which would silently fail.

@import "stdlib/testing/test"

// =============================================================================
// Basic @on visible mutation
// =============================================================================

@test "@on visible mutation fires when element enters viewport" {
  @fixture {
    <div class="vis-mut-target" style="position:fixed;top:0;left:0;height:10px;width:100%;"></div>
  }
  @given .vis-mut-target {
    @on &.visible {
      $testVal <- "fired";
    }
  }

  @eval (window.SpacetimeLocal = window.SpacetimeLocal || {})
  // Element is in the fixture viewport, so IO should fire immediately
  @wait 100ms
  @assert (window.SpacetimeLocal.testVal === "fired")
    "mutation should have set variable via IntersectionObserver"
}

@test "@on visible mutation fires IntersectionObserver not addEventListener" {
  // A 'visible' DOM event listener would never fire — only IO works
  @fixture {
    <div class="vis-io-test" style="position:fixed;top:0;left:0;height:10px;width:100%;"></div>
  }
  @given .vis-io-test {
    @on &.visible {
      $visResult <- true;
    }
  }

  @eval (window.SpacetimeLocal = window.SpacetimeLocal || {})
  @wait 100ms
  @assert (window.SpacetimeLocal.visResult === true)
    "should fire via IntersectionObserver, not addEventListener('visible')"
}

// =============================================================================
// @emit inside @on visible
// =============================================================================

@test "@on visible mutation fires @emit event" {
  @fixture {
    <div class="vis-emitter" style="position:fixed;top:0;left:0;height:10px;width:100%;"></div>
  }
  @given .vis-emitter {
    @on &.visible {
      $emitReceived <- true;
    }
  }

  @eval (window.SpacetimeLocal = window.SpacetimeLocal || {})
  @wait 100ms
  @assert (window.SpacetimeLocal.emitReceived === true)
    "emit should dispatch custom event from visible handler"
}

// =============================================================================
// Offscreen element should not trigger
// =============================================================================

@test "@on visible mutation does not fire when element is offscreen" {
  @fixture {
    <div class="vis-offscreen" style="position:absolute;top:200vh;height:10px;"></div>
  }
  @given .vis-offscreen {
    @on &.visible {
      $offscreenVal <- "shouldnt-fire";
    }
  }

  @eval (window.SpacetimeLocal = window.SpacetimeLocal || {})
  @wait 150ms
  @assert (window.SpacetimeLocal.offscreenVal === undefined)
    "should not fire for offscreen element"
}

// =============================================================================
// Click mutation regression test
// =============================================================================

@test "@on click mutation still works (no regression)" {
  @fixture {
    <button class="vis-click-test"></button>
  }
  @given .vis-click-test {
    @on &.click {
      $clickVal <- "clicked";
    }
  }

  @eval (window.SpacetimeLocal = window.SpacetimeLocal || {})
  @when .vis-click-test click
  @wait 50ms
  @assert (window.SpacetimeLocal.clickVal === "clicked")
    "click mutation should still work via addEventListener"
}

// =============================================================================
// BUG-185 regression: a BOTTOM-anchored fixed element must fire too
// =============================================================================

// Found via a live user report ("host pin doesn't update to the host
// being saved", traced to `stdlib/__host__/index.st`'s own
// `.st-host-widget { position: fixed; bottom: 16px; ... }` never
// firing its mount-time `@on visible { $status(); } }` at all).
// `on-mutation-handler` (stdlib/primitives/animation/drivers.st) used to
// hardcode `rootMargin: '0px 0px -90% 0px'`, restricting the
// IntersectionObserver's observed region to only the TOP 10% of the
// viewport (a narrow, undocumented assumption for a reveal-on-scroll
// HEADER use case) -- every test ABOVE in this file uses a `top:0`
// fixture, so that restriction never surfaced here despite this exact
// file's own doc comment claiming full coverage. A real bottom-anchored
// fixed element (this widget's own real, live shape) never entered that
// narrowed region and its IntersectionObserver simply never fired.
// Fixed by widening `rootMargin` to `'0px'` (the real, full viewport).
//
// IMPORTANT (found + corrected during this session's own verification):
// this test MUST run `needs layout` (a real CDP-backed Chromium), NOT the
// default headless-V8 logic rung. `src/headless_shims.js`'s
// `IntersectionObserver` shim is a pure inline-style heuristic that only
// ever inspects `style.top`/`display`/`visibility` -- it reads `rootMargin`
// out of `opts` but NEVER CONSULTS it, and never looks at `style.bottom`
// either. A `bottom:0` fixture therefore reports `isIntersecting: true`
// UNCONDITIONALLY under that shim, regardless of whether the real
// `rootMargin` fix is even present -- confirmed empirically this session
// by temporarily re-reverting the real fix in drivers.st and re-running
// this exact test under the default (non-`needs layout`) tier: it kept
// passing, proving a headless-tier version of this test provides FALSE
// confidence and cannot actually catch this class of bug. Real pixel-
// accurate `rootMargin`/geometry behavior is a `layout`-rung concern
// (this file's own convention, see `tests/bugs/BUG-revelation-headline-
// clipping.test.st` for the established `needs layout` pattern), which is
// backed by a REAL browser's REAL `IntersectionObserver` implementation
// end to end -- the only tier where this regression is meaningful.
@test "@on visible mutation fires for a BOTTOM-anchored fixed element (BUG-185)" needs layout {
  @fixture {
    <div class="vis-bottom-target" style="position:fixed;bottom:0;left:0;height:10px;width:100%;"></div>
  }
  @given .vis-bottom-target {
    @on &.visible {
      $bottomVisVal <- "fired";
    }
  }

  @eval (window.SpacetimeLocal = window.SpacetimeLocal || {})
  @wait 300ms
  @assert (window.SpacetimeLocal.bottomVisVal === "fired")
    "a bottom-anchored fixed element must fire @on visible just like a top-anchored one (BUG-185)"
}
