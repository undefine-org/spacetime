/// Unit Tests: Value Functions Module
///
/// Tests dynamic value functions: wave, random, noise, parallax.

@import "stdlib/testing/test"

// =============================================================================
// Wave Function
// =============================================================================

@test "stWave returns 0 at progress 0 for sine" {
  @fixture { <div class="test"></div> }

  @let (value = ST.stWave(0, 'sin', 1, 1, 0))
  @assert (Math.abs(value) < 0.01)
    "Sine wave should be 0 at start"
}

@test "stWave returns 1 at quarter for sine" {
  @fixture { <div class="test"></div> }

  @let (value = ST.stWave(0.25, 'sin', 1, 1, 0))
  @assert (Math.abs(value - 1) < 0.01)
    "Sine wave should be 1 at quarter cycle"
}

@test "stWave cosine starts at 1" {
  @fixture { <div class="test"></div> }

  @let (value = ST.stWave(0, 'cos', 1, 1, 0))
  @assert (Math.abs(value - 1) < 0.01)
    "Cosine wave should be 1 at start"
}

@test "stWave amplitude scales output" {
  @fixture { <div class="test"></div> }

  @let (value = ST.stWave(0.25, 'sin', 1, 10, 0))
  @assert (Math.abs(value - 10) < 0.01)
    "Amplitude should scale wave"
}

@test "stWave frequency affects period" {
  @fixture { <div class="test"></div> }

  // At freq=2, quarter cycle happens at progress 0.125
  @let (value = ST.stWave(0.125, 'sin', 2, 1, 0))
  @assert (Math.abs(value - 1) < 0.01)
    "Higher frequency should shorten period"
}

@test "stWave phase shifts wave" {
  @fixture { <div class="test"></div> }

  // Phase 0.25 shifts sine by quarter cycle, making it act like cosine
  @let (value = ST.stWave(0, 'sin', 1, 1, 0.25))
  @assert (Math.abs(value - 1) < 0.01)
    "Phase should shift wave"
}

// =============================================================================
// Random Function
// =============================================================================

@test "stRandom returns value in range" {
  @fixture { <div class="test"></div> }

  @eval (eval("for (var seed = 0; seed < 10; seed++) { var value = ST.stRandom(0, 100, seed); if (value < 0 || value > 100) throw new Error('Random value ' + value + ' should be in [0, 100]') }"))

  @assert (true) "All random values were in range"
}

@test "stRandom is deterministic with same seed" {
  @fixture { <div class="test"></div> }

  @let (first = ST.stRandom(0, 100, 42))
  @let (second = ST.stRandom(0, 100, 42))

  @assert (first === second)
    "Same seed should produce same value"
}

@test "stRandom varies with different seeds" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.__values = []; for (var i = 0; i < 5; i++) { window.__values.push(ST.stRandom(0, 100, i)) }"))

  // Check that not all values are the same
  @eval (eval("window.__unique = new Set(window.__values).size"))
  @assert (window.__unique > 1)
    "Different seeds should produce different values"
}

// =============================================================================
// Noise Function
// =============================================================================

@test "stNoise returns bounded values" {
  @fixture { <div class="test"></div> }

  @eval (eval("for (var idx = 0; idx < 20; idx++) { var value = ST.stNoise(0.5, 1, idx); if (value < -2 || value > 2) throw new Error('Noise value ' + value + ' should be bounded') }"))

  @assert (true) "All noise values were bounded"
}

@test "stNoise varies smoothly" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.__noiseValues = []; for (var i = 0; i < 10; i++) { window.__noiseValues.push(ST.stNoise(0.1, 1, i)) }"))

  // Adjacent values should not differ too drastically
  @eval (eval("window.__smoothOk = true; for (var i = 1; i < window.__noiseValues.length; i++) { var diff = Math.abs(window.__noiseValues[i] - window.__noiseValues[i-1]); if (diff >= 1) { window.__smoothOk = false; break } }"))

  @assert (window.__smoothOk === true)
    "Adjacent noise values should not jump too much"
}

@test "stNoise frequency affects variation" {
  @fixture { <div class="test"></div> }

  // Low frequency should change slowly
  @eval (eval("window.__lowFreq = []; for (var i = 0; i < 5; i++) { window.__lowFreq.push(ST.stNoise(0.1, 1, i)) }"))

  // High frequency should change faster
  @eval (eval("window.__highFreq = []; for (var i = 0; i < 5; i++) { window.__highFreq.push(ST.stNoise(2, 1, i)) }"))

  // High frequency should have more local variation
  @assert (window.__lowFreq[0] !== window.__highFreq[0])
    "Different frequencies should produce different patterns"
}

// =============================================================================
// Parallax Function
// =============================================================================

@test "stParallax at progress 0 returns 0" {
  @fixture { <div class="test"></div> }

  @let (value = ST.stParallax(100, 0))
  @assert (value === 0)
    "Parallax at 0 progress should be 0"
}

@test "stParallax at progress 1 returns full range" {
  @fixture { <div class="test"></div> }

  @let (value = ST.stParallax(100, 1))
  @assert (value === 100)
    "Parallax at full progress should be full range"
}

@test "stParallax scales linearly" {
  @fixture { <div class="test"></div> }

  @assert (ST.stParallax(100, 0.25) === 25) "25% progress"
  @assert (ST.stParallax(100, 0.5) === 50) "50% progress"
  @assert (ST.stParallax(100, 0.75) === 75) "75% progress"
}

@test "stParallax handles negative ranges" {
  @fixture { <div class="test"></div> }

  @let (value = ST.stParallax(-50, 0.5))
  @assert (value === -25)
    "Should handle negative ranges"
}

// =============================================================================
// Spring Function
// =============================================================================

@test "stSpring moves toward target" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.__state = __obj('v', 0); window.__current = 0; for (var i = 0; i < 100; i++) { window.__current = ST.stSpring(window.__current, 100, __obj('stiffness', 100, 'damping', 10), window.__state) }"))

  @assert (window.__current > 50)
    "Spring should move toward target"
}

@test "stSpring settles at target" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.__state = __obj('v', 0); window.__current = 0; for (var i = 0; i < 500; i++) { window.__current = ST.stSpring(window.__current, 100, __obj('stiffness', 100, 'damping', 20), window.__state) }"))

  @assert (Math.abs(window.__current - 100) < 1)
    "Spring should settle near target"
}

// =============================================================================
// resolveValue
// =============================================================================

@test "resolveValue passes through regular values" {
  @fixture { <div class="test"></div> }

  @assert (ST.resolveValue(42) === 42) "Number should pass through"
  @assert (ST.resolveValue('hello') === 'hello') "String should pass through"
  @assert (ST.resolveValue('100px') === '100px') "Unit string should pass through"
}

@test "resolveValue evaluates __st_fn_ placeholders" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.ctx = __obj('progress', 0.25, 'elementIndex', 0)"))
  @let (value = ST.resolveValue('__st_fn_wave(sin,1,10,0)', ctx))

  @eval (eval("window.__tc = typeof value === 'number'"))
  @assert (window.__tc === true) "Should evaluate to number"
  @assert (Math.abs(value - 10) < 0.01) "Should be wave value at 0.25"
}

// =============================================================================
// evaluateRuntimeFunction
// =============================================================================

@test "evaluateRuntimeFunction handles wave" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.ctx = __obj('progress', 0.5)"))
  @eval (eval("window.__value = ST.evaluateRuntimeFunction('wave', ['sin', 1, 10, 0], ctx)"))

  @assert (Math.abs(window.__value) < 0.01)
    "Wave at 0.5 progress should be ~0"
}

@test "evaluateRuntimeFunction handles parallax" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.ctx = __obj('progress', 0.5)"))
  @eval (eval("window.__value = ST.evaluateRuntimeFunction('parallax', [100], ctx)"))

  @assert (window.__value === 50)
    "Parallax with 100 range at 0.5 should be 50"
}

@test "evaluateRuntimeFunction warns for unknown functions" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.__value = ST.evaluateRuntimeFunction('unknown', [], {})"))
  @assert (window.__value === 0)
    "Unknown function should return 0"
}
