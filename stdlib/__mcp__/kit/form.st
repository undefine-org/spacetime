/// MCP interaction kit — FORM (PLAN-043 / FEAT-123; value collection PLAN-044).
///
/// A reusable Spacetime function the agent mounts into a region to collect typed
/// input from the human. The submit returns through the ONE generic signal sink
/// (`/__mcp/signal/{instance}`) — no bespoke route, no custom page JS. It renders
/// a prompt + one labeled input per declared field + a submit button.
///
/// Mount it with input.json shaped:
///   { "kind": "form",
///     "prompt": "New deployment",
///     "fields": [ { "name": "service", "label": "Service", "type": "text",
///                   "placeholder": "api", "value": "" },
///                 { "name": "replicas", "label": "Replicas", "type": "number",
///                   "value": "3" } ] }
///
/// On submit `st_await` returns
///   { type: "mcp-action", payload: { action: "kit-submit",
///     values: { <field name>: <current value>, ... } } }
/// so the agent reads `payload.values` as the human's typed input (FUP-068,
/// PLAN-044). The submit button uses `@mcp-submit`, which gathers every named
/// control in the region into `payload.values` — no per-form route, one sink.

@import "stdlib/__mcp__";

// Publish $mcpPrompt + $mcpFields from input.json.
@data inline $mcpPrompt : "";
@data inline $mcpFields : [];
@mcp-input;

// One labeled field row. `type` / `placeholder` / `value` come from the field
// descriptor; `name` is what @mcp-submit reads to build payload.values.
@template &kit-field($field) {
  <label class="kit-field">
    <span class="kit-field-label">`$field.label`</span>
    <input class="kit-field-input" name="`$field.name`" type="`$field.type`" placeholder="`$field.placeholder`" value="`$field.value`">
  </label>
}

<main class="kit kit-form" data-st-instance>
  <p class="kit-prompt"></p>
  <div class="kit-fields"></div>
  <div class="kit-actions">
    <button class="kit-btn kit-submit">Submit</button>
  </div>
</main>

// Region host bridge (nearest [data-st-instance] — the kit root).
[data-st-instance] {
  @mcp-host
}

.kit-prompt {
  text: $mcpPrompt;
}

.kit-fields {
  @each($mcpFields as $field) {
    &kit-field($field);
  }
}

// Submit COLLECTS the field values (FUP-068): @mcp-submit gathers every named
// control in the region into payload.values = { field name: current value } and
// emits through the one sink, so st_await returns the typed input — not just
// "submitted".
.kit-submit {
  @mcp-submit(action: "kit-submit")
}

// ── Warm-dark kit styling (self-contained; the stage region is layout-neutral). ──
.kit {
  --kit-bg: #241e19;
  --kit-cream: #f0e6d6;
  --kit-taupe: #b8a98f;
  --kit-umber: #7a6b54;
  --kit-ember: #e0915a;
  --kit-ember-hot: #ed9f6a;
  --kit-panel: rgba(58, 48, 42, 0.34);
  --kit-line: rgba(122, 107, 84, 0.4);
  --kit-serif: "Fraunces", "Source Serif 4", Georgia, serif;
  --kit-sans: Inter, ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif;
  box-sizing: border-box;
  min-height: 100%;
  display: grid;
  align-content: center;
  gap: 22px;
  max-width: 540px;
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
.kit-fields {
  display: grid;
  gap: 16px;
}
.kit-field {
  display: grid;
  gap: 6px;
}
.kit-field-label {
  color: var(--kit-taupe);
  font-size: 13px;
  font-weight: 600;
}
.kit-field-input {
  padding: 12px 14px;
  border: 1px solid var(--kit-line);
  border-radius: 12px;
  background: var(--kit-panel);
  color: var(--kit-cream);
  font-family: var(--kit-sans);
  font-size: 15px;
  transition: border-color 200ms ease;
}
.kit-field-input:focus {
  outline: none;
  border-color: var(--kit-ember);
}
.kit-actions {
  display: flex;
  justify-content: flex-end;
}
.kit-btn {
  padding: 13px 26px;
  border: 0;
  border-radius: 14px;
  font-family: var(--kit-sans);
  font-size: 15px;
  font-weight: 600;
  cursor: pointer;
  transition: background 200ms ease, transform 200ms ease;
}
.kit-submit {
  background: var(--kit-ember);
  color: #221a13;
}
.kit-submit:hover { background: var(--kit-ember-hot); transform: translateY(-1px); }
