/// MCP interaction kit — REVIEW (PLAN-044 / FEAT-123).
///
/// A reusable Spacetime function the agent mounts into a region to ask the human
/// to APPROVE or REQUEST CHANGES on some content (a diff, a plan, a generated
/// artifact), optionally with a typed comment. The decision + comment return
/// through the ONE generic signal sink (`/__mcp/signal/{instance}`) — no bespoke
/// route, no custom page JS. It is the gated-approval kit: the highest-value
/// agent↔human turn (sign off a plan, review a change, accept/refine output).
///
/// Mount it with input.json shaped:
///   { "kind": "review",
///     "prompt": "Approve this migration plan?",
///     "body": "1. snapshot db\n2. blue/green cutover\n3. verify\n4. drop old",
///     "approveLabel": "Approve",          // optional, default "Approve"
///     "rejectLabel": "Request changes" }  // optional, default "Request changes"
///
/// On click `st_await` returns
///   { type: "mcp-action",
///     payload: { action: "kit-approve" | "kit-reject",
///                values: { comment: "<the human's typed note, may be empty>" } } }
/// so the agent reads `payload.action` as the decision and
/// `payload.values.comment` as the rationale. Both buttons use `@mcp-submit`,
/// which collects the comment textarea (the one named control) into the envelope.

@import "stdlib/__mcp__";

// Publish $mcpPrompt + $mcpReviewBody from the mounted instance's input.json.
@data inline $mcpPrompt : "";
@data inline $mcpReviewBody : "";
@mcp-input;

<main class="kit kit-review" data-st-instance>
  <p class="kit-prompt"></p>
  <pre class="kit-review-body"></pre>
  <label class="kit-field">
    <span class="kit-field-label">Comment (optional)</span>
    <textarea class="kit-field-input kit-comment" name="comment" rows="3" placeholder="Notes for the agent…"></textarea>
  </label>
  <div class="kit-actions">
    <button class="kit-btn kit-reject">Request changes</button>
    <button class="kit-btn kit-approve">Approve</button>
  </div>
</main>

// Region host bridge (nearest [data-st-instance] — the kit root).
[data-st-instance] {
  @mcp-host
}

.kit-prompt {
  text: $mcpPrompt;
}

.kit-review-body {
  text: $mcpReviewBody;
}

// Both decisions COLLECT the comment textarea (@mcp-submit gathers named
// controls) so st_await carries the decision AND the rationale. The decision is
// the ACTION name — wording cannot change the contract.
.kit-approve {
  @mcp-submit(action: "kit-approve")
}
.kit-reject {
  @mcp-submit(action: "kit-reject")
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
  --kit-mono: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  box-sizing: border-box;
  min-height: 100%;
  display: grid;
  align-content: center;
  gap: 18px;
  max-width: 620px;
  margin: 0 auto;
  padding: 48px 32px;
  color: var(--kit-cream);
  font-family: var(--kit-sans);
}
.kit * { box-sizing: border-box; }
.kit-prompt {
  margin: 0;
  font-family: var(--kit-serif);
  font-size: 27px;
  font-weight: 600;
  line-height: 1.3;
  color: var(--kit-cream);
}
.kit-review-body {
  margin: 0;
  padding: 18px 20px;
  border-radius: 14px;
  border: 1px solid var(--kit-line);
  background: rgba(28, 23, 20, 0.6);
  color: var(--kit-taupe);
  font-family: var(--kit-mono);
  font-size: 13px;
  line-height: 1.6;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  max-height: 40vh;
  overflow-y: auto;
}
.kit-review-body:empty { display: none; }
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
.kit-comment {
  resize: vertical;
  min-height: 64px;
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
.kit-reject {
  background: var(--kit-panel);
  color: var(--kit-taupe);
}
.kit-reject:hover { color: var(--kit-cream); }
.kit-approve {
  background: var(--kit-ember);
  color: #221a13;
}
.kit-approve:hover { background: var(--kit-ember-hot); }
