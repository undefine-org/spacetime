//! BUG-371 — `inspect --layer expansion` must report the macro the compiler
//! actually expanded, identically on every run.
//!
//! The directive `on` is shared by several `%macro` siblings (`on-driver-body`,
//! `on-driver-form-as-invalid`, `on-driver-form-legacy-space`, …). Parse picks
//! between them using literal words the capture scorer cannot see, and records
//! the winner in `FormMatch::matched_macro`.
//!
//! `inspect` used to discard that and re-resolve from `macro_name` alone. The
//! fallback iterates a `HashMap`, whose order is randomised per process, so the
//! reported primitive flipped across identical runs of ONE file — observed as
//! `drive`, `drive-body`, `drive-arms`, `drive-legacy-space`, and the ERROR
//! sibling `drive-as-error`, which exists to REFUSE invalid syntax.
//!
//! Compilation was never affected (it passes `matched_macro` as `preferred`),
//! which is exactly why this survived: `check` passed, `emit` was byte-stable,
//! and only the introspection layer lied. A tool built on that layer would
//! reject a valid page roughly half the time.
//!
//! These tests assert the property directly — same input, same answer, many
//! times — rather than pinning one primitive name, so they keep their meaning
//! if the stdlib's `on` forms are renamed.

use std::collections::HashSet;
use std::path::Path;

/// Mirror of `inspect`'s own registry construction. An EMPTY `MetaRegistry::new()`
/// knows no macros at all, so every lookup returns `None` and every assertion
/// below would hold vacuously — the first draft of this file did exactly that and
/// passed with the fix reverted. Load the stdlib the way the CLI does.
fn stdlib_registry() -> spacetime::metasystem::MetaRegistry {
    let mut registry = spacetime::metasystem::MetaRegistry::new();
    let _ = registry.load_stdlib_from_dir(Path::new("stdlib/runtime"));
    let _ = registry.load_stdlib_from_dir(Path::new("stdlib/macros"));
    let _ = registry.load_stdlib_from_dir(Path::new("stdlib/primitives"));
    // …and every other stdlib directory (BUG-372). Mirrors
    // `cli::inspect::create_registry_with_stdlib`; sorted for determinism.
    if let Ok(entries) = std::fs::read_dir("stdlib") {
        let mut dirs: Vec<_> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .filter(|p| {
                !matches!(
                    p.file_name().and_then(|n| n.to_str()),
                    Some("runtime") | Some("macros") | Some("primitives")
                )
            })
            .collect();
        dirs.sort();
        for dir in dirs {
            let _ = registry.load_stdlib_from_dir(&dir);
        }
    }
    assert!(
        !registry.get_all_macros_by_form_directive("on").is_empty(),
        "stdlib must load: no `on` macros registered, so these tests would be vacuous"
    );
    registry
}

/// A page whose `@on` bind is valid and unambiguous to the parser.
const PAGE: &str = r#"
@import "stdlib/macros/on"
@import "stdlib/macros/data-kind"
@import "stdlib/macros/host"
@host $chat : live("A.B")
@data inline $draft : "";
@data signal $send() to $chat { send emit "send" receive to R { _ => Failed ($.reply); } policy latest }
.a { @on &.click { $send ($draft,); } }
"#;

/// Parse is the source of truth: it must select the same `%macro` every time.
#[test]
fn parse_selects_the_same_macro_every_run() {
    let seen: HashSet<String> = (0..24)
        .map(|_| {
            let ast = spacetime::parser::parse(PAGE).expect("page parses");
            ast.scopes
                .iter()
                .flat_map(|s| s.matches.iter())
                .filter(|fm| fm.macro_name == "on")
                .map(|fm| format!("{:?}", fm.matched_macro))
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect();

    assert_eq!(
        seen.len(),
        1,
        "parse must be deterministic; saw several outcomes: {seen:?}"
    );
    let only = seen.into_iter().next().unwrap();
    assert!(
        only.contains("on-driver-body"),
        "the body form `@on &.click {{ … }}` must select the body macro, got {only}"
    );
}

/// The registry accessor `inspect` uses must agree with parse, and must never
/// fall through to the score-and-tie path when parse already decided.
#[test]
fn registry_honours_the_macro_parse_selected() {
    let reg = stdlib_registry();

    // With a `preferred` name, resolution is exactly that macro — no scoring.
    for name in ["on-driver-body", "on-driver-form-legacy-space"] {
        if reg.get_macro(name).is_some() {
            let got = reg
                .get_macro_for_match(Some(name), "on")
                .expect("preferred macro resolves");
            assert_eq!(
                got.name, name,
                "get_macro_for_match must return the preferred macro verbatim"
            );
        }
    }

    // Repeated resolution of the SAME inputs is stable.
    let answers: HashSet<String> = (0..24)
        .map(|_| {
            stdlib_registry()
                .get_macro_for_match(Some("on-driver-body"), "on")
                .map(|m| m.name.clone())
                .unwrap_or_default()
        })
        .collect();
    assert_eq!(answers.len(), 1, "resolution must be stable: {answers:?}");
}

/// A valid page must never be reported as expanding through a `%diagnostic`
/// error sibling. That is the user-visible shape of the bug: the introspection
/// layer accusing correct code of being invalid.
#[test]
fn valid_page_never_reports_an_error_primitive() {
    // A FRESH registry per iteration is essential. `HashMap` randomises its
    // iteration order per instance, so reusing one registry samples ONE order
    // twenty-four times and cannot see the bug.
    let mut seen: HashSet<String> = HashSet::new();

    for _ in 0..24 {
        let reg = stdlib_registry();
        let ast = spacetime::parser::parse(PAGE).expect("page parses");
        for fm in ast.scopes.iter().flat_map(|s| s.matches.iter()) {
            let Some(def) = reg.get_macro_for_match(fm.matched_macro.as_deref(), &fm.macro_name)
            else {
                continue;
            };
            seen.insert(def.name.clone());
            for binding in &def.binds {
                assert!(
                    !binding.primitive.contains("as-error"),
                    "valid `{}` reported as expanding to the error sibling `{}` \
                     (resolved macro `{}`)",
                    fm.macro_name,
                    binding.primitive,
                    def.name
                );
            }
        }
    }

    // And the resolution itself must be single-valued across those fresh
    // registries — one macro per directive in this page, never a set.
    let on_macros: Vec<_> = seen.iter().filter(|n| n.starts_with("on-")).collect();
    assert_eq!(
        on_macros.len(),
        1,
        "`@on` must resolve to exactly one macro across fresh registries, saw {on_macros:?}"
    );
}

/// BUG-372 — `inspect` must see every stdlib macro, not the three directories
/// someone happened to list.
///
/// `create_registry_with_stdlib` loaded `runtime`, `macros`, `primitives`.
/// Twenty stdlib directories define `%macro`s, so `@view`/`@match` (in
/// `stdlib/enum`), and everything in `mobile`, `3d`, `text`, `dnd`, … resolved
/// to nothing. `inspect --layer expansion` printed `"expands_to": null`, which
/// is indistinguishable from "this macro binds no primitives" — a silent
/// omission, not an error, on a page the compiler builds without complaint.
#[test]
fn every_stdlib_directory_that_defines_macros_is_visible() {
    let reg = stdlib_registry();

    // A macro from OUTSIDE the three originally-loaded directories.
    let view = reg.get_macro("view").expect(
        "`@view` (stdlib/enum/dispatch.st) must be registered: inspect's registry \
         has to cover every stdlib directory, not a hardcoded three",
    );
    assert!(
        view.binds.iter().any(|b| b.primitive == "dispatch-mount"),
        "`@view` must bind dispatch-mount, got {:?}",
        view.binds.iter().map(|b| &b.primitive).collect::<Vec<_>>()
    );

    assert!(
        reg.get_macro("match").is_some(),
        "`@match` (stdlib/enum/dispatch.st) must be registered"
    );
}

/// The registry must be built the same way twice: it decides which macro wins a
/// name collision, and `read_dir` yields entries in filesystem order.
#[test]
fn registry_construction_is_order_stable() {
    let names = |r: &spacetime::metasystem::MetaRegistry| {
        let mut v: Vec<String> = r
            .get_all_macros_by_form_directive("on")
            .iter()
            .map(|m| m.name.clone())
            .collect();
        v.sort();
        v
    };
    let first = names(&stdlib_registry());
    for _ in 0..8 {
        assert_eq!(
            first,
            names(&stdlib_registry()),
            "stdlib loading must be deterministic across constructions"
        );
    }
}
