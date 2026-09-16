/// Forge Deck — HOST shell (FEAT-151).
///
/// Mounted standalone via `st_mount { source: <this file>, agent_control: true }`.
/// `agent_control: true` means its otherwise-unknown `@mcp-action` clicks route to
/// the DRIVING AGENT through `st_await` instead of being rejected at the sink
/// (PLAN-045) — the human clicks a forge directive, the agent receives it, drives
/// a headless Blender build (tools/blender-mcp), and recomposes `forge-preview.st`
/// into THIS page's `stage` region with the fresh export — one DOM, no reload.
///
/// Data flow (no page JavaScript anywhere):
///   - `$mcpInput.steps`     — the five forge directives, each `{id,label,done}`.
///   - `$mcpInput.activeId`  — the currently active/next directive id (agent-set
///                             on every re-mount so the rail highlights in sync
///                             with the real forge state, not just clicks).
///   - `$mcpInput.log`       — terse agent log lines, newest first.
///   - `data-st-region="stage"` — the region the agent composes forge-preview.st
///     into via `st_mount { host: <this instance>, region: "stage" }`.

@import "stdlib/__mcp__";

// @mcp-input publishes the mounted instance's input.json as $mcpInput (the
// generic binding, mcp-host.st's %macro mcp-input) -- no local @data override:
// a local `@data inline $mcpInput` would collide with this binding's writer.
@mcp-input;

@template &forge-step($step) {
  <button class="forge-step" data-step-id="`$step.id`" data-done="`$step.done`" data-active="`$step.id == $mcpInput.activeId`">
    <span class="forge-step-dot"></span>
    <span class="forge-step-label">`$step.label`</span>
  </button>
}

@template &forge-log-line($entry) {
  <p class="forge-log-line">`$entry`</p>
}

<div class="forge-deck" data-st-instance>

  <!-- Top status bar: wordmark + agent connection state. -->
  <header class="forge-topbar">
    <span class="forge-wordmark">Forge Deck</span>
    <span class="forge-status"><span class="forge-status-dot"></span>connected to agent</span>
  </header>

  <div class="forge-body">
    <!-- Left rail: forge directives + agent log. -->
    <aside class="forge-rail">
      <p class="forge-rail-label">Forge directives</p>
      <div class="forge-steps"></div>
      <p class="forge-rail-label forge-log-label">Agent log</p>
      <div class="forge-log"></div>
    </aside>

    <!-- The live 3D stage. The agent composes forge-preview.st here. -->
    <main class="forge-stage-region" data-st-region="stage">
      <div class="forge-stage-empty">
        <p class="forge-stage-empty-note">Waiting for the agent to compose the preview stage&hellip;</p>
      </div>
    </main>
  </div>
</div>

// Region host bridge (nearest [data-st-instance] — the deck root).
[data-st-instance] {
  @mcp-host
}

// Stage region mount (FEAT-126 / BUG-111): @mcp-region polls this instance's
// stage region for a composed guest bundle (st_mount { host, region: "stage" });
// @view (bound on the SAME root) mounts the guest's main template reactively the
// moment registerBundle sets this root's element-scoped stageTpl signal.
.forge-stage-region {
  @mcp-region
  @view $stageTpl {
    _ => &$stageTpl();
  }
}

.forge-steps {
  @each($mcpInput.steps as $step) {
    &forge-step($step);
  }
}

.forge-log {
  @each($mcpInput.log as $entry) {
    &forge-log-line($entry);
  }
}

// Each directive click emits a semantic action the agent reads via st_await:
// { action: "forge-directive", target: "<step id>" }. agent_control:true on
// the mount routes this (an otherwise-unknown action) straight to the agent.
.forge-step {
  @mcp-action(action: "forge-directive", targetAttr: "data-step-id")
}

// ── Warm-dark "Forge Deck" styling (self-contained; no inherited chrome). ──
.forge-deck {
  --forge-bg: #0a0906;
  --forge-panel: rgba(36, 30, 25, 0.55);
  --forge-panel-line: rgba(122, 107, 84, 0.32);
  --forge-cream: #f1ece0;
  --forge-taupe: #b8a98f;
  --forge-ember: #e0915a;
  --forge-ember-hot: #ed9f6a;
  --forge-green: #6fbf73;
  --forge-serif: "Fraunces", "Source Serif 4", Georgia, serif;
  --forge-sans: Inter, ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif;
  box-sizing: border-box;
  display: grid;
  grid-template-rows: auto 1fr;
  width: 100%;
  min-height: 100vh;
  background: var(--forge-bg);
  color: var(--forge-cream);
  font-family: var(--forge-sans);
}
.forge-deck * { box-sizing: border-box; }

.forge-topbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 24px;
  border-bottom: 1px solid var(--forge-panel-line);
}
.forge-wordmark {
  font-family: var(--forge-serif);
  font-size: 20px;
  font-weight: 600;
  letter-spacing: -0.01em;
}
.forge-status {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  color: var(--forge-taupe);
}
.forge-status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--forge-green);
  box-shadow: 0 0 0 0 rgba(111, 191, 115, 0.6);
  animation: forge-pulse 1.8s ease-out infinite;
}
@keyframes forge-pulse {
  0%   { box-shadow: 0 0 0 0 rgba(111, 191, 115, 0.55); }
  70%  { box-shadow: 0 0 0 8px rgba(111, 191, 115, 0); }
  100% { box-shadow: 0 0 0 0 rgba(111, 191, 115, 0); }
}

.forge-body {
  display: grid;
  grid-template-columns: 280px 1fr;
  min-height: 0;
}

.forge-rail {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 20px 16px;
  border-right: 1px solid var(--forge-panel-line);
  overflow-y: auto;
}
.forge-rail-label {
  margin: 4px 0 0;
  font-size: 11px;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--forge-taupe);
}
.forge-log-label {
  margin-top: 18px;
}

.forge-steps {
  display: grid;
  gap: 8px;
}
.forge-step {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 14px;
  border: 0;
  border-left: 3px solid transparent;
  border-radius: 10px;
  background: var(--forge-panel);
  color: var(--forge-taupe);
  font-family: var(--forge-sans);
  font-size: 14px;
  font-weight: 600;
  text-align: left;
  cursor: pointer;
  transition: background 200ms ease, color 200ms ease, border-color 200ms ease, transform 200ms ease;
}
.forge-step:hover {
  transform: translateY(-1px);
  color: var(--forge-cream);
}
.forge-step-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--forge-panel-line);
  flex: none;
}
// Boolean attribute bindings render as a PRESENT empty-string attribute when
// true (HTML boolean-attribute idiom: __v === true ? '' : __v, the same
// convention <input disabled> uses) and are REMOVED entirely when false — so
// select on presence, not on a literal "true" string value.
.forge-step[data-active] {
  border-left-color: var(--forge-ember);
  background: rgba(224, 145, 90, 0.14);
  color: var(--forge-cream);
  box-shadow: 0 0 0 1px rgba(224, 145, 90, 0.25);
}
.forge-step[data-active] .forge-step-dot {
  background: var(--forge-ember);
  box-shadow: 0 0 10px 1px rgba(224, 145, 90, 0.7);
}
.forge-step[data-done] {
  color: var(--forge-cream);
}
.forge-step[data-done] .forge-step-dot {
  background: var(--forge-green);
}

.forge-log {
  display: grid;
  gap: 2px;
  max-height: 260px;
  overflow-y: auto;
  padding: 10px 12px;
  border-radius: 10px;
  background: rgba(0, 0, 0, 0.24);
}
.forge-log-line {
  margin: 0;
  padding: 3px 0;
  font-family: ui-monospace, "SF Mono", Menlo, monospace;
  font-size: 11.5px;
  line-height: 1.5;
  color: var(--forge-taupe);
  white-space: pre-wrap;
  word-break: break-word;
}

.forge-stage-region {
  position: relative;
  min-height: 0;
}
.forge-stage-empty {
  display: grid;
  place-items: center;
  height: 100%;
  min-height: 480px;
  color: var(--forge-taupe);
  font-size: 14px;
}
// A composed guest becomes a SIBLING of the empty-state inside the region root
// (see stdlib/__mcp__/env.st's same pattern) — hide the empty note once a guest
// sibling exists, so the composed forge-preview stage takes over the surface.
.forge-stage-region:has(> :not(.forge-stage-empty)) .forge-stage-empty {
  display: none;
}
