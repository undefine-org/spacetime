/// Spacetime MCP module
///
/// Import with:
///   @import "stdlib/__mcp__"
///
/// The module is the Spacetime-side prelude for the MCP Function Environment:
/// - host bridge primitive/macros (`@mcp-host`) for page -> MCP events
/// - kit functions (picker/form/confirm) as they migrate out of bespoke Rust
/// - env browser workbench (`env.st` + `workbench/*.st`) for one-page environment/tab inspection
///
/// FEAT-122 ships the minimal host bridge shape; FEAT-123 grows the full module
/// layout without changing the generic `/__mcp/signal/{instance}` transport.

@import "./index.st";
