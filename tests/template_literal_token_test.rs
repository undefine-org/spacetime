//! FUP-046 — WITHDRAWN AS SPECIFIED. Read this before attempting it again.
//!
//! The item asks for a `TEMPLATE_STRING` token so a backtick-delimited literal
//! lexes as ONE opaque token, making a `{` inside it text rather than a brace.
//! I implemented exactly that. It passed all seven gates below and CAUSED A
//! CATASTROPHIC REGRESSION:
//!
//!     lib      3100 passed / 3 failed  ->  3061 / 130
//!     examples 35  ->  55
//!     demos    36  ->  82
//!     fixtures 6   ->  31
//!     headless 744 ->  0 passed, 0 failed   (the harness stopped running)
//!
//! WHY, and it is not subtle once seen: in Spacetime the backtick is THE HOLE
//! SIGIL. `` `$c.id` `` interpolates a value; AGENTS.md states it outright —
//! "`` ` `` is THE hole form everywhere". The corpus contains 4,934 of them.
//! A token that consumes from one backtick to the next does not make templates
//! opaque; it swallows every hole in the language and everything between two of
//! them.
//!
//! FUP-046 was written from the perspective of JS `@eval` bodies, where a
//! backtick really is a template delimiter. It is correct about the DEFECT (a
//! brace inside a JS template miscounts in `parse_arrow_fn_balanced`) and wrong
//! about the FIX, because the same character means something else — and more
//! common — in Spacetime itself.
//!
//! WHAT A CORRECT FIX WOULD NEED: the lexer cannot decide from the character
//! alone. It would have to know it is inside a FOREIGN-CODE ISLAND (an `@eval`
//! payload, an `%emit js` body) where JS lexing rules apply, and lex a template
//! only there. That is a lexer MODE, not a token — a materially larger change
//! than the item describes, and one that must not alter how a backtick lexes in
//! Spacetime source.
//!
//! CONSEQUENCE FOR THE VALUE-TYPE WORK: a `%capture_type` grammar consumes
//! tokens, so with no hole token no grammar can classify a hole. The six-sigil
//! escape check in `validate_declaration_value` can therefore shrink to ONE
//! (backtick), but cannot yet reach zero. That single remaining sigil is
//! blocked on the lexer-mode work above, not on anything in the validator.
//!
//! The gates below are kept because they are correct about the BEHAVIOUR a
//! template token should have, and because the reverted implementation passed
//! all seven — which is precisely why a green gate is not proof that a change
//! is safe. They are ignored, not deleted.

use spacetime::syntax::cst::{Lexer, SyntaxKind};

fn kinds(src: &str) -> Vec<SyntaxKind> {
    Lexer::new(src)
        .tokenize()
        .into_iter()
        .map(|t| t.kind)
        .filter(|k| !k.is_trivia() && *k != SyntaxKind::EOF)
        .collect()
}

fn count(src: &str, kind: SyntaxKind) -> usize {
    kinds(src).into_iter().filter(|k| *k == kind).count()
}

/// The core claim: a template is one token, so its interior is opaque.
#[test]
#[ignore = "FUP-046: a TEMPLATE_STRING token swallows Spacetime holes — see the module doc"]
fn a_template_literal_is_a_single_token() {
    let toks = kinds("`hello`");
    assert_eq!(
        toks.len(),
        1,
        "a template literal must lex as ONE token, got {toks:?}"
    );
}

/// The defect FUP-046 actually reports: a brace inside a template must not be
/// counted as a brace. This is what breaks `parse_arrow_fn_balanced`.
#[test]
#[ignore = "FUP-046: a TEMPLATE_STRING token swallows Spacetime holes — see the module doc"]
fn a_brace_inside_a_template_is_not_a_brace_token() {
    assert_eq!(
        count("`<b>}</b>`", SyntaxKind::R_BRACE),
        0,
        "a closing brace inside a template literal is text, not a brace token"
    );
    assert_eq!(
        count("`{unbalanced`", SyntaxKind::L_BRACE),
        0,
        "an opening brace inside a template literal is text too"
    );
}

/// A template legitimately spans newlines — unlike a Spacetime string literal,
/// which is single-line by design (BUG-072). This is why the quoted-string
/// callback could not simply be reused.
#[test]
#[ignore = "FUP-046: a TEMPLATE_STRING token swallows Spacetime holes — see the module doc"]
fn a_template_literal_may_span_newlines() {
    let src = "`line one\nline two`";
    assert_eq!(
        kinds(src).len(),
        1,
        "a multi-line template is still one token"
    );
}

/// `${ … }` interpolation is part of the template. Its braces are the
/// template's own, and must stay opaque to an outer balancer.
#[test]
#[ignore = "FUP-046: a TEMPLATE_STRING token swallows Spacetime holes — see the module doc"]
fn interpolation_braces_stay_inside_the_template() {
    assert_eq!(
        count("`a ${x} b`", SyntaxKind::L_BRACE),
        0,
        "an interpolation's braces belong to the template"
    );
}

/// An escaped backtick does not close the template.
#[test]
#[ignore = "FUP-046: a TEMPLATE_STRING token swallows Spacetime holes — see the module doc"]
fn an_escaped_backtick_does_not_terminate() {
    assert_eq!(
        kinds("`a \\` b`").len(),
        1,
        "a backslash-escaped backtick is content, not a terminator"
    );
}

/// THE BOUND, and the reason this test exists at all: an unterminated backtick
/// must NOT swallow the rest of the file. A lone backtick in prose stands as
/// punctuation and the following source keeps lexing — the same recovery the
/// quoted-string callback performs at EOF, and for the same reason (BUG-040 /
/// BUG-041: a swallowed closing brace reports "expected R_BRACE, found EOF"
/// nowhere near the real problem).
#[test]
#[ignore = "FUP-046: a TEMPLATE_STRING token swallows Spacetime holes — see the module doc"]
fn an_unterminated_backtick_does_not_swallow_the_file() {
    let toks = kinds("` and then .rule { color: red; }");
    assert!(
        toks.len() > 5,
        "an unterminated backtick must leave the following source lexable, got {toks:?}"
    );
    assert_eq!(
        count("` and then .rule { color: red; }", SyntaxKind::L_BRACE),
        1,
        "the real brace after an unterminated backtick must still be a brace"
    );
}

/// The end-to-end case from the item: an arrow `@eval` body whose template
/// contains a brace must not terminate early.
#[test]
#[ignore = "FUP-046: a TEMPLATE_STRING token swallows Spacetime holes — see the module doc"]
fn an_arrow_body_survives_a_braced_template() {
    let src = "(() => { el.innerHTML = `<b>}</b>`; })";
    assert_eq!(
        count(src, SyntaxKind::L_BRACE),
        1,
        "only the arrow body's own brace is a brace"
    );
    assert_eq!(
        count(src, SyntaxKind::R_BRACE),
        1,
        "the closing brace in the template must not close the arrow body"
    );
}
