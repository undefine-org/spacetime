/// Unit Tests: Stagger Module
///
/// Tests stagger delay calculations for coordinated animations.

@import "stdlib/testing/test"

// =============================================================================
// Basic Stagger - From First
// =============================================================================

@test "calculateStagger from first element" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.config = __obj('delay', 100, 'from', 'first')"))

  @assert (ST.calculateStagger(0, 5, config) === 0)
    "First element should have no delay"
  @assert (ST.calculateStagger(4, 5, config) === 100)
    "Last element should have full delay"
}

@test "calculateStagger from first with intermediate elements" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.config = __obj('delay', 100, 'from', 'first')"))

  @let (second = ST.calculateStagger(1, 5, config))
  @let (third = ST.calculateStagger(2, 5, config))

  @assert (second === 25) "Second element should have 25% delay"
  @assert (third === 50) "Third element should have 50% delay"
}

// =============================================================================
// Stagger - From Last
// =============================================================================

@test "calculateStagger from last element" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.config = __obj('delay', 100, 'from', 'last')"))

  @assert (ST.calculateStagger(4, 5, config) === 0)
    "Last element should have no delay"
  @assert (ST.calculateStagger(0, 5, config) === 100)
    "First element should have full delay"
}

// =============================================================================
// Stagger - From Center
// =============================================================================

@test "calculateStagger from center" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.config = __obj('delay', 100, 'from', 'center')"))

  // With 5 elements, center is index 2
  @let (center = ST.calculateStagger(2, 5, config))
  @let (edge = ST.calculateStagger(0, 5, config))

  @assert (center === 0) "Center element should have no delay"
  @assert (edge === 100) "Edge elements should have full delay"
}

@test "calculateStagger from center is symmetric" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.config = __obj('delay', 100, 'from', 'center')"))

  @let (first = ST.calculateStagger(0, 5, config))
  @let (last = ST.calculateStagger(4, 5, config))

  @assert (first === last)
    "First and last should have equal delay (symmetric)"
}

// =============================================================================
// Stagger - From Index
// =============================================================================

@test "calculateStagger from specific index" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.config = __obj('delay', 100, 'from', 1)"))

  @let (atIndex = ST.calculateStagger(1, 5, config))
  @let (away = ST.calculateStagger(4, 5, config))

  @assert (atIndex === 0) "Element at from index should have no delay"
  @assert (away > 0) "Elements away from index should have delay"
}

// =============================================================================
// Stagger - Random
// =============================================================================

@test "calculateStagger random is within range" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.config = __obj('delay', 100, 'from', 'random')"))

  @eval (eval("window.__rangeOk = true; for (var i = 0; i < 10; i++) { var delay = ST.calculateStagger(i, 10, config); if (delay < 0 || delay > 100) { window.__rangeOk = false; break } }"))
  @assert (window.__rangeOk === true)
    "All random delays should be in [0, 100]"
}

// =============================================================================
// Grid Stagger
// =============================================================================

@test "calculateStagger with grid" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.config = __obj('delay', 100, 'grid', [3, 3])"))

  // Center of 3x3 grid is index 4 (row 1, col 1)
  @let (center = ST.calculateStagger(4, 9, config))
  @let (corner = ST.calculateStagger(0, 9, config))

  @assert (center === 0) "Center of grid should have no delay"
  @assert (corner > 0) "Corner of grid should have delay"
}

@test "calculateStagger grid corners have equal delay" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.config = __obj('delay', 100, 'grid', [3, 3])"))

  @let (topLeft = ST.calculateStagger(0, 9, config))
  @let (topRight = ST.calculateStagger(2, 9, config))
  @let (bottomLeft = ST.calculateStagger(6, 9, config))
  @let (bottomRight = ST.calculateStagger(8, 9, config))

  // All corners should be equidistant from center
  @assert (Math.abs(topLeft - topRight) < 0.01)
    "Top corners should have equal delay"
  @assert (Math.abs(topLeft - bottomLeft) < 0.01)
    "Left corners should have equal delay"
  @assert (Math.abs(topLeft - bottomRight) < 0.01)
    "Opposite corners should have equal delay"
}

// =============================================================================
// Edge Cases
// =============================================================================

@test "calculateStagger with no config returns 0" {
  @fixture { <div class="test"></div> }

  @assert (ST.calculateStagger(0, 5, null) === 0)
    "No config should return 0"
  @eval (eval("window.__emptyConfig = {}"))
  @assert (ST.calculateStagger(0, 5, window.__emptyConfig) === 0)
    "Empty config should return 0"
  @eval (eval("window.__zeroDelay = __obj('delay', 0)"))
  @assert (ST.calculateStagger(0, 5, window.__zeroDelay) === 0)
    "Zero delay should return 0"
}

@test "calculateStagger with single element" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.config = __obj('delay', 100, 'from', 'first')"))

  // With only 1 element, denominator is 1 (clamped from 0)
  @let (delay = ST.calculateStagger(0, 1, config))
  @assert (delay === 0)
    "Single element should have no delay"
}

// =============================================================================
// ST.stagger Helper
// =============================================================================

@test "ST.stagger creates config object" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.config = ST.stagger(100, __obj('from', 'center'))"))

  @assert (config.delay === 100) "Should have delay"
  @eval (eval("window.__configFrom = config['from']"))
  @assert (window.__configFrom === 'center') "Should have from"
}

@test "ST.stagger defaults to first" {
  @fixture { <div class="test"></div> }

  @eval (eval("window.config = ST.stagger(50)"))

  @eval (eval("window.__configFrom = config['from']"))
  @assert (window.__configFrom === 'first')
    "Should default to 'first'"
}
