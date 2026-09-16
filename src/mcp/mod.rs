//! Spacetime MCP server (PLAN-037).
//!
//! `spacetime mcp` exposes Spacetime as a Model Context Protocol server over
//! stdio, designed to be registered in Spell's `spell.kdl`:
//!
//! ```kdl
//! mcp {
//!     server "spacetime" type=stdio {
//!         command "cargo"
//!         args "run" "--quiet" "--" "mcp"
//!     }
//! }
//! ```
//!
//! The mental model is two nested, uniformly-manipulable concepts:
//!
//! ```text
//! Environment  ≡  an import path (a workspace directory)
//!    └─ Tab    ≡  an entry point (a file on disk, OR inline .st code)
//!
//! Function     ≡  live .st source + declared/introspected contract + revision
//!    └─ Instance ≡ mounted Function + input JSON + URL + event stream
//! ```
//!
//! Environments/tabs can be discovered, spun up, or attached to. Functions can be
//! put, mounted, awaited, and inspected. The generic function path is the zen
//! vertical slice toward PLAN-038's self-describing MCP: one registry, one mount,
//! one signal sink, one event stream. Later waves replace declared contracts with
//! compiler-introspected @data signal/@handle contracts and add CDP (W3).

mod actions;
// `pub(crate)` so the shared Structure IR producer (`crate::introspect`, PLAN-064
// B2) can consume the bundle/template types — one introspection substrate, not a
// copy per host.
pub(crate) mod bundle;
mod live;
mod origin;
mod protocol;
mod resources;
pub(crate) mod state;
pub mod stdlib_source;
mod tools;

/// CSS `@scope`-wrapping + lint utilities for region-mounted bundles (PLAN-041).
/// Pure helpers — FEAT-126's `registerBundle` wires them in at registration time.
pub mod css_scope;

/// Entry point for the `spacetime mcp` subcommand. Blocks serving stdio until
/// the client closes stdin.
pub fn run(workspace_root: std::path::PathBuf) -> std::io::Result<()> {
    protocol::serve(workspace_root)
}

/// Entry point for the standalone `spacetime workbench` subcommand (FUP-110).
/// Boots ONLY the HTTP live surface (no MCP stdio transport, no harness
/// process) bound to a FIXED port, prints the stable workbench URL, and blocks
/// forever (Ctrl+C to stop) — a self-contained server an agent or a human can
/// drive with a real browser/puppeteer, independent of any MCP client.
pub async fn run_http_only(workspace_root: std::path::PathBuf, port: u16) -> std::io::Result<u16> {
    let workbench = state::WorkbenchSession::new(workspace_root);
    let addr = format!("127.0.0.1:{port}");
    let server = live::LiveServer::start_on(workbench, &addr)?;
    Ok(server.port)
}
