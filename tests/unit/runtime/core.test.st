/// Unit Tests: Runtime Core Module (st.js)
///
/// Tests the core ST runtime functionality: signals, cleanup, watch, derive.

@import "stdlib/testing/test"

// =============================================================================
// Signal Storage
// =============================================================================

@test "ST.set stores value in signal store" {
  @fixture {
    <div class="store-test">Store Test</div>
  }

  @let (el = document.querySelector('.store-test'))
  @eval (ST.set(el, 'count', 42))

  @assert (ST.get(el, 'count') === 42)
    "Signal value should be stored and retrievable"
}

@test "ST.set updates CSS variable" {
  @fixture {
    <div class="css-var-test">CSS Var Test</div>
  }

  @let (el = document.querySelector('.css-var-test'))
  @eval (ST.set(el, 'progress', 0.5))

  // Check CSS variable was set
  @eval (eval("window.__cssValue = el.style.getPropertyValue('--st-progress')"))
  @assert (__cssValue === '0.5')
    "CSS variable should be set automatically"
}

@test "ST.set skips update when value unchanged" {
  @fixture {
    <div class="skip-update-test">Skip Update</div>
  }

  @let (el = document.querySelector('.skip-update-test'))
  @eval (eval("window.__callCount = 0"))

  @eval (ST.set(el, 'value', 10))
  @eval (eval("ST.watch(el, 'value', function() { window.__callCount++ })"))

  // Initial watch call
  @assert (window.__callCount === 1) "Initial watch should fire"

  // Set same value - should not trigger
  @eval (ST.set(el, 'value', 10))

  // Microtask to allow batched updates
  @flush

  @assert (window.__callCount === 1)
    "Setting same value should not trigger watcher"
}

// =============================================================================
// Watch System
// =============================================================================

@test "ST.watch calls callback with current value" {
  @fixture {
    <div class="watch-current-test">Watch Current</div>
  }

  @let (el = document.querySelector('.watch-current-test'))
  @eval (ST.set(el, 'name', 'Alice'))

  @eval (eval("window.__receivedValue = null; ST.watch(el, 'name', function(v) { window.__receivedValue = v })"))

  @assert (window.__receivedValue === 'Alice')
    "Watch should immediately receive current value"
}

@test "ST.watch returns unsubscribe function" {
  @fixture {
    <div class="unsub-test">Unsub Test</div>
  }

  @let (el = document.querySelector('.unsub-test'))
  @eval (ST.set(el, 'counter', 0))
  @eval (eval("window.__callCount = 0; window.__unsub = ST.watch(el, 'counter', function() { window.__callCount++ })"))
  @eval (ST.set(el, 'counter', 1))
  @flush

  @assert (window.__callCount === 2) "Should have been called twice"

  @eval (window.__unsub())
  @eval (ST.set(el, 'counter', 2))
  @flush

  @assert (window.__callCount === 2)
    "Should not be called after unsubscribe"
}

@test "ST.watchAll waits for all signals" {
  @fixture {
    <div class="watch-all-test">Watch All</div>
  }

  @let (el = document.querySelector('.watch-all-test'))
  @eval (eval("window.__received = null"))

  @eval (ST.set(el, 'x', 10))
  @eval (eval("ST.watchAll(el, ['x', 'y'], function(vals) { window.__received = vals })"))

  @assert (window.__received === null)
    "Should not fire until all signals have values"

  @eval (ST.set(el, 'y', 20))
  @flush

  @assert (window.__received !== null)
    "Should fire when all signals are set"
  @assert (window.__received.x === 10 && window.__received.y === 20)
    "Should receive all values"
}

// =============================================================================
// Derived Signals
// =============================================================================

@test "ST.derive computes from dependencies" {
  @fixture {
    <div class="derive-test">Derive Test</div>
  }

  @let (el = document.querySelector('.derive-test'))
  @eval (ST.set(el, 'width', 100))
  @eval (ST.set(el, 'height', 50))

  @eval (eval("ST.derive(el, 'area', ['width', 'height'], function(v) { return v.width * v.height })"))

  @flush

  @assert (ST.get(el, 'area') === 5000)
    "Derived value should be computed"
}

@test "ST.derive updates when dependencies change" {
  @fixture {
    <div class="derive-update-test">Derive Update</div>
  }

  @let (el = document.querySelector('.derive-update-test'))
  @eval (ST.set(el, 'a', 5))
  @eval (ST.set(el, 'b', 3))

  @eval (eval("ST.derive(el, 'sum', ['a', 'b'], function(v) { return v.a + v.b })"))
  @flush

  @assert (ST.get(el, 'sum') === 8) "Initial sum should be 8"

  @eval (ST.set(el, 'a', 10))
  @flush

  @assert (ST.get(el, 'sum') === 13)
    "Sum should update when dependency changes"
}

// =============================================================================
// Cleanup
// =============================================================================

@test "ST.onCleanup registers cleanup function" {
  @fixture {
    <div class="cleanup-test">Cleanup Test</div>
  }

  @let (el = document.querySelector('.cleanup-test'))
  @eval (eval("window.__cleaned = false; ST.onCleanup(el, function() { window.__cleaned = true })"))

  @assert (window.__cleaned === false) "Should not clean up yet"

  @eval (ST.dispose(el))

  @assert (window.__cleaned === true) "Should have run cleanup"
}

@test "ST.dispose clears signal store" {
  @fixture {
    <div class="dispose-test">Dispose Test</div>
  }

  @let (el = document.querySelector('.dispose-test'))
  @eval (eval("ST.set(el, 'data', __obj('key', 'value'))"))

  @assert (ST.get(el, 'data') !== undefined) "Data should exist"

  @eval (ST.dispose(el))

  @assert (ST.get(el, 'data') === undefined)
    "Data should be cleared after dispose"
}

// =============================================================================
// State Binding
// =============================================================================

@test "ST.bindState sets data-st-state attribute" {
  @fixture {
    <div class="bind-state-test">Bind State</div>
  }

  @let (el = document.querySelector('.bind-state-test'))
  @eval (ST.set(el, 'active', false))
  @eval (ST.bindState(el, 'active', 'active'))

  @assert (el.dataset.stState !== 'active')
    "State should not be set initially"

  @eval (ST.set(el, 'active', true))
  @flush

  @assert (el.dataset.stState === 'active')
    "State should be set when signal becomes truthy"
}

@test "ST.bindStyle applies style changes" {
  @fixture {
    <div class="bind-style-test">Bind Style</div>
  }

  @let (el = document.querySelector('.bind-style-test'))
  @eval (ST.set(el, 'opacity', 1))
  @eval (ST.bindStyle(el, 'opacity', 'opacity'))

  @assert (el.style.opacity === '1') "Initial opacity should be set"

  @eval (ST.set(el, 'opacity', 0.5))
  @flush

  @assert (el.style.opacity === '0.5')
    "Opacity should update with signal"
}

// =============================================================================
// Timing Instrumentation
// =============================================================================

@test "ST._timing.enable and report work" {
  @fixture { <div class="test"></div> }
  @eval (ST._timing.enable())

  // Simulate some RAF recordings
  @eval (ST._timing.recordRAF(0))
  @eval (ST._timing.recordRAF(16.67))
  @eval (ST._timing.recordRAF(33.34))

  @eval (eval("window.__report = ST._timing.report()"))

  @assert (window.__report.sampleCount === 2)
    "Should have recorded 2 delta samples"
  @assert (window.__report.avgDelta > 0)
    "Average delta should be positive"

  @eval (ST._timing.disable())
}
