/// Unit Tests: Interpolation Module
///
/// Tests keyframe interpolation and property application.

@import "stdlib/testing/test"

// =============================================================================
// Linear Interpolation
// =============================================================================

@test "ST.lerp interpolates between values" {
  @fixture { <div class="test"></div> }

  @assert (ST.lerp(0, 100, 0) === 0) "lerp at 0 should be start"
  @assert (ST.lerp(0, 100, 1) === 100) "lerp at 1 should be end"
  @assert (ST.lerp(0, 100, 0.5) === 50) "lerp at 0.5 should be midpoint"
  @assert (ST.lerp(0, 100, 0.25) === 25) "lerp at 0.25 should be quarter"
}

@test "ST.lerp handles negative values" {
  @fixture { <div class="test"></div> }

  @assert (ST.lerp(-50, 50, 0.5) === 0) "lerp should handle negatives"
  @assert (ST.lerp(100, -100, 0.5) === 0) "lerp should handle reverse"
}

// =============================================================================
// Keyframe Interpolation
// =============================================================================

@test "ST.interpolate with single keyframe returns value" {
  @fixture { <div class="test"></div> }

  @eval (eval("var _k = Object.create(null); _k.at = 0; _k.value = 42; window.__keyframes = [_k]"))
  @assert (ST.interpolate(window.__keyframes, 0.5) === 42)
    "Single keyframe should always return its value"
}

@test "ST.interpolate at keyframe boundaries" {
  @fixture { <div class="test"></div> }

  @eval (eval("var _a = Object.create(null); _a.at = 0; _a.value = 0; var _b = Object.create(null); _b.at = 1; _b.value = 100; window.__keyframes = [_a, _b]"))

  @assert (ST.interpolate(window.__keyframes, 0) === 0) "Should return start at 0"
  @assert (ST.interpolate(window.__keyframes, 1) === 100) "Should return end at 1"
}

@test "ST.interpolate between keyframes" {
  @fixture { <div class="test"></div> }

  @eval (eval("var _a = Object.create(null); _a.at = 0; _a.value = 0; var _b = Object.create(null); _b.at = 1; _b.value = 100; window.__keyframes = [_a, _b]"))

  @let (mid = ST.interpolate(window.__keyframes, 0.5))
  @assert (Math.abs(mid - 50) < 0.01)
    "Midpoint should be 50"
}

@test "ST.interpolate with multiple keyframes" {
  @fixture { <div class="test"></div> }

  @eval (eval("var _a = Object.create(null); _a.at = 0; _a.value = 0; var _b = Object.create(null); _b.at = 0.5; _b.value = 100; var _c = Object.create(null); _c.at = 1; _c.value = 50; window.__keyframes = [_a, _b, _c]"))

  @assert (ST.interpolate(window.__keyframes, 0.25) === 50)
    "Should interpolate in first segment"
  @assert (ST.interpolate(window.__keyframes, 0.75) === 75)
    "Should interpolate in second segment"
}

@test "ST.interpolate with easing" {
  @fixture { <div class="test"></div> }

  @eval (eval("var _a = Object.create(null); _a.at = 0; _a.value = 0; _a.easing = 'easeInQuad'; var _b = Object.create(null); _b.at = 1; _b.value = 100; window.__keyframes = [_a, _b]"))

  @let (mid = ST.interpolate(window.__keyframes, 0.5))
  // With easeInQuad, midpoint should be 25 (0.5^2 = 0.25)
  @assert (Math.abs(mid - 25) < 0.01)
    "Should apply easing function"
}

@test "ST.interpolate with units" {
  @fixture { <div class="test"></div> }

  @eval (eval("var _a = Object.create(null); _a.at = 0; _a.value = '0px'; var _b = Object.create(null); _b.at = 1; _b.value = '100px'; window.__keyframes = [_a, _b]"))

  @let (result = ST.interpolate(window.__keyframes, 0.5))
  @assert (result === '50px' || result === 50 + 'px')
    "Should preserve units"
}

@test "ST.interpolate with colors" {
  @fixture { <div class="test"></div> }

  @eval (eval("var _a = Object.create(null); _a.at = 0; _a.value = '#ff0000'; var _b = Object.create(null); _b.at = 1; _b.value = '#0000ff'; window.__keyframes = [_a, _b]"))

  @let (result = ST.interpolate(window.__keyframes, 0.5))
  @assert (result !== null) "Should return interpolated color"
  // Result should be oklch() format
  @assert (result.startsWith('oklch(') || result.startsWith('rgb'))
    "Should return valid color"
}

// =============================================================================
// Property Application
// =============================================================================

@test "ST.applyProperty sets regular CSS properties" {
  @fixture { <div class="test"></div> }

  @fixture {
    <div class="apply-prop-test">Apply Prop</div>
  }

  @let (el = document.querySelector('.apply-prop-test'))
  @eval (ST.applyProperty(el, 'opacity', 0.5))

  @assert (el.style.opacity === '0.5')
    "Should set opacity"
}

@test "ST.applyProperty sets CSS custom properties" {
  @fixture { <div class="test"></div> }

  @fixture {
    <div class="apply-var-test">Apply Var</div>
  }

  @let (el = document.querySelector('.apply-var-test'))
  @eval (ST.applyProperty(el, '--my-color', 'red'))

  @eval (eval("window.__val = el.style.getPropertyValue('--my-color')"))
  @assert (window.__val === 'red')
    "Should set custom property"
}

@test "ST.applyProperty handles transform functions" {
  @fixture { <div class="test"></div> }

  @fixture {
    <div class="apply-transform-test">Apply Transform</div>
  }

  @let (el = document.querySelector('.apply-transform-test'))
  @eval (ST.applyProperty(el, 'translateX', 100))
  @eval (ST.applyProperty(el, 'rotate', 45))

  @let (transform = el.style.transform)
  @assert (transform.includes('translateX(100px)'))
    "Should include translateX with px"
  @assert (transform.includes('rotate(45deg)'))
    "Should include rotate with deg"
}

@test "ST.applyProperty composes transforms" {
  @fixture { <div class="test"></div> }

  @fixture {
    <div class="compose-test">Compose</div>
  }

  @let (el = document.querySelector('.compose-test'))
  @eval (ST.applyProperty(el, 'translateX', 50))
  @eval (ST.applyProperty(el, 'translateY', 100))
  @eval (ST.applyProperty(el, 'scale', 1.5))

  @let (transform = el.style.transform)
  @assert (transform.includes('translateX') && transform.includes('translateY'))
    "Should compose multiple transforms"
}

// =============================================================================
// setTransform Helper
// =============================================================================

@test "ST.setTransform is alias for applyProperty" {
  @fixture { <div class="test"></div> }

  @fixture {
    <div class="set-transform-test">Set Transform</div>
  }

  @let (el = document.querySelector('.set-transform-test'))
  @eval (ST.setTransform(el, 'scale', 2))

  @assert (el.style.transform.includes('scale(2)'))
    "setTransform should work like applyProperty for transforms"
}
