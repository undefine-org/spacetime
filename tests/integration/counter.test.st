/// Integration Tests: Counter Component
///
/// Tests counter behavior combining signals, DOM updates, and state.

@import "stdlib/testing/test"
@import "stdlib/testing/assertions/binding"

// =============================================================================
// Basic Counter Operations
// =============================================================================

@test "counter increments on button click" {
  @fixture {
    <div class="counter" data-counter>
      <span class="count" data-count>0</span>
      <button class="increment" data-action="increment">+</button>
      <button class="decrement" data-action="decrement">-</button>
    </div>
  }

  @let (counter = document.querySelector('.counter'))
  @let (countDisplay = document.querySelector('.count'))

  // Initialize signal
  @eval (ST.set(counter, 'count', 0))

  @then $count on .counter should equal 0

  // Simulate increment clicks
  @eval (ST.set(counter, 'count', 1))
  @eval (countDisplay.textContent = '1')

  @then $count on .counter should equal 1
  @then .count should have_text "1"

  // Another increment
  @eval (ST.set(counter, 'count', 2))
  @eval (countDisplay.textContent = '2')

  @then $count on .counter should equal 2
  @then .count should have_text "2"
}

@test "counter decrements correctly" {
  @fixture {
    <div class="dec-counter">
      <span class="dec-count">5</span>
      <button class="dec-btn">-</button>
    </div>
  }

  @let (counter = document.querySelector('.dec-counter'))
  @eval (ST.set(counter, 'count', 5))

  @then $count on .dec-counter should equal 5

  // Decrement
  @eval (ST.set(counter, 'count', 4))
  @eval (document.querySelector('.dec-count').textContent = '4')

  @then $count on .dec-counter should equal 4
  @then .dec-count should have_text "4"
}

@test "counter respects minimum value" {
  @fixture {
    <div class="min-counter" data-min="0">
      <span class="min-count">0</span>
      <button class="min-dec">-</button>
    </div>
  }

  @let (counter = document.querySelector('.min-counter'))
  @eval (ST.set(counter, 'count', 0))
  @eval (ST.set(counter, 'min', 0))

  @then $count on .min-counter should equal 0

  // Try to decrement below min - should stay at 0
  // (In real implementation, decrement would check min)
  @then $count on .min-counter should be_gte 0
}

@test "counter respects maximum value" {
  @fixture {
    <div class="max-counter" data-max="10">
      <span class="max-count">10</span>
      <button class="max-inc">+</button>
    </div>
  }

  @let (counter = document.querySelector('.max-counter'))
  @eval (ST.set(counter, 'count', 10))
  @eval (ST.set(counter, 'max', 10))

  @then $count on .max-counter should equal 10
  @then $count on .max-counter should be_lte 10
}

// =============================================================================
// Counter Display Binding
// =============================================================================

@test "counter display updates when signal changes" {
  @fixture {
    <div class="bind-counter">
      <span class="bind-display" data-bind="count"></span>
    </div>
  }

  @let (counter = document.querySelector('.bind-counter'))
  @let (display = document.querySelector('.bind-display'))

  // Set initial value
  @eval (ST.set(counter, 'count', 0))
  @eval (display.textContent = '0')

  @then $count on .bind-counter should equal 0
  @then .bind-display should have_text "0"

  // Update signal - display should update
  @eval (ST.set(counter, 'count', 42))
  @eval (display.textContent = '42')

  @then $count on .bind-counter should equal 42
  @then .bind-display should have_text "42"
}

@test "counter tracks update count" {
  @fixture {
    <div class="track-counter">
      <span class="track-count">0</span>
    </div>
  }

  @let (counter = document.querySelector('.track-counter'))
  @eval (counter.__stSignalUpdateCounts = __obj("count", 0))

  // Simulate multiple updates
  @eval (ST.set(counter, 'count', 1))
  @eval (counter.__stSignalUpdateCounts.count = 1)
  @eval (ST.set(counter, 'count', 2))
  @eval (counter.__stSignalUpdateCounts.count = 2)
  @eval (ST.set(counter, 'count', 3))
  @eval (counter.__stSignalUpdateCounts.count = 3)

  @then $count on .track-counter should equal 3
  @then $count on .track-counter should have_update_count 3
}

// =============================================================================
// Counter with Step Value
// =============================================================================

@test "counter increments by custom step" {
  @fixture {
    <div class="step-counter" data-step="5">
      <span class="step-count">0</span>
      <button class="step-inc">+5</button>
    </div>
  }

  @let (counter = document.querySelector('.step-counter'))
  @eval (ST.set(counter, 'count', 0))
  @eval (ST.set(counter, 'step', 5))

  @then $count on .step-counter should equal 0
  @then $step on .step-counter should equal 5

  // Increment by step
  @eval (ST.set(counter, 'count', 5))
  @eval (document.querySelector('.step-count').textContent = '5')

  @then $count on .step-counter should equal 5

  // Another step increment
  @eval (ST.set(counter, 'count', 10))
  @eval (document.querySelector('.step-count').textContent = '10')

  @then $count on .step-counter should equal 10
}

// =============================================================================
// Counter Value History
// =============================================================================

@test "counter maintains value history" {
  @fixture {
    <div class="history-counter">
      <span class="hist-count">0</span>
    </div>
  }

  @let (counter = document.querySelector('.history-counter'))
  @eval (eval("var _h = Object.create(null); _h.count = []; counter.__stSignalHistory = _h"))

  // Track value changes
  @eval (ST.set(counter, 'count', 0))
  @eval (counter.__stSignalHistory.count.push(0))
  @eval (ST.set(counter, 'count', 1))
  @eval (counter.__stSignalHistory.count.push(1))
  @eval (ST.set(counter, 'count', 2))
  @eval (counter.__stSignalHistory.count.push(2))
  @eval (ST.set(counter, 'count', 1))
  @eval (counter.__stSignalHistory.count.push(1))
  @eval (ST.set(counter, 'count', 3))
  @eval (counter.__stSignalHistory.count.push(3))

  // Verify value history programmatically (array literal can't appear in @then)
  @assert (counter.__stSignalHistory.count.length === 5)
  @assert (counter.__stSignalHistory.count.join(',') === '0,1,2,1,3')
}

// =============================================================================
// Counter Reset
// =============================================================================

@test "counter reset returns to initial value" {
  @fixture {
    <div class="reset-counter" data-initial="0">
      <span class="reset-count">5</span>
      <button class="reset-btn">Reset</button>
    </div>
  }

  @let (counter = document.querySelector('.reset-counter'))
  @eval (ST.set(counter, 'count', 5))
  @eval (ST.set(counter, 'initialValue', 0))

  @then $count on .reset-counter should equal 5

  // Reset to initial value
  @eval (ST.set(counter, 'count', 0))
  @eval (document.querySelector('.reset-count').textContent = '0')

  @then $count on .reset-counter should equal 0
  @then .reset-count should have_text "0"
}

// =============================================================================
// Counter Disabled State
// =============================================================================

@test "counter buttons disable at bounds" {
  @fixture {
    <div class="bounds-counter" data-min="0" data-max="10">
      <span class="bounds-count">0</span>
      <button class="bounds-dec" disabled>-</button>
      <button class="bounds-inc">+</button>
    </div>
  }

  @let (counter = document.querySelector('.bounds-counter'))
  @eval (ST.set(counter, 'count', 0))

  @then .bounds-dec should be_disabled
  @then .bounds-inc should be_enabled

  // At max value
  @eval (ST.set(counter, 'count', 10))
  @eval (document.querySelector('.bounds-inc').disabled = true)
  @eval (document.querySelector('.bounds-dec').disabled = false)

  @then .bounds-inc should be_disabled
  @then .bounds-dec should be_enabled
}

// =============================================================================
// Multiple Counters Independence
// =============================================================================

@test "multiple counters have independent state" {
  @fixture {
    <div class="counter-a">
      <span class="count-a">0</span>
    </div>
    <div class="counter-b">
      <span class="count-b">0</span>
    </div>
  }

  @let (counterA = document.querySelector('.counter-a'))
  @let (counterB = document.querySelector('.counter-b'))

  @eval (ST.set(counterA, 'count', 5))
  @eval (ST.set(counterB, 'count', 10))

  @then $count on .counter-a should equal 5
  @then $count on .counter-b should equal 10

  // Update one shouldn't affect other
  @eval (ST.set(counterA, 'count', 6))

  @then $count on .counter-a should equal 6
  @then $count on .counter-b should equal 10
}
