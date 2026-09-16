/// Unit Tests: Easing Module
///
/// Tests the easing functions for smooth animations.

@import "stdlib/testing/test"

// =============================================================================
// Basic Easing Functions
// =============================================================================

@test "linear easing is identity function" {
  @fixture { <div class="test"></div> }

  @assert (ST.easings.linear(0) === 0) "linear(0) should be 0"
  @assert (ST.easings.linear(0.5) === 0.5) "linear(0.5) should be 0.5"
  @assert (ST.easings.linear(1) === 1) "linear(1) should be 1"
}

@test "easeInQuad starts slow, ends fast" {
  @fixture { <div class="test"></div> }

  @let (start = ST.easings.easeInQuad(0.25))
  @let (mid = ST.easings.easeInQuad(0.5))
  @let (end = ST.easings.easeInQuad(0.75))

  @assert (start < 0.25) "Should be slower at start"
  @assert (mid === 0.25) "midpoint should be 0.25"
  @assert (end > 0.5) "Should be faster at end"
}

@test "easeOutQuad starts fast, ends slow" {
  @fixture { <div class="test"></div> }

  @let (start = ST.easings.easeOutQuad(0.25))
  @let (mid = ST.easings.easeOutQuad(0.5))
  @let (end = ST.easings.easeOutQuad(0.75))

  @assert (start > 0.25) "Should be faster at start"
  @assert (end < 1) "Should slow down at end"
}

@test "easeInOutQuad symmetric around midpoint" {
  @fixture { <div class="test"></div> }

  @let (q1 = ST.easings.easeInOutQuad(0.25))
  @let (q3 = ST.easings.easeInOutQuad(0.75))

  // Should be symmetric: f(0.25) + f(0.75) = 1
  @assert (Math.abs((q1 + q3) - 1) < 0.01)
    "easeInOut should be symmetric"
}

// =============================================================================
// All Easings Have Correct Boundary Values
// =============================================================================

@test "all easings start at 0 and end at 1" {
  @fixture { <div class="test"></div> }

  // @let binds across steps (a bare `var` inside a nested eval() would be trapped
  // in that eval's scope and invisible to the later @assert).
  @let (easingNames = 'linear,easeInQuad,easeOutQuad,easeInOutQuad,easeInCubic,easeOutCubic,easeInOutCubic,easeInQuart,easeOutQuart,easeInOutQuart,easeInExpo,easeOutExpo,easeInOutExpo,easeInCirc,easeOutCirc,easeInOutCirc,easeInBack,easeOutBack,easeInOutBack,easeOutElastic,easeOutBounce'.split(','))

  @eval (eval("for (var i = 0; i < easingNames.length; i++) { var nm = easingNames[i]; var fn = ST.easings[nm]; if (fn === undefined) throw new Error('Easing ' + nm + ' should exist'); if (Math.abs(fn(0) - 0) >= 0.001) throw new Error(nm + '(0) should be ~0'); if (Math.abs(fn(1) - 1) >= 0.001) throw new Error(nm + '(1) should be ~1'); }"))

  @assert (easingNames.length === 21) "Should have tested 21 standard easings"

  // Spring easings have slight tolerance on end value (damping may not reach exactly 1)
  @eval (eval("var springNames = 'springGentle,springBouncy,springStiff'.split(','); for (var i = 0; i < springNames.length; i++) { var name = springNames[i]; var fn = ST.easings[name]; if (fn === undefined) throw new Error('Spring easing ' + name + ' should exist'); if (Math.abs(fn(0) - 0) >= 0.001) throw new Error(name + '(0) should be ~0'); if (Math.abs(fn(1) - 1) >= 0.05) throw new Error(name + '(1) should be ~1 (within 0.05)'); }"))
}

// =============================================================================
// getEasing Helper
// =============================================================================

@test "ST.getEasing returns function by name" {
  @fixture { <div class="test"></div> }

  @let (ease = ST.getEasing('easeOutQuad'))
  @eval (eval("window.__tc = typeof ease === 'function'"))
  @assert (window.__tc === true)
    "getEasing should return a function"
  @assert (ease(0.5) === ST.easings.easeOutQuad(0.5))
    "Should be the same function"
}

@test "ST.getEasing returns linear for unknown" {
  @fixture { <div class="test"></div> }

  @let (ease = ST.getEasing('nonexistent'))
  @assert (ease(0.5) === 0.5)
    "Unknown easing should default to linear"
}

@test "ST.getEasing passes through functions" {
  @fixture { <div class="test"></div> }

  @let (custom = function(t) { return t * t * t })

  @let (ease = ST.getEasing(custom))
  @assert (ease === custom)
    "Should return custom function as-is"
}

// =============================================================================
// CSS Naming Aliases
// =============================================================================

@test "CSS ease aliases work" {
  @fixture { <div class="test"></div> }

  @assert (ST.easings['ease'] !== undefined)
    "ease alias should exist"
  @assert (ST.easings['ease-in'] !== undefined)
    "ease-in alias should exist"
  @assert (ST.easings['ease-out'] !== undefined)
    "ease-out alias should exist"
  @assert (ST.easings['ease-in-out'] !== undefined)
    "ease-in-out alias should exist"
}

// =============================================================================
// Elastic and Bounce Characteristics
// =============================================================================

@test "easeOutElastic overshoots" {
  @fixture { <div class="test"></div> }

  // Elastic easing should overshoot (go above 1) at some point
  @eval (eval("var overshot = false; for (var t = 0; t <= 1; t += 0.01) { if (ST.easings.easeOutElastic(t) > 1) { overshot = true; break; } } window.__overshot = overshot"))

  @assert (window.__overshot === true)
    "Elastic easing should overshoot at some point"
}

@test "easeOutBounce stays in bounds" {
  @fixture { <div class="test"></div> }

  // Bounce should stay between 0 and 1
  @eval (eval("var allInBounds = true; for (var t = 0; t <= 1; t += 0.01) { var v = ST.easings.easeOutBounce(t); if (v < 0 || v > 1) { allInBounds = false; break; } } window.__allInBounds = allInBounds"))

  @assert (window.__allInBounds === true)
    "Bounce values should all be in [0,1]"
}

// =============================================================================
// Easing resolution (--ease-* forms)
// =============================================================================

@test "--ease-out-expo resolves to easing function" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.ease = ST.getEasing('ease-out-expo')"))
  @eval (eval("window.__typecheck = typeof ease === 'function'"))
  @assert (window.__typecheck === true)
    "ease-out-expo should resolve to a function"
  @eval (window.__notLinear = ease(0.5) !== 0.5)
  @assert (window.__notLinear === true)
    "ease-out-expo should not be linear (f(0.5) !== 0.5)"
}

@test "--ease-out-back resolves to easing function" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.ease = ST.getEasing('ease-out-back')"))
  @eval (eval("window.__typecheck2 = typeof ease === 'function'"))
  @assert (window.__typecheck2 === true)
    "ease-out-back should resolve to a function"
  @assert (ease(1) === 1)
    "ease-out-back(1) should equal 1"
}

// =============================================================================
// Spring Easings
// =============================================================================

@test "spring-gentle resolves via getEasing kebab conversion" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.__sg = ST.getEasing('spring-gentle')"))
  @eval (eval("window.__sgType = typeof window.__sg === 'function'"))
  @assert (window.__sgType === true)
    "spring-gentle should resolve to a function"
  @assert (Math.abs(window.__sg(0)) < 0.001)
    "spring-gentle(0) should be ~0"
  @assert (Math.abs(window.__sg(1) - 1) < 0.05)
    "spring-gentle(1) should be ~1"
}

@test "spring-bouncy resolves and overshoots" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.__sb = ST.getEasing('spring-bouncy')"))
  @eval (eval("window.__sbType = typeof window.__sb === 'function'"))
  @assert (window.__sbType === true)
    "spring-bouncy should resolve to a function"

  // Bouncy spring should overshoot 1 at some point
  @eval (eval("var overshot = false; for (var t = 0; t <= 1; t += 0.01) { if (window.__sb(t) > 1) { overshot = true; break; } } window.__sbOver = overshot"))
  @assert (window.__sbOver === true)
    "spring-bouncy should overshoot at some point"
}

@test "spring-stiff converges quickly" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.__ss = ST.getEasing('spring-stiff')"))
  @eval (eval("window.__ssType = typeof window.__ss === 'function'"))
  @assert (window.__ssType === true)
    "spring-stiff should resolve to a function"
  @assert (Math.abs(window.__ss(1) - 1) < 0.01)
    "spring-stiff(1) should be ~1"

  // At t=0.5, stiff spring should already be close to 1
  @eval (eval("window.__ssMid = window.__ss(0.5)"))
  @assert (window.__ssMid > 0.9)
    "spring-stiff should be near target by t=0.5"
}

@test "all spring easings start at 0" {
  @fixture { <div class="test"></div> }

  @assert (Math.abs(ST.easings.springGentle(0)) < 0.001)
    "springGentle(0) should be ~0"
  @assert (Math.abs(ST.easings.springBouncy(0)) < 0.001)
    "springBouncy(0) should be ~0"
  @assert (Math.abs(ST.easings.springStiff(0)) < 0.001)
    "springStiff(0) should be ~0"
}
