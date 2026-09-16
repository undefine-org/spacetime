/// MCP interaction kit — PICKER (PLAN-043 / FEAT-123).
///
/// A reusable Spacetime function the agent mounts into a region of the live MCP
/// environment to ask the human to choose ONE of N options. The choice returns
/// through the ONE generic signal sink (`/__mcp/signal/{instance}`) — no bespoke
/// Rust route, no custom page JS. It composes only existing MCP primitives:
///   - `@mcp-host`   installs the region's host bridge (nearest [data-st-instance]).
///   - `@mcp-input`  loads the mounted instance's input.json and publishes the
///                   kit sources `$mcpPrompt` (scalar) + `$mcpOptions` (array).
///   - `@each`       renders one option button per row.
///   - `@mcp-action` emits the human's pick as a generic `mcp-action` envelope.
///
/// Mount it with input.json shaped:
///   { "kind": "picker",
///     "prompt": "Which migration strategy?",
///     "options": [ { "id": "a", "label": "In-place", "detail": "fast, risky" },
///                  { "id": "b", "label": "Blue/green", "detail": "safe, slower" } ] }
///
/// On click the page POSTs one envelope; `st_await` returns
///   { type: "mcp-action", payload: { action: "kit-choice", value: "<option id>", ... } }
/// so the agent reads `payload.value` as the chosen option id.

@import "stdlib/__mcp__";

// Publish $mcpPrompt + $mcpOptions from the mounted instance's input.json.
@data inline $mcpPrompt : "";
@data inline $mcpOptions : [];
@mcp-input;

// One option row. Carries its id in `data-choice-id` so the action directive can
// read the picked id off the clicked element (per-option dynamic value).
@template &kit-option($opt) {
  <button class="kit-option" data-choice-id="`$opt.id`">
    <span class="kit-option-label">`$opt.label`</span>
    <span class="kit-option-detail">`$opt.detail`</span>
  </button>
}

<main class="kit kit-picker" data-st-instance>
  <p class="kit-prompt"></p>
  <div class="kit-options"></div>
</main>

// Region host bridge (resolves the nearest [data-st-instance] — the kit root).
[data-st-instance] {
  @mcp-host
}

// Prompt text + option list, rendered from the published kit sources.
.kit-prompt {
  text: $mcpPrompt;
}

.kit-options {
  @each($mcpOptions as $opt) {
    &kit-option($opt);
  }
}

// Each option emits the human's pick through the one generic sink. `valueAttr`
// reads the clicked button's `data-choice-id`, so the envelope carries the
// chosen option's id as `payload.value` (action "kit-choice").
.kit-option {
  @mcp-action(action: "kit-choice", valueAttr: "data-choice-id")
}

// ── Warm-dark kit styling (self-contained: the stage region is layout-neutral,
//    so the kit owns its own palette + type; it inherits nothing from the host). ──
.kit {
  --kit-bg: #241e19;
  --kit-cream: #f0e6d6;
  --kit-taupe: #b8a98f;
  --kit-umber: #7a6b54;
  --kit-ember: #e0915a;
  --kit-ember-hot: #ed9f6a;
  --kit-panel: rgba(58, 48, 42, 0.34);
  --kit-panel-hover: rgba(224, 145, 90, 0.14);
  --kit-serif: "Fraunces", "Source Serif 4", Georgia, serif;
  --kit-sans: Inter, ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif;
  box-sizing: border-box;
  min-height: 100%;
  display: grid;
  align-content: center;
  justify-items: stretch;
  gap: 22px;
  max-width: 560px;
  margin: 0 auto;
  padding: 56px 32px;
  color: var(--kit-cream);
  font-family: var(--kit-sans);
}
.kit * { box-sizing: border-box; }
.kit-prompt {
  margin: 0;
  font-family: var(--kit-serif);
  font-size: 28px;
  font-weight: 600;
  line-height: 1.3;
  color: var(--kit-cream);
}
.kit-options {
  display: grid;
  gap: 12px;
}
.kit-option {
  display: grid;
  gap: 4px;
  text-align: left;
  padding: 16px 20px;
  border: 0;
  border-radius: 16px;
  background: var(--kit-panel);
  color: var(--kit-cream);
  cursor: pointer;
  transition: background 200ms ease, transform 200ms ease;
}
.kit-option:hover {
  background: var(--kit-panel-hover);
  transform: translateY(-1px);
}
.kit-option-label {
  font-family: var(--kit-serif);
  font-size: 17px;
  font-weight: 600;
}
.kit-option-detail {
  color: var(--kit-taupe);
  font-size: 13px;
}
.kit-option-detail:empty { display: none; }
