//! FUP-046, done structurally: the backtick is a SIGIL, so it lexes like one.
//!
//! The first attempt (see `template_literal_token_test.rs`) gave the backtick a
//! SPAN-CONSUMING token that ate from one backtick to the next. It passed its
//! own gates and destroyed the corpus, because in Spacetime the backtick is the
//! HOLE sigil and there are 4,934 of them.
//!
//! The structural reading: EVERY other sigil in this language is a
//! NON-CONSUMING, fixed-width token. `$` `&` `@` `%` `#` are one char; `=>`
//! `->` `<-` are two. Not one of them decides how far it reaches — the PARSER
//! decides that, from grammar. The only span-consuming tokens are strings and
//! comments, and they are precisely where this lexer's scar tissue lives
//! (BUG-040, BUG-041, BUG-072, BUG-224 — every one an "it swallowed the rest of
//! the file" bug).
//!
//! So the backtick becomes a one-char BACKTICK token and nothing else. The
//! parser keeps deciding what a hole spans, exactly as it already does.
//!
//! What it is TODAY is worse than absent. `raw_tokenize` has no error category:
//! an unrecognised byte is pushed as `RawToken::Minus /* placeholder */`,
//! keeping its own span but wearing another token's identity. `refine` then
//! applies the vendor-prefix rule (MINUS + IDENT fuse for `-webkit-*`) and
//! swallows the backtick into the identifier after it. Measured:
//!
//!     "a `b` c"  ->  IDENT("a")  IDENT("`b")  MINUS("`")  IDENT("c")
//!
//! The hole sigil is INSIDE an identifier, and its partner is a minus sign.
//! That is the leak: Spacetime syntax reaching the meta-transpiler disguised as
//! arithmetic. Every downstream consumer that balances braces, counts operators,
//! or reads an identifier is reasoning about a fiction.

use spacetime::syntax::cst::{Lexer, SyntaxKind};

fn kinds(src: &str) -> Vec<SyntaxKind> {
    Lexer::new(src)
        .tokenize()
        .into_iter()
        .map(|t| t.kind)
        .filter(|k| !k.is_trivia() && *k != SyntaxKind::EOF)
        .collect()
}

fn texts(src: &str) -> Vec<String> {
    Lexer::new(src)
        .tokenize()
        .into_iter()
        .filter(|t| !t.kind.is_trivia() && t.kind != SyntaxKind::EOF)
        .map(|t| t.text)
        .collect()
}

/// The sigil is its own token — like `$`, `&`, `@`, `%`.
#[test]
fn a_backtick_is_its_own_token() {
    assert_eq!(
        kinds("`"),
        vec![SyntaxKind::BACKTICK],
        "a lone backtick must lex as BACKTICK, not as a disguised MINUS"
    );
}

/// THE REGRESSION THAT MATTERS. A backtick must never fuse into an identifier.
#[test]
fn a_backtick_never_fuses_into_an_identifier() {
    assert_eq!(
        texts("a `b` c"),
        vec!["a", "`", "b", "`", "c"],
        "the vendor-prefix merge rule must not swallow a backtick"
    );
}

/// Both delimiters are the SAME token — a hole is symmetric.
#[test]
fn both_delimiters_lex_alike() {
    assert_eq!(
        kinds("`$x`"),
        vec![
            SyntaxKind::BACKTICK,
            SyntaxKind::DOLLAR,
            SyntaxKind::IDENT,
            SyntaxKind::BACKTICK
        ],
        "opening and closing backtick must be the same kind"
    );
}

/// NON-CONSUMING is the whole point: the token stops at one byte, and the
/// parser decides the span. This is what the reverted attempt got wrong.
#[test]
fn a_backtick_does_not_consume_what_follows() {
    let ks = kinds("`a` `b`");
    assert_eq!(
        ks.iter().filter(|k| **k == SyntaxKind::BACKTICK).count(),
        4,
        "four backticks means four tokens — a consuming token would give two"
    );
}

/// The vendor-prefix rule it was hiding inside must still work.
#[test]
fn vendor_prefixes_still_fuse() {
    assert_eq!(texts("-webkit-mask"), vec!["-webkit-mask"]);
}

/// And `--name`, the OTHER sigil built from minus, is untouched.
#[test]
fn a_token_reference_still_lexes_whole() {
    assert_eq!(texts("--ink"), vec!["--ink"]);
}

/// Arithmetic minus is not collateral damage.
#[test]
fn a_real_minus_is_still_a_minus() {
    assert_eq!(kinds("a - b")[1], SyntaxKind::MINUS);
}
