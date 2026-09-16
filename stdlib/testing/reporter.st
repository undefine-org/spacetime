/// Test Reporter UI
///
/// A visual test reporter built with Spacetime, demonstrating
/// the language's own capabilities for its test framework.
///
/// @example usage
/// <test-reporter>
///   <div slot="summary">
///     <span slot="passed">0</span> passed,
///     <span slot="failed">0</span> failed,
///     <span slot="skipped">0</span> skipped
///   </div>
///   <ul slot="tests"></ul>
///   <ul slot="errors"></ul>
/// </test-reporter>


/// Test Reporter Primitive
///
/// Provides reactive data for the test reporter UI.
///
/// @yields $status - Current reporter status (idle, running, passed, failed)
/// @yields $progress - Test progress (0-1)
/// @yields $summary - Summary text

%primitive test_reporter_data(&container) {
  %emit js {
    let __status = 'idle';
    let __progress = 0;

    const updateStatus = (s) => {
      __status = s;
      %yield __status -> $status;
    };

    const updateProgress = (current, total) => {
      __progress = total > 0 ? current / total : 0;
      %yield __progress -> $progress;
    };

    // Listen for test events
    %&container.addEventListener('test.start', (e) => {
      updateStatus('running');
      updateProgress(0, e.detail.total);
    });

    %&container.addEventListener('test.run', () => {
      // Could update progress here if we track individual test runs
    });

    %&container.addEventListener('test.pass', () => {
      // Individual test passed
    });

    %&container.addEventListener('test.fail', () => {
      // Individual test failed
    });

    %&container.addEventListener('test.complete', (e) => {
      const { passed, failed, skipped } = e.detail;
      updateStatus(failed > 0 ? 'failed' : 'passed');
      updateProgress(1, 1);
      %yield `${passed} passed, ${failed} failed, ${skipped} skipped` -> $summary;
    });

    %yield __status -> $status;
    %yield __progress -> $progress;
    %yield '' -> $summary;
  }

  %exports {
    $status: string
    $progress: number
    $summary: string
  }
}


/// Test Reporter Component Styles
///
/// Styles the test reporter UI with state-based animations.

[data-test-reporter] {
  // Bind to test reporter data
  %binds {
    test_reporter_data(&self) -> { $status, $progress, $summary }
  }

  // Base styles
  font-family: system-ui, -apple-system, sans-serif;
  padding: 1rem;
  border-radius: 0.5rem;
  background: oklch(0.98 0 0);
  border: 1px solid oklch(0.9 0 0);

  // Summary section — reactive on the $status signal yielded by test_reporter_data;
  // $status IS the state, tracked via reactive classes.
  [slot="summary"] {
    .is-idle: $status == "idle";
    .is-running: $status == "running";
    .is-passed: $status == "passed";
    .is-failed: $status == "failed";

    padding: 1rem;
    border-radius: 0.25rem;
    font-weight: 500;
    transition: background 300ms ease-out;
  }
  [slot="summary"].is-idle    { background: oklch(0.95 0 0);    color: oklch(0.4 0 0); }
  [slot="summary"].is-running { background: oklch(0.9 0.08 80);  color: oklch(0.3 0.1 80); }
  [slot="summary"].is-passed  { background: oklch(0.9 0.12 145); color: oklch(0.3 0.15 145); }
  [slot="summary"].is-failed  { background: oklch(0.9 0.12 25);  color: oklch(0.3 0.15 25); }

  // Progress bar
  [slot="progress"] {
    height: 4px;
    background: oklch(0.9 0 0);
    border-radius: 2px;
    overflow: hidden;
    margin: 0.5rem 0;

    &::after {
      content: '';
      display: block;
      height: 100%;
      background: oklch(0.6 0.15 145);
      width: calc(var(--st-progress, 0) * 100%);
      transition: width 200ms ease-out;
    }
  }

  // Test list container
  [slot="tests"] {
    list-style: none;
    padding: 0;
    margin: 1rem 0;

    // Individual test items rendered by @each
    > [data-test-item] {
      padding: 0.5rem 0.75rem;
      margin: 0.25rem 0;
      border-radius: 0.25rem;
      display: flex;
      align-items: center;
      gap: 0.5rem;
      transition: background 200ms ease-out, opacity 200ms ease-out;

      // State-as-attribute: the per-item `data-state` (set reactively by the @each
      // binding below) drives styling via attribute selectors — the same
      // `[data-state="x"]` channel the status icon (&::before) uses.
      &[data-state="pending"] { background: oklch(0.95 0 0);    opacity: 0.6; }
      &[data-state="running"] { background: oklch(0.95 0.05 80); opacity: 1; }
      &[data-state="passed"]  { background: oklch(0.95 0.08 145); opacity: 1; }
      &[data-state="failed"]  { background: oklch(0.95 0.08 25);  opacity: 1; }
      &[data-state="skipped"] { background: oklch(0.95 0 0);    opacity: 0.5; }

      // Status icon
      &::before {
        content: '';
        width: 0.5rem;
        height: 0.5rem;
        border-radius: 50%;
        flex-shrink: 0;
      }

      &[data-state="pending"]::before {
        background: oklch(0.7 0 0);
      }

      &[data-state="running"]::before {
        background: oklch(0.7 0.15 80);
        animation: pulse 1s ease-in-out infinite;
      }

      &[data-state="passed"]::before {
        background: oklch(0.6 0.2 145);
      }

      &[data-state="failed"]::before {
        background: oklch(0.6 0.2 25);
      }

      &[data-state="skipped"]::before {
        background: oklch(0.7 0 0);
        opacity: 0.5;
      }

      // Test name
      [slot="name"] {
        flex: 1;
        font-size: 0.875rem;
      }

      // Duration
      [slot="duration"] {
        font-size: 0.75rem;
        color: oklch(0.5 0 0);
      }

      // Entry animation
      :entering {
        @fade-in(distance: 10px, duration: 200ms)
      }
    }
  }

  // Error list
  [slot="errors"] {
    list-style: none;
    padding: 0;
    margin: 1rem 0;

    > [data-error-item] {
      padding: 0.75rem;
      margin: 0.5rem 0;
      border-radius: 0.25rem;
      background: oklch(0.95 0.08 25);
      border-left: 3px solid oklch(0.6 0.2 25);

      :entering {
        @fade-in(distance: 10px, duration: 200ms)
      }

      :exiting {
        @fade-out(duration: 150ms)
      }

      [slot="name"] {
        font-weight: 500;
        color: oklch(0.3 0.15 25);
        margin-bottom: 0.25rem;
      }

      [slot="message"] {
        font-size: 0.875rem;
        color: oklch(0.4 0.1 25);
        font-family: ui-monospace, monospace;
      }

      [slot="stack"] {
        font-size: 0.75rem;
        color: oklch(0.5 0.05 25);
        font-family: ui-monospace, monospace;
        margin-top: 0.5rem;
        white-space: pre-wrap;
        display: none;
      }

      &[data-expanded] [slot="stack"] {
        display: block;
      }
    }
  }

  // Controls
  [slot="controls"] {
    display: flex;
    gap: 0.5rem;
    margin-top: 1rem;

    button {
      padding: 0.5rem 1rem;
      border: none;
      border-radius: 0.25rem;
      font-size: 0.875rem;
      cursor: pointer;
      transition: background 150ms ease-out, transform 100ms ease-out;

      &:active {
        transform: scale(0.98);
      }

      &[data-action="run"] {
        background: oklch(0.5 0.15 240);
        color: white;

        &:hover {
          background: oklch(0.45 0.15 240);
        }
      }

      &[data-action="clear"] {
        background: oklch(0.92 0 0);
        color: oklch(0.3 0 0);

        &:hover {
          background: oklch(0.88 0 0);
        }
      }
    }
  }
}

// Pulse animation for running indicator
@keyframes pulse {
  0%, 100% {
    opacity: 1;
    transform: scale(1);
  }
  50% {
    opacity: 0.6;
    transform: scale(1.2);
  }
}


/// Test List Binding
///
/// Binds test list to the reporter.

%macro test-list {

  %form {
    @test_list
  }

  %binds {
    test_runner(&self) -> { $tests, $results, $currentTest }
  }

  @each($tests) {
    template: "test-item";
    [slot="name"]: $.name;

    // The per-item state is the item's own status field, projected onto `data-state`
    // (which the test-item template styles via `[data-state="x"]`). $.status is set
    // by the test runner in $results; the reactive attribute tracks it.
    data-state <- $.status;
  }
}


/// Error List Binding
///
/// Binds error list to the reporter.

%macro error-list {

  %form {
    @error_list
  }

  %binds {
    test_runner(&self) -> { $results }
  }

  @each($results.errors) {
    template: "error-item";
    [slot="name"]: $.name;
    [slot="message"]: $.error;
    [slot="stack"]: $.stack;

    :entering {
      @fade-in(distance: 10px, duration: 200ms)
    }

    :exiting {
      @fade-out(duration: 150ms)
    }
  }
}
