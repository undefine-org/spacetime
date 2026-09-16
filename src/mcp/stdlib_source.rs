//! Dev-mode filesystem overlay for embedded `__mcp__` stdlib sources (FUP-071).
//!
//! The MCP server's own Spacetime sources (the workbench shell `env.st`, kits)
//! are baked into the binary via `include_str!` (see `stdlib_embedded::mcp`).
//! That makes the workbench shell FROZEN at compile time: editing `env.st` on
//! disk has no effect on a running server, so the only refresh path was a
//! stop-the-world rebuild + reload — and rebuilding the server's own isolated
//! target dir relinks the live binary and crashes it (see commit fb944a4f).
//!
//! This module restores the tight edit→see-it loop for the workbench SHELL by
//! reading `__mcp__/*.st` from the FILESYSTEM when present, falling back to the
//! embedded bytes otherwise. Precedence (highest first):
//!
//! 1. `SPACETIME_STDLIB_DIR` env var (set by `spacetime mcp --stdlib-dir <dir>`):
//!    an explicit alternate `stdlib/` root → `<dir>/__mcp__/<rel>`.
//! 2. Auto-detect: `<workspace_root>/stdlib/__mcp__/<rel>` if it exists (dev:
//!    running from the repo, mirrors how `STDLIB_REGISTRY` already prefers the
//!    on-disk stdlib over embedded).
//! 3. Embedded fallback: `stdlib_embedded::mcp::FILES` (production — no on-disk
//!    stdlib, e.g. a distributed binary).
//!
//! Only the `env.st` BODY needs this overlay: `env.st`'s `@import`ed files
//! (`workbench/*.st`, `primitives/*.st`) and kit sources are already read from
//! disk per compile by `parser::resolve_imports` / the agent passing file
//! contents, so they are live without any change.

use std::borrow::Cow;
use std::path::Path;

/// Environment variable that overrides the stdlib root used for the `__mcp__`
/// filesystem overlay. Set by the `spacetime mcp --stdlib-dir <dir>` flag.
pub const STDLIB_DIR_ENV: &str = "SPACETIME_STDLIB_DIR";

/// Resolve the source text for an `__mcp__` stdlib file given a path relative to
/// the `__mcp__` module root (e.g. `"env.st"`, `"kit/confirm.st"`).
///
/// Returns disk contents (owned) when a filesystem copy is found via the
/// override or auto-detect rules, else the embedded bytes (borrowed `'static`).
/// Returns `None` only if `rel` matches neither a readable on-disk file nor an
/// embedded entry.
pub fn resolve_mcp_stdlib_source(workspace_root: &Path, rel: &str) -> Option<Cow<'static, str>> {
    // 1 + 2: filesystem candidates in precedence order.
    for root in candidate_stdlib_roots(workspace_root) {
        let path = root.join("__mcp__").join(rel);
        if let Ok(contents) = std::fs::read_to_string(&path) {
            log::debug!(
                "[FUP-071] __mcp__ overlay: read {} from disk",
                path.display()
            );
            return Some(Cow::Owned(contents));
        }
    }

    // 3: embedded fallback.
    embedded_mcp_source(rel).map(Cow::Borrowed)
}

/// Candidate `stdlib/` roots to probe for the overlay, highest precedence first.
fn candidate_stdlib_roots(workspace_root: &Path) -> Vec<std::path::PathBuf> {
    let mut roots = Vec::new();
    if let Some(dir) = std::env::var_os(STDLIB_DIR_ENV) {
        let dir = std::path::PathBuf::from(dir);
        if !dir.as_os_str().is_empty() {
            roots.push(dir);
        }
    }
    roots.push(workspace_root.join("stdlib"));
    roots
}

/// Look up an `__mcp__` file's embedded bytes by its module-relative path.
fn embedded_mcp_source(rel: &str) -> Option<&'static str> {
    crate::stdlib_embedded::mcp::FILES
        .iter()
        .find(|f| f.path == rel)
        .map(|f| f.content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard};

    // `SPACETIME_STDLIB_DIR` is process-global; serialize env-touching tests.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn lock_env() -> MutexGuard<'static, ()> {
        ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn falls_back_to_embedded_when_no_disk_copy() {
        let _g = lock_env();
        unsafe { std::env::remove_var(STDLIB_DIR_ENV) };
        // A workspace with no on-disk stdlib → embedded env.st.
        let tmp = std::env::temp_dir().join("fup071-empty-ws");
        let src = resolve_mcp_stdlib_source(&tmp, "env.st")
            .expect("env.st must resolve from embedded fallback");
        assert!(
            src.contains("data-st-region"),
            "embedded env.st should contain the stage region marker"
        );
        // Unknown file → None from both disk and embedded.
        assert!(resolve_mcp_stdlib_source(&tmp, "does-not-exist.st").is_none());
    }

    #[test]
    fn prefers_disk_over_embedded_via_workspace_autodetect() {
        let _g = lock_env();
        unsafe { std::env::remove_var(STDLIB_DIR_ENV) };
        let tmp = std::env::temp_dir().join(format!("fup071-ws-{}", std::process::id()));
        let mcp_dir = tmp.join("stdlib").join("__mcp__");
        std::fs::create_dir_all(&mcp_dir).unwrap();
        std::fs::write(mcp_dir.join("env.st"), "DISK_OVERLAY_MARKER").unwrap();

        let src = resolve_mcp_stdlib_source(&tmp, "env.st").unwrap();
        assert_eq!(src.as_ref(), "DISK_OVERLAY_MARKER");

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn env_override_takes_precedence_over_workspace() {
        let _g = lock_env();
        let ws = std::env::temp_dir().join(format!("fup071-ws2-{}", std::process::id()));
        let ws_mcp = ws.join("stdlib").join("__mcp__");
        std::fs::create_dir_all(&ws_mcp).unwrap();
        std::fs::write(ws_mcp.join("env.st"), "WORKSPACE_COPY").unwrap();

        let override_root = std::env::temp_dir().join(format!("fup071-ovr-{}", std::process::id()));
        let ovr_mcp = override_root.join("__mcp__");
        std::fs::create_dir_all(&ovr_mcp).unwrap();
        std::fs::write(ovr_mcp.join("env.st"), "OVERRIDE_COPY").unwrap();

        unsafe { std::env::set_var(STDLIB_DIR_ENV, &override_root) };
        let src = resolve_mcp_stdlib_source(&ws, "env.st").unwrap();
        unsafe { std::env::remove_var(STDLIB_DIR_ENV) };

        assert_eq!(src.as_ref(), "OVERRIDE_COPY");

        std::fs::remove_dir_all(&ws).ok();
        std::fs::remove_dir_all(&override_root).ok();
    }
}
