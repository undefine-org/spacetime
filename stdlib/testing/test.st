/// Testing Framework for Spacetime
///
/// Provides a declarative testing DSL using the metasystem.
/// Tests are defined using @test, @given, @when, @then syntax.
///
/// @example
/// @test "toggle switches state on click" {
///   @given [data-toggle] { data-state: "off" }
///   @when [data-toggle] click
///   @then [data-toggle] should have_state "on"
/// }


/// Test types
@type TestDef {
  name: string;
  fn: any;                           // Test function (async)
  skip?: boolean;
}

@type TestError {
  name: string;
  error: string;
}

@type TestResults {
  passed: number;
  failed: number;
  skipped: number;
  errors: TestError[];
}


/// Test Runner Primitive
///
/// Manages test registration and execution in the browser.
/// Sets up global test infrastructure.
///
/// @param &container - Container element for test results
/// @yields $tests - Array of registered tests
/// @yields $results - Current test results
/// @yields $runTests - Function to run tests
/// @yields $currentTest - Name of currently running test

%primitive test_runner(&container) {
  %emit js {
    const __st_tests = [];
    const __st_results = { passed: 0, failed: 0, skipped: 0, errors: [] };
    let __st_currentTest = null;

    window.__spacetime_register_test = (name, fn, options = {}) => {
      __st_tests.push({ name, fn, ...options });
    };

    window.__spacetime_run_tests = async (filter) => {
      __st_results.passed = 0;
      __st_results.failed = 0;
      __st_results.skipped = 0;
      __st_results.errors = [];

      const tests = filter
        ? __st_tests.filter(t => t.name.includes(filter))
        : __st_tests;

      // Emit test.start event
      %&container.dispatchEvent(new CustomEvent('test.start', { detail: { total: tests.length } }));

      for (const test of tests) {
        __st_currentTest = test.name;
        %yield __st_currentTest -> $currentTest;

        // Emit test.run event
        %&container.dispatchEvent(new CustomEvent('test.run', { detail: { name: test.name } }));

        if (test.skip) {
          __st_results.skipped++;
          console.log(`SKIP: ${test.name}`);
          continue;
        }

        try {
          await test.fn();
          __st_results.passed++;
          console.log(`PASS: ${test.name}`);
          %&container.dispatchEvent(new CustomEvent('test.pass', { detail: { name: test.name } }));
        } catch (e) {
          // A fidelity refusal (ST._rung.require on a backend that can't provide the
          // needed rung) is NOT a failure — it is a deferral to a higher-fidelity
          // backend (e.g. CDP). Count it as SKIPPED so a logic-rung run reports 0
          // failures while still surfacing the deferral. On the higher backend the
          // same test actually runs.
          if (e && e.__stRungSkip) {
            __st_results.skipped++;
            console.log(`SKIP (needs '${e.__stRungNeed}' fidelity): ${test.name}`);
            %&container.dispatchEvent(new CustomEvent('test.skip', { detail: { name: test.name, need: e.__stRungNeed } }));
          } else {
            __st_results.failed++;
            __st_results.errors.push({
              name: test.name,
              error: e.message,
              stack: e.stack
            });
            console.error(`FAIL: ${test.name}`);
            console.error(`  ${e.message}`);
            %&container.dispatchEvent(new CustomEvent('test.fail', { detail: { name: test.name, error: e.message } }));
          }
        }
      }

      __st_currentTest = null;
      %yield __st_currentTest -> $currentTest;
      %yield { ...__st_results } -> $results;

      // Emit test.complete event
      %&container.dispatchEvent(new CustomEvent('test.complete', { detail: __st_results }));

      console.log(`\nResults: ${__st_results.passed} passed, ${__st_results.failed} failed, ${__st_results.skipped} skipped`);
      return __st_results;
    };

    %yield __st_tests -> $tests;
    %yield __st_results -> $results;
    %yield window.__spacetime_run_tests -> $runTests;
    %yield __st_currentTest -> $currentTest;

    %cleanup {
      delete window.__spacetime_register_test;
      delete window.__spacetime_run_tests;
    }
  }

  %exports {
    $tests: TestDef[]
    $results: TestResults
    $runTests: fn(filter?: string) ~> object  // Run tests, returns results
    $currentTest: string?
  }
}


/// @test Macro
///
/// Registers a test with a name and body containing @given, @when, @then directives.
///
/// @example
/// @test "element becomes visible on click" {
///   @given .box { opacity: 0 }
///   @when .box click
///   @then .box should be_visible
/// }

%macro test {

  %form {
    @test $name:string {
      $body:block
    }
  }

  // Note: test_runner primitive should be set up once per test file, not per test.
  // The test harness is included in the test HTML by the test runner.

  %emit js {
    window.__spacetime_register_test(%$name, async () => {
      const __ctx = { startTime: Date.now() };
      try {
        %$body.js
      } finally {
        // Test isolation: run every cleanup a probe registered (fixtures/mounts
        // append nodes to document.body via @fixture/@mount and push a remover
        // onto __ctx.cleanup). Without draining it here, fixtures from one test
        // persist into the next — a later `document.querySelector('.x')` then
        // resolves a STALE instance from an earlier test (the cross-test leak
        // that made an isolated-passing test fail inside a suite).
        if (__ctx.cleanup) {
          for (const __fn of __ctx.cleanup.slice().reverse()) {
            try { __fn(); } catch (__e) { /* cleanup is best-effort */ }
          }
          __ctx.cleanup.length = 0;
        }
      }
    });
  }
}


/// @test … needs <rung> Macro (PLAN-027 W1)
///
/// Declares the fidelity RUNG a test needs. `needs` may only RAISE the floor
/// above what the assertions infer; a backend that can't provide it refuses to
/// run the test (reports FAILED) rather than faking it.
///
/// @example
/// @test "hero wraps to two lines" needs layout {
///   @mount @self .hero
///   @then .hero should have_height 96
/// }
%macro test-needs {

  %form {
    @test $name:string needs $rung:ident {
      $body:block
    }
  }

  %emit js {
    window.__spacetime_register_test(%$name, async () => {
      const __ctx = { startTime: Date.now() };
      ST._rung.setCurrent(%$rung);
      ST._rung.require(%$rung, %$name);
      %$body.js
    }, { needs: %$rung });
  }
}


/// @test-skip Macro
///
/// Registers a test that will be skipped during execution.
/// Useful for temporarily disabling tests.
///
/// @example
/// @test-skip "not yet implemented" {
///   @then .feature should exist
/// }

%macro test-skip {

  %form {
    @test-skip $name:string {
      $_body:block
    }
  }

  %emit js {
    window.__spacetime_register_test(%$name, async () => {}, { skip: true });
  }
}


/// @test-only Macro
///
/// Only runs this test (and other .only tests), skipping all others.
/// Useful during development to focus on specific tests.
///
/// @example
/// @test-only "focus on this test" {
///   @given .box { opacity: 1 }
///   @then .box should be_visible
/// }

%macro test-only {

  %form {
    @test-only $name:string {
      $body:block
    }
  }

  %emit js {
    window.__spacetime_register_test(%$name, async () => {
      const __ctx = { startTime: Date.now() };
      %$body.js
    }, { only: true });
  }
}


/// @given Macro - Element State
///
/// Sets up initial state for an element by selector.
/// Can set data attributes, CSS properties, or content.
///
/// @example
/// @given [data-toggle] { data-state: "off" }
/// @given .box { opacity: 0; transform: translateY(20px) }

%macro given {

  %form {
    @given $target:selector $state:properties
  }

  %emit js {
    {
      const __givenEl = ST.subject(%$target);
      if (!__givenEl) throw new Error(`@given: Element not found: ${%$target}`);

      const __givenProps = %$state;
      for (const [k, v] of Object.entries(__givenProps)) {
        if (k === 'data-state') {
          __givenEl.dataset.state = v;
        } else if (k.startsWith('data-')) {
          __givenEl.dataset[k.slice(5)] = v;
        } else if (k === 'textContent' || k === 'innerText') {
          __givenEl[k] = v;
        } else if (k === 'innerHTML') {
          __givenEl.innerHTML = v;
        } else if (k === 'value') {
          __givenEl.value = v;
        } else if (k === 'checked') {
          __givenEl.checked = v;
        } else if (k === 'disabled') {
          __givenEl.disabled = v;
        } else {
          __givenEl.style[k] = v;
        }
      }
      __ctx.element = __givenEl;
    }
  }
}


/// @given Macro - Directive Body
///
/// Applies directive macros (e.g., @on, @cycle, scoped state + reactive class)
/// to an element selected by CSS selector.
///
/// @example
/// @given .toggle { $on bool: false; .is-on: $on; @on click { $on <- !$on; } }

%macro given-directives {

  %form {
    @given $target:selector {
      $body:block
    }
  }

  %emit js {
    {
      const __givenEl = ST.subject(%$target);
      if (!__givenEl) throw new Error(`@given: Element not found: ${%$target}`);
      __ctx.element = __givenEl;

      // BUG-217: a `@given` body carries EITHER element state (`data-state:
      // "modified"`, `textContent: "x"`, `value:`, `checked:`, `disabled:`) or
      // Spacetime constructs (scoped state, reactive classes, `@on`). Both are
      // written `@given <sel> { … }`, and the sibling `%macro given` that declares
      // `$state:properties` can never claim them: its capture is INLINE, so its
      // form has no body_capture at all and this macro matches every `@given` with
      // a brace body.
      //
      // The body was then compiled as a SCOPE, where `data-state: "modified"` is a
      // CSS declaration — it emitted a CSS rule and NO JavaScript, so the element
      // was never touched. Silently: 8 of the framework's own self-tests asserted
      // on state that nothing had set.
      //
      // So apply the state properties here, where the intent lives, and let the
      // rest still compile as a scope. `.state_props` yields only the
      // `name: value` pairs the body declares; a body with none yields `{}` and
      // this loop is a no-op, leaving the scope path exactly as it was.
      const __givenProps = %$body.state_props;
      for (const [k, v] of Object.entries(__givenProps)) {
        if (k === 'data-state') {
          __givenEl.dataset.state = v;
        } else if (k.startsWith('data-')) {
          // `data-my-attr` → dataset.myAttr, matching the DOM's own camelCasing.
          __givenEl.dataset[k.slice(5).replace(/-([a-z])/g, (_, c) => c.toUpperCase())] = v;
        } else if (k === 'textContent' || k === 'innerText') {
          __givenEl[k] = v;
        } else if (k === 'innerHTML') {
          __givenEl.innerHTML = v;
        } else if (k === 'value') {
          __givenEl.value = v;
        } else if (k === 'checked') {
          __givenEl.checked = (v === true || v === 'true');
        } else if (k === 'disabled') {
          __givenEl.disabled = (v === true || v === 'true');
        } else {
          __givenEl.style[k] = v;
        }
      }

      // Compile the body as a `target { … }` SCOPE (BUG-119 §1) so nested CSS
      // bindings (`.cls: $sig`), scoped-state decls (`$x bool: …`), AND directives
      // (`@on click`) all resolve against the given selector — the author's intent.
      // (Bare `.js` recompiled selectorless and silently dropped CSS-scope bodies.)
      %$body.scoped_js
    }
  }
}


/// @given Macro - Data Binding
///
/// Sets up initial value for a data binding.
///
/// @example
/// @given $count = 0
/// @given $items = []

%macro given-data {

  %form {
    @given $binding:binding = $value:expr
  }

  %emit js {
    %$binding = %$value;
    __ctx.data = __ctx.data || {};
    __ctx.data['%$binding'] = %$value;
  }
}


/// @given Macro - Visibility/State
///
/// Sets an element's visibility or interactive state.
///
/// @example
/// @given .modal is hidden
/// @given .button is disabled

%macro given-visibility {

  %form {
    @given $target:selector is $state:ident
  }

  %emit js {
    {
      const __givenEl = ST.subject(%$target);
      if (!__givenEl) throw new Error(`@given: Element not found: ${%$target}`);
      switch ('%$state') {
        case 'hidden':
          __givenEl.style.display = 'none';
          __givenEl.hidden = true;
          break;
        case 'visible':
          __givenEl.style.display = '';
          __givenEl.hidden = false;
          break;
        case 'disabled':
          __givenEl.disabled = true;
          break;
        case 'enabled':
          __givenEl.disabled = false;
          break;
        case 'checked':
          __givenEl.checked = true;
          break;
        case 'unchecked':
          __givenEl.checked = false;
          break;
        default:
          __givenEl.dataset.stState = '%$state';
      }
      __ctx.element = __givenEl;
    }
  }
}


/// @behavior Macro
///
/// Test-only helper: registers a transition table on an element via the test
/// runtime (window.__st_register_machine) so a test can drive scripted state
/// changes. Page code uses $-signals + reactive class instead.
///
/// @example
/// @behavior .toggle-switch machine(initial: "off") {
///     @transition(from: "off", to: "on", on: click)
///     @transition(from: "on", to: "off", on: click)
/// }

%macro test-behavior {

  %form {
    @behavior $target:selector machine(initial: $initial:string) {
      $transitions:transitions*
    }
  }

  %emit js {
    {
      const __behaviorEl = ST.subject(%$target);
      if (!__behaviorEl) throw new Error(`@behavior: Element not found: ${%$target}`);

      // Build transitions array
      const __transitions = [];
      %for $t in $transitions {
        __transitions.push({
          from: %$t.from,
          to: %$t.to,
          on: '%$t.on',
          after: %$t.after
        });
      }

      // Register state machine
      window.__st_register_machine(__behaviorEl, {
        initial: %$initial,
        transitions: __transitions
      });

      __ctx.element = __behaviorEl;
    }
  }
}


/// @when Macro
///
/// Performs an action on an element - click, input, focus, etc.
/// After the action, waits for a frame to allow DOM updates.
///
/// @example
/// @when .button click
/// @when .input input "hello world"
/// @when .field focus
/// @when .item keydown "Enter"

%macro when {

  %form {
    @when $target:selector $action:ident $value:expr = null
  }

  %emit js {
    {
    const __selector = %$target;
    // Ensure any pending component init (listeners, state) is applied before we
    // drive the element. Init is microtask-scheduled and, for instances appended
    // via invokeTemplate, the chain spans MULTIPLE ticks: the DOM MutationObserver
    // delivers via Promise.resolve().then (tick 1) → ST._scheduleInit re-defers
    // ST._flushInit via queueMicrotask (tick 2). A single await drains only one
    // tick, so drain to a fixed point before flushing (the production browser path
    // runs these on the real microtask queue; headless must pump it explicitly).
    if (typeof ST !== 'undefined') { for (let __i = 0; __i < 6; __i++) await Promise.resolve(); if (ST._flushInit) ST._flushInit(); }
    const __whenEl = (__selector === window || __selector === 'window') ? window
      : (__selector === document || __selector === 'document') ? document
      : ST.subject(__selector);
    if (!__whenEl) throw new Error(`@when: Element not found: ${__selector}`);

    const __action = %$action;
    const __value = %$value;

    switch (__action) {
      case 'click':
        // Dispatch a real bubbling MouseEvent rather than el.click() — the latter
        // is a no-op under LinkeDOM (V8 backend), so @when .x click would not fire
        // the handler. dispatchEvent works on both LinkeDOM and real browsers.
        __whenEl.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
        break;
      case 'dblclick':
        __whenEl.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
        break;
      case 'input':
        __whenEl.value = __value;
        __whenEl.dispatchEvent(new Event('input', { bubbles: true }));
        break;
      case 'change':
        __whenEl.value = __value;
        __whenEl.dispatchEvent(new Event('change', { bubbles: true }));
        break;
      case 'focus':
        __whenEl.focus();
        __whenEl.dispatchEvent(new FocusEvent('focus', { bubbles: true }));
        break;
      case 'blur':
        __whenEl.blur();
        __whenEl.dispatchEvent(new FocusEvent('blur', { bubbles: false }));
        break;
      case 'hover':
      case 'mouseenter':
        __whenEl.dispatchEvent(new MouseEvent('mouseenter', { bubbles: true }));
        break;
      case 'mouseleave':
        __whenEl.dispatchEvent(new MouseEvent('mouseleave', { bubbles: true }));
        break;
      case 'keydown':
      case 'press':
        __whenEl.dispatchEvent(new KeyboardEvent('keydown', { key: __value, bubbles: true }));
        break;
      case 'keyup':
        __whenEl.dispatchEvent(new KeyboardEvent('keyup', { key: __value, bubbles: true }));
        break;
      case 'keypress':
        __whenEl.dispatchEvent(new KeyboardEvent('keypress', { key: __value, bubbles: true }));
        break;
      case 'scroll':
        __whenEl.scrollTop = __value || 0;
        __whenEl.dispatchEvent(new Event('scroll', { bubbles: true }));
        break;
      case 'scroll_down':
        if (__whenEl === window || __whenEl === document) {
          window.scrollBy(0, __value || 50);
        } else {
          __whenEl.scrollTop = (__whenEl.scrollTop || 0) + (__value || 50);
        }
        __whenEl.dispatchEvent(new Event('scroll', { bubbles: true }));
        break;
      case 'scroll_up':
        if (__whenEl === window || __whenEl === document) {
          window.scrollBy(0, -(__value || 50));
        } else {
          __whenEl.scrollTop = Math.max(0, (__whenEl.scrollTop || 0) - (__value || 50));
        }
        __whenEl.dispatchEvent(new Event('scroll', { bubbles: true }));
        break;
      case 'submit':
        __whenEl.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
        break;
      case 'reset':
        if (__whenEl.tagName === 'FORM') __whenEl.reset();
        __whenEl.dispatchEvent(new Event('reset', { bubbles: true }));
        break;
      case 'check':
        __whenEl.checked = true;
        __whenEl.dispatchEvent(new Event('change', { bubbles: true }));
        break;
      case 'uncheck':
        __whenEl.checked = false;
        __whenEl.dispatchEvent(new Event('change', { bubbles: true }));
        break;
      case 'select':
        __whenEl.value = __value;
        __whenEl.dispatchEvent(new Event('change', { bubbles: true }));
        break;
      case 'drag':
        __whenEl.dispatchEvent(new DragEvent('dragstart', { bubbles: true }));
        break;
      case 'drop':
        __whenEl.dispatchEvent(new DragEvent('drop', { bubbles: true }));
        break;
      default:
        // Generic event dispatch
        __whenEl.dispatchEvent(new Event(__action, { bubbles: true }));
    }

    // Allow microtasks and DOM updates to complete. ST.set batches dependent
    // notifications into a queueMicrotask (st.js), so a reactive re-render (class
    // toggle, content binding) lands one or more ticks AFTER the action. Drain to
    // a fixed point so the assertion that follows observes the settled DOM.
    if (globalThis.__spacetime_headless__) {
      for (let __i = 0; __i < 6; __i++) await Promise.resolve();
    } else {
      await new Promise(r => requestAnimationFrame(() => setTimeout(r, 16)));
    }

    __ctx.targetElement = __whenEl;
    __ctx.lastAction = __action;
    }
  }
}


/// @then <subject> { <claim>; * } Macro (PLAN-027 W3)
///
/// Block-form assertion: one subject, many `;`-terminated claims. Replaces the
/// family of discrete `then-*` macros with a single grammar (see
/// stdlib/capture-types/claims.st). The claims array is interpolated as a JS
/// literal and iterated at runtime (no %for needed); each claim becomes a
/// throwing check.
///
/// Claim shapes: `<prop> <op> <value>;` or bare `<prop>;` (truthy/exists).
/// Props: state | text | value | count | exist | be_visible | be_hidden | <attr>.
/// Ops: == != >= <= < > ~=(contains) ~>(sequence) matches(regex).
///
/// @example
/// @then .wizard {
///   state == "complete";
///   count >= 3;
///   text ~= "Done";
///   exist;
/// }
/// @then { <nested directives> } — the GROUPING form (no subject).
///
/// Sibling of `then-claims` below, and the distinction is the SUBJECT:
///   `@then .sel { state == "x"; }`  → claims about ONE element (then-claims)
///   `@then { @find ".a" {…} @assert … }` → a GROUP of independent assertions
///
/// The two are disjoint by construction — `$subject:selector` cannot match an
/// empty head — so no ambiguity is introduced by having both.
///
/// Why this exists as its OWN declaration rather than an optional subject:
/// a `$subject:selector?` would make `@then { … }` parse as then-claims with
/// a missing subject, and every nested `@find`/`@assert` would then be read as
/// a CLAIM (`<prop> <op> <value>;`) and fail far from the cause. Separate
/// forms keep each error pointing at what the author actually wrote.
///
/// BUG-300: the shape was in use at 13 corpus sites (stdlib/editable tests,
/// tests/fixtures/native-template/*) with NO declaration behind it. It rode
/// the same trailing-token leniency `@use`'s clauses rode (BUG-299), and
/// surfaced as E0946 when BUG-264 closed that leniency.
///
/// @example
/// @then {
///   @find ".ed p" { have_text "Hello"; }
///   @assert (window.__x === 1) "the side effect ran";
/// }
%macro then-group {

  %form {
    @then {
      $body:block
    }
  }

  // A grouping head emits nothing of its own: it exists so the nested
  // directives it wraps are parsed in assertion position and expanded by
  // their own macros. The `window.__st_noteAssertion()` accounting stays
  // with the leaf assertions, so a group never inflates the assertion count.
  //
  // `%$body.js` — the EXPANDED emission of the block, not its source text.
  // `%$body` alone interpolates the raw tokens, which parse but never run:
  // the harness caught exactly that ("Test made no assertions— its claims did
  // not compile to anything"), which is why this macro's gate is a headless
  // RUN and not a compile check. `@test` above interpolates the same way.
  %emit js {
    {
      %$body.js
    }
  }
}

%macro then-claims {

  %form {
    @then $subject:selector {
      $claims:claim_block
    }
  }

  %emit js {
    {
    window.__st_noteAssertion();
      const __subject = %$subject;
      const __claims = %$claims;
      // `ST.subject`, not a raw querySelector: a subject may be `&name` or
      // `&name .desc`, which is not valid CSS and silently resolves to null.
      // The inline `@then … should …` form already routed through it; this
      // block form did not, so the SAME subject worked in one spelling and
      // reported `got undefined` in the other.
      const __thenEl = ST.subject(__subject);
      const __claimProp = (el, name) => {
        // Signal reference: `$signalName`. Resolve across the stores a signal can
        // live in: element-scope (ST.get), body-scope reactive Proxy
        // (window.SpacetimeLocal / _localState), and ST.signals.
        if (name && name.charAt(0) === '$') {
          const __sig = name.slice(1);
          if (el && typeof ST !== 'undefined' && ST.get) {
            const __v = ST.get(el, __sig);
            if (typeof __v !== 'undefined') return __v;
          }
          if (typeof window !== 'undefined' && window.SpacetimeLocal && (__sig in window.SpacetimeLocal)) {
            return window.SpacetimeLocal[__sig];
          }
          if (typeof window !== 'undefined' && window._localState && (__sig in window._localState)) {
            return window._localState[__sig];
          }
          if (typeof ST !== 'undefined' && ST.signals && ST.signals[__sig] && ST.signals[__sig].get) {
            return ST.signals[__sig].get();
          }
          return undefined;
        }
        switch (name) {
          // DOM / structural
          case 'exist': return el !== null;
          case 'count': return el ? ST.subjectAll(__subject).length : 0;
          case 'text': return el ? el.textContent : undefined;
          case 'value': return el ? el.value : undefined;
          case 'state': return el ? el.dataset.stState : undefined;
          // Layout (real-browser only — refuses on V8)
          case 'be_visible':
            ST._rung.require('layout', '@then ' + __subject + ' be_visible');
            return !!(el && el.offsetParent !== null);
          case 'be_hidden':
            ST._rung.require('layout', '@then ' + __subject + ' be_hidden');
            return !el || el.offsetParent === null;
          // State-machine introspection (parity with then-* state assertions)
          case 'state_history': return el ? (el.__stStateHistory || []) : undefined;
          case 'transition_count': return el ? (el.__stTransitionCount || 0) : undefined;
          case 'previous_state': {
            const h = el && el.__stStateHistory; return (h && h.length >= 2) ? h[h.length - 2] : undefined;
          }
          case 'has_state_machine': return !!(el && el.__stStateHistory);
          case 'animating': return !!(el && el.__stAnimating);
          // Timeline introspection
          case 'timeline_running': return !!(el && el.__stTimelineRunning);
          case 'timeline_progress': return el ? (el.__stTimelineProgress ?? 0) : undefined;
          default: return el ? (el.dataset[name] ?? el.getAttribute(name)) : undefined;
        }
      };
      const __claimCmp = (actual, op, expected) => {
        switch (op) {
          case '==': return actual === expected || String(actual) === String(expected);
          case '!=': return actual !== expected && String(actual) !== String(expected);
          case '>=': return Number(actual) >= Number(expected);
          case '<=': return Number(actual) <= Number(expected);
          case '<':  return Number(actual) < Number(expected);
          case '>':  return Number(actual) > Number(expected);
          case '~=':
            if (Array.isArray(actual)) return actual.includes(expected);
            return String(actual).includes(String(expected));
          case '~>': {
            if (!Array.isArray(actual) || !Array.isArray(expected)) return false;
            let i = 0; for (const a of actual) { if (a === expected[i]) i++; } return i === expected.length;
          }
          case 'matches': return new RegExp(expected).test(String(actual));
          default: throw new Error('@then: unknown op ' + op);
        }
      };
      // Normalize the prop capture: { id } -> 'name', { sig } -> '$name'.
      const __propName = (p) => {
        if (p && typeof p === 'object') return (typeof p.sig !== 'undefined') ? ('$' + p.sig) : p.id;
        return p;
      };
      // Claim values arrive as capture TEXT (records carry source text, not
      // spliced code — nested-Expr-as-text, PLAN-079/BUG-204). Evaluate the
      // text at claim time: literals (`"a"`, `5`, `["a","b"]`) parse as
      // themselves; a bare `$sig` reads the signal (element-scope first, then
      // page-global) so `$count == 5` compares VALUES, not the text "$count".
      const __claimValue = (v) => {
        if (typeof v !== 'string') return v;
        const bare = /^\$([A-Za-z_][A-Za-z0-9_.]*)$/.exec(v.trim());
        if (bare) return __claimProp(__thenEl, '$' + bare[1]);
        try { return (new Function('return (' + v + ');'))(); }
        catch (e) { throw new Error('@then ' + __subject + ': could not evaluate claim value ' + JSON.stringify(v) + ': ' + e); }
      };
      for (const __claim of __claims) {
        const __pname = __propName(__claim.prop);
        const __actual = __claimProp(__thenEl, __pname);
        if (typeof __claim.op !== 'undefined') {
          if (!__claimCmp(__actual, __claim.op, __claimValue(__claim.value))) {
            throw new Error('@then ' + __subject + ': ' + __pname + ' ' + __claim.op + ' ' + JSON.stringify(__claim.value) + ' (got ' + JSON.stringify(__actual) + ')');
          }
        } else {
          if (!__actual) {
            throw new Error('@then ' + __subject + ': expected ' + __pname + ' to be truthy (got ' + JSON.stringify(__actual) + ')');
          }
        }
      }
    }
  }
}


/// @then Macro
///
/// Asserts a condition on an element.
/// Supports many assertion types: exist, be_visible, have_text, have_class, etc.
///
/// @example
/// @then .box should exist
/// @then .modal should be_visible
/// @then [data-toggle] should have_state "on"
/// @then .title should have_text "Hello"
/// @then .items should have_length 5

%macro then {

  %form {
    @then $target:selector should $assertion:ident $expected:expr? = undefined
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __thenEl = ST.subject(%$target);
    const __assertion = %$assertion;
    const __expected = %$expected;
    const __selector = %$target;

    const __assert = (cond, msg) => {
      if (!cond) throw new Error(msg);
    };

    switch (__assertion) {
      // Existence
      case 'exist':
        __assert(__thenEl !== null, `Expected ${__selector} to exist`);
        break;
      case 'not_exist':
        __assert(__thenEl === null, `Expected ${__selector} to not exist`);
        break;

      // Visibility
      case 'be_visible':
        ST._rung.require('layout', `@then ${__selector} should be_visible`);
        __assert(__thenEl && __thenEl.offsetParent !== null, `Expected ${__selector} to be visible`);
        break;
      case 'be_hidden':
        ST._rung.require('layout', `@then ${__selector} should be_hidden`);
        __assert(!__thenEl || __thenEl.offsetParent === null, `Expected ${__selector} to be hidden`);
        break;

      // Content
      case 'have_text':
        __assert(__thenEl && __thenEl.textContent.includes(__expected),
          `Expected ${__selector} to have text "${__expected}", got "${__thenEl?.textContent}"`);
        break;
      case 'have_exact_text':
        __assert(__thenEl && __thenEl.textContent.trim() === __expected,
          `Expected ${__selector} to have exact text "${__expected}", got "${__thenEl?.textContent?.trim()}"`);
        break;
      case 'have_html':
        __assert(__thenEl && __thenEl.innerHTML.includes(__expected),
          `Expected ${__selector} to contain HTML "${__expected}"`);
        break;

      // Form values
      case 'have_value':
        __assert(__thenEl && __thenEl.value === __expected,
          `Expected ${__selector} value to be "${__expected}", got "${__thenEl?.value}"`);
        break;
      case 'be_checked':
        __assert(__thenEl && __thenEl.checked === true,
          `Expected ${__selector} to be checked`);
        break;
      case 'be_unchecked':
        __assert(__thenEl && __thenEl.checked === false,
          `Expected ${__selector} to be unchecked`);
        break;
      case 'be_disabled':
        __assert(__thenEl && __thenEl.disabled === true,
          `Expected ${__selector} to be disabled`);
        break;
      case 'be_enabled':
        __assert(__thenEl && __thenEl.disabled === false,
          `Expected ${__selector} to be enabled`);
        break;

      // Focus
      case 'be_focused':
        __assert(document.activeElement === __thenEl,
          `Expected ${__selector} to be focused`);
        break;
      case 'not_be_focused':
        __assert(document.activeElement !== __thenEl,
          `Expected ${__selector} to not be focused`);
        break;

      // Classes
      case 'have_class':
        __assert(__thenEl && __thenEl.classList.contains(__expected),
          `Expected ${__selector} to have class "${__expected}"`);
        break;
      case 'not_have_class':
        __assert(__thenEl && !__thenEl.classList.contains(__expected),
          `Expected ${__selector} to not have class "${__expected}"`);
        break;

      // Data attributes and state
      case 'have_state':
        __assert(__thenEl && __thenEl.dataset.state === __expected,
          `Expected ${__selector} to have state "${__expected}", got "${__thenEl?.dataset.state}"`);
        break;
      case 'have_data':
        const [__dataKey, __dataVal] = __expected.split('=').map(s => s.trim());
        const __actualData = __thenEl?.dataset[__dataKey];
        __assert(__actualData === __dataVal,
          `Expected ${__selector} to have data-${__dataKey}="${__dataVal}", got "${__actualData}"`);
        break;

      // Attributes
      case 'have_attr':
        const [__attrName, __attrVal] = __expected.split('=').map(s => s.trim());
        const __actualAttr = __thenEl?.getAttribute(__attrName);
        if (__attrVal !== undefined) {
          __assert(__actualAttr === __attrVal,
            `Expected ${__selector} to have attribute ${__attrName}="${__attrVal}", got "${__actualAttr}"`);
        } else {
          __assert(__thenEl?.hasAttribute(__attrName),
            `Expected ${__selector} to have attribute ${__attrName}`);
        }
        break;
      case 'not_have_attr':
        __assert(!__thenEl?.hasAttribute(__expected),
          `Expected ${__selector} to not have attribute ${__expected}`);
        break;

      // Styles
      case 'have_style':
        const [__styleProp, __styleVal] = __expected.split(':').map(s => s.trim());
        ST._rung.require('layout', `@then ${__selector} should have_style`);
        const __computed = getComputedStyle(__thenEl)[__styleProp];
        __assert(__computed === __styleVal,
          `Expected ${__selector} to have ${__styleProp}: ${__styleVal}, got ${__computed}`);
        break;

      // Collection length
      case 'have_length':
      case 'have_count':
        const __els = ST.subjectAll(__selector);
        __assert(__els.length === Number(__expected),
          `Expected ${__selector} to have count ${__expected}, got ${__els.length}`);
        break;

      // Contains child element or text
      case 'contain':
        if (__expected.match(/^[.#\[]/)) {
          __assert(__thenEl && __thenEl.querySelector(__expected) !== null,
            `Expected ${__selector} to contain ${__expected}`);
        } else {
          __assert(__thenEl && __thenEl.textContent.includes(__expected),
            `Expected ${__selector} to contain text "${__expected}"`);
        }
        break;
      case 'not_contain':
        if (__expected.match(/^[.#\[]/)) {
          __assert(!__thenEl || __thenEl.querySelector(__expected) === null,
            `Expected ${__selector} to not contain ${__expected}`);
        } else {
          __assert(!__thenEl || !__thenEl.textContent.includes(__expected),
            `Expected ${__selector} to not contain text "${__expected}"`);
        }
        break;

      // Position/dimensions
      case 'have_width':
        ST._rung.require('layout', `@then ${__selector} should have_width`);
        const __width = __thenEl?.offsetWidth;
        __assert(__width === Number(__expected),
          `Expected ${__selector} to have width ${__expected}px, got ${__width}px`);
        break;
      case 'have_height':
        ST._rung.require('layout', `@then ${__selector} should have_height`);
        const __height = __thenEl?.offsetHeight;
        __assert(__height === Number(__expected),
          `Expected ${__selector} to have height ${__expected}px, got ${__height}px`);
        break;

      // Empty state
      case 'be_empty':
        __assert(__thenEl && __thenEl.children.length === 0,
          `Expected ${__selector} to be empty, has ${__thenEl?.children.length} children`);
        break;
      case 'not_be_empty':
        __assert(__thenEl && __thenEl.children.length > 0,
          `Expected ${__selector} to not be empty`);
        break;

      default:
        throw new Error(`Unknown assertion: ${__assertion}`);
    }
    }
  }
}


/// @wait Macro
///
/// Waits for a specified duration before continuing.
///
/// @example
/// @wait 500ms
/// @wait 1s

%macro wait {

  %form {
    @wait $duration:time
  }

  %emit js {
    {
    // In headless mode, bound the wait but keep it long enough that real timers
    // (setTimeout/setInterval) the test is waiting on actually fire. A hard 10ms
    // cap silently broke any test combining `@wait Nms` with a real timer of
    // delay > 10ms (e.g. @cycle intervals, contenteditable blur's 100ms debounce):
    // the V8 event loop runs timers on a real clock, so the callback never ran
    // within the capped wait. Cap at 250ms instead — covers every logic-rung
    // timer in the suite (max declared timer < 250ms) while still bounding the
    // rare long animation wait (a `@wait 1s` collapses to 250ms). Genuinely
    // long timing assertions belong on the timing rung (virtual clock / CDP).
    const __waitTime = globalThis.__spacetime_headless__
      ? Math.min(ST.parseMs(%$duration), 250)
      : ST.parseMs(%$duration);
    await new Promise(r => setTimeout(r, __waitTime));
    // Headless: pump the microtask queue + apply pending component init so that
    // instances appended (invokeTemplate / @each) since the last probe are mounted
    // before assertions run. The MutationObserver→_scheduleInit→_flushInit chain
    // spans multiple ticks; setTimeout alone does not drain it. (See @when.)
    if (globalThis.__spacetime_headless__ && typeof ST !== 'undefined') {
      for (let __i = 0; __i < 6; __i++) await Promise.resolve();
      if (ST._flushInit) ST._flushInit();
    }
    }
  }
}


/// @wait_until Macro
///
/// Waits for a condition to become true, with timeout.
///
/// @example
/// @wait_until document.querySelector('.ready') timeout: 5s
/// @wait_until $loading == false timeout: 3s

%macro wait-until {

  %form {
    @wait_until $condition:expr ( timeout : $timeout:time = 5000 )?
  }

  %emit js {
    {
    const __timeout = ST.parseMs(%$timeout);
    const __start = Date.now();
    // In headless mode, use faster polling (or immediate for sync tests)
    const __pollInterval = globalThis.__spacetime_headless__ ? 1 : 50;

    while (Date.now() - __start < __timeout) {
      try {
        if (%$condition) break;
      } catch {}
      await new Promise(r => setTimeout(r, __pollInterval));
    }

    if (Date.now() - __start >= __timeout) {
      throw new Error(`Timeout after ${__timeout}ms waiting for condition`);
    }
    }
  }
}


/// @wait_for Macro
///
/// Waits for an element to appear in the DOM.
///
/// @example
/// @wait_for .loading-complete timeout: 10s
/// @wait_for [data-loaded] timeout: 5s

%macro wait-for {

  %form {
    @wait_for $selector:selector ( timeout : $timeout:time = 5000 )?
  }

  %emit js {
    {
    const __selector = %$selector;
    const __timeout = ST.parseMs(%$timeout);
    const __start = Date.now();
    // In headless mode, use faster polling
    const __pollInterval = globalThis.__spacetime_headless__ ? 1 : 50;

    while (Date.now() - __start < __timeout) {
      if (ST.subject(__selector)) break;
      await new Promise(r => setTimeout(r, __pollInterval));
    }

    if (!ST.subject(__selector)) {
      throw new Error(`Timeout after ${__timeout}ms waiting for ${__selector}`);
    }
    }
  }
}


/// @flush Macro
///
/// Yields to the microtask queue, allowing pending reactive updates
/// (signal watchers, derived recomputations, DOM bindings) to settle.
///
/// @example
/// @eval (ST.set(el, "count", 1))
/// @flush
/// @then [.counter] should have_text "1"

%macro flush {

  %form {
    @flush
  }

  %emit js {
    await new Promise(function(r) { queueMicrotask(r) });
  }
}


/// @fixture Macro
///
/// Creates a temporary DOM fixture for testing.
/// The fixture is automatically cleaned up after the test.
///
/// @example
/// @fixture {
///   <div data-toggle data-state="off">Toggle</div>
/// }

%macro fixture {

  %form {
    @fixture {
      $content:block
    }
  }

  %emit js {
    {
    const __fixture = document.createElement('div');
    __fixture.id = '__st_test_fixture_' + Date.now();
    __fixture.innerHTML = `%$content.fixture_html`;
    document.body.appendChild(__fixture);
    __ctx.fixture = __fixture;
    // Compile nested Spacetime directives (animations, events, state machines)
    %$content.js
    // Register cleanup
    __ctx.cleanup = __ctx.cleanup || [];
    __ctx.cleanup.push(() => __fixture.remove());
    }
  }
}


/// @mount Macro (PLAN-027 W3)
///
/// Mounts REAL Spacetime into the test world: the inline block is compiled as
/// real .st (markup + nested primitives/directives), not faked HTML. This is the
/// canonical replacement for @fixture — a test body is a real program. Multiple
/// @mount blocks compose into one world.
///
/// @example
/// @mount {
///   .box { $open bool: false; @on click { $open <- !$open } }
/// }
///
/// File-reference (`@mount "path.st"`) and self-reference (`@mount @self`) are
/// tracked as a follow-up (FEAT-066); inline real-.st mounting works today.
%macro mount {

  %form {
    @mount {
      $content:block
    }
  }

  %emit js {
    {
    const __mount = document.createElement('div');
    __mount.id = '__st_mount_' + Date.now();
    __mount.innerHTML = `%$content.fixture_html`;
    document.body.appendChild(__mount);
    __ctx.fixture = __mount;
    // Compile + run the mounted block's real Spacetime directives.
    %$content.js
    // Initialize the mounted component SYNCHRONOUSLY so a following @when can
    // drive it (init is normally microtask-scheduled; tests run synchronously).
    if (typeof ST !== 'undefined' && ST._flushInit) ST._flushInit();
    __ctx.cleanup = __ctx.cleanup || [];
    __ctx.cleanup.push(() => __mount.remove());
    }
  }
}


/// @cleanup Macro
///
/// Registers cleanup code to run after the test.
///
/// @example
/// @cleanup {
///   document.querySelector('.test-element').remove();
/// }

%macro cleanup {

  %form {
    @cleanup {
      $code:expr
    }
  }

  %emit js {
    __ctx.cleanup = __ctx.cleanup || [];
    __ctx.cleanup.push(() => { %$code });
  }
}


/// @snapshot Macro
///
/// Takes a DOM snapshot for visual regression testing.
///
/// @example
/// @snapshot "component initial state"
/// @snapshot "component after interaction" of .modal

/// Two SIBLING forms, not one with an optional clause (BUG-217).
///
/// `@snapshot $name:string of: $selector:selector? = ""` reads as "the selector is
/// optional", but the literal `of:` before it is NOT — a form matches its literals
/// in order, so the bare `@snapshot "name"` could never match and compiled to
/// NOTHING. Every bare-form snapshot test failed on state no one had captured, and
/// 13 of them sat red long enough to be taken as normal.
///
/// An optional TRAILING CLAUSE (literal + capture together) needs two forms. The
/// scoped one is tried first via the specificity ranking its extra literal earns,
/// and the bare one catches the rest. Same shape as `each` / `each-filtered`.
%macro snapshot-of {

  %form {
    @snapshot $name:string of: $selector:selector
  }

  %emit js {
    {
    const __snapName = %$name;
    const __snapSelector = %$selector;
    const __snapEl = __snapSelector
      ? document.querySelector(__snapSelector)
      : document.body;

    const __snapshot = {
      name: __snapName,
      html: __snapEl?.innerHTML,
      timestamp: Date.now()
    };

    __ctx.snapshots = __ctx.snapshots || [];
    __ctx.snapshots.push(__snapshot);

    // Store for comparison
    window.__spacetime_snapshots = window.__spacetime_snapshots || {};
    window.__spacetime_snapshots[__snapName] = __snapshot;
    }
  }
}

%macro snapshot {

  %form {
    @snapshot $name:string
  }

  %emit js {
    {
    const __snapName = %$name;
    const __snapSelector = "";
    const __snapEl = __snapSelector
      ? document.querySelector(__snapSelector)
      : document.body;

    const __snapshot = {
      name: __snapName,
      html: __snapEl?.innerHTML,
      timestamp: Date.now()
    };

    __ctx.snapshots = __ctx.snapshots || [];
    __ctx.snapshots.push(__snapshot);

    // Store for comparison
    window.__spacetime_snapshots = window.__spacetime_snapshots || {};
    window.__spacetime_snapshots[__snapName] = __snapshot;
    }
  }
}


/// @no_assert Macro
///
/// Explicit opt-out from BUG-309's zero-assertion rule. A test whose purpose is
/// 'this compiles' (no claim to make) marks its body with this directive to
/// register that running without assertions is intentional, so the runner does
/// not fail it as vacuous. Any @test that genuinely asserts must NOT use it.
///
/// @example
/// @test "macro x compiles" { @no_assert }
%macro no-assert {

  %form {
    @no_assert
  }

  %emit js {
    window.__st_markNoAssert();
  }
}


/// @assert Macro
///
/// General-purpose assertion for custom conditions.
///
/// @example
/// @assert $count > 0 "count should be positive"
/// @assert document.querySelectorAll('.item').length === 3

/// A failure message is most useful when it reports the value that failed
/// ("expected 3, got " + rows.length), which needs an EXPRESSION, not a literal.
///
/// The message slot is therefore an ordered choice: a bare STRING literal, or a
/// parenthesized expression. Before BUG-216 the slot was `:string?` alone, and a
/// parenthesized message simply did not match — which made the whole step VANISH
/// from the emitted bundle instead of erroring, so `@assert (false) ("x " + a.b)`
/// compiled to nothing and PASSED. A test that reported a computed value had
/// silently stopped asserting.
%macro assert {

  %form {
    @assert $condition:expr $message:string? = ""
  }

  %emit js {
    {
    window.__st_noteAssertion();
    const __cond = %$condition;
    const __msg = %$message;
    if (!__cond) {
      throw new Error((__msg === undefined || __msg === null || __msg === '') ? 'Assertion failed' : String(__msg));
    }
    }
  }
}


/// @eval Macro
///
/// Evaluates a raw JavaScript expression in test context.
/// Use for setup code that can't be expressed with other testing macros.
///
/// @example
/// @eval (window.__testVar = true)
/// @eval (document.body.innerHTML = '<div class="test"></div>')

%macro eval {

  %form {
    @eval $expr:expr
  }

  %emit js {
    %$expr;
  }
}


/// @let Macro
///
/// Binds a variable for use in LATER test steps. The expression is
/// `(name = value)`.
///
/// IMPORTANT — cross-step scope: every test-body step (@let / @eval / @then /
/// @when / @assert) compiles to a SEPARATE file-scope directive that the
/// pipeline wraps in its OWN IIFE (`(function(){ ... })()`), deliberately, so
/// sibling steps can't clash on a `const` (e.g. two `@media` `const mql`). A
/// `const`/`let` here would therefore be trapped inside the @let step's IIFE and
/// be invisible to the next step — the historic `X is not defined` failures
/// (counter/form/modal). So @let emits a BARE assignment: in the (non-strict)
/// test runtime that creates a property on the global object, which every later
/// step's IIFE closes over by the same bare name. One binding, visible suite-step
/// to suite-step, no IIFE-isolation regression for other directives.
///
/// @example
/// @let (obj = { name: 'Alice', age: 30 })
/// @assert (obj.name === 'Alice')

%macro let {

  %form {
    @let $assignment:expr
  }

  %emit js {
    %$assignment;
  }
}


/// @mock Macro
///
/// Mocks a function or API for testing.
///
/// @example
/// @mock fetch returns { ok: true, json: () => ({ data: [] }) }

%macro mock {

  %form {
    @mock $name:ident returns $value:expr
  }

  %emit js {
    {
    const __originalFn = window['%$name'];
    const __mockValue = %$value;

    window['%$name'] = typeof __mockValue === 'function'
      ? __mockValue
      : () => Promise.resolve(__mockValue);

    __ctx.mocks = __ctx.mocks || [];
    __ctx.mocks.push({ name: '%$name', original: __originalFn });

    __ctx.cleanup = __ctx.cleanup || [];
    __ctx.cleanup.push(() => {
      window['%$name'] = __originalFn;
    });
    }
  }
}


/// @try Macro
///
/// Wraps code that might throw, capturing the error into the test-local `$error`.
/// Used for testing error-throwing behavior. A later `@assert ($error !== null)`
/// reads the caught error by the same name.
///
/// IMPLEMENTATION (BUG-112): `$error` is a plain JS test-local, NOT a page signal.
/// The emit body writes it as `$$error` — the `$$` escape (sibling of `%%`->`%`)
/// collapses to a LITERAL `$error` identifier instead of lowering through
/// `ST.get(el, "error")` (a signal-get, which is an invalid assignment target and
/// references an `el` that selectorless test directives no longer bind after
/// FUP-072). Like @let, it is a bare (global) assignment so the binding survives
/// the per-step IIFE and is visible to the following @assert step by the same name.
///
/// @example
/// @try { throwingFn() }
/// @assert ($error !== null) "Should have thrown"

%macro try {

  %form {
    @try {
      $body:block
    }
  }

  %emit js {
    $$error = null;
    try {
      %$body.js
    } catch (__e) {
      $$error = __e;
    }
  }
}


/// @run Macro
///
/// Executes arbitrary JavaScript expression within a test.
/// Use sparingly - prefer declarative @given/@when/@then.
///
/// @example
/// @run document.querySelector('.item').textContent = 'Updated'

%macro run {

  %form {
    @run $code:expr
  }

  %emit js {
    %$code;
  }
}

