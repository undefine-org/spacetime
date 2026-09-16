/// Spacetime MCP prelude.
///
/// `@mcp-host` is selector-scoped: it resolves the NEAREST ancestor
/// `[data-st-instance]` region root and attaches one `__stHost` bridge there
/// (FEAT-127 region isolation — no `window.__stHost` global).
/// `@mcp-input` loads the mounted instance's `input.json` into `$mcpInput`;
/// `@mcp-input(refresh: 1000)` additionally polls for refreshed snapshots.
/// Selector-scoped `@mcp-click(...)` emits typed events through the
/// nearest-ancestor bridge;
/// selector-scoped `@mcp-action(...)` emits semantic workbench action requests:
///
/// ```st
/// @import "stdlib/__mcp__";
///
/// <div data-st-instance="inst-1">
///   <button class="approve">Approve</button>
/// </div>
///
/// [data-st-instance] {
///   @mcp-host
/// }
///
/// .approve {
///   @mcp-click(type: "approved", correlation: "review-1", value: "approve")
/// }
///
/// .open-env {
///   @mcp-action(action: "st_env_open", targetAttr: "data-env-path")
/// }
/// ```
///
/// The one-page environment browser lives at `env.st`; its documented pieces live
/// under `workbench/` so data, card templates, list bindings, actions, and styles
/// can evolve independently.
///
/// The interaction KIT (`kit/picker.st`, `kit/confirm.st`, `kit/form.st`) are
/// normal Spacetime functions the agent mounts into a region via `st_mount`; each
/// asks the human something and returns the answer through the SAME generic
/// `/__mcp/signal/{instance}` sink (read by `st_await`) — no bespoke route per
/// interaction. See `kit/index.st` for the manifest + mount shape.
///
/// The mounted instance page tags its body root with `data-st-instance` +
/// `data-st-origin` (region-root identity). All JavaScript lives inside
/// `primitives/host.st` `%primitive` blocks; author pages stay declarative
/// Spacetime.

@import "./primitives/host.st";
@import "./primitives/bundle.st";
