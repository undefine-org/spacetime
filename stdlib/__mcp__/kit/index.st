/// MCP interaction kit (PLAN-043 / FEAT-123).
///
/// Reusable agent-interface functions, each a normal Spacetime function the agent
/// mounts into a region of the live MCP environment via `st_mount`. The human's
/// answer returns through the ONE generic signal sink (`/__mcp/signal/{instance}`)
/// and is read by `st_await` — there is no bespoke Rust route per interaction.
///
///   picker.st  — choose ONE of N options.   → mcp-action "kit-choice"  (payload.value = option id)
///   confirm.st — a yes/no decision.          → mcp-action "kit-confirm" | "kit-cancel"
///   form.st    — collect typed input.        → mcp-action "kit-submit"  (payload.values = { field: value })
///   review.st  — approve / request changes.  → mcp-action "kit-approve" | "kit-reject"  (payload.values.comment)
///
/// form + review collect the region's named controls into `payload.values` via
/// the shared `@mcp-submit` primitive (no per-kit route, one sink) — PLAN-044.
///
/// Each kit is a SELF-CONTAINED page: it `@import "stdlib/__mcp__"`, declares its
/// data sources (`$mcpPrompt` + `$mcpOptions`/`$mcpFields`), loads them from the
/// mounted instance's `input.json` via `@mcp-input`, installs the region host
/// bridge with `@mcp-host`, and emits the human's answer with `@mcp-action`. No
/// custom page JavaScript — all behaviour composes the existing `host.st`
/// primitives.
///
/// This index is a MANIFEST, not an aggregator: importing it does NOT ship any
/// kit UI. A kit ships only when the agent mounts that ONE function, e.g.
///
///   st_mount { source: "<contents of stdlib/__mcp__/kit/picker.st>",
///              region: "stage",
///              input: { kind: "picker", prompt: "…", options: [ { id, label, detail } ] } }
///
/// then `st_await` returns the chosen `payload.value`. The kit files inherit
/// nothing from the host page (the stage region is layout-neutral), so each kit
/// carries its own warm-dark palette + type.

@import "stdlib/__mcp__";
