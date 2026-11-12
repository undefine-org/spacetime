/// State Machine Assertions
///
/// First-party assertions for inspecting state machine internals.
/// These provide direct access to state history, transition counts,
/// and timing that external frameworks cannot see.
///
/// @example
/// @then .wizard should have_state "step2"
/// @then .wizard should have_state_history ["step1", "step2"]
/// @then .wizard should have_transition_count 2
/// @then .loading should have_state_duration "pending" < 1000ms

// =============================================================================
// Current State Assertions
// =============================================================================

/// Assert element is in a specific state
///
/// First-party capability: Direct access to state machine current state.
/// External frameworks can only check data attributes or classes.

%macro then-has-state {

  %form {
    @then $target:selector should have_state $state:string
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __expectedState = %$state;

    // Check data-st-state attribute (standard Spacetime pattern)
    const __currentState = __el.dataset.stState || __el.dataset.state;

    if (!__currentState) {
      throw new Error(`Element ${%$target} has no state. Expected: "${__expectedState}"`);
    }

    if (__currentState !== __expectedState) {
      throw new Error(`Expected state "${__expectedState}", got "${__currentState}"`);
    }
    }
  }

  %example "@then-has-state" {
    @then .accordion should have_state "expanded"
  }
}

/// Assert element is NOT in a specific state

%macro then-not-has-state {

  %form {
    @then $target:selector should not_have_state $state:string
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __forbiddenState = %$state;
    const __currentState = __el.dataset.stState || __el.dataset.state;

    if (__currentState === __forbiddenState) {
      throw new Error(`Expected element NOT to be in state "${__forbiddenState}", but it is`);
    }
    }
  }

  %example "@then-not-has-state" {
    @then .modal should not_have_state "closed"
  }
}

// =============================================================================
// State History Assertions
// =============================================================================

/// Assert element has passed through specific states in order
///
/// First-party capability: Access full state history.
/// External frameworks can only see current state.

%macro then-state-history {

  %form {
    @then $target:selector should have_state_history $history:array
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __expectedHistory = %$history;

    // State history is tracked on the element
    const __actualHistory = __el.__stStateHistory || [];

    if (__actualHistory.length < __expectedHistory.length) {
      throw new Error(
        `State history too short. Expected ${__expectedHistory.length} states, ` +
        `got ${__actualHistory.length}: [${__actualHistory.join(', ')}]`
      );
    }

    // Check if expected states appear in order (not necessarily contiguous)
    let __searchIdx = 0;
    for (const __state of __expectedHistory) {
      const __foundIdx = __actualHistory.indexOf(__state, __searchIdx);
      if (__foundIdx === -1) {
        throw new Error(
          `State "${__state}" not found in history after index ${__searchIdx}. ` +
          `Full history: [${__actualHistory.join(', ')}]`
        );
      }
      __searchIdx = __foundIdx + 1;
    }
    }
  }

  %example "@then-state-history" {
    @then .wizard should have_state_history ["intro", "step1", "step2"]
  }
}

/// Assert element has exact state history

%macro then-exact-state-history {

  %form {
    @then $target:selector should have_exact_state_history $history:array
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __expectedHistory = %$history;
    const __actualHistory = __el.__stStateHistory || [];

    if (__actualHistory.length !== __expectedHistory.length) {
      throw new Error(
        `State history length mismatch. Expected ${__expectedHistory.length}, ` +
        `got ${__actualHistory.length}: [${__actualHistory.join(', ')}]`
      );
    }

    for (let __i = 0; __i < __expectedHistory.length; __i++) {
      if (__actualHistory[__i] !== __expectedHistory[__i]) {
        throw new Error(
          `State mismatch at index ${__i}. Expected "${__expectedHistory[__i]}", ` +
          `got "${__actualHistory[__i]}". Full history: [${__actualHistory.join(', ')}]`
        );
      }
    }
    }
  }

  %example "@then-exact-state-history" {
    @then .carousel should have_exact_state_history ["slide1", "slide2", "slide3"]
  }
}

/// Assert element's last state was a specific value

%macro then-previous-state {

  %form {
    @then $target:selector should have_previous_state $state:string
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __expectedPrevious = %$state;
    const __history = __el.__stStateHistory || [];

    if (__history.length < 2) {
      throw new Error(`Not enough state history to have a previous state. History: [${__history.join(', ')}]`);
    }

    const __previousState = __history[__history.length - 2];

    if (__previousState !== __expectedPrevious) {
      throw new Error(`Expected previous state "${__expectedPrevious}", got "${__previousState}"`);
    }
    }
  }

  %example "@then-previous-state" {
    @then .notification should have_previous_state "showing"
  }
}

// =============================================================================
// Transition Count Assertions
// =============================================================================

/// Assert element has transitioned a specific number of times
///
/// First-party capability: Track exact transition count.
/// External frameworks would need to watch for changes manually.

%macro then-transition-count {

  %form {
    @then $target:selector should have_transition_count $count:number
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __expectedCount = %$count;
    const __actualCount = __el.__stTransitionCount || 0;

    if (__actualCount !== __expectedCount) {
      throw new Error(`Expected ${__expectedCount} transitions, got ${__actualCount}`);
    }
    }
  }

  %example "@then-transition-count" {
    @then .toggle should have_transition_count 3
  }
}

/// Assert element has transitioned at least N times

%macro then-transition-count-gte {

  %form {
    @then $target:selector should have_transition_count >= $count:number
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __minCount = %$count;
    const __actualCount = __el.__stTransitionCount || 0;

    if (__actualCount < __minCount) {
      throw new Error(`Expected at least ${__minCount} transitions, got ${__actualCount}`);
    }
    }
  }

  %example "@then-transition-count-gte" {
    @test "the slider moved at least twice" {
      @fixture { <div class="slider" data-st-state="first"></div> }
      @let (el = document.querySelector('.slider'))
      @eval (el.__stTransitionCount = 2)
      @then .slider should have_transition_count >= 2
    }
  }
}

// =============================================================================
// State Duration Assertions
// =============================================================================

/// Assert element was in a state for a specific duration (less than)
///
/// First-party capability: Track how long each state lasted.
/// External frameworks cannot measure internal state durations.

%macro then-state-duration-lt {

  %form {
    @then $target:selector should have_state_duration $state:string < $duration:time
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __stateName = %$state;
    const __maxDuration = ST.parseMs(%$duration);

    const __durations = __el.__stStateDurations || {};
    const __actualDuration = __durations[__stateName];

    if (__actualDuration === undefined) {
      throw new Error(`No duration recorded for state "${__stateName}". Available: ${Object.keys(__durations).join(', ') || '(none)'}`);
    }

    if (__actualDuration >= __maxDuration) {
      throw new Error(`State "${__stateName}" duration ${__actualDuration}ms exceeds limit of ${__maxDuration}ms`);
    }
    }
  }

  %example "@then-state-duration-lt" {
    @then .loading should have_state_duration "pending" < 1000ms
  }
}

/// Assert element was in a state for at least a specific duration

%macro then-state-duration-gte {

  %form {
    @then $target:selector should have_state_duration $state:string >= $duration:time
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __stateName = %$state;
    const __minDuration = ST.parseMs(%$duration);

    const __durations = __el.__stStateDurations || {};
    const __actualDuration = __durations[__stateName];

    if (__actualDuration === undefined) {
      throw new Error(`No duration recorded for state "${__stateName}". Available: ${Object.keys(__durations).join(', ') || '(none)'}`);
    }

    if (__actualDuration < __minDuration) {
      throw new Error(`State "${__stateName}" duration ${__actualDuration}ms is less than minimum ${__minDuration}ms`);
    }
    }
  }

  %example "@then-state-duration-gte" {
    @then .tooltip should have_state_duration "visible" >= 500ms
  }
}

// =============================================================================
// Last Transition Assertions
// =============================================================================

/// Assert the last state transition was from one state to another
///
/// First-party capability: Know exact transition that occurred.
/// External frameworks can only see current state.

%macro then-last-transition {

  %form {
    @then $target:selector should have_last_transition from $from:string to $to:string
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __expectedFrom = %$from;
    const __expectedTo = %$to;

    const __lastTransition = __el.__stLastTransition;

    if (!__lastTransition) {
      throw new Error(`No transition recorded for element ${%$target}`);
    }

    if (__lastTransition.from !== __expectedFrom || __lastTransition.to !== __expectedTo) {
      throw new Error(
        `Expected transition ${__expectedFrom} → ${__expectedTo}, ` +
        `got ${__lastTransition.from} → ${__lastTransition.to}`
      );
    }
    }
  }

  %example "@then-last-transition" {
    @then .wizard should have_last_transition from "step1" to "step2"
  }
}

// =============================================================================
// State Machine Existence
// =============================================================================

/// Assert element has a state machine attached

%macro then-has-state-machine {

  %form {
    @then $target:selector should have_state_machine
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __hasStateMachine = __el.dataset.stState !== undefined ||
                              __el.dataset.state !== undefined ||
                              __el.__stStateHistory !== undefined;

    if (!__hasStateMachine) {
      throw new Error(`Element ${%$target} does not have a state machine attached`);
    }
    }
  }

  %example "@then-has-state-machine" {
    @then .accordion should have_state_machine
  }
}

// =============================================================================
// State Entry/Exit Event Assertions
// =============================================================================

/// Assert that entering a state triggered a callback
///
/// First-party capability: Hook into state entry events.

%macro then-entered-state {

  %form {
    @then $target:selector should have_entered_state $state:string
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __state = %$state;
    const __entryLog = __el.__stStateEntries || [];

    if (!__entryLog.includes(__state)) {
      throw new Error(
        `State "${__state}" was never entered. ` +
        `Entered states: [${__entryLog.join(', ') || '(none)'}]`
      );
    }
    }
  }

  %example "@then-entered-state" {
    @then .modal should have_entered_state "open"
  }
}

/// Assert that exiting a state triggered a callback

%macro then-exited-state {

  %form {
    @then $target:selector should have_exited_state $state:string
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __state = %$state;
    const __exitLog = __el.__stStateExits || [];

    if (!__exitLog.includes(__state)) {
      throw new Error(
        `State "${__state}" was never exited. ` +
        `Exited states: [${__exitLog.join(', ') || '(none)'}]`
      );
    }
    }
  }

  %example "@then-exited-state" {
    @then .modal should have_exited_state "loading"
  }
}

// =============================================================================
// Wait for State
// =============================================================================

/// Wait for element to enter a specific state
///
/// First-party capability: Subscribe to state change events.
/// External frameworks must poll or use arbitrary timeouts.

%macro wait-for-state {

  %form {
    @wait_for_state $target:selector $state:string timeout $timeout:time = 5000
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __selector = %$target;
    const __targetState = %$state;
    const __timeout = ST.parseMs(%$timeout);

    const __el = ST.subject(__selector);
    if (!__el) throw new Error(`Element not found: ${__selector}`);

    // If already in target state, return immediately
    const __currentState = __el.dataset.stState || __el.dataset.state;
    if (__currentState === __targetState) {
      return;
    }

    // Otherwise wait for state change or timeout
    await new Promise((resolve, reject) => {
      const __timer = setTimeout(() => {
        __observer.disconnect();
        const __finalState = __el.dataset.stState || __el.dataset.state;
        reject(new Error(
          `Timeout after ${__timeout}ms waiting for state "${__targetState}". ` +
          `Current state: "${__finalState}"`
        ));
      }, __timeout);

      const __observer = new MutationObserver((mutations) => {
        const __newState = __el.dataset.stState || __el.dataset.state;
        if (__newState === __targetState) {
          clearTimeout(__timer);
          __observer.disconnect();
          resolve();
        }
      });

      __observer.observe(__el, {
        attributes: true,
        attributeFilter: ['data-st-state', 'data-state']
      });
    });
    }
  }

  %example "@wait-for-state" {
    @test "the modal opens" {
      @fixture { <div class="modal" data-st-state="closed">Hello</div> }
      @eval (setTimeout(() => { document.querySelector('.modal').dataset.stState = 'open'; }, 50))
      @wait_for_state .modal "open" timeout 2s
      @then .modal should be_visible
    }
  }
}

// =============================================================================
// State Transition Timing
// =============================================================================

/// Assert time between two states was within bounds

%macro then-transition-time-lt {

  %form {
    @then $target:selector should have_transition_time from $from:string to $to:string < $duration:time
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __el = ST.subject(%$target);
    if (!__el) throw new Error(`Element not found: ${%$target}`);

    const __fromState = %$from;
    const __toState = %$to;
    const __maxDuration = ST.parseMs(%$duration);

    const __transitionTimes = __el.__stTransitionTimes || {};
    const __key = `${__fromState}->${__toState}`;
    const __actualTime = __transitionTimes[__key];

    if (__actualTime === undefined) {
      throw new Error(
        `No transition time recorded for ${__fromState} → ${__toState}. ` +
        `Available: ${Object.keys(__transitionTimes).join(', ') || '(none)'}`
      );
    }

    if (__actualTime >= __maxDuration) {
      throw new Error(
        `Transition ${__fromState} → ${__toState} took ${__actualTime}ms, ` +
        `exceeds limit of ${__maxDuration}ms`
      );
    }
    }
  }

  %example "@then-transition-time-lt" {
    @then .form should have_transition_time from "validating" to "submitted" < 500ms
  }
}

// =============================================================================
// Enable State Tracking
// =============================================================================

/// Enable detailed state tracking for performance analysis

%macro enable-state-tracking {

  %form {
    @enable_state_tracking
  }

  %emit js {
    {
    __ctx.stateTrackingEnabled = true;

    // Patch state setters to record timing
    const __originalSetState = window.__stSetState;
    if (__originalSetState) {
      window.__stSetState = (el, newState) => {
        const __oldState = el.dataset.stState || el.dataset.state;
        const __now = Date.now();

        // Initialize tracking objects
        el.__stStateHistory = el.__stStateHistory || [];
        el.__stStateDurations = el.__stStateDurations || {};
        el.__stStateEntries = el.__stStateEntries || [];
        el.__stStateExits = el.__stStateExits || [];
        el.__stTransitionTimes = el.__stTransitionTimes || {};
        el.__stTransitionCount = el.__stTransitionCount || 0;

        // Record exit from old state
        if (__oldState) {
          el.__stStateExits.push(__oldState);
          if (el.__stStateStartTime) {
            el.__stStateDurations[__oldState] = __now - el.__stStateStartTime;
          }
          // Record transition time
          const __key = `${__oldState}->${newState}`;
          el.__stTransitionTimes[__key] = __now - (el.__stLastTransitionTime || __now);
        }

        // Record entry to new state
        el.__stStateHistory.push(newState);
        el.__stStateEntries.push(newState);
        el.__stLastTransition = { from: __oldState, to: newState, time: __now };
        el.__stStateStartTime = __now;
        el.__stLastTransitionTime = __now;
        el.__stTransitionCount++;

        // Call original
        return __originalSetState(el, newState);
      };
    }
    }
  }

  %example "@enable-state-tracking" {
    @test "the wizard reaches complete" {
      @enable_state_tracking
      @fixture { <div class="wizard" data-st-state="start"></div> }
      @eval (document.querySelector('.wizard').dataset.stState = 'complete')
      @wait_for_state .wizard "complete" timeout 2s
      @capture_state_report
    }
  }
}

%macro capture-state-report {

  %form {
    @capture_state_report
  }

  %emit js {
    {
    if (!__ctx.stateTrackingEnabled) {
      console.warn('State tracking not enabled - call @enable_state_tracking first');
    }

    // Collect state reports from all tracked elements
    const __report = {
      elements: []
    };

    document.querySelectorAll('[data-st-state], [data-state]').forEach((el) => {
      if (el.__stStateHistory) {
        __report.elements.push({
          selector: el.tagName.toLowerCase() +
                   (el.id ? `#${el.id}` : '') +
                   (el.className ? `.${el.className.split(' ').join('.')}` : ''),
          currentState: el.dataset.stState || el.dataset.state,
          history: el.__stStateHistory,
          transitionCount: el.__stTransitionCount || 0,
          durations: el.__stStateDurations || {}
        });
      }
    });

    __ctx.stateReport = __report;
    console.log('[State Report]', __report);
    }
  }
}


