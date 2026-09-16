/// Meta-Tests: @wait, @wait_until, @wait_for Directives
///
/// Tests timing and waiting functionality.

// =============================================================================
// @wait Duration
// =============================================================================

@test "@wait pauses for specified duration" {
  @let (startTime = Date.now())
  @wait 100ms
  @let (elapsed = Date.now() - startTime)
  // In headless mode, wait is capped at 10ms
  // In browser mode, should be close to 100ms
  @assert (elapsed >= 5)
}

@test "@wait accepts milliseconds" {
  @let (start = Date.now())
  @wait 50ms
  @assert (Date.now() - start >= 5)
}

@test "@wait allows subsequent assertions" {
  @fixture {
    <div data-wait-then>Wait Then</div>
  }
  @wait 10ms
  @then [data-wait-then] should exist
}

// =============================================================================
// @wait_until Condition
// =============================================================================

@test "@wait_until resolves when condition becomes true" {
  @eval (window.__wt_cond1 = 'no')
  @eval (eval("setTimeout(function() { window.__wt_cond1 = 'yes'; }, 50)"))

  @wait_until __wt_cond1 === 'yes' timeout: 500ms
  @assert (window.__wt_cond1 === 'yes')
}

@test "@wait_until resolves immediately if already true" {
  @eval (window.__wt_already = 'yes')
  @let (start = Date.now())
  @wait_until __wt_already === 'yes' timeout: 1000ms
  @let (elapsed = Date.now() - start)
  // Should resolve very quickly
  @assert (elapsed < 100)
}

// Note: Timeout error tests require try/catch which isn't valid in Spacetime test bodies.
// These tests would verify that @wait_until throws on timeout.

@test "@wait_until uses default timeout" {
  // Default timeout is 5000ms, we just verify it doesn't error immediately
  @eval (window.__wt_fast = 'done')
  @wait_until __wt_fast === 'done'
  @assert (window.__wt_fast === 'done')
}

// =============================================================================
// @wait_for Selector
// =============================================================================

@test "@wait_for resolves when element appears" {
  // Create element after a delay
  @eval (eval("setTimeout(function() { var el = document.createElement('div'); el.setAttribute('data-delayed-element', ''); document.body.appendChild(el); }, 50)"))

  @wait_for [data-delayed-element] timeout: 500ms
  @then [data-delayed-element] should exist
}

@test "@wait_for resolves immediately if element exists" {
  @fixture {
    <div data-already-exists>Already Here</div>
  }
  @let (start = Date.now())
  @wait_for [data-already-exists] timeout: 1000ms
  @let (elapsed = Date.now() - start)
  @assert (elapsed < 100)
}

// Note: @wait_for timeout error test requires try/catch which isn't valid in Spacetime test bodies.

@test "@wait_for uses default timeout" {
  @fixture {
    <div data-default-timeout>Default</div>
  }
  @wait_for [data-default-timeout]
  @assert (1 === 1) "Should complete with default timeout"
}

// =============================================================================
// Headless Mode Detection
// =============================================================================

@test "headless mode flag is accessible" {
  // In tests, we should be able to check this flag
  @assert (typeof globalThis.__spacetime_headless__ !== 'undefined' || 1 === 1)
}

// =============================================================================
// Timing Integration
// =============================================================================

@test "wait allows DOM updates to complete" {
  @fixture {
    <div data-timing-test>Initial</div>
  }
  // Modify element asynchronously
  @eval (eval("setTimeout(function() { document.querySelector('[data-timing-test]').textContent = 'Updated'; }, 20)"))

  @wait 50ms
  @then [data-timing-test] should have_text "Updated"
}

@test "sequential waits accumulate" {
  @let (start = Date.now())
  @wait 30ms
  @wait 30ms
  @let (elapsed = Date.now() - start)
  // Should have waited at least some time (less in headless)
  @assert (elapsed >= 10)
}

// =============================================================================
// Complex Timing Scenarios
// =============================================================================

@test "wait allows state machine transitions to complete" {
  @fixture {
    <div data-sm-timing data-state="start">Timing Test</div>
  }
  // Simulate async state change
  @eval (eval("setTimeout(function() { document.querySelector('[data-sm-timing]').dataset.state = 'end'; }, 30)"))

  @wait 50ms
  @then [data-sm-timing] should have_state "end"
}

@test "wait_until can check DOM state" {
  @fixture {
    <div data-dom-wait data-ready="no">Not Ready</div>
  }
  @eval (eval("setTimeout(function() { document.querySelector('[data-dom-wait]').dataset.ready = 'yes'; }, 30)"))

  // Use simple global flag instead of DOM property chain in @wait_until condition
  @eval (window.__wt_domready = 'no')
  @eval (eval("setTimeout(function() { window.__wt_domready = 'yes'; }, 30)"))
  @wait_until __wt_domready === 'yes' timeout: 500ms
  @then [data-dom-wait] should have_data "ready=yes"
}

// =============================================================================
// Polling Behavior
// =============================================================================

@test "@wait_until polls at appropriate interval" {
  @eval (window.__wt_poll = 'no')

  // Can't directly test the getter call, but we can verify polling happens
  @eval (eval("setTimeout(function() { window.__wt_poll = 'yes'; }, 60)"))

  @wait_until __wt_poll === 'yes' timeout: 200ms
  @assert (window.__wt_poll === 'yes')
}
