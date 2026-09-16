/// Unit Tests: Reactive Binding Assertions
///
/// Tests the first-party signal/binding assertion primitives.

@import "stdlib/testing/test"
@import "stdlib/testing/assertions/binding"

// =============================================================================
// Signal Value Assertions
// =============================================================================

@test "signal equals passes with correct value" {
  @fixture {
    <div class="sig-eq-test">Signal Equals</div>
  }

  @let (el = document.querySelector('.sig-eq-test'))
  @eval (ST.set(el, 'count', 42))

  @then $count on .sig-eq-test should equal 42
}

@test "signal equals passes with object value" {
  @fixture {
    <div class="sig-obj-test">Signal Object</div>
  }

  @let (el = document.querySelector('.sig-obj-test'))
  @eval (eval("var _u = Object.create(null); _u.name = 'Alice'; _u.age = 30; ST.set(el, 'user', _u)"))

  // Object comparisons require programmatic assertion
  @eval (eval("var u = ST.get(el, 'user'); if (u.name !== 'Alice' || u.age !== 30) throw new Error('user should match')"))
  @assert (true) "Signal object value should match"
}

@test "signal equals fails with wrong value" {
  @fixture {
    <div class="sig-wrong-test">Signal Wrong</div>
  }

  @let (el = document.querySelector('.sig-wrong-test'))
  @eval (ST.set(el, 'count', 10))

  @try { @then $count on .sig-wrong-test should equal 42 }
  @assert ($error !== null)
    "Should throw when value differs"
}

@test "signal not_equal passes when different" {
  @fixture {
    <div class="sig-ne-test">Signal Not Equal</div>
  }

  @let (el = document.querySelector('.sig-ne-test'))
  @eval (ST.set(el, 'status', 'success'))

  @then $status on .sig-ne-test should not_equal "error"
}

@test "signal be_gt passes when greater" {
  @fixture {
    <div class="sig-gt-test">Signal GT</div>
  }

  @let (el = document.querySelector('.sig-gt-test'))
  @eval (ST.set(el, 'progress', 0.75))

  @then $progress on .sig-gt-test should be_gt 0.5
}

@test "signal be_gte passes when equal" {
  @fixture {
    <div class="sig-gte-test">Signal GTE</div>
  }

  @let (el = document.querySelector('.sig-gte-test'))
  @eval (ST.set(el, 'count', 5))

  @then $count on .sig-gte-test should be_gte 5
}

@test "signal be_lt passes when less" {
  @fixture {
    <div class="sig-lt-test">Signal LT</div>
  }

  @let (el = document.querySelector('.sig-lt-test'))
  @eval (ST.set(el, 'errors', 2))

  @then $errors on .sig-lt-test should be_lt 5
}

@test "signal be_lte passes when equal" {
  @fixture {
    <div class="sig-lte-test">Signal LTE</div>
  }

  @let (el = document.querySelector('.sig-lte-test'))
  @eval (ST.set(el, 'retries', 3))

  @then $retries on .sig-lte-test should be_lte 3
}

// =============================================================================
// Signal Type Assertions
// =============================================================================

@test "signal be_type string passes" {
  @fixture {
    <div class="type-str-test">Type String</div>
  }

  @let (el = document.querySelector('.type-str-test'))
  @eval (ST.set(el, 'name', 'Alice'))

  @then $name on .type-str-test should be_type "string"
}

@test "signal be_type array passes" {
  @fixture {
    <div class="type-arr-test">Type Array</div>
  }

  @let (el = document.querySelector('.type-arr-test'))
  @eval (ST.set(el, 'items', Array.of(1, 2, 3)))

  @then $items on .type-arr-test should be_type "array"
}

@test "signal be_type fails with wrong type" {
  @fixture {
    <div class="type-fail-test">Type Fail</div>
  }

  @let (el = document.querySelector('.type-fail-test'))
  @eval (ST.set(el, 'count', 42))

  @try { @then $count on .type-fail-test should be_type "string" }
  @assert ($error !== null)
    "Should throw when type differs"
}

@test "signal be_truthy passes" {
  @fixture {
    <div class="truthy-test">Truthy</div>
  }

  @let (el = document.querySelector('.truthy-test'))
  @eval (ST.set(el, 'isReady', true))

  @then $isReady on .truthy-test should be_truthy
}

@test "signal be_falsy passes" {
  @fixture {
    <div class="falsy-test">Falsy</div>
  }

  @let (el = document.querySelector('.falsy-test'))
  @eval (ST.set(el, 'hasErrors', false))

  @then $hasErrors on .falsy-test should be_falsy
}

// =============================================================================
// Signal Tracking Assertions
// =============================================================================

@test "signal have_update_count tracks updates" {
  @fixture {
    <div class="update-count-test">Update Count</div>
  }

  @let (el = document.querySelector('.update-count-test'))
  @eval (eval("var _c = Object.create(null); _c.count = 3; el.__stSignalUpdateCounts = _c"))

  @then $count on .update-count-test should have_update_count 3
}

@test "signal have_update_count >= passes when above" {
  @fixture {
    <div class="update-gte-test">Update GTE</div>
  }

  @let (el = document.querySelector('.update-gte-test'))
  @eval (eval("var _c = Object.create(null); _c.count = 10; el.__stSignalUpdateCounts = _c"))

  @then $count on .update-gte-test should have_update_count >= 5
}

@test "signal have_value_history tracks values" {
  @fixture {
    <div class="history-test">Value History</div>
  }

  @let (el = document.querySelector('.history-test'))
  @eval (eval("var _h = Object.create(null); _h.count = [0, 1, 2, 3, 4, 5]; el.__stSignalHistory = _h"))

  // Verify history contains the expected subsequence [0, 2, 4]
  @eval (eval("var hist = el.__stSignalHistory.count; var expected = [0, 2, 4]; var idx = 0; for (var i = 0; i < hist.length && idx < expected.length; i++) { if (hist[i] === expected[idx]) idx++ } if (idx !== expected.length) throw new Error('Value history should contain subsequence [0, 2, 4]')"))
  @assert (true) "Value history should contain subsequence"
}

@test "signal have_recomputed passes when recomputed" {
  @fixture {
    <div class="recompute-test">Recompute</div>
  }

  @let (el = document.querySelector('.recompute-test'))
  @eval (eval("var _c = Object.create(null); _c.fullName = 2; el.__stRecomputeCounts = _c"))

  @then $fullName on .recompute-test should have_recomputed
}

@test "signal have_recompute_count matches" {
  @fixture {
    <div class="recompute-count-test">Recompute Count</div>
  }

  @let (el = document.querySelector('.recompute-count-test'))
  @eval (eval("var _c = Object.create(null); _c.total = 5; el.__stRecomputeCounts = _c"))

  @then $total on .recompute-count-test should have_recompute_count 5
}

// =============================================================================
// Signal Existence Assertions
// =============================================================================

@test "have_signal passes when signal exists" {
  @fixture {
    <div class="has-sig-test">Has Signal</div>
  }

  @let (el = document.querySelector('.has-sig-test'))
  @eval (ST.set(el, 'count', 0))

  @then .has-sig-test should have_signal "count"
}

@test "have_signal fails when signal missing" {
  @fixture {
    <div class="no-sig-test">No Signal</div>
  }

  @try { @then .no-sig-test should have_signal "nonexistent" }
  @assert ($error !== null)
    "Should throw when signal missing"
}

@test "not_have_signal passes when missing" {
  @fixture {
    <div class="not-sig-test">Not Signal</div>
  }

  @then .not-sig-test should not_have_signal "reactive"
}

// =============================================================================
// Signal Watcher Assertions
// =============================================================================

@test "have_watcher_count tracks watchers" {
  @fixture {
    <div class="watcher-test">Watcher Test</div>
  }

  @let (el = document.querySelector('.watcher-test'))
  @eval (ST.set(el, 'theme', 'dark'))

  // Add some watchers
  @eval (eval("ST.watch(el, 'theme', function() {}); ST.watch(el, 'theme', function() {}); ST.watch(el, 'theme', function() {})"))

  @then $theme on .watcher-test should have_watcher_count 3
}

// =============================================================================
// Signal Array Assertions
// =============================================================================

@test "signal contain passes when value in array" {
  @fixture {
    <div class="contain-test">Contain Test</div>
  }

  @let (el = document.querySelector('.contain-test'))
  @eval (ST.set(el, 'selectedIds', Array.of(1, 2, 3, 4, 5)))

  @then $selectedIds on .contain-test should contain 3
}

@test "signal contain fails when value not in array" {
  @fixture {
    <div class="not-contain-test">Not Contain</div>
  }

  @let (el = document.querySelector('.not-contain-test'))
  @eval (ST.set(el, 'ids', Array.of(1, 2, 3)))

  @try { @then $ids on .not-contain-test should contain 10 }
  @assert ($error !== null)
    "Should throw when value not in array"
}

@test "signal have_length passes with correct length" {
  @fixture {
    <div class="length-test">Length Test</div>
  }

  @let (el = document.querySelector('.length-test'))
  @eval (ST.set(el, 'items', 'a,b,c'.split(',')))

  @then $items on .length-test should have_length 3
}

// =============================================================================
// Wait for Signal
// =============================================================================

@test "wait_for_signal resolves immediately if at value" {
  @fixture {
    <div class="wait-sig-test">Wait Signal</div>
  }

  @let (el = document.querySelector('.wait-sig-test'))
  @eval (ST.set(el, 'status', 'ready'))

  @eval (eval("window.__start = Date.now()"))
  @wait_for_signal $status on .wait-sig-test to_equal "ready" timeout 1000ms
  @eval (eval("window.__elapsed = Date.now() - window.__start"))

  @assert (window.__elapsed < 100)
    "Should resolve immediately when at expected value"
}

@test "wait_for_signal waits for value change" {
  @fixture {
    <div class="wait-change-test">Wait Change</div>
  }

  @let (el = document.querySelector('.wait-change-test'))
  @eval (ST.set(el, 'status', 'loading'))

  // Change after 100ms
  @eval (eval("setTimeout(function() { ST.set(el, 'status', 'complete') }, 100)"))

  @eval (eval("window.__start = Date.now()"))
  @wait_for_signal $status on .wait-change-test to_equal "complete" timeout 2000ms
  @eval (eval("window.__elapsed = Date.now() - window.__start"))

  @assert (window.__elapsed >= 90 && window.__elapsed < 500)
    "Should wait for value change"
}

@test "wait_for_signal to_be_truthy waits for truthy value" {
  @fixture {
    <div class="wait-truthy-test">Wait Truthy</div>
  }

  @let (el = document.querySelector('.wait-truthy-test'))
  @eval (ST.set(el, 'isReady', false))

  // Set to true after 100ms
  @eval (eval("setTimeout(function() { ST.set(el, 'isReady', true) }, 100)"))

  @eval (eval("window.__start = Date.now()"))
  @wait_for_signal $isReady on .wait-truthy-test to_be_truthy timeout 2000ms
  @eval (eval("window.__elapsed = Date.now() - window.__start"))

  @assert (window.__elapsed >= 90 && window.__elapsed < 500)
    "Should wait for truthy value"
}

// =============================================================================
// Signal Tracking Enable/Capture
// =============================================================================

@test "enable_signal_tracking and capture work" {
  @fixture {
    <div class="track-test">Track Test</div>
  }

  @enable_signal_tracking

  @let (el = document.querySelector('.track-test'))
  @eval (ST.set(el, 'counter', 1))
  @eval (ST.set(el, 'counter', 2))
  @eval (ST.set(el, 'counter', 3))

  @capture_signal_report

  @assert (__ctx.signalReport !== undefined)
    "Signal report should be captured"
}
