//! BUG-261: a driver's `as` clause must never silently drop a name.
//!
//! `on_as` captures `$name:binding` and is OPTIONAL in every `@on` head, so a
//! wrong-sigil name failed the capture and the WHOLE CLAUSE VANISHED — no
//! diagnostic, because an optional capture that does not match is not an
//! error. The driver then published under its counter fallback
//! (`__drive_visible_1`) and the author's name was gone.
//!
//! Measured, before the fix:
//!
//! | written      | published             |
//! |--------------|-----------------------|
//! | `as $reveal` | `"reveal"`            |
//! | `as &reveal` | `"__drive_visible_1"` |
//! | `as reveal`  | `"__drive_visible_1"` |
//!
//! The NEGATIVE is load-bearing here: a positive-only gate ("the correct
//! spelling works") passes while two of three spellings silently lie.

use std::process::Command;

fn check(src: &str) -> String {
    let dir = std::env::temp_dir().join(format!("st_on_as_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let f = dir.join("index.st");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_spacetime"))
        .args(["check", f.to_str().unwrap()])
        .output()
        .expect("check runs");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

const PAGE: &str = r#"@version 2026-06-09;
<div class="stage"><div class="a">A</div></div>
@form motion --rise { opacity: 0 -> 1; }
"#;

/// THE NEGATIVE — an element sigil must be REFUSED, not dropped.
#[test]
fn an_element_sigil_in_an_as_clause_is_refused() {
    let out = check(&format!(
        "{PAGE}.a {{ @on &.hover(300ms) as &lift: --rise; }}\n"
    ));
    assert!(
        out.contains("E0953"),
        "`as &lift` must be REFUSED — it silently published under a counter \
         fallback before BUG-261. Got:\n{out}"
    );
}

/// The refusal must name the CORRECT SPELLING, not just complain. A
/// diagnostic that does not say what to write is a worse silent drop: the
/// author sees a wall and guesses.
#[test]
fn the_refusal_names_the_right_sigil_and_the_name() {
    let out = check(&format!(
        "{PAGE}.a {{ @on &.hover(300ms) as &lift: --rise; }}\n"
    ));
    assert!(
        out.contains("as $lift"),
        "the hint must spell the fix (`as $lift`), carrying the author's own \
         name through. Got:\n{out}"
    );
}

/// THE POSITIVE — without it the negative above is satisfied by an `@on`
/// that refuses every `as` clause.
#[test]
fn a_signal_sigil_in_an_as_clause_still_compiles() {
    let out = check(&format!(
        "{PAGE}.a {{ @on &.hover(300ms) as $lift: --rise; }}\n"
    ));
    assert!(
        !out.contains("E0953"),
        "`as $lift` is the CORRECT spelling and must compile. Got:\n{out}"
    );
    assert!(
        out.contains("passed"),
        "expected a clean check. Got:\n{out}"
    );
}

/// The body form takes the same clause and must refuse identically. Two heads
/// declare `$as:on_as?`; a fix applied to one of them is a fix that returns.
#[test]
fn the_body_form_refuses_an_element_sigil_too() {
    let out = check(&format!(
        "{PAGE}.a {{ @on &.hover(300ms) as &lift {{ opacity: 0 -> 1; }} }}\n"
    ));
    assert!(
        out.contains("E0953"),
        "the `@on ... {{ body }}` head carries the same `as` clause and must \
         refuse the same wrong sigil. Got:\n{out}"
    );
}

/// THE BARE NEGATIVE — `as lift` is the ordinary-looking spelling most likely
/// to fall through an optional capture. Refusing it is load-bearing: accepting
/// it publishes under the counter fallback and erases `lift` without a trace.
#[test]
fn a_bare_name_in_an_as_clause_is_refused_with_its_fix() {
    let out = check(&format!(
        "{PAGE}.a {{ @on &.hover(300ms) as lift: --rise; }}\n"
    ));
    assert!(
        out.contains("E0953") && out.contains("as $lift"),
        "`as lift` must be REFUSED with its exact signal spelling — it silently \
         published under a counter fallback before BUG-261. Got:\n{out}"
    );
}

/// The body head has the same optional clause. This separate negative prevents
/// a colon-only sibling from making `as lift { ... }` silently disappear.
#[test]
fn the_body_form_refuses_a_bare_name_too() {
    let out = check(&format!(
        "{PAGE}.a {{ @on &.hover(300ms) as lift {{ opacity: 0 -> 1; }} }}\n"
    ));
    assert!(
        out.contains("E0953"),
        "the `@on ... {{ body }}` head must REFUSE bare `as lift`, not lose the \
         clause through its optional capture. Got:\n{out}"
    );
}

/// THE POSITIVE — the bare-name sibling must not collide with an absent `as`,
/// either valid `$` form, or the arms head that starts with a signal. Without
/// these values, a grammar that rejects every nearby head makes the negatives
/// above pass while breaking valid programs.
#[test]
fn bare_as_error_siblings_leave_neighboring_heads_valid() {
    let out = check(&format!(
        "{PAGE}\
         .a {{ @on &.hover: --rise; }}\
         .b {{ @on &.hover as $lift: --rise; }}\
         .c {{ @on &.hover as $lift {{ opacity: 0 -> 1; }} }}\
         .d {{ @on $sig {{ true => --rise; false => --rise; }} }}\n"
    ));
    assert!(
        !out.contains("E0953") && out.contains("passed"),
        "bare-name error siblings must not capture valid no-`as`, `$`-signal, or \
         arms heads. Got:\n{out}"
    );
}
