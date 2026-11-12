/// Reactive Binding Assertions
///
/// First-party assertions for inspecting signal values and updates.
/// These provide direct access to reactive binding internals that
/// external frameworks cannot see.
///
/// @example
/// @then $count should equal 6
/// @then $count should have_update_count 3
/// @then $count on .display should equal 6

// =============================================================================
// Signal Value Assertions
// =============================================================================

/// Assert a signal equals a specific value
///
/// First-party capability: Direct access to signal values.
/// External frameworks must query DOM or computed styles.

%macro then-signal-equals {

  %form {
    @then $signal:binding on $target:selector should equal $expected:expr
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __signalName = %$signal;
    const __expected = %$expected;

    const __actual = (typeof ST.resolve === 'function') ? ST.resolve(__el, __signalName) : ST.get(__el, __signalName);

    if (__actual === undefined) {
      throw new Error(`Signal "${__signalName}" not found on element ${%$target}`);
    }

    // Deep equality check for objects/arrays
    const __isEqual = (a, b) => {
      if (a === b) return true;
      if (typeof a !== typeof b) return false;
      if (typeof a !== 'object' || a === null) return false;
      if (Array.isArray(a) !== Array.isArray(b)) return false;

      const keysA = Object.keys(a);
      const keysB = Object.keys(b);
      if (keysA.length !== keysB.length) return false;

      return keysA.every(key => __isEqual(a[key], b[key]));
    };

    if (!__isEqual(__actual, __expected)) {
      throw new Error(
        `Signal "${__signalName}" expected ${JSON.stringify(__expected)}, ` +
        `got ${JSON.stringify(__actual)}`
      );
    }
    }
  }

  %example "@then-signal-equals" {
    @test "clicking + counts up" {
      @fixture {
        <div class="counter"><button class="inc">+</button></div>
      }
      @given .counter {
        $count number: 0;
        .inc { @on &.click { $count <- $count + 1; } }
      }
      @when .inc click
      @then $count on .counter should equal 1
    }
  }
}

/// Assert a signal does NOT equal a specific value

%macro then-signal-not-equals {

  %form {
    @then $signal:binding on $target:selector should not_equal $unexpected:expr
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __signalName = %$signal;
    const __unexpected = %$unexpected;

    const __actual = (typeof ST.resolve === 'function') ? ST.resolve(__el, __signalName) : ST.get(__el, __signalName);

    if (__actual === __unexpected) {
      throw new Error(`Signal "${__signalName}" should not equal ${JSON.stringify(__unexpected)}`);
    }
    }
  }

  %example "@then-signal-not-equals" {
    @test "a fresh form is not in error" {
      @fixture { <form class="signup"></form> }
      @given .signup { $status string: "idle"; }
      @then $status on .signup should not_equal "error"
    }
  }
}

/// Assert a signal is greater than a value

%macro then-signal-gt {

  %form {
    @then $signal:binding on $target:selector should be_gt $threshold:number
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __signalName = %$signal;
    const __threshold = %$threshold;

    const __actual = (typeof ST.resolve === 'function') ? ST.resolve(__el, __signalName) : ST.get(__el, __signalName);

    if (typeof __actual !== 'number') {
      throw new Error(`Signal "${__signalName}" is not a number: ${typeof __actual}`);
    }

    if (__actual <= __threshold) {
      throw new Error(`Signal "${__signalName}" expected > ${__threshold}, got ${__actual}`);
    }
    }
  }

  %example "@then-signal-gt" {
    @then $progress on .loader should be_gt 0.5
  }
}

/// Assert a signal is greater than or equal to a value

%macro then-signal-gte {

  %form {
    @then $signal:binding on $target:selector should be_gte $threshold:number
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __signalName = %$signal;
    const __threshold = %$threshold;

    const __actual = (typeof ST.resolve === 'function') ? ST.resolve(__el, __signalName) : ST.get(__el, __signalName);

    if (typeof __actual !== 'number') {
      throw new Error(`Signal "${__signalName}" is not a number: ${typeof __actual}`);
    }

    if (__actual < __threshold) {
      throw new Error(`Signal "${__signalName}" expected >= ${__threshold}, got ${__actual}`);
    }
    }
  }

  %example "@then-signal-gte" {
    @test "the counter never goes negative" {
      @fixture { <div class="counter"></div> }
      @given .counter { $count number: 0; }
      @then $count on .counter should be_gte 0
    }
  }
}

/// Assert a signal is less than a value

%macro then-signal-lt {

  %form {
    @then $signal:binding on $target:selector should be_lt $threshold:number
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __signalName = %$signal;
    const __threshold = %$threshold;

    const __actual = (typeof ST.resolve === 'function') ? ST.resolve(__el, __signalName) : ST.get(__el, __signalName);

    if (typeof __actual !== 'number') {
      throw new Error(`Signal "${__signalName}" is not a number: ${typeof __actual}`);
    }

    if (__actual >= __threshold) {
      throw new Error(`Signal "${__signalName}" expected < ${__threshold}, got ${__actual}`);
    }
    }
  }

  %example "@then-signal-lt" {
    @test "the form starts with few errors" {
      @fixture { <form class="signup"></form> }
      @given .signup { $errors number: 0; }
      @then $errors on .signup should be_lt 5
    }
  }
}

/// Assert a signal is less than or equal to a value

%macro then-signal-lte {

  %form {
    @then $signal:binding on $target:selector should be_lte $threshold:number
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __signalName = %$signal;
    const __threshold = %$threshold;

    const __actual = (typeof ST.resolve === 'function') ? ST.resolve(__el, __signalName) : ST.get(__el, __signalName);

    if (typeof __actual !== 'number') {
      throw new Error(`Signal "${__signalName}" is not a number: ${typeof __actual}`);
    }

    if (__actual > __threshold) {
      throw new Error(`Signal "${__signalName}" expected <= ${__threshold}, got ${__actual}`);
    }
    }
  }

  %example "@then-signal-lte" {
    @test "the fetch stays within its retry budget" {
      @fixture { <div class="fetch"></div> }
      @given .fetch { $retries number: 2; }
      @then $retries on .fetch should be_lte 3
    }
  }
}

// =============================================================================
// Signal Type Assertions
// =============================================================================

/// Assert a signal is of a specific type

%macro then-signal-type {

  %form {
    @then $signal:binding on $target:selector should be_type $type:string
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __signalName = %$signal;
    const __expectedType = %$type;

    const __actual = (typeof ST.resolve === 'function') ? ST.resolve(__el, __signalName) : ST.get(__el, __signalName);

    let __actualType;
    if (__actual === null) {
      __actualType = 'null';
    } else if (Array.isArray(__actual)) {
      __actualType = 'array';
    } else {
      __actualType = typeof __actual;
    }

    if (__actualType !== __expectedType) {
      throw new Error(
        `Signal "${__signalName}" expected type "${__expectedType}", ` +
        `got "${__actualType}"`
      );
    }
    }
  }

  %example "@then-signal-type" {
    @test "the list holds an array and the user a name" {
      @fixture { <div class="profile"></div> }
      @given .profile {
        $items number[]: [];
        $name string: "Ada";
      }
      @then $items on .profile should be_type "array"
      @then $name on .profile should be_type "string"
    }
  }
}

/// Assert a signal is truthy

%macro then-signal-truthy {

  %form {
    @then $signal:binding on $target:selector should be_truthy
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __signalName = %$signal;
    const __actual = (typeof ST.resolve === 'function') ? ST.resolve(__el, __signalName) : ST.get(__el, __signalName);

    if (!__actual) {
      throw new Error(`Signal "${__signalName}" expected truthy, got ${JSON.stringify(__actual)}`);
    }
    }
  }

  %example "@then-signal-truthy" {
    @then $isLoggedIn on .app should be_truthy
  }
}

/// Assert a signal is falsy

%macro then-signal-falsy {

  %form {
    @then $signal:binding on $target:selector should be_falsy
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __signalName = %$signal;
    const __actual = (typeof ST.resolve === 'function') ? ST.resolve(__el, __signalName) : ST.get(__el, __signalName);

    if (__actual) {
      throw new Error(`Signal "${__signalName}" expected falsy, got ${JSON.stringify(__actual)}`);
    }
    }
  }

  %example "@then-signal-falsy" {
    @test "a fresh form has no errors" {
      @fixture { <form class="signup"></form> }
      @given .signup { $hasErrors bool: false; }
      @then $hasErrors on .signup should be_falsy
    }
  }
}

// =============================================================================
// Signal Update Count Assertions
// =============================================================================

/// Assert a signal has been updated a specific number of times
///
/// First-party capability: Track exact update count.
/// External frameworks cannot track internal signal updates.

%macro then-signal-update-count {

  %form {
    @then $signal:binding on $target:selector should have_update_count $count:number
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __signalName = %$signal;
    const __expectedCount = %$count;

    const __updateCounts = __el.__stSignalUpdateCounts || {};
    const __actualCount = __updateCounts[__signalName] || 0;

    if (__actualCount !== __expectedCount) {
      throw new Error(
        `Signal "${__signalName}" expected ${__expectedCount} updates, ` +
        `got ${__actualCount}`
      );
    }
    }
  }

  %example "@then-signal-update-count" {
    @test "three clicks, three updates" {
      @fixture {
        <div class="counter"><button class="inc">+</button></div>
      }
      @enable_signal_tracking
      @given .counter {
        $count number: 0;
        .inc { @on &.click { $count <- $count + 1; } }
      }
      @when .inc click
      @when .inc click
      @when .inc click
      @then $count on .counter should have_update_count 3
    }
  }
}

/// Assert a signal has been updated at least N times

%macro then-signal-update-count-gte {

  %form {
    @then $signal:binding on $target:selector should have_update_count >= $count:number
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __signalName = %$signal;
    const __minCount = %$count;

    const __updateCounts = __el.__stSignalUpdateCounts || {};
    const __actualCount = __updateCounts[__signalName] || 0;

    if (__actualCount < __minCount) {
      throw new Error(
        `Signal "${__signalName}" expected at least ${__minCount} updates, ` +
        `got ${__actualCount}`
      );
    }
    }
  }

  %example "@then-signal-update-count-gte" {
    @test "the slider reports every step" {
      @fixture { <div class="slider"></div> }
      @enable_signal_tracking
      @let (el = document.querySelector('.slider'))
      @eval ([1, 2, 3].forEach(v => ST.set(el, 'position', v)))
      @then $position on .slider should have_update_count >= 3
    }
  }
}

// =============================================================================
// Signal History Assertions
// =============================================================================

/// Assert signal value history contains specific values
///
/// First-party capability: Track all values a signal has held.
/// External frameworks can only see current value.

%macro then-signal-value-history {

  %form {
    @then $signal:binding on $target:selector should have_value_history $history:array
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __signalName = %$signal;
    const __expectedHistory = %$history;

    const __valueHistories = __el.__stSignalHistory || {};
    const __actualHistory = __valueHistories[__signalName] || [];

    // Check if expected values appear in order
    let __searchIdx = 0;
    for (const __value of __expectedHistory) {
      let __found = false;
      for (let __i = __searchIdx; __i < __actualHistory.length; __i++) {
        if (JSON.stringify(__actualHistory[__i]) === JSON.stringify(__value)) {
          __searchIdx = __i + 1;
          __found = true;
          break;
        }
      }
      if (!__found) {
        throw new Error(
          `Value ${JSON.stringify(__value)} not found in signal history after index ${__searchIdx}. ` +
          `Full history: ${JSON.stringify(__actualHistory)}`
        );
      }
    }
    }
  }

  %example "@then-signal-value-history" {
    @test "the counter steps through every value" {
      @fixture {
        <div class="counter"><button class="inc">+</button></div>
      }
      @enable_signal_tracking
      @given .counter {
        $count number: 0;
        .inc { @on &.click { $count <- $count + 1; } }
      }
      @when .inc click
      @when .inc click
      @then $count on .counter should have_value_history [1, 2]
    }
  }
}

// =============================================================================
// Computed Signal Assertions
// =============================================================================

/// Assert a computed signal was recomputed
///
/// First-party capability: Know when derivations recalculate.
/// External frameworks cannot track internal computations.

%macro then-signal-recomputed {

  %form {
    @then $signal:binding on $target:selector should have_recomputed
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __signalName = %$signal;

    const __recomputeCounts = __el.__stRecomputeCounts || {};
    const __count = __recomputeCounts[__signalName] || 0;

    if (__count === 0) {
      throw new Error(`Computed signal "${__signalName}" has not been recomputed`);
    }
    }
  }

  %example "@then-signal-recomputed" {
    @test "renaming recomputes the full name" {
      @fixture { <div class="user"></div> }
      @enable_signal_tracking
      @let (el = document.querySelector('.user'))
      @eval (ST.set(el, 'first', 'Ada'))
      @eval (ST.derive(el, 'fullName', ['first'], () => ST.get(el, 'first') + ' Lovelace'))
      @eval (ST.set(el, 'first', 'Grace'))
      @then $fullName on .user should have_recomputed
    }
  }
}

/// Assert recompute count for a derived signal

%macro then-signal-recompute-count {

  %form {
    @then $signal:binding on $target:selector should have_recompute_count $count:number
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __signalName = %$signal;
    const __expectedCount = %$count;

    const __recomputeCounts = __el.__stRecomputeCounts || {};
    const __actualCount = __recomputeCounts[__signalName] || 0;

    if (__actualCount !== __expectedCount) {
      throw new Error(
        `Computed signal "${__signalName}" expected ${__expectedCount} recomputes, ` +
        `got ${__actualCount}`
      );
    }
    }
  }

  %example "@then-signal-recompute-count" {
    @test "each price change recomputes the total once" {
      @fixture { <div class="cart"></div> }
      @enable_signal_tracking
      @let (el = document.querySelector('.cart'))
      @eval (ST.set(el, 'price', 10))
      @eval (ST.derive(el, 'total', ['price'], () => ST.get(el, 'price') * 2))
      @eval (ST.set(el, 'price', 12))
      @then $total on .cart should have_recompute_count 2
    }
  }
}

// =============================================================================
// Signal Existence
// =============================================================================

/// Assert an element has a specific signal

%macro then-has-signal {

  %form {
    @then $target:selector should have_signal $name:string
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __signalName = %$name;
    const __signals = ST.signals.get(__el);

    if (!__signals || !(__signalName in __signals)) {
      const __available = __signals ? Object.keys(__signals).join(', ') : '(none)';
      throw new Error(
        `Signal "${__signalName}" not found on element. ` +
        `Available signals: ${__available}`
      );
    }
    }
  }

  %example "@then-has-signal" {
    @then .counter should have_signal "count"
  }
}

/// Assert an element does NOT have a specific signal

%macro then-not-has-signal {

  %form {
    @then $target:selector should not_have_signal $name:string
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __signalName = %$name;
    const __signals = ST.signals.get(__el);

    if (__signals && __signalName in __signals) {
      throw new Error(`Signal "${__signalName}" should not exist on element, but it does`);
    }
    }
  }

  %example "@then-not-has-signal" {
    @then .static-element should not_have_signal "reactive"
  }
}

// =============================================================================
// Signal Watcher Assertions
// =============================================================================

/// Assert a signal has active watchers
///
/// First-party capability: Know how many effects are subscribed.
/// External frameworks cannot see subscription counts.

%macro then-signal-watcher-count {

  %form {
    @then $signal:binding on $target:selector should have_watcher_count $count:number
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __signalName = %$signal;
    const __expectedCount = %$count;

    const __signals = ST.signals.get(__el);
    if (!__signals || !__signals[__signalName]) {
      throw new Error(`Signal "${__signalName}" not found on element`);
    }

    const __actualCount = __signals[__signalName].d?.size || 0;

    if (__actualCount !== __expectedCount) {
      throw new Error(
        `Signal "${__signalName}" expected ${__expectedCount} watchers, ` +
        `got ${__actualCount}`
      );
    }
    }
  }

  %example "@then-signal-watcher-count" {
    @then $theme on .app should have_watcher_count 3
  }
}

// =============================================================================
// Wait for Signal
// =============================================================================

/// Wait for a signal to reach a specific value
///
/// First-party capability: Subscribe to signal changes.
/// External frameworks must poll or use arbitrary timeouts.

%macro wait-for-signal {

  %form {
    @wait_for_signal $signal:binding on $target:selector to_equal $expected:expr timeout $timeout:time = 5000
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __selector = %$target;
    const __signalName = %$signal;
    const __expected = %$expected;
    const __timeout = ST.parseMs(%$timeout);

    const __el = ST.subject(__selector);
    if (!__el) throw new Error(`Element not found: ${__selector}`);

    // If already at expected value, return immediately
    const __current = ST.get(__el, __signalName);
    if (__current === __expected) {
      return;
    }

    // Otherwise wait for signal change or timeout
    await new Promise((resolve, reject) => {
      const __timer = setTimeout(() => {
        __unwatch();
        const __final = ST.get(__el, __signalName);
        reject(new Error(
          `Timeout after ${__timeout}ms waiting for signal "${__signalName}" ` +
          `to equal ${JSON.stringify(__expected)}. Current: ${JSON.stringify(__final)}`
        ));
      }, __timeout);

      const __unwatch = ST.watch(__el, __signalName, (value) => {
        if (value === __expected) {
          clearTimeout(__timer);
          __unwatch();
          resolve();
        }
      });
    });
    }
  }

  %example "@wait-for-signal" {
    @test "the loader finishes" {
      @fixture { <div class="loader"><button class="go">Load</button></div> }
      @given .loader {
        $status string: "idle";
        .go { @on &.click { $status <- "complete"; } }
      }
      @when .go click
      @wait_for_signal $status on .loader to_equal "complete" timeout 5s
    }
  }
}

/// Wait for a signal to become truthy

%macro wait-for-signal-truthy {

  %form {
    @wait_for_signal $signal:binding on $target:selector to_be_truthy timeout $timeout:time = 5000
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __selector = %$target;
    const __signalName = %$signal;
    const __timeout = ST.parseMs(%$timeout);

    const __el = ST.subject(__selector);
    if (!__el) throw new Error(`Element not found: ${__selector}`);

    const __current = ST.get(__el, __signalName);
    if (__current) {
      return;
    }

    await new Promise((resolve, reject) => {
      const __timer = setTimeout(() => {
        __unwatch();
        const __final = ST.get(__el, __signalName);
        reject(new Error(
          `Timeout after ${__timeout}ms waiting for signal "${__signalName}" ` +
          `to be truthy. Current: ${JSON.stringify(__final)}`
        ));
      }, __timeout);

      const __unwatch = ST.watch(__el, __signalName, (value) => {
        if (value) {
          clearTimeout(__timer);
          __unwatch();
          resolve();
        }
      });
    });
    }
  }

  %example "@wait-for-signal-truthy" {
    @test "the app becomes ready" {
      @fixture { <div class="app"><button class="start">Start</button></div> }
      @given .app {
        $isReady bool: false;
        .start { @on &.click { $isReady <- true; } }
      }
      @when .start click
      @wait_for_signal $isReady on .app to_be_truthy timeout 3s
    }
  }
}

// =============================================================================
// Enable Signal Tracking
// =============================================================================

/// Enable detailed signal tracking for debugging

%macro enable-signal-tracking {

  %form {
    @enable_signal_tracking
  }

  %emit js {
    {
    __ctx.signalTrackingEnabled = true;

    // Patch ST.set to track updates
    const __originalSet = ST.set.bind(ST);
    ST.set = (el, name, value) => {
      // Initialize tracking
      el.__stSignalUpdateCounts = el.__stSignalUpdateCounts || {};
      el.__stSignalHistory = el.__stSignalHistory || {};

      // Track update count
      el.__stSignalUpdateCounts[name] = (el.__stSignalUpdateCounts[name] || 0) + 1;

      // Track value history
      if (!el.__stSignalHistory[name]) {
        el.__stSignalHistory[name] = [];
      }
      el.__stSignalHistory[name].push(value);

      // Call original
      return __originalSet(el, name, value);
    };

    // Patch ST.derive to track recomputes
    const __originalDerive = ST.derive.bind(ST);
    ST.derive = (el, name, deps, compute) => {
      el.__stRecomputeCounts = el.__stRecomputeCounts || {};

      const __wrappedCompute = (values) => {
        el.__stRecomputeCounts[name] = (el.__stRecomputeCounts[name] || 0) + 1;
        return compute(values);
      };

      return __originalDerive(el, name, deps, __wrappedCompute);
    };

    // Store cleanup to restore originals
    __ctx.cleanup.push(() => {
      ST.set = __originalSet;
      ST.derive = __originalDerive;
    });
    }
  }

  %example "@enable-signal-tracking" {
    @test "one click, one update" {
      @fixture {
        <div class="counter"><button class="inc">+</button></div>
      }
      @enable_signal_tracking
      @given .counter {
        $count number: 0;
        .inc { @on &.click { $count <- $count + 1; } }
      }
      @when .inc click
      @then $count on .counter should have_update_count 1
    }
  }
}

%macro capture-signal-report {

  %form {
    @capture_signal_report
  }

  %emit js {
    {
    if (!__ctx.signalTrackingEnabled) {
      console.warn('Signal tracking not enabled - call @enable_signal_tracking first');
    }

    const __report = {
      elements: []
    };

    // Find all elements with signals
    const __allElements = document.querySelectorAll('*');
    __allElements.forEach((el) => {
      const __signals = ST.signals.get(el);
      if (__signals) {
        const __elementReport = {
          selector: el.tagName.toLowerCase() +
                   (el.id ? `#${el.id}` : '') +
                   (el.className ? `.${el.className.split(' ').join('.')}` : ''),
          signals: {}
        };

        for (const [name, signal] of Object.entries(__signals)) {
          __elementReport.signals[name] = {
            value: signal.v,
            watcherCount: signal.d?.size || 0,
            updateCount: el.__stSignalUpdateCounts?.[name] || 0,
            recomputeCount: el.__stRecomputeCounts?.[name] || 0
          };
        }

        __report.elements.push(__elementReport);
      }
    });

    __ctx.signalReport = __report;
    console.log('[Signal Report]', __report);
    }
  }
}

// =============================================================================
// Signal Array Assertions
// =============================================================================

/// Assert signal array contains a value

%macro then-signal-contains {

  %form {
    @then $signal:binding on $target:selector should contain $value:expr
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __signalName = %$signal;
    const __searchValue = %$value;

    const __actual = (typeof ST.resolve === 'function') ? ST.resolve(__el, __signalName) : ST.get(__el, __signalName);

    if (!Array.isArray(__actual)) {
      throw new Error(`Signal "${__signalName}" is not an array`);
    }

    const __found = __actual.some(item =>
      JSON.stringify(item) === JSON.stringify(__searchValue)
    );

    if (!__found) {
      throw new Error(
        `Signal "${__signalName}" does not contain ${JSON.stringify(__searchValue)}. ` +
        `Values: ${JSON.stringify(__actual)}`
      );
    }
    }
  }

  %example "@then-signal-contains" {
    @test "picking an item selects it" {
      @fixture { <ul class="list"><li class="pick">5</li></ul> }
      @given .list {
        $selectedIds number[]: [];
        .pick { @on &.click { $selectedIds <- [...$selectedIds, 5]; } }
      }
      @when .pick click
      @then $selectedIds on .list should contain 5
    }
  }
}

/// Assert signal array length

%macro then-signal-length {

  %form {
    @then $signal:binding on $target:selector should have_length $length:number
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __signalName = %$signal;
    const __expectedLength = %$length;

    const __actual = (typeof ST.resolve === 'function') ? ST.resolve(__el, __signalName) : ST.get(__el, __signalName);

    if (!Array.isArray(__actual)) {
      throw new Error(`Signal "${__signalName}" is not an array`);
    }

    if (__actual.length !== __expectedLength) {
      throw new Error(
        `Signal "${__signalName}" expected length ${__expectedLength}, ` +
        `got ${__actual.length}`
      );
    }
    }
  }

  %example "@then-signal-length" {
    @test "each add puts one item in the cart" {
      @fixture { <div class="cart"><button class="add">Add</button></div> }
      @given .cart {
        $items string[]: [];
        .add { @on &.click { $items <- [...$items, "tea"]; } }
      }
      @when .add click
      @when .add click
      @then $items on .cart should have_length 2
    }
  }
}


