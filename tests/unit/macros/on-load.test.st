/// On Load Animation Tests
///
/// Tests named @on &.load driver publications and their animated keyframes.
/// Verifies element-scoped progress, delay parameters, and multi-property animations.
///
/// SHAPE (BUG-194/BUG-200): drivers at file scope in selector blocks;
/// @fixture markup only; @wait for the async selector-init scan before
/// timeline assertions. The fixture-nested form was silently dropped.

@import "stdlib/testing/test"

.load-test {
  @on &.load(name: "intro", duration: 600ms) {
    opacity: 0 -> 1;
  }
}

.reg-test {
  @on &.load(name: "fade-in", duration: 400ms) {
    opacity: 0 -> 1;
  }
}

.delayed-load {
  @on &.load(name: "delayed", duration: 500ms, delay: 200ms) {
    opacity: 0 -> 1;
  }
}

.multi-load {
  @on &.load(name: "slide-up", duration: 800ms) {
    opacity: 0 -> 1;
    translate-y: 20px -> 0;
  }
}

.triple-load {
  @on &.load(name: "entrance", duration: 600ms) {
    opacity: 0 -> 1;
    translate-y: 30px -> 0;
    scale: 0.95 -> 1;
  }
}

.hero {
  // No `once:` — `.visible` binds `load-driver`, which is one-shot by
  // construction (`if (started) return;` guards play(), and the observer
  // disconnects after the first fire). The param was redundant AND undeclared,
  // so it silently absorbed until BUG-352 made it E0960.
  @on &.visible(threshold: 0) {
    opacity: 0 -> 1;
    translate-y: 40px -> 0;
  }
}

// =============================================================================
// Basic @load Animation
// =============================================================================

@test "@load triggers animation on page load" {
  @fixture {
    <div class="load-test" style="opacity: 0"></div>
  }
  @wait 100ms
  @then .load-test should exist
  @let (el = document.querySelector('.load-test'))
  @assert (ST.resolve(el, 'intro') !== undefined)
    "@load should publish intro progress on its element"
}

@test "@load publishes named driver progress" {
  @fixture {
    <div class="reg-test"></div>
  }
  @wait 100ms
  @then .reg-test should exist
  @let (el = document.querySelector('.reg-test'))
  @assert (ST.resolve(el, 'fade-in') !== undefined)
    "@load should publish fade-in progress on its element"
}

// =============================================================================
// @load with Delay Parameter
// =============================================================================

@test "@load with delay parameter defers animation start" {
  @fixture {
    <div class="delayed-load" style="opacity: 0"></div>
  }
  @wait 100ms
  @then .delayed-load should exist
  @let (el = document.querySelector('.delayed-load'))
  @assert (ST.resolve(el, 'delayed') !== undefined)
    "Delayed @load should publish progress before starting"
}

// =============================================================================
// @load Multi-Property Animation
// =============================================================================

@test "@load animates multiple properties" {
  @fixture {
    <div class="multi-load" style="opacity: 0; transform: translateY(20px)"></div>
  }
  @wait 100ms
  @then .multi-load should exist
  @let (el = document.querySelector('.multi-load'))
  @assert (ST.resolve(el, 'slide-up') !== undefined)
    "Multi-property @load should publish slide-up progress"
}

@test "@load with three properties" {
  @fixture {
    <div class="triple-load"></div>
  }
  @wait 100ms
  @then .triple-load should exist
  @let (el = document.querySelector('.triple-load'))
  @assert (ST.resolve(el, 'entrance') !== undefined)
    "Three-property @load should publish entrance progress"
}

// =============================================================================
// @on visible publishes element-scoped visibility
// =============================================================================

@test "@on visible publishes element-scoped visibility" {
  @fixture {
    <div class="hero"></div>
  }
  @wait 100ms
  @then .hero should exist
  @let (el = document.querySelector('.hero'))
  @assert (ST.resolve(el, 'visible') !== undefined)
    "Visible driver should publish visibility on its element"
}
