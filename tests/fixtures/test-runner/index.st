/// =============================================================================
/// Spacetime Test Runner - Live 3-Pane UI
/// =============================================================================
///
/// A Cypress-style test runner built entirely in Spacetime DSL.
/// Features live reload, WebSocket integration, and state-based animations.

@import "./modules/presets.st"

/// =============================================================================
/// Type Definitions
/// =============================================================================

@type TestCase {
  id: string
  name: string
  status: "pending" | "running" | "passed" | "failed" | "skipped"
  duration: number?
  error: string?
  logs: LogEntry[]
}

@type LogEntry {
  level: "log" | "info" | "warn" | "error"
  message: string
  timestamp: number
}

@type TestSuite {
  id: string
  name: string
  status: "pending" | "running" | "passed" | "failed"
  tests: TestCase[]
}

@type TestResults {
  passed: number
  failed: number
  pending: number
  progress: number
}


/// =============================================================================
/// Data Sources
/// =============================================================================

@data suites: TestSuite[] {
  src: "/api/tests"
  cache: 0
}


/// =============================================================================
/// Test Runner Root - Local State
/// =============================================================================

.test-runner {
  $selectedTest: "";
  $testResults: { passed: 0, failed: 0, pending: 0, progress: 0 };
  $changedFiles: [];
  $filterQuery: "";
  $watchMode: true;

  /// WebSocket Connection for Live Reload
  @websocket(url: "ws://localhost:3030/ws", reconnect: 2000ms) {
    /// Handle reload messages from file watcher
    @on Reload {
      $changedFiles <- $.files;
      trigger("refresh-preview");
    }

    /// Handle test result updates
    @on TestResults {
      $testResults <- $.results;
      trigger("update-stats");
    }

    /// Handle ping/pong for connection health
    /// TODO: Implement send() function
    /// @on Ping {
    ///   send({ type: "Pong" });
    /// }

    /// Handle errors
    /// TODO: Implement console.error() function
    /// @on Error {
    ///   console.error("WebSocket error:", $.message);
    /// }
  }
}


/// =============================================================================
/// Layout Animations - Panel Reveal on Load
/// =============================================================================

.runner {
  display: grid;
  grid-template-columns: 280px 1fr 320px;
  grid-template-rows: 1fr auto;
  height: 100vh;
  gap: 0;

  @on load runner-init(800ms) {
    > .runner__tree {
      opacity: 0 -> 1;
      translate-x: -20px -> 0;
      easing: --ease-out;
    }

    > .runner__preview {
      opacity: 0 -> 1;
      scale: 0.98 -> 1;
      delay: 200ms;
      easing: --ease-out;
    }

    > .runner__inspector {
      opacity: 0 -> 1;
      translate-x: 20px -> 0;
      delay: 300ms;
      easing: --ease-out;
    }
  }
}


/// =============================================================================
/// Left Panel - Test Tree
/// =============================================================================

.runner__tree {
  background: var(--surface-1);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  overflow: hidden;

  .panel__header {
    padding: 1rem;
    border-bottom: 1px solid var(--border);
    display: flex;
    justify-content: space-between;
    align-items: center;

    h2 {
      font-size: 1rem;
      font-weight: 600;
      color: var(--text);
    }
  }

  .tree__toolbar {
    padding: 0.75rem;
    border-bottom: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .tree__list {
    flex: 1;
    overflow-y: auto;
    padding: 0.5rem;
  }
}


/// WebSocket Connection Status Indicator
/// Reactive on $wsConnected (set by the WS open/close handler); the indicator
/// color tracks the signal.
.ws-status {
  .is-connected: $wsConnected;

  &::before {
    content: "●";
    font-size: 0.75rem;
    color: var(--error);
  }

  &.is-connected::before {
    color: var(--success);
  }

  font-size: 0.75rem;
  display: flex;
  align-items: center;
  gap: 0.25rem;

  &::after {
    content: attr(data-label);
    color: var(--muted);
  }
}


/// Test Suite Component
.suite {
  margin: 0.25rem 0;
  border-radius: 4px;
  overflow: hidden;

  .suite__header {
    padding: 0.5rem 0.75rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: var(--surface-2);
    list-style: none;

    @on &.hover suite-hover(200ms) {
      background: transparent -> var(--hover)
      easing: --ease-out
    }

    .suite__icon {
      font-size: 0.75rem;
      opacity: 0.6;
    }

    .suite__name {
      flex: 1;
      font-size: 0.875rem;
      font-weight: 500;
    }

    .suite__count {
      font-size: 0.75rem;
      color: var(--muted);
    }
  }

  .suite__tests {
    padding: 0.25rem;

    @on toggle suite-toggle(300ms) {
      max-height: 0 -> auto
      opacity: 0 -> 1
      easing: --ease-out
    }
  }
}


/// Test Case Component
.test {
  padding: 0.5rem 0.75rem;
  margin: 0.125rem 0;
  border-radius: 4px;
  display: flex;
  align-items: center;
  gap: 0.5rem;
  cursor: pointer;

  /// Click to select test
  /// TODO: Implement click handler when imperative @on click is supported
  /// @on click {
  ///   $selectedTest: $test;
  ///   trigger("test-selected");
  /// }

  /// Hover effect
  @on &.hover test-hover(200ms) {
    background: transparent -> var(--hover);
    easing: --ease-out;
  }

  /// Selection state
  @states {
    [data-selected="true"] {
      background: var(--primary-dim);
      border-left: 2px solid var(--primary);
    }
  }

  .test__icon {
    font-size: 1rem;
    width: 1rem;
    text-align: center;
  }

  /// Status-based icon and animation
  .test__icon[data-status="pending"]::before {
    content: "○";
    color: var(--muted);
  }

  .test__icon[data-status="running"]::before {
    content: "◐";
    color: var(--info);

    @loop spin(1s) {
      rotate: 0deg -> 360deg;
      easing: --linear;
    }
  }

  .test__icon[data-status="passed"]::before {
    content: "✓";
    color: var(--success);

    @time pass-pop(300ms) {
      scale: 0.8 -> 1.3 -> 1;
      easing: --bounce;
    }
  }

  .test__icon[data-status="failed"]::before {
    content: "✗";
    color: var(--error);

    @time fail-shake(400ms) {
      translate-x: 0 -> -3px -> 3px -> -2px -> 2px -> 0;
    }
  }

  .test__icon[data-status="skipped"]::before {
    content: "○";
    color: var(--muted);
    opacity: 0.5;
  }

  .test__name {
    flex: 1;
    font-size: 0.875rem;
  }

  .test__duration {
    font-size: 0.75rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
}


/// =============================================================================
/// Center Panel - Live Preview
/// =============================================================================

.runner__preview {
  background: var(--surface-0);
  display: flex;
  flex-direction: column;

  .panel__header {
    padding: 1rem;
    border-bottom: 1px solid var(--border);
    display: flex;
    justify-content: space-between;
    align-items: center;

    h2 {
      font-size: 1rem;
      font-weight: 600;
    }

    .preview__controls {
      display: flex;
      gap: 0.25rem;
    }
  }

  .preview__viewport {
    flex: 1;
    padding: 1rem;
    overflow: hidden;
  }

  .preview__iframe {
    width: 100%;
    height: 100%;
    border: none;
    border-radius: 8px;
    background: white;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);

    /// Reload animation triggered by WebSocket
    @on reload preview-reload(400ms) {
      opacity: 1 -> 0.5 -> 1
      scale: 1 -> 0.99 -> 1
      easing: --ease-in-out
    }
  }
}


/// =============================================================================
/// Right Panel - Inspector
/// =============================================================================

.runner__inspector {
  background: var(--surface-1);
  border-left: 1px solid var(--border);
  display: flex;
  flex-direction: column;

  $activeTab <- "console";

  .panel__header {
    padding: 1rem;
    border-bottom: 1px solid var(--border);

    h2 {
      font-size: 0.875rem;
      font-weight: 500;
      color: $selectedTest?.name ?? "No test selected";
    }
  }

  .inspector__tabs {
    display: flex;
    border-bottom: 1px solid var(--border);
    background: var(--surface-2);

    // Tab selection via $devTab signal + reactive class (per-tab .is-active below).
    // `.tab { .is-active: $devTab == &.dataset.tab }` is not expressible (no per-element
    // attr compare), so each tab compares to its literal id where the markup defines it.

    .tab {
      padding: 0.75rem 1rem;
      border: none;
      background: none;
      color: var(--muted);
      font-size: 0.875rem;
      cursor: pointer;
      border-bottom: 2px solid transparent;
      transition: color 150ms ease-out, border-color 150ms ease-out;

      @on &.hover {
        color: var(--text);
      }

      @states {
        [data-active]  {
          color: var(--primary);
          border-bottom-color: var(--primary);
        }
      }
    }
  }

  .inspector__content {
    flex: 1;
    overflow: hidden;

    .tab-panel {
      height: 100%;
      display: none;
      overflow-y: auto;

      @states {
        [data-active] {
          display: block;
        }
      }
    }
  }
}


/// Console Output Panel
.console-output {
  padding: 0.5rem;
  font-family: ui-monospace, 'Cascadia Code', 'Source Code Pro', monospace;
  font-size: 0.75rem;
}


/// =============================================================================
/// First-Party Inspector Panels
/// =============================================================================

/// Shared viz section styles
.viz-section {
  padding: 0.5rem;

  .viz-section__title {
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--muted);
    margin-bottom: 0.75rem;
  }
}


/// Timeline Visualization (First-Party)
.timeline-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.timeline-entry {
  background: var(--surface-2);
  border-radius: 4px;
  padding: 0.5rem 0.75rem;

  @on enter timeline-entry-enter(200ms) {
    opacity: 0 -> 1;
    translate-y: 10px -> 0;
    easing: --ease-out;
  }

  .timeline-entry__header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.25rem;
  }

  .timeline-entry__name {
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--text);
  }

  .timeline-entry__progress {
    font-size: 0.75rem;
    font-variant-numeric: tabular-nums;
    color: var(--primary);
  }

  .timeline-entry__bar {
    height: 4px;
    background: var(--border);
    border-radius: 2px;
    overflow: hidden;
    margin-bottom: 0.25rem;
  }

  .timeline-entry__fill {
    height: 100%;
    background: var(--primary);
    border-radius: 2px;
    transition: width 100ms linear;

    @states {
      [data-status="running"] {
        background: var(--info);
      }
      [data-status="complete"] {
        background: var(--success);
      }
    }
  }

  .timeline-entry__meta {
    display: flex;
    gap: 0.75rem;
    font-size: 0.75rem;
    color: var(--muted);
  }

  .timeline-entry__driver {
    &::before {
      content: "⚡";
      margin-right: 0.25rem;
    }
  }

  .timeline-entry__duration {
    &::before {
      content: "⏱";
      margin-right: 0.25rem;
    }
  }
}


/// State Visualization (First-Party)
.state-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.state-entry {
  background: var(--surface-2);
  border-radius: 4px;
  padding: 0.5rem 0.75rem;

  @on enter state-entry-enter(200ms) {
    opacity: 0 -> 1;
    translate-y: 10px -> 0;
    easing: --ease-out;
  }

  .state-entry__header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  .state-entry__selector {
    font-family: ui-monospace, monospace;
    font-size: 0.75rem;
    color: var(--muted);
  }

  .state-entry__current {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--primary);
    padding: 0.125rem 0.375rem;
    background: var(--primary-dim);
    border-radius: 3px;
  }

  .state-entry__history {
    display: flex;
    gap: 0.25rem;
    flex-wrap: wrap;
    margin-bottom: 0.5rem;

    /// State history node
    .state-node {
      padding: 0.125rem 0.375rem;
      background: var(--surface-0);
      border-radius: 3px;
      font-size: 0.75rem;
      color: var(--muted);

      &[data-current="true"] {
        background: var(--primary-dim);
        color: var(--primary);
      }

      /// Arrow between states
      &:not(:last-child)::after {
        content: "→";
        margin-left: 0.5rem;
        color: var(--border);
      }
    }
  }

  .state-entry__meta {
    font-size: 0.75rem;
    color: var(--muted);
  }
}


/// Signal Visualization (First-Party)
.signal-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.signal-entry {
  background: var(--surface-2);
  border-radius: 4px;
  padding: 0.5rem 0.75rem;

  @on enter signal-entry-enter(200ms) {
    opacity: 0 -> 1;
    translate-y: 10px -> 0;
    easing: --ease-out;
  }

  /// Flash animation when signal updates
  @on update signal-update-flash(400ms) {
    background: var(--surface-2) -> var(--primary-dim) -> var(--surface-2);
    easing: --ease-out;
  }

  .signal-entry__header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.25rem;
  }

  .signal-entry__name {
    font-family: ui-monospace, monospace;
    font-size: 0.875rem;
    color: var(--primary);

    &::before {
      content: "$";
      opacity: 0.6;
    }
  }

  .signal-entry__value {
    font-family: ui-monospace, monospace;
    font-size: 0.875rem;
    color: var(--text);
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .signal-entry__meta {
    display: flex;
    gap: 0.75rem;
    font-size: 0.75rem;
    color: var(--muted);
  }

  .signal-entry__type {
    &::before {
      content: "📦";
      margin-right: 0.25rem;
    }
  }

  .signal-entry__updates {
    &::before {
      content: "↺";
      margin-right: 0.25rem;
    }
  }

  .signal-entry__watchers {
    &::before {
      content: "👁";
      margin-right: 0.25rem;
    }
  }
}


/// Empty State for Inspector Panels
.viz-empty {
  padding: 2rem;
  text-align: center;
  color: var(--muted);
  font-size: 0.875rem;

  .viz-empty__icon {
    font-size: 2rem;
    margin-bottom: 0.5rem;
    opacity: 0.5;
  }
}


/// Console Output Panel Styles (continued from above)

  /// Populate with logs from selected test
  /// TODO: Re-enable when @each supports complex expressions
  /// @each $log in $selectedTest?.logs ?? [] {
  ///   > .log-entry {
  ///     padding: 0.25rem 0.5rem;
  ///     margin: 0.125rem 0;
  ///     border-radius: 2px;
  ///     display: flex;
  ///     gap: 0.5rem;
  ///     align-items: flex-start;
  ///
  ///     /// Entry animation
  ///     :entering {
  ///       opacity: 0 -> 1
  ///       translate-y: 10px -> 0
  ///       duration: 200ms
  ///       easing: --ease-out
  ///     }
  ///
  ///     @states {
  ///       [data-level="log"] {
  ///         color: var(--text);
  ///       }
  ///       [data-level="info"] {
  ///         color: var(--info);
  ///       }
  ///       [data-level="warn"] {
  ///         background: rgba(217, 153, 34, 0.1);
  ///         color: var(--warning);
  ///       }
  ///       [data-level="error"] {
  ///         background: rgba(248, 81, 73, 0.1);
  ///         color: var(--error);
  ///       }
  ///     }
  ///
  ///     .log__time {
  ///       color: var(--muted);
  ///       flex-shrink: 0;
  ///     }
  ///
  ///     .log__icon {
  ///       flex-shrink: 0;
  ///     }
  ///
  ///     .log__message {
  ///       flex: 1;
  ///       word-break: break-word;
  ///     }
  ///   }
  /// }
}


/// =============================================================================
/// Bottom Panel - Status Bar
/// =============================================================================

.runner__status {
  grid-column: 1 / -1;
  background: var(--surface-2);
  border-top: 1px solid var(--border);
  padding: 0.75rem 1rem;
  display: flex;
  gap: 1rem;
  align-items: center;

  .status__progress {
    flex: 1;
    height: 4px;
    background: var(--border);
    border-radius: 2px;
    overflow: hidden;

    .progress__bar {
      height: 100%;
      background: var(--primary);
      width: calc($testResults.progress * 100%);
      transition: width 200ms ease-out;

      @loop progress-pulse(1s) {
        opacity: 1 -> 0.7 -> 1
        easing: --ease-in-out
      }
    }
  }

  .status__stats {
    display: flex;
    gap: 1rem;
    font-size: 0.875rem;

    .stat {
      display: flex;
      align-items: center;
      gap: 0.25rem;

      .stat__icon {
        font-size: 1rem;
      }

      .stat__value {
        font-weight: 600;
        font-variant-numeric: tabular-nums;
      }
    }

    .stat--passed {
      color: var(--success);

      .passed-count {
        text: $testResults.passed;
      }
    }

    .stat--failed {
      color: var(--error);

      .failed-count {
        text: $testResults.failed;
      }
    }

    .stat--pending {
      color: var(--muted);

      .pending-count {
        text: $testResults.pending;
      }
    }
  }

  .status__actions {
    display: flex;
    gap: 0.5rem;
  }
}


/// =============================================================================
/// Button Styles
/// =============================================================================

.btn {
  padding: 0.5rem 1rem;
  border: none;
  border-radius: 4px;
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 150ms ease-out;
  display: flex;
  align-items: center;
  gap: 0.5rem;

  @on &.click {
    @time btn-press(100ms) {
      scale: 1 -> 0.95 -> 1
    }
  }

  .btn__icon {
    font-size: 1rem;
  }
}

.btn--primary {
  background: var(--primary);
  color: white;

  @on &.hover {
    background: var(--primary-bright);
  }
}

.btn--secondary {
  background: var(--surface-2);
  color: var(--text);

  @on &.hover {
    background: var(--hover);
  }
}

.btn--sm {
  padding: 0.375rem 0.75rem;
  font-size: 0.75rem;
}

.btn--icon {
  padding: 0.375rem;
  background: transparent;
  color: var(--muted);

  @on &.hover {
    background: var(--hover);
    color: var(--text);
  }
}


/// =============================================================================
/// Form Controls
/// =============================================================================

.search-input {
  padding: 0.5rem 0.75rem;
  background: var(--surface-0);
  border: 1px solid var(--border);
  border-radius: 4px;
  color: var(--text);
  font-size: 0.875rem;
  width: 100%;

  &::placeholder {
    color: var(--muted);
  }

  @on &.focus {
    border-color: var(--primary);
    outline: none;
  }
}


/// =============================================================================
/// Helper Functions
/// =============================================================================

@fn formatDuration($ms number): string {
  ms < 1000 ? ms + "ms" : (ms / 1000).toFixed(2) + "s"
}


/// =============================================================================
/// Steps Panel - Step-Through Visualization
/// =============================================================================

.steps-viz {
  padding: 0;
  height: 100%;
  display: flex;
  flex-direction: column;
}

.steps-list {
  flex: 1;
  overflow-y: auto;
  padding: 0.5rem;
}

.steps-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 2rem;
  color: var(--muted);
  text-align: center;

  .steps-empty__icon {
    font-size: 2rem;
    margin-bottom: 0.5rem;
    opacity: 0.5;
  }

  .steps-empty__text {
    font-size: 0.875rem;
  }
}

.step-entry {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  margin: 0.25rem 0;
  background: var(--surface-2);
  border-radius: 4px;
  border-left: 3px solid transparent;

  @on enter step-enter(200ms) {
    opacity: 0 -> 1;
    translate-y: 10px -> 0;
    easing: --ease-out;
  }

  .step-entry__indicator {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 50%;
    background: var(--muted);
    flex-shrink: 0;
  }

  .step-entry__content {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 0;
  }

  .step-entry__type {
    flex-shrink: 0;
  }

  .step-entry__selector {
    font-family: ui-monospace, monospace;
    font-size: 0.75rem;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .step-entry__action {
    font-size: 0.75rem;
    color: var(--muted);
  }

  .step-entry__duration {
    font-size: 0.625rem;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }

  /// Step status states
  @states {
    [data-step-status="pending"] {
      .step-entry__indicator {
        background: var(--muted);
      }
    }

    [data-step-status="running"] {
      border-left-color: var(--info);

      .step-entry__indicator {
        background: var(--info);
        @loop pulse(1s) {
          scale: 1 -> 1.3 -> 1;
          opacity: 1 -> 0.5 -> 1;
          easing: --ease-in-out;
        }
      }
    }

    [data-step-status="passed"] {
      border-left-color: var(--success);

      .step-entry__indicator {
        background: var(--success);
      }

      @time pass-flash(300ms) {
        background: var(--surface-2) -> rgba(63, 185, 80, 0.1) -> var(--surface-2);
      }
    }

    [data-step-status="failed"] {
      border-left-color: var(--error);
      background: rgba(248, 81, 73, 0.05);

      .step-entry__indicator {
        background: var(--error);
      }

      @time fail-flash(300ms) {
        background: var(--surface-2) -> rgba(248, 81, 73, 0.1) -> rgba(248, 81, 73, 0.05);
      }
    }

    /// Current step highlight when stepping through
    [data-step-current="true"] {
      box-shadow: 0 0 0 2px var(--primary);
    }
  }
}


/// =============================================================================
/// Theme Variables
/// =============================================================================

:root {
  --surface-0: #0d1117;
  --surface-1: #161b22;
  --surface-2: #21262d;
  --border: #30363d;
  --muted: #8b949e;
  --text: #c9d1d9;

  --primary: #58a6ff;
  --primary-bright: #79b8ff;
  --primary-dim: rgba(88, 166, 255, 0.1);

  --success: #3fb950;
  --error: #f85149;
  --warning: #d29922;
  --info: #58a6ff;

  --hover: rgba(255, 255, 255, 0.05);
  --shadow-lg: 0 8px 24px rgba(0, 0, 0, 0.4);
}
