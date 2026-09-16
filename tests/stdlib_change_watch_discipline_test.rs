//! STRUCTURAL GUARD (BUG-252 / FEAT-169): no hand-rolled "a value moved" watch.
//!
//! Three stdlib primitives independently hand-rolled the same subscription —
//! "run this when the author's signal changes" — and all three got it wrong the
//! same two ways:
//!
//!   1. they subscribed with a bare `ST.watch(el, …)`, which sees only ONE
//!      element's signal store, while an author's read resolves by NEAREST
//!      OWNER (this element, an ancestor scope, or the global page store). A
//!      signal declared at file scope therefore never reached them.
//!
//!   2. they primed by CALL COUNT — `if (!primed) { primed = true; return; }` —
//!      which assumes a subscription always opens with a synchronous
//!      current-value call. `ST.watch` makes that call only when THIS element's
//!      store already holds the name, so for an ancestor- or globally-owned
//!      signal the FIRST callback IS the first change, and the guard ate it.
//!
//! Together those made `@on $signal { … }`, `@on $sig.change`, `@handle`, and
//! `@effect` silently do nothing for file-scope signals — every one of them a
//! shipped, documented, gate-green feature.
//!
//! `ST.watchChanges` now expresses this need once, correctly. This test makes
//! the broken shape UNWRITABLE rather than merely discouraged: a new primitive
//! that reaches for the old pattern fails the suite with the reason and the
//! replacement, at the moment it is written.
//!
//! WHY A REPO GATE AND NOT A COMPILER CHECK: the defect is a semantic mismatch
//! between two runtime helpers, not a malformed program. The compiler cannot
//! know whether a given `ST.watch` call means "tell me the value now" (which
//! `@when` legitimately wants) or "tell me when it moves". The distinguishing
//! evidence is the priming flag sitting beside it — a source-level tell, which
//! is what this gate reads.

use std::fs;
use std::path::{Path, PathBuf};

/// Collect every `.st` file under a directory.
fn st_files(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Vendored third-party code is not ours to hold to this rule.
            if path.file_name().is_some_and(|n| n == "vendor") {
                continue;
            }
            st_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "st") {
            out.push(path);
        }
    }
}

/// A primitive that primes a subscription by CALL COUNT is, by construction,
/// trying to observe changes rather than values — which is exactly what
/// `ST.watchChanges` does correctly. The flag is the tell.
#[test]
fn no_hand_rolled_change_watch_in_stdlib() {
    let mut files = Vec::new();
    st_files(Path::new("stdlib"), &mut files);
    assert!(
        !files.is_empty(),
        "found no stdlib .st files — is the test running from the repo root?"
    );

    let mut offenders = Vec::new();

    for path in &files {
        let Ok(src) = fs::read_to_string(path) else {
            continue;
        };
        // The tell: a priming flag guarding a callback body.
        let primes_by_call_count = src.contains("if (!primed)");
        if !primes_by_call_count {
            continue;
        }
        // Paired with a bare ST.watch — the subscription that cannot see the
        // scopes the author's reads resolve through.
        let bare_watch = src.contains("ST.watch(");
        if bare_watch {
            offenders.push(path.display().to_string());
        }
    }

    assert!(
        offenders.is_empty(),
        "hand-rolled change-watch found in:\n  {}\n\n\
         This is the BUG-252 shape: a `primed` flag beside a bare `ST.watch(…)`.\n\
         It is wrong twice over — `ST.watch` sees only ONE element's store while\n\
         the author's read resolves by nearest owner (element / ancestor /\n\
         global), and call-count priming eats the FIRST change for any signal\n\
         this element does not own.\n\n\
         Use `ST.watchChanges(el, name, (next, prev) => …)` instead: it\n\
         subscribes in the scope reads resolve through, primes from the current\n\
         value, and delivers genuine changes exactly once.\n\n\
         If you truly want the CURRENT VALUE delivered immediately (what\n\
         `@when` wants — a section already active at mount must theme itself),\n\
         use `ST.watch` WITHOUT a priming flag; that is a different need and\n\
         this gate does not object to it.",
        offenders.join("\n  ")
    );
}

/// The affordance must keep existing, with the contract the primitives rely on.
/// A rename or a signature change that silently drops scope resolution would
/// re-open the whole bug class, and the primitives would still LOOK correct.
#[test]
fn watch_changes_affordance_exists_and_resolves_in_scope() {
    let src = fs::read_to_string("public/runtime/st.js").expect("read st.js");

    assert!(
        src.contains("ST.watchChanges = function"),
        "ST.watchChanges must exist — signal-arms, the change driver, and @effect all depend on it"
    );

    let body_start = src
        .find("ST.watchChanges = function")
        .expect("watchChanges present");
    let body = &src[body_start..(body_start + 1200).min(src.len())];

    assert!(
        body.contains("watchScoped"),
        "watchChanges must subscribe through watchScoped so the WATCH mirrors the RESOLVE \
         (FUP-094): reads resolve by nearest owner, so the subscription must cover the same scopes"
    );
    assert!(
        body.contains("resolve"),
        "watchChanges must prime and re-derive through ST.resolve — the same lookup the author's \
         expression uses — so priming is state-based rather than call-count-based"
    );
}
