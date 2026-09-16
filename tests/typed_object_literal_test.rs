//! A typed declaration accepts an object literal value. (BUG-342)
//!
//! # Why this file exists
//!
//! ```st
//! $u object: { name: "Ada" };     // E0946 — grammar mismatch
//! ```
//!
//! …while every neighbouring shape compiles:
//!
//! ```st
//! $u: { name: "Ada" };            // OK  (no type)
//! $u object: [1, 2];              // OK  (type + array)
//! $u object: 3;                   // OK  (type + scalar)
//! $l string[]: ["a"];             // OK  (typed array)
//! ```
//!
//! The type NAME is irrelevant (`$u Foo: { … }` fails identically), so this was
//! never a registry or keyword lookup. Adding a type to a declaration made a
//! value shape that already worked stop working — the language stopped being
//! learnable by pattern, which is the standing rule this violates.
//!
//! ## Root cause
//!
//! `{` is ambiguous in Spacetime: it opens a BODY in most positions and an
//! OBJECT only in value position. `directive()` disambiguates with
//! `starts_object_literal`, which already handles both key shapes and uses
//! POSITION as the discriminator (an object value follows the `:` that
//! introduces it; a body follows a directive head).
//!
//! `variable_ref()` had no such guard — it took ANY `{` as a body. So the object
//! became a body, the value went missing, and the form matcher reported a
//! grammar mismatch against a declaration whose value it never received.
//!
//! The fix reuses the existing guard rather than adding a second notion of "is
//! this an object", which is how the two paths came to disagree in the first
//! place.
//!
//! Written RED: seen failing with E0946 before the guard was applied.

use std::path::{Path, PathBuf};
use std::process::Command;

fn check_src(src: &str) -> (bool, String) {
    // A per-call unique directory. An earlier version keyed on the source
    // POINTER, which is not stable across calls and collided — two checks raced
    // on one `index.st` and a passing shape reported red. A gate whose harness
    // is flaky cannot tell you which colour means what.
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scratch")
        .join(format!(
            "objlit-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let file = dir.join("index.st");
    std::fs::write(&file, src).expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .arg("check")
        .arg(&file)
        .output()
        .expect("spacetime binary should run");
    let _ = std::fs::remove_dir_all(&dir);
    let mut text = String::from_utf8_lossy(&out.stdout).to_string();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.success(), text)
}

fn page(decl: &str) -> String {
    format!("<div class=\"a\">hi</div>\n\n{decl}\n")
}

/// The reported case.
#[test]
fn a_typed_declaration_accepts_an_object_literal() {
    let (ok, out) = check_src(&page("$u object: { name: \"Ada\" };"));
    assert!(
        ok,
        "`$u object: {{ … }}` must compile — the untyped form and the typed \
         ARRAY form both already do, so adding a type must not remove a value \
         shape:\n{out}"
    );
}

/// The type name must stay irrelevant — this is not a registry lookup.
#[test]
fn the_type_name_does_not_change_the_answer() {
    let (ok, out) = check_src(&page("$u Foo: { name: \"Ada\" };"));
    assert!(
        ok,
        "an arbitrary type name must behave exactly like `object` here:\n{out}"
    );
}

/// A quoted-key object (the JSON shape) must work too.
#[test]
fn a_typed_declaration_accepts_a_quoted_key_object() {
    let (ok, out) = check_src(&page("$u object: { \"name\": \"Ada\" };"));
    assert!(ok, "the JSON-shaped object literal must compile:\n{out}");
}

/// Over-widening guard 1: the shapes that already worked must keep working.
#[test]
fn the_neighbouring_shapes_still_compile() {
    // NB `$l string[]: ["a"];` is NOT in this list. The bug notes recorded it as
    // working; it does not compile at HEAD. That is a separate defect (a typed
    // ARRAY declaration), filed rather than folded in — asserting it here would
    // make this gate red for two unrelated reasons and hide whichever was fixed
    // second.
    for decl in [
        "$u: { name: \"Ada\" };",
        "$u object: [1, 2];",
        "$u object: 3;",
    ] {
        let (ok, out) = check_src(&page(decl));
        assert!(ok, "`{decl}` regressed:\n{out}");
    }
}

/// Over-widening guard 2: a `{` that is genuinely a BODY must still be a body.
///
/// This is the whole risk of the fix. `{ ident: … }` is also the shape of a CSS
/// declaration block, and treating one as an object value would swallow bodies
/// across the tree — which is exactly what an earlier, position-blind version of
/// the object guard did.
#[test]
fn a_body_is_still_a_body() {
    let (ok, out) = check_src("<div class=\"a\">hi</div>\n\n.a { color: red; }\n");
    assert!(
        ok,
        "a CSS body must not be swallowed as an object literal:\n{out}"
    );

    let (ok2, out2) = check_src(
        "<div class=\"a\">hi</div>\n\n$n number: 0;\n.a { @on &.click { $n <- 1; } }\n",
    );
    assert!(
        ok2,
        "a directive body must not be swallowed as an object literal:\n{out2}"
    );
}

#[allow(dead_code)]
fn unused(_: PathBuf) {}
