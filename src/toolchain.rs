//! Where the TOOLCHAIN lives, as distinct from where the user's SITE lives.
//!
//! Two roots exist in every compile and conflating them is a real bug class
//! (BUG-227): the *site dir* is the user's project, while the *workspace root*
//! is what `parser::resolve_imports` joins `stdlib/` onto to resolve
//! `@import "stdlib/..."`. Pass the site dir where the workspace root belongs
//! and every stdlib import fails — silently, because a failed route compile
//! degrades to an empty bundle plus a warning. That is how a `spacetime-host
//! push` shipped a deployment whose `/spacetime.js` was zero bytes.
//!
//! The dev server never hit it: `spacetime serve projects/x/` runs from the
//! repo root, so the CWD-relative `./stdlib` candidate happens to resolve.
//! `build` and `push` run from the user's project directory, where it does not.
//! ONE resolver, used by every caller, is the only way that stays fixed.
//!
//! Precedence mirrors `mcp::stdlib_source::candidate_stdlib_roots`, which
//! already had to solve this for the workbench overlay — same rule, one place:
//!
//!   1. `SPACETIME_STDLIB_DIR`   — explicit override (its PARENT is the root)
//!   2. the current directory    — running inside the toolchain checkout
//!   3. any ancestor of the CWD  — running from a subdirectory of it
//!   4. any ancestor of the site — a project nested in the checkout
//!
//! A caller that finds none of these is running a distributed binary with no
//! on-disk stdlib; the embedded stdlib registry serves those imports instead,
//! so `None` is a legitimate answer and not an error by itself.

use std::path::{Path, PathBuf};

/// A directory is a toolchain root when it holds the `stdlib/` tree the
/// compiler resolves `@import "stdlib/..."` against.
fn is_toolchain_root(dir: &Path) -> bool {
    // A directory named `stdlib` is not enough: `docs/stdlib/` holds only prose
    // (VENDORING.md) yet satisfied a bare `is_dir()` check, so running from
    // `docs/` picked it as THE toolchain root and produced a ZERO-BYTE bundle —
    // a silent, successful, empty build (BUG-350).
    //
    // Require a load-bearing member instead: `stdlib/primitives` is where the
    // metasystem's own definitions live, so a root without it could never have
    // served a compile anyway. This asks the question the caller actually means
    // — "can I load a stdlib from here?" — rather than "is there something
    // spelled stdlib here?".
    dir.join("stdlib").join("primitives").is_dir()
}

/// Resolve the workspace root to compile `site_dir` against.
///
/// `site_dir` is only ever used to walk UPWARD looking for an enclosing
/// checkout; it is never itself returned unless it genuinely contains a
/// `stdlib/`. Returns `None` when no on-disk stdlib exists anywhere in scope
/// (a distributed binary), leaving the embedded stdlib as the resolver.

/// The path the process was asked to compile, set once by the CLI before any
/// registry loads.
///
/// BUG-326 — the metasystem registry is a process-global `LazyLock`, so it
/// cannot take a per-file argument, and every one of its call sites therefore
/// asked `workspace_root_for(Path::new("."))` — the CWD. That made the loaded
/// stdlib a function of the shell's location: `cd / && spacetime check
/// /abs/page.st` fell back to the EMBEDDED registry and failed with a complaint
/// about an internal primitive the author never wrote.
///
/// The CLI knows the real answer before any of that runs — it was handed a
/// path. Recording it here lets the global loaders ask about the FILE instead
/// of the shell, without threading an argument through a `LazyLock`.
static COMPILE_ANCHOR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// Record what this process was asked to compile. First call wins; later calls
/// are ignored, so a single invocation has ONE anchor even when it walks many
/// files.
pub fn set_compile_anchor(path: &Path) {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };
    let _ = COMPILE_ANCHOR.set(absolute);
}

/// The toolchain root for whatever this process was asked to compile.
///
/// Used by the process-global stdlib loaders, which have no file to hand.
pub fn anchored_workspace_root() -> Option<PathBuf> {
    match COMPILE_ANCHOR.get() {
        Some(anchor) => workspace_root_for(anchor),
        None => workspace_root_for(Path::new(".")),
    }
}

pub fn workspace_root_for(site_dir: &Path) -> Option<PathBuf> {
    // NB: deliberately NOT keyed off `SPACETIME_STDLIB_DIR`. That variable is
    // the MCP workbench's `__mcp__` OVERLAY switch (`spacetime mcp
    // --stdlib-dir`), a narrower thing than "which toolchain am I": its own
    // tests point it at a temp dir with no grammar in it. Consuming it here
    // would let an unrelated overlay silently redirect every compile's grammar,
    // and — because the registry is a one-shot `LazyLock` — the first test to
    // set it would poison every later parse in the process. Two knobs, two
    // meanings; they are not the same question.

    // 1. The checkout ENCLOSING THE SITE, before any CWD-derived guess. The
    //    site's own location is a fact about the site; the CWD is a fact about
    //    the shell that happened to invoke us. Preferring the CWD would make a
    //    build's stdlib depend on where it was launched from -- which is the
    //    class of bug this function exists to end (BUG-227), and it silently
    //    compiles a site against a DIFFERENT checkout's stdlib whenever the two
    //    disagree.
    let absolute = if site_dir.is_absolute() {
        Some(site_dir.to_path_buf())
    } else {
        std::env::current_dir().ok().map(|cwd| cwd.join(site_dir))
    };
    if let Some(absolute) = absolute
        && let Some(found) = nearest_root(&absolute)
    {
        return Some(found);
    }

    // 2. The current directory and its ancestors -- the fallback for a site
    //    that lives outside any checkout while the toolchain is being run from
    //    inside one (e.g. `cargo run -- build /tmp/scratch-site`).
    if let Some(cwd) = std::env::current_dir().ok().as_deref()
        && let Some(found) = nearest_root(cwd)
    {
        return Some(found);
    }

    // 3. The checkout containing THIS BINARY -- the last resort when neither the
    //    site nor the cwd sits inside one (`cd / && spacetime check /abs/page.st`).
    //
    //    BUG-326: without this, resolution ran out of options and the load fell
    //    back to the EMBEDDED stdlib, which surfaces as a confusing complaint
    //    about an internal primitive the author never wrote (`unknown
    //    primitive: element-refs`) rather than "stdlib not found". Same file,
    //    same binary, different shell location, different answer.
    //
    //    `incremental_cache::from_full_load` already bolted this on at its own
    //    call site; `stdlib_registry` did not, so the two layers disagreed about
    //    where the stdlib was. It belongs HERE, in the one function that answers
    //    the question, so every caller gets the same answer.
    //
    //    A genuinely distributed binary has no enclosing checkout, finds
    //    nothing, and still lands on the embedded stdlib as intended.
    let exe = std::env::current_exe().ok()?;
    nearest_root(&exe)
}

/// Walk `start` and its ancestors, returning the first toolchain root.
fn nearest_root(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);
    while let Some(dir) = current {
        if is_toolchain_root(dir) {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A project nested inside a checkout resolves to the CHECKOUT, not to
    /// itself -- the exact shape `projects/<name>/` has, and the exact case
    /// that shipped an empty bundle.
    #[test]
    fn nested_site_resolves_to_the_enclosing_checkout() {
        let tmp = tempfile::tempdir().unwrap();
        let checkout = tmp.path().join("verse");
        let site = checkout.join("projects").join("mysite");
        std::fs::create_dir_all(checkout.join("stdlib").join("primitives")).unwrap();
        std::fs::create_dir_all(&site).unwrap();

        let root = nearest_root(&site).expect("must find the enclosing checkout");
        assert_eq!(root, checkout);
        assert_ne!(root, site, "the SITE is not a toolchain root");
    }

    /// A site with no enclosing stdlib yields None, so the caller falls back to
    /// the embedded stdlib rather than fabricating a bogus root.
    #[test]
    fn standalone_site_has_no_toolchain_root() {
        let tmp = tempfile::tempdir().unwrap();
        let site = tmp.path().join("standalone");
        std::fs::create_dir_all(&site).unwrap();
        assert!(nearest_root(&site).is_none());
    }

    /// A checkout that IS the site (the repo's own demos/) resolves to itself.
    #[test]
    fn a_root_containing_stdlib_resolves_to_itself() {
        let tmp = tempfile::tempdir().unwrap();
        let checkout = tmp.path().join("verse");
        std::fs::create_dir_all(checkout.join("stdlib").join("primitives")).unwrap();
        assert_eq!(nearest_root(&checkout).unwrap(), checkout);
    }
}
