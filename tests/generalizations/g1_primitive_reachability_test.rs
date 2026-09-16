//! G1 — A DECLARED CAPABILITY THAT NOTHING CAN REACH IS DEAD, AND DEAD CODE
//! MISEXPANDS.
//!
//! # Why this exists
//!
//! `element-ref` (`stdlib/primitives/dom.st:42`) is declared, bound by nothing an
//! author can write, and lives in a file `stdlib/index.st` does not even import.
//! It is also the primitive BUG-328 mis-expanded: a matcher bug read
//! `&$bounds.rect.width` as the CSS selector `.rect.width`, expanded `element-ref`
//! against that literal string, and emitted a second `const el` into a scope that
//! already had one — a bundle that does not parse, i.e. every page importing
//! stdlib was dead.
//!
//! So an unreachable declaration is not inert. It stays MATCHABLE, and the cost of
//! a matcher bug is proportional to how much unreachable surface it can land on.
//!
//! This is the third instance of one shape this arc:
//!   - E0964: every `%form` capture must have a consumer
//!   - the `analyze_*` test: every analysis must have a production caller
//!   - this: every `%primitive` must have a way to be reached
//! Three instances is a rule, not a coincidence.
//!
//! # Why this is a REPORT and not yet a failing lint
//!
//! Reachability has several mechanisms and a naive scan gets it badly wrong. A
//! primitive can be reached by:
//!   - a `%binds` block in a `%macro`            (the common case)
//!   - `%uses` (cross-primitive prelude deps)
//!   - a filter pipe by NAME (`| uppercase`) — never a call
//!   - the compiler itself, by name, from Rust
//!   - another primitive's `%emit` body
//!
//! A first pass that only understood `%binds`/`%uses` flagged 74 of 205
//! primitives, including `uppercase`, `each` and `test` — all obviously live. A
//! lint that cries wolf on a third of stdlib would be turned off in a week, and
//! turning it into a hard failure on that basis would be the same mistake as a
//! green suite that proves nothing: a signal nobody can trust.
//!
//! So this test ESTABLISHES THE MEASUREMENT and pins the honest baseline. It
//! fails only when the unreachable set GROWS, which is the property actually
//! worth defending while the classifier is still being taught the mechanisms.
//! Tightening it to zero is tracked in PLAN-140 G1.

use std::collections::HashSet;
use std::path::Path;

/// Every `%primitive <name>` declared under `stdlib/`.
fn declared_primitives(root: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    walk(&root.join("stdlib"), &mut |path, text| {
        for line in text.lines() {
            let t = line.trim_start();
            if let Some(rest) = t.strip_prefix("%primitive ")
                && let Some(name) = rest
                    .split(|c: char| c == '(' || c.is_whitespace())
                    .next()
                    .filter(|n| !n.is_empty())
            {
                out.push((
                    name.to_string(),
                    path.strip_prefix(root).unwrap_or(path).display().to_string(),
                ));
            }
        }
    });
    out.sort();
    out.dedup();
    out
}

/// Every name reachable by any mechanism we can currently see.
fn reachable_names(root: &Path) -> HashSet<String> {
    let mut refs: HashSet<String> = HashSet::new();

    walk(&root.join("stdlib"), &mut |path, text| {
        let is_decl_line = |l: &str| l.trim_start().starts_with("%primitive ");

        for line in text.lines() {
            if is_decl_line(line) {
                continue;
            }
            // `name(` — a bind/call in %binds, %emit, or another primitive body.
            let bytes = line.as_bytes();
            let mut i = 0;
            while i < bytes.len() {
                if bytes[i] == b'(' {
                    let end = i;
                    let mut s = i;
                    while s > 0 && {
                        let c = bytes[s - 1];
                        c.is_ascii_alphanumeric() || c == b'-' || c == b'_'
                    } {
                        s -= 1;
                    }
                    if s < end {
                        refs.insert(line[s..end].to_string());
                    }
                }
                // `| name` — a filter pipe reaches a primitive by NAME, not a call.
                if bytes[i] == b'|' && bytes.get(i + 1) != Some(&b'|') {
                    let mut s = i + 1;
                    while s < bytes.len() && bytes[s] == b' ' {
                        s += 1;
                    }
                    let mut e = s;
                    while e < bytes.len() && {
                        let c = bytes[e];
                        c.is_ascii_alphanumeric() || c == b'-'
                    } {
                        e += 1;
                    }
                    if e > s {
                        refs.insert(line[s..e].to_string());
                    }
                }
                i += 1;
            }
            // `primitive: <name>` inside a `%registers driver(...)` block.
            //
            // A driver macro names its implementation as registry DATA rather
            // than calling it:
            //
            //     %registers driver(visible) {
            //       primitive: intersection
            //       ...
            //     }
            //
            // That is a real reachability edge — `@on &.visible(...)` projects
            // through it — and it looks nothing like a call, so a
            // call-shaped scan cannot see it. This is the kinds-as-data design
            // working exactly as intended, which is precisely why a reachability
            // classifier has to understand data as well as calls.
            if let Some(rest) = line.trim_start().strip_prefix("primitive:") {
                let n = rest.trim().trim_end_matches(',').trim();
                if !n.is_empty() {
                    refs.insert(n.to_string());
                }
            }

            // `%uses a, b` — ANYWHERE on the line, not just at its start.
            //
            // A module primitive is reached by `%uses`, and the idiomatic
            // spelling puts it on the signature's closing line:
            //
            //     ) %uses scroll-registry {
            //
            // Matching only a line-leading `%uses` missed every one of those,
            // which made five live prelude registries (element-refs,
            // scroll-registry, data-store, data-channel, entity-world) look
            // unreachable. A classifier that cannot see a real mechanism does
            // not report dead code — it reports its own blind spot, and acting
            // on that would have deleted working infrastructure.
            if let Some(at) = line.find("%uses ") {
                let rest = &line[at + "%uses ".len()..];
                // Stop at the body brace: `%uses a, b {` names a and b only.
                let rest = rest.split('{').next().unwrap_or(rest);
                for n in rest.split(',') {
                    let n = n.trim().trim_end_matches(';').trim();
                    if !n.is_empty() {
                        refs.insert(n.to_string());
                    }
                }
            }
            let _ = path;
        }
    });

    // The compiler reaches some primitives by name from Rust.
    walk(&root.join("src"), &mut |_path, text| {
        for line in text.lines() {
            let mut rest = line;
            while let Some(q) = rest.find('"') {
                rest = &rest[q + 1..];
                if let Some(e) = rest.find('"') {
                    let lit = &rest[..e];
                    if !lit.is_empty()
                        && lit.len() < 40
                        && lit
                            .chars()
                            .all(|c| c.is_ascii_lowercase() || c == '-' || c.is_ascii_digit())
                    {
                        refs.insert(lit.to_string());
                    }
                    rest = &rest[e + 1..];
                } else {
                    break;
                }
            }
        }
    });

    refs
}

fn walk(dir: &Path, f: &mut impl FnMut(&Path, &str)) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n == "vendor" || n == "target") {
                continue;
            }
            walk(&p, f);
        } else if p.extension().is_some_and(|x| x == "st" || x == "rs")
            && let Ok(t) = std::fs::read_to_string(&p)
        {
            f(&p, &t);
        }
    }
}

/// The unreachable set must not GROW.
///
/// A new `%primitive` that nothing can reach is a new patch of surface for the
/// next matcher bug to land on — which is exactly what `element-ref` was for
/// BUG-328. Adding one should be a deliberate act with a reason, not something
/// that happens quietly.
#[test]
fn the_unreachable_primitive_set_does_not_grow() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let declared = declared_primitives(root);
    let reachable = reachable_names(root);

    let unreachable: Vec<&(String, String)> = declared
        .iter()
        .filter(|(n, _)| !reachable.contains(n))
        .collect();

    // The honest baseline, measured against the classifier above.
    //
    // History, because the NUMBER is less interesting than what moved it:
    //   24 -> 19  taught it `%uses` mid-line (`) %uses scroll-registry {`), which
    //             is the idiomatic spelling. Five live prelude registries had
    //             looked dead purely because of where the token sat on the line.
    //   19 -> 14  taught it `primitive: <name>` inside `%registers driver(...)`.
    //             A driver macro names its implementation as registry DATA, not
    //             as a call — kinds-as-data working as designed, and invisible to
    //             a call-shaped scan.
    //
    // Both drops were the classifier learning a real mechanism, NOT code being
    // deleted. That distinction is the whole reason this is baselined rather
    // than asserted at zero: a lint that cannot see a mechanism does not report
    // dead code, it reports its own blind spot — and acting on that would have
    // deleted working infrastructure.
    //
    // Lowering it is progress; raising it needs a reason in the commit message.
    const BASELINE: usize = 14;

    assert!(
        unreachable.len() <= BASELINE,
        "the unreachable-primitive set GREW to {} (baseline {BASELINE}).\n\
         A primitive nothing can reach still MATCHES, and a matcher bug turns it \
         into emitted code — that is precisely how `element-ref`, reachable by \
         nothing, became BUG-328's duplicate `const el` and killed every page \
         importing stdlib.\n\
         Either give it a `%binds`/`%uses`/filter-pipe consumer, delete it, or \
         raise the baseline WITH a reason.\n\
         Current set:\n{}",
        unreachable.len(),
        unreachable
            .iter()
            .map(|(n, f)| format!("  {n:24} {f}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// `element-ref` is the canary. It is the primitive BUG-328 mis-expanded, and it
/// must not regain a phantom consumer without someone noticing.
#[test]
fn element_ref_is_still_declared_where_bug_328_found_it() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let declared = declared_primitives(root);
    let found = declared.iter().find(|(n, _)| n == "element-ref");
    assert!(
        found.is_some(),
        "`element-ref` disappeared. If it was deleted, that is good — remove this \
         test with it. If it moved, update PLAN-140 G1, which cites it as the \
         worked example of why unreachable declarations are not inert."
    );
}
