//! BUG-212 — a bodyless `if (cond) return;` in `%emit js` was rewritten into
//! semantically DIFFERENT JavaScript, silently.
//!
//! `extract_cond_and_body` parses `if (cond) { body }` out of an emit body. Its
//! "between the `)` and the `{`" phase skipped every character until it found a
//! `{` ANYWHERE ahead — so for
//!
//! ```js
//! if (!x) return;
//! const f = () => { x.dataset.hit = '1'; };
//! ```
//!
//! it discarded `return; const f = () =>` and adopted the ARROW's braces as the
//! if's body. Three corruptions from one skip:
//!
//!   1. the `return` was deleted;
//!   2. the guard INVERTED — work meant to run when `x` exists now ran only when
//!      it was missing, dereferencing the pointer the guard just proved null;
//!   3. `const f` was dropped, so the following `addEventListener('input', f)`
//!      referenced an undeclared binding.
//!
//! No diagnostic. The emitted guard still reads plausibly in review because the
//! `if` is right there — which is why this is #A despite having had no live
//! outage: it silently corrupts the NEXT primitive written with the early-return
//! idiom, the shape a competent JS author reaches for first.

use spacetime::compiler::Compiler;

fn emit_primitive_js(body: &str) -> String {
    let src = format!(
        r#"%primitive mini-guard(&el) {{
  %emit js {{
{body}
  }}
}}
%macro mini-guard {{ %form {{ @mini-guard() }} %binds {{ mini-guard(&self) }} }}
body {{ <div class="m"><input class="q"></div> }}
.m {{ @mini-guard; }}
"#
    );
    let dir = tempfile::tempdir().expect("tempdir");
    let entry = dir.path().join("index.st");
    std::fs::write(&entry, src).expect("write");
    Compiler::from_file(&entry, dir.path())
        .unwrap_or_else(|e| panic!("compile failed: {e}"))
        .compile()
        .js
}

/// The reported case, end to end: the guard must survive intact.
#[test]
fn a_bodyless_return_guard_followed_by_an_arrow_declaration_round_trips() {
    let js = emit_primitive_js(
        "    const x = %&el.querySelector('.q');\n\
         \x20   if (!x) return;\n\
         \x20   const f = () => { x.dataset.hit = '1'; };\n\
         \x20   x.addEventListener('input', f);",
    );

    assert!(
        js.contains("if (!x) return;"),
        "the guard must keep its `return` — deleting it inverts the whole primitive"
    );
    assert!(
        js.contains("const f = () =>"),
        "the arrow declaration must survive; dropping it leaves the following \
         listener referencing an undeclared binding"
    );
    assert!(
        !js.contains("if (!x) {\n    x.dataset.hit"),
        "the guard must NOT absorb the arrow's body — that is the inversion: work \
         for when `x` EXISTS would run only when it is missing"
    );
}

/// The shapes that already worked must keep working — this fix narrows when the
/// structural parse applies, so a braced `if` must still be parsed structurally.
#[test]
fn braced_if_forms_are_unaffected() {
    let js = emit_primitive_js(
        "    const x = %&el.querySelector('.q');\n\
         \x20   if (!x) { return; }\n\
         \x20   const f = () => { x.dataset.hit = '1'; };\n\
         \x20   x.addEventListener('input', f);",
    );
    assert!(
        js.contains("return"),
        "a BRACED guard must still emit its return"
    );
    assert!(
        js.contains("const f = () =>"),
        "and must not swallow the following declaration"
    );
}

/// A bodyless `if` whose statement is NOT a return (the other half of the shape)
/// must also pass through unchanged.
#[test]
fn a_bodyless_if_with_a_plain_statement_round_trips() {
    let js = emit_primitive_js(
        "    const x = %&el.querySelector('.q');\n\
         \x20   if (!x) x = document.body;\n\
         \x20   const f = () => { x.dataset.hit = '1'; };\n\
         \x20   x.addEventListener('input', f);",
    );
    assert!(
        js.contains("if (!x) x = document.body;"),
        "a bodyless if with a plain statement must round-trip verbatim"
    );
    assert!(js.contains("const f = () =>"), "and not lose what follows");
}

/// Whitespace and newlines between `)` and `{` are legal and must still parse as
/// a normal braced `if` — the fix only refuses on a non-brace TOKEN.
#[test]
fn a_newline_between_the_condition_and_its_brace_still_parses_as_braced() {
    let js = emit_primitive_js(
        "    const x = %&el.querySelector('.q');\n\
         \x20   if (!x)\n\
         \x20   {\n\
         \x20     x = document.body;\n\
         \x20   }\n\
         \x20   const f = () => { x.dataset.hit = '1'; };",
    );
    assert!(
        js.contains("document.body"),
        "a brace on the next line is still the if's body"
    );
    assert!(
        js.contains("const f = () =>"),
        "and the following declaration survives"
    );
}
