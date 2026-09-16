/// MCP interaction kit — CONFIRM (PLAN-043 / FEAT-123).
///
/// A reusable Spacetime function the agent mounts into a region to ask the human
/// a yes/no decision. The answer returns through the ONE generic signal sink
/// (`/__mcp/signal/{instance}`) — no bespoke route, no custom page JS. It is the
/// smallest kit: a prompt + two buttons, each emitting a fixed `mcp-action`.
///
/// Mount it with input.json shaped:
///   { "kind": "confirm",
///     "prompt": "Delete the production database?",
///     "confirmLabel": "Delete",      // optional, default "Confirm"
///     "cancelLabel": "Keep it" }     // optional, default "Cancel"
///
/// On click `st_await` returns
///   { type: "mcp-action", payload: { action: "kit-confirm" | "kit-cancel", ... } }
/// so the agent reads `payload.action` as the decision. (Labels render from the
/// input; the decision is carried by the ACTION name, not the label, so wording
/// never changes the contract.)

@import "stdlib/__mcp__";

// Publish $mcpPrompt (+ optional labels carried on $mcpInput) from input.json.
@data inline $mcpPrompt : "";
@mcp-input;

<main class="kit kit-confirm" data-st-instance>
  <p class="kit-prompt"></p>
  <div class="kit-actions">
    <button class="kit-btn kit-btn-cancel">Cancel</button>
    <button class="kit-btn kit-btn-confirm">Confirm</button>
  </div>
</main>

// Region host bridge (nearest [data-st-instance] — the kit root).
[data-st-instance] {
  @mcp-host
}

.kit-prompt {
  text: $mcpPrompt;
}

// The decision is the ACTION name — wording cannot change the contract.
.kit-btn-confirm {
  @mcp-action(action: "kit-confirm", value: "confirm")
}
.kit-btn-cancel {
  @mcp-action(action: "kit-cancel", value: "cancel")
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
  --kit-serif: "Fraunces", "Source Serif 4", Georgia, serif;
  --kit-sans: Inter, ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif;
  box-sizing: border-box;
  min-height: 100%;
  display: grid;
  align-content: center;
  gap: 28px;
  max-width: 520px;
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
.kit-actions {
  display: flex;
  gap: 12px;
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
  transition: background 200ms ease, transform 200ms ease, color 200ms ease;
}
.kit-btn:hover { transform: translateY(-1px); }
.kit-btn-cancel {
  background: var(--kit-panel);
  color: var(--kit-taupe);
}
.kit-btn-cancel:hover { color: var(--kit-cream); }
.kit-btn-confirm {
  background: var(--kit-ember);
  color: #221a13;
}
.kit-btn-confirm:hover { background: var(--kit-ember-hot); }
