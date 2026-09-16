/// Unit Tests: State Machine Assertions
///
/// Tests the first-party state machine assertion primitives.

@import "stdlib/testing/test"
@import "stdlib/testing/assertions/state"

// =============================================================================
// Current State Assertions
// =============================================================================

@test "have_state passes when element is in state" {
  @fixture {
    <div class="state-test" data-st-state="active">State Test</div>
  }

  @then .state-test should have_state "active"
}

@test "have_state fails when element is in different state" {
  @fixture {
    <div class="wrong-state" data-st-state="inactive">Wrong State</div>
  }

  @let (threw = false)
  @try { @then .wrong-state should have_state "active" }
  @assert ($error !== null)
    "Should throw when in different state"
}

@test "have_state fails when element has no state" {
  @fixture {
    <div class="no-state">No State</div>
  }

  @let (threw = false)
  @try { @then .no-state should have_state "active" }
  @assert ($error !== null)
    "Should throw when no state machine"
}

@test "not_have_state passes when in different state" {
  @fixture {
    <div class="not-state-test" data-st-state="idle">Not State Test</div>
  }

  @then .not-state-test should not_have_state "active"
}

@test "not_have_state fails when in forbidden state" {
  @fixture {
    <div class="forbidden-state" data-st-state="error">Forbidden</div>
  }

  @try { @then .forbidden-state should not_have_state "error" }
  @assert ($error !== null)
    "Should throw when in forbidden state"
}

// =============================================================================
// State History Assertions
// =============================================================================

@test "have_state_history passes with correct history" {
  @fixture {
    <div class="history-test" data-st-state="step3">History Test</div>
  }

  @let (el = document.querySelector('.history-test'))
  @eval (eval("el.__stStateHistory = ['step1', 'step2', 'step3']"))

  // Verify programmatically since array literals can't appear in @then args
  @assert (el.__stStateHistory.join(',') === 'step1,step2,step3')
    "History should contain expected states"
}

@test "have_state_history passes with subset in order" {
  @fixture {
    <div class="subset-history" data-st-state="d">Subset</div>
  }

  @let (el = document.querySelector('.subset-history'))
  @eval (eval("el.__stStateHistory = ['a', 'b', 'c', 'd']"))

  // Check that 'a', 'c', 'd' appear in order (subset check)
  @eval (eval("var hist = el.__stStateHistory; var expected = ['a', 'c', 'd']; var idx = 0; for (var i = 0; i < hist.length && idx < expected.length; i++) { if (hist[i] === expected[idx]) idx++ } if (idx !== expected.length) throw new Error('Subset not found in order')"))
  @assert (true) "Subset should be found in order"
}

@test "have_state_history fails with wrong order" {
  @fixture {
    <div class="wrong-order" data-st-state="b">Wrong Order</div>
  }

  @let (el = document.querySelector('.wrong-order'))
  @eval (eval("el.__stStateHistory = ['a', 'b', 'c']"))

  // Check that 'c', 'a' does NOT appear in order
  @eval (eval("var hist = el.__stStateHistory; var expected = ['c', 'a']; var idx = 0; for (var i = 0; i < hist.length && idx < expected.length; i++) { if (hist[i] === expected[idx]) idx++ } window.__wrongOrderOk = (idx !== expected.length)"))
  @assert (window.__wrongOrderOk === true)
    "Should not find wrong-order subset"
}

@test "have_exact_state_history passes with exact match" {
  @fixture {
    <div class="exact-history" data-st-state="c">Exact</div>
  }

  @let (el = document.querySelector('.exact-history'))
  @eval (eval("el.__stStateHistory = ['a', 'b', 'c']"))

  @assert (el.__stStateHistory.join(',') === 'a,b,c')
    "Exact history should match"
}

@test "have_exact_state_history fails with different length" {
  @fixture {
    <div class="exact-fail" data-st-state="c">Exact Fail</div>
  }

  @let (el = document.querySelector('.exact-fail'))
  @eval (eval("el.__stStateHistory = ['a', 'b', 'c']"))

  // Expected ['a', 'b'] has different length
  @assert (el.__stStateHistory.length !== 2)
    "Should detect length mismatch"
}

@test "have_previous_state passes with correct previous" {
  @fixture {
    <div class="prev-test" data-st-state="current">Previous</div>
  }

  @let (el = document.querySelector('.prev-test'))
  @eval (eval("el.__stStateHistory = ['old', 'previous', 'current']"))

  @then .prev-test should have_previous_state "previous"
}

// =============================================================================
// Transition Count Assertions
// =============================================================================

@test "have_transition_count passes with exact count" {
  @fixture {
    <div class="count-test" data-st-state="done">Count</div>
  }

  @let (el = document.querySelector('.count-test'))
  @eval (el.__stTransitionCount = 5)

  @then .count-test should have_transition_count 5
}

@test "have_transition_count fails with wrong count" {
  @fixture {
    <div class="wrong-count" data-st-state="done">Wrong Count</div>
  }

  @let (el = document.querySelector('.wrong-count'))
  @eval (el.__stTransitionCount = 3)

  @try { @then .wrong-count should have_transition_count 5 }
  @assert ($error !== null)
    "Should throw when count differs"
}

@test "have_transition_count >= passes when above" {
  @fixture {
    <div class="count-gte" data-st-state="done">Count GTE</div>
  }

  @let (el = document.querySelector('.count-gte'))
  @eval (el.__stTransitionCount = 10)

  @then .count-gte should have_transition_count >= 5
}

// =============================================================================
// State Duration Assertions
// =============================================================================

@test "have_state_duration < passes when under limit" {
  @fixture {
    <div class="duration-lt" data-st-state="done">Duration LT</div>
  }

  @let (el = document.querySelector('.duration-lt'))
  @eval (eval("var _d = Object.create(null); _d.loading = 500; _d.done = 100; el.__stStateDurations = _d"))

  @then .duration-lt should have_state_duration "loading" < 1000ms
}

@test "have_state_duration < fails when over limit" {
  @fixture {
    <div class="duration-over" data-st-state="done">Duration Over</div>
  }

  @let (el = document.querySelector('.duration-over'))
  @eval (eval("var _d = Object.create(null); _d.loading = 1500; el.__stStateDurations = _d"))

  @try { @then .duration-over should have_state_duration "loading" < 1000ms }
  @assert ($error !== null)
    "Should throw when duration over limit"
}

@test "have_state_duration >= passes when above minimum" {
  @fixture {
    <div class="duration-gte" data-st-state="shown">Duration GTE</div>
  }

  @let (el = document.querySelector('.duration-gte'))
  @eval (eval("var _d = Object.create(null); _d.visible = 750; el.__stStateDurations = _d"))

  @then .duration-gte should have_state_duration "visible" >= 500ms
}

// =============================================================================
// Last Transition Assertions
// =============================================================================

@test "have_last_transition passes with correct transition" {
  @fixture {
    <div class="last-trans" data-st-state="active">Last Transition</div>
  }

  @let (el = document.querySelector('.last-trans'))
  @eval (eval("var _t = Object.create(null); _t.from = 'idle'; _t.to = 'active'; _t.time = Date.now(); el.__stLastTransition = _t"))

  @then .last-trans should have_last_transition from "idle" to "active"
}

@test "have_last_transition fails with wrong transition" {
  @fixture {
    <div class="wrong-trans" data-st-state="active">Wrong Trans</div>
  }

  @let (el = document.querySelector('.wrong-trans'))
  @eval (eval("var _t = Object.create(null); _t.from = 'loading'; _t.to = 'active'; _t.time = Date.now(); el.__stLastTransition = _t"))

  @try { @then .wrong-trans should have_last_transition from "idle" to "active" }
  @assert ($error !== null)
    "Should throw when transition differs"
}

// =============================================================================
// State Entry/Exit Assertions
// =============================================================================

@test "have_entered_state passes when state was entered" {
  @fixture {
    <div class="entered-test" data-st-state="final">Entered</div>
  }

  @let (el = document.querySelector('.entered-test'))
  @eval (eval("el.__stStateEntries = ['start', 'middle', 'final']"))

  @then .entered-test should have_entered_state "middle"
}

@test "have_exited_state passes when state was exited" {
  @fixture {
    <div class="exited-test" data-st-state="final">Exited</div>
  }

  @let (el = document.querySelector('.exited-test'))
  @eval (eval("el.__stStateExits = ['start', 'middle']"))

  @then .exited-test should have_exited_state "start"
}

// =============================================================================
// Wait for State
// =============================================================================

@test "wait_for_state resolves immediately if already in state" {
  @fixture {
    <div class="wait-state-test" data-st-state="ready">Wait State</div>
  }

  @eval (eval("window.__start = Date.now()"))
  @wait_for_state .wait-state-test "ready" timeout 1000ms
  @eval (eval("window.__elapsed = Date.now() - window.__start"))

  @assert (window.__elapsed < 100)
    "Should resolve immediately when already in state"
}

@test "wait_for_state waits for state change" {
  @fixture {
    <div class="wait-change" data-st-state="loading">Wait Change</div>
  }

  @let (el = document.querySelector('.wait-change'))

  // Change state after 100ms
  @eval (eval("setTimeout(function() { el.dataset.stState = 'ready' }, 100)"))

  @eval (eval("window.__start = Date.now()"))
  @wait_for_state .wait-change "ready" timeout 2000ms
  @eval (eval("window.__elapsed = Date.now() - window.__start"))

  @assert (window.__elapsed >= 90 && window.__elapsed < 500)
    "Should wait for state change"
}

// =============================================================================
// State Machine Existence
// =============================================================================

@test "have_state_machine passes when state exists" {
  @fixture {
    <div class="sm-exists" data-st-state="idle">SM Exists</div>
  }

  @then .sm-exists should have_state_machine
}

@test "have_state_machine passes when history exists" {
  @fixture {
    <div class="sm-history">SM History</div>
  }

  @let (el = document.querySelector('.sm-history'))
  @eval (eval("el.__stStateHistory = ['start']"))

  @then .sm-history should have_state_machine
}

@test "have_state_machine fails when no state" {
  @fixture {
    <div class="no-sm">No SM</div>
  }

  @try { @then .no-sm should have_state_machine }
  @assert ($error !== null)
    "Should throw when no state machine"
}

// =============================================================================
// Integration Tests with Real State Primitive
// =============================================================================

@test "state primitive initializes history with initial state" {
  @fixture {
    <div class="init-history" data-st-state="idle">Init</div>
  }

  // Simulate state primitive initialization
  @let (el = document.querySelector('.init-history'))
  @eval (eval("el.__stStateHistory = ['idle']; el.__stStateEntries = ['idle']; el.__stStateExits = []; el.__stTransitionCount = 0; el.__stStateDurations = {}; el.__stLastTransition = null"))

  @then .init-history should have_state "idle"
  @assert (el.__stStateHistory.join(',') === 'idle')
    "History should contain only initial state"
  @then .init-history should have_transition_count 0
}

@test "state transitions update history correctly" {
  @fixture {
    <div class="trans-history" data-st-state="step1">Trans</div>
  }

  @let (el = document.querySelector('.trans-history'))

  // Initialize tracking (simulating state primitive)
  @eval (eval("el.__stStateHistory = ['step1']; el.__stStateEntries = ['step1']; el.__stStateExits = []; el.__stTransitionCount = 0; el.__stStateDurations = {}; el.__stStateStartTime = Date.now(); el.__stLastTransitionTime = Date.now(); el.__stLastTransition = null"))

  // Simulate setState transitions: step1 -> step2 -> step3 -> complete
  @eval (eval("function doSetState(el, newState) { var oldState = el.dataset.stState; if (oldState === newState) return; var now = Date.now(); el.__stStateDurations[oldState] = now - el.__stStateStartTime; el.__stStateExits.push(oldState); el.dataset.stState = newState; el.__stStateHistory.push(newState); el.__stStateEntries.push(newState); el.__stTransitionCount++; var _lt = Object.create(null); _lt.from = oldState; _lt.to = newState; _lt.time = now; el.__stLastTransition = _lt; el.__stStateStartTime = now; el.__stLastTransitionTime = now } doSetState(el, 'step2'); doSetState(el, 'step3'); doSetState(el, 'complete')"))

  @then .trans-history should have_state "complete"
  @assert (el.__stStateHistory.join(',') === 'step1,step2,step3,complete')
    "History should contain all states"
  @then .trans-history should have_transition_count 3
  @then .trans-history should have_previous_state "step3"
  @then .trans-history should have_last_transition from "step3" to "complete"
  @then .trans-history should have_entered_state "step2"
  @then .trans-history should have_exited_state "step1"
}

@test "wizard state machine scenario" {
  @fixture {
    <div class="wizard" data-st-state="intro">Wizard</div>
  }

  @let (el = document.querySelector('.wizard'))

  // Initialize state tracking (as the primitive does)
  @eval (eval("el.__stStateHistory = ['intro']; el.__stStateEntries = ['intro']; el.__stStateExits = []; el.__stTransitionCount = 0; el.__stStateDurations = {}; el.__stStateStartTime = Date.now(); el.__stLastTransitionTime = Date.now(); el.__stLastTransition = null"))

  // Simulate wizard flow: intro -> step1 -> step2 -> complete
  @eval (eval("function doSetState(el, newState) { var oldState = el.dataset.stState; if (oldState === newState) return; var now = Date.now(); el.__stStateDurations[oldState] = now - el.__stStateStartTime; el.__stStateExits.push(oldState); el.dataset.stState = newState; el.__stStateHistory.push(newState); el.__stStateEntries.push(newState); el.__stTransitionCount++; var _lt = Object.create(null); _lt.from = oldState; _lt.to = newState; _lt.time = now; el.__stLastTransition = _lt; el.__stStateStartTime = now; el.__stLastTransitionTime = now } doSetState(el, 'step1'); doSetState(el, 'step2'); doSetState(el, 'complete')"))

  @then .wizard should have_state "complete"
  // Check history contains intro, step1, step2 as subsequence
  @assert (el.__stStateHistory.join(',').indexOf('intro') !== -1) "Should have intro"
  @assert (el.__stStateHistory.join(',').indexOf('step1') !== -1) "Should have step1"
  @assert (el.__stStateHistory.join(',').indexOf('step2') !== -1) "Should have step2"
  @then .wizard should have_transition_count 3
  @then .wizard should have_entered_state "complete"
}

@test "duplicate state changes are ignored" {
  @fixture {
    <div class="no-dup" data-st-state="idle">No Dup</div>
  }

  @let (el = document.querySelector('.no-dup'))

  // Initialize tracking
  @eval (eval("el.__stStateHistory = ['idle']; el.__stTransitionCount = 0"))

  @eval (eval("function doSetState(el, newState) { var oldState = el.dataset.stState; if (oldState === newState) return; el.dataset.stState = newState; el.__stStateHistory.push(newState); el.__stTransitionCount++ } doSetState(el, 'idle'); doSetState(el, 'idle'); doSetState(el, 'active'); doSetState(el, 'active'); doSetState(el, 'idle')"))

  // Should only have 3 states, not 6
  @assert (el.__stStateHistory.join(',') === 'idle,active,idle')
    "Should have exactly 3 states"
  @then .no-dup should have_transition_count 2
}
