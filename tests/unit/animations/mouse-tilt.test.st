/// Mouse Axis Verification Tests
///
/// Validates @mouse driver with axis routing, verifying actual computed
/// transforms after synthetic mousemove events. Tests cover:
/// - axis:"both" {x,y} object progress and CSS variable propagation
/// - Per-property axis routing (suffix -x → progress.x, -y → progress.y)
/// - 3-keyframe ranges with center-neutral output
/// - invert:true flipping both axes
/// - Single-axis x/y with scalar progress
/// - Multi-property parallax with per-property axis routing
///
/// SHAPE (BUG-194/BUG-200): drivers are declared at FILE SCOPE in a selector
/// block and @fixture carries markup only. The previous shape (directive nested
/// inside a @test/@given body) compiled the whole test empty, so these tests
/// were vacuous until repaired.
///
/// NOTE: File-scope @mouse with nested child selectors currently drops the
/// animation body in the keyframes capture, so the animated properties live on
/// the driver element itself; the assertions target that same element.

@version 2026-07-26;

@import "stdlib/testing/test"

.t1-parent {
  @on &.mouse(name: t1-tilt, axis: both, invert: false) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                rotate-x: -10deg -> 0 -> 10deg;
        rotate-y: -10deg -> 0 -> 10deg;
      }
}

.t2-parent {
  @on &.mouse(name: t2-tilt, axis: both, invert: false) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                rotate-x: -10deg -> 0 -> 10deg;
        rotate-y: 10deg -> 0 -> -10deg;
      }
}

.t3-parent {
  @on &.mouse(name: t3-tilt, axis: both, invert: true) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                rotate-x: -10deg -> 0 -> 10deg;
        rotate-y: 10deg -> 0 -> -10deg;
      }
}

.t4-parent {
  @on &.mouse(name: t4-slide, axis: x, invert: false) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                translate-x: 0 -> 100px;
      }
}

.t5-parent {
  @on &.mouse(name: t5-vtilt, axis: y, invert: false) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                rotate-x: -10deg -> 0 -> 10deg;
      }
}

.t6-parent {
  @on &.mouse(name: t6-parallax, axis: both, invert: false) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                translate-x: -10px -> 0 -> 10px;
        translate-y: -10px -> 0 -> 10px;
        rotate-x: -5deg -> 0 -> 5deg;
        rotate-y: 5deg -> 0 -> -5deg;
      }
}

// =============================================================================
// Test 1: axis:"both" — signal produces {x, y} object, CSS vars set
// =============================================================================

@test "@mouse axis both produces {x,y} progress and sets CSS vars" {
  @fixture {
    <div class="t1-parent" style="position: fixed; top: 0; left: 0; width: 200px; height: 200px;">
      <div class="t1-child">Child</div>
    </div>
  }
  @wait 100ms

  // Dispatch mousemove at (150, 50) on 200x200 element at origin
  // Expected: x = 150/200 = 0.75, y = 50/200 = 0.25
  @eval (eval("var el = document.querySelector('.t1-parent'); el.getBoundingClientRect = function() { return {left: 0, top: 0, width: 200, height: 200, right: 200, bottom: 200, x: 0, y: 0}; };"))
  @eval (eval("var el = document.querySelector('.t1-parent'); var r = el.getBoundingClientRect(); el.dispatchEvent(new MouseEvent('mousemove', {clientX: r.left + 150, clientY: r.top + 50, bubbles: true}))"))
  @wait 50ms

  // apply-animations sets --st-{name}-x and --st-{name}-y on root for {x,y} drivers
  @let (parent = document.querySelector('.t1-parent'))
  @let (xVal = parseFloat(parent.style.getPropertyValue('--st-t1-tilt-x')))
  @let (yVal = parseFloat(parent.style.getPropertyValue('--st-t1-tilt-y')))
  @assert (Math.abs(xVal - 0.75) < 0.05) "progress.x should be ~0.75"
  @assert (Math.abs(yVal - 0.25) < 0.05) "progress.y should be ~0.25"
}

// =============================================================================
// Test 2: axis:"both" — rotate-x uses progress.x, rotate-y uses progress.y
// =============================================================================

@test "@mouse axis both routes properties by suffix: -x uses progress.x, -y uses progress.y" {
  @fixture {
    <div class="t2-parent" style="position: fixed; top: 0; left: 0; width: 200px; height: 200px; perspective: 800px;">
      <div class="t2-child">Tilt</div>
    </div>
  }
  @wait 100ms

  // Mousemove at (180, 20) → progress {x:0.9, y:0.1}
  // rotate-x (suffix -x) uses progress.x = 0.9:
  //   keyframes [-10deg@0 → 0@0.5 → 10deg@1], between 0@0.5 and 10deg@1.0
  //   local = (0.9-0.5)/0.5 = 0.8 → 0 + 10*0.8 = 8deg
  // rotate-y (suffix -y) uses progress.y = 0.1:
  //   keyframes [10deg@0 → 0@0.5 → -10deg@1], between 10deg@0 and 0@0.5
  //   local = 0.1/0.5 = 0.2 → 10 + (0-10)*0.2 = 8deg
  @eval (eval("var el = document.querySelector('.t2-parent'); el.getBoundingClientRect = function() { return {left: 0, top: 0, width: 200, height: 200, right: 200, bottom: 200, x: 0, y: 0}; };"))
  @eval (eval("var el = document.querySelector('.t2-parent'); var r = el.getBoundingClientRect(); el.dispatchEvent(new MouseEvent('mousemove', {clientX: r.left + 180, clientY: r.top + 20, bubbles: true}))"))
  @wait 50ms

  @let (parent = document.querySelector('.t2-parent'))
  @let (transform = parent.style.transform)
  @assert (transform && transform.includes('rotateX(8deg)')) "rotateX should be 8deg (progress.x=0.9)"
  @assert (transform && transform.includes('rotateY(8deg)')) "rotateY should be 8deg (progress.y=0.1)"
}

// =============================================================================
// Test 3: axis:"both" + invert:true — flips progress
// =============================================================================

@test "@mouse axis both with invert true flips progress values" {
  @fixture {
    <div class="t3-parent" style="position: fixed; top: 0; left: 0; width: 200px; height: 200px; perspective: 800px;">
      <div class="t3-child">Inverted</div>
    </div>
  }
  @wait 100ms

  // Mousemove at (180, 20): raw {x:0.9, y:0.1} → inverted {x:0.1, y:0.9}
  // rotate-x uses inverted progress.x = 0.1:
  //   between -10deg@0 and 0@0.5, local=0.2 → -10 + 10*0.2 = -8deg
  // rotate-y uses inverted progress.y = 0.9:
  //   between 0@0.5 and -10deg@1.0, local=0.8 → 0 + (-10)*0.8 = -8deg
  @eval (eval("var el = document.querySelector('.t3-parent'); el.getBoundingClientRect = function() { return {left: 0, top: 0, width: 200, height: 200, right: 200, bottom: 200, x: 0, y: 0}; };"))
  @eval (eval("var el = document.querySelector('.t3-parent'); var r = el.getBoundingClientRect(); el.dispatchEvent(new MouseEvent('mousemove', {clientX: r.left + 180, clientY: r.top + 20, bubbles: true}))"))
  @wait 50ms

  @let (parent = document.querySelector('.t3-parent'))
  @let (transform = parent.style.transform)
  @assert (transform && transform.includes('rotateX(-8deg)')) "rotateX should be -8deg (inverted progress.x=0.1)"
  @assert (transform && transform.includes('rotateY(-8deg)')) "rotateY should be -8deg (inverted progress.y=0.9)"
}

// =============================================================================
// Test 4: axis:"x" — scalar horizontal progress
// =============================================================================

@test "@mouse axis x uses scalar horizontal progress for translate-x" {
  @fixture {
    <div class="t4-parent" style="position: fixed; top: 0; left: 0; width: 200px; height: 200px;">
      <div class="t4-child">Horizontal</div>
    </div>
  }
  @wait 100ms

  // Mousemove at x=150 out of 200 → scalar progress 0.75
  // translate-x: 0 + 100*0.75 = 75px
  @eval (eval("var el = document.querySelector('.t4-parent'); el.getBoundingClientRect = function() { return {left: 0, top: 0, width: 200, height: 200, right: 200, bottom: 200, x: 0, y: 0}; };"))
  @eval (eval("var el = document.querySelector('.t4-parent'); var r = el.getBoundingClientRect(); el.dispatchEvent(new MouseEvent('mousemove', {clientX: r.left + 150, clientY: r.top + 100, bubbles: true}))"))
  @wait 50ms

  @let (parent = document.querySelector('.t4-parent'))
  @let (transform = parent.style.transform)
  @assert (transform && transform.includes('translateX(75px)')) "translateX should be 75px at progress 0.75"

  // Scalar driver sets --st-{name} (not -x/-y) on root
  @let (scalar = parseFloat(parent.style.getPropertyValue('--st-t4-slide')))
  @assert (Math.abs(scalar - 0.75) < 0.05) "scalar CSS var should be ~0.75"
}

// =============================================================================
// Test 5: axis:"y" — scalar vertical progress
// =============================================================================

@test "@mouse axis y uses scalar vertical progress for rotate-x" {
  @fixture {
    <div class="t5-parent" style="position: fixed; top: 0; left: 0; width: 200px; height: 200px; perspective: 800px;">
      <div class="t5-child">Vertical</div>
    </div>
  }
  @wait 100ms

  // Mousemove at y=50 out of 200 → scalar progress 0.25
  // rotate-x at 0.25: between -10deg@0 and 0@0.5, local=0.5
  //   → -10 + 10*0.5 = -5deg
  @eval (eval("var el = document.querySelector('.t5-parent'); el.getBoundingClientRect = function() { return {left: 0, top: 0, width: 200, height: 200, right: 200, bottom: 200, x: 0, y: 0}; };"))
  @eval (eval("var el = document.querySelector('.t5-parent'); var r = el.getBoundingClientRect(); el.dispatchEvent(new MouseEvent('mousemove', {clientX: r.left + 100, clientY: r.top + 50, bubbles: true}))"))
  @wait 50ms

  @let (parent = document.querySelector('.t5-parent'))
  @let (transform = parent.style.transform)
  @assert (transform && transform.includes('rotateX(-5deg)')) "rotateX should be -5deg at progress 0.25"
}

// =============================================================================
// Test 6: Multi-property parallax — per-property axis routing
// =============================================================================

@test "@mouse multi-property parallax routes axes per property suffix" {
  @fixture {
    <div class="t6-parent" style="position: fixed; top: 0; left: 0; width: 200px; height: 200px; perspective: 800px;">
      <div class="t6-translate">Translate child</div>
      <div class="t6-rotate">Rotate child</div>
    </div>
  }
  @wait 100ms

  // Mousemove at (160, 40) → progress {x:0.8, y:0.2}
  @eval (eval("var el = document.querySelector('.t6-parent'); el.getBoundingClientRect = function() { return {left: 0, top: 0, width: 200, height: 200, right: 200, bottom: 200, x: 0, y: 0}; };"))
  @eval (eval("var el = document.querySelector('.t6-parent'); var r = el.getBoundingClientRect(); el.dispatchEvent(new MouseEvent('mousemove', {clientX: r.left + 160, clientY: r.top + 40, bubbles: true}))"))
  @wait 50ms

  // translate-x (suffix -x) uses progress.x=0.8: between 0@0.5 and 10px@1.0
  //   local=0.6 → 0 + 10*0.6 = 6px
  // translate-y (suffix -y) uses progress.y=0.2: between -10px@0 and 0@0.5
  //   local=0.4 → -10 + 10*0.4 = -6px
  // rotate-x (suffix -x) uses progress.x=0.8: between 0@0.5 and 5deg@1.0
  //   local=0.6 → 0 + 5*0.6 = 3deg
  // rotate-y (suffix -y) uses progress.y=0.2: between 5deg@0 and 0@0.5
  //   local=0.4 → 5 + (-5)*0.4 = 3deg
  @let (parent = document.querySelector('.t6-parent'))
  @let (transform = parent.style.transform)
  @assert (transform && eval("var t = parent.style.transform; var i = t.indexOf('translateX('); Math.abs(parseFloat(t.slice(i + 11)) - 6) < 0.1")) "translateX should be ~6px (progress.x=0.8)"
  @assert (transform && eval("var t = parent.style.transform; var i = t.indexOf('translateY('); Math.abs(parseFloat(t.slice(i + 11)) - (-6)) < 0.1")) "translateY should be ~-6px (progress.y=0.2)"
  @assert (transform && eval("var t = parent.style.transform; var i = t.indexOf('rotateX('); Math.abs(parseFloat(t.slice(i + 8)) - 3) < 0.1")) "rotateX should be ~3deg (progress.x=0.8)"
  @assert (transform && eval("var t = parent.style.transform; var i = t.indexOf('rotateY('); Math.abs(parseFloat(t.slice(i + 8)) - 3) < 0.1")) "rotateY should be ~3deg (progress.y=0.2)"
}
