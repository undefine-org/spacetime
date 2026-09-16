/// BUG-195 (GitHub issue #7, enhancement): there is no declarative macro to
/// bind POINTER COORDINATES to a transform / CSS variable. The `pointer`
/// primitive exists (stdlib/primitives/pointer.st yields $normX/$normY/
/// $centerX/$centerY/$pointerType) but its only consumers are the
/// self-contained @magnetic and @cursor behaviors -- no author-facing driver
/// macro exposes the coords. `@pointer { ... }` is NOT a registered
/// directive: it compiles as an unknown-at-rule passthrough (verified), so
/// nothing binds and nothing animates -- silently.
///
/// WHY IT MATTERS (from the issue): per-element mouse parallax is a common
/// cinematic device; projects/sakazuki had to fall back to a static
/// scroll-scrubbed scatter because pointer coords can't drive a transform
/// declaratively. Issue's suggested direction: a @pointer/@track macro (or
/// pointer-driver analogous to scroll-driver) exposing $centerX/$centerY as
/// a bindable driver, e.g.
///     @pointer(scope: &self) { translate-x: 0 -> $centerX * 40px }
///
/// WHAT EXISTS TODAY (pinned green as a control below): @mouse
/// (stdlib/macros/timeline.st %macro mouse-timeline) is a scalar-progress
/// driver -- it maps pointer position to a single $progress 0..1 (or {x,y}
/// with axis:"both") and scrubs keyframes with it. That covers tilt-style
/// effects but NOT coord-binding into expressions ($centerX * 40px), and the
/// issue explicitly asks for the coord surface. The control documents the
/// existing surface so the gap stays precise: scalar progress EXISTS,
/// coord-binding / @pointer does NOT.
///
/// Expected RED until a @pointer/@track (or equivalent coord-binding)
/// directive is registered and actually drives transforms. The RED test
/// below uses the issue's suggested @pointer shape; if the shipped surface
/// ends up named/shaped differently, update the test to the shipped syntax
/// -- the BEHAVIORAL claim (pointer coords declaratively drive a transform)
/// is the pin.
///
/// NOTE: the driver is declared at FILE SCOPE and the @test body contains
/// markup + probes only -- an animation-keyframe directive placed INSIDE a
/// @test body is compiled away entirely (BUG-194), which would make this
/// test vacuously green.

@version 2026-07-26;

@import "stdlib/testing/test"

// RED surface: @pointer is not a registered directive. Today this block is
// an unknown-at-rule passthrough -- it binds nothing, so the element's
// transform is never set no matter where the pointer moves.
.bug195-parallax-target {
    @on &.pointer(name: bug195-parallax, axis: x, invert: false) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                translate-x: -40px -> 0 -> 40px;
      }
}

// GREEN control surface: @mouse is registered and drives a scalar progress.
.bug195-mouse-target {
    @on &.mouse(name: bug195-mouse-control, axis: x, invert: false) {
        easing: linear;
        stagger: 0;
        staggerFrom: first;
                translate-x: 0 -> 100px;
      }
}

@test "pointer coords bind declaratively to a transform via @pointer (RED, BUG-195)" {
    @fixture {
        <div class="bug195-parallax-target" style="position: fixed; top: 0; left: 0; width: 200px; height: 200px;">x</div>
    }
    @wait 100ms
    // Headless LinkeDOM has no layout: getBoundingClientRect() is all zeros
    // and the mouse driver's handleMove early-returns on rect.width === 0
    // (stdlib/primitives/animation/drivers.st). Shim a real rect on the
    // target element (same pattern as BUG-190's matchMedia shim) so the
    // pointer-position path is actually exercised.
    @eval (eval("var el = document.querySelector('.bug195-parallax-target'); el.getBoundingClientRect = function() { return {left: 0, top: 0, width: 200, height: 200, right: 200, bottom: 200, x: 0, y: 0}; }"))
    // Move the pointer to the element's right edge: with a bound coord
    // driver the translate-x must leave its neutral/initial state.
    @eval (eval("var el = document.querySelector('.bug195-parallax-target'); el.dispatchEvent(new MouseEvent('mousemove', {clientX: 200, clientY: 100, bubbles: true}))"))
    @wait 100ms
    @let (el = document.querySelector('.bug195-parallax-target'))
    @let (t = el.style.transform)
    // NB: in the headless LinkeDOM world an unset inline style.transform is
    // `undefined` (NOT ""), so a bare `t !== ""` check is vacuously true --
    // assert a genuinely set value instead.
    @assert (t && t !== "" && t !== "none")
        "@pointer must register as a real driver and set a transform from pointer position -- today it is an unknown-at-rule passthrough that binds nothing (BUG-195 / issue #7)"
}

@test "CONTROL: the existing @mouse scalar-progress driver DOES drive transforms (must stay green)" {
    @fixture {
        <div class="bug195-mouse-target" style="position: fixed; top: 0; left: 0; width: 200px; height: 200px;">x</div>
    }
    @wait 100ms
    // Same headless-no-layout shim as the RED test: give the target a real
    // rect so the driver's handleMove rect.width === 0 guard passes.
    @eval (eval("var el = document.querySelector('.bug195-mouse-target'); el.getBoundingClientRect = function() { return {left: 0, top: 0, width: 200, height: 200, right: 200, bottom: 200, x: 0, y: 0}; }"))
    @eval (eval("var el = document.querySelector('.bug195-mouse-target'); el.dispatchEvent(new MouseEvent('mousemove', {clientX: 200, clientY: 100, bubbles: true}))"))
    @wait 100ms
    @let (el = document.querySelector('.bug195-mouse-target'))
    @let (t = el.style.transform)
    @assert (t && t !== "" && t !== "none")
        "control: @mouse (the existing scalar-progress surface) must set a transform after pointer movement with a shimmed layout rect"
}
