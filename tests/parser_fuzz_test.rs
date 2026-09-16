//! Parser robustness fuzz (PLAN-027 W4, compiler tier — libfuzzer-free).
//!
//! Asserts `spacetime::parser::parse` never PANICS on adversarial / random
//! input (it must return Result, never unwind). Complements the nightly
//! `cargo fuzz` targets in `fuzz/` with a deterministic, always-runnable check.

use proptest::prelude::*;

/// A curated adversarial corpus: directive fragments, unbalanced delimiters,
/// nested braces, lone quotes, and the historically-crashing shapes (BUG-052,
/// the lone-quote expand.rs slice panic, deep nesting).
const ADVERSARIAL: &[&str] = &[
    "",
    "\"",
    "'",
    "(",
    ")",
    "{",
    "}",
    "@",
    "@assert (",
    "@assert (\"",
    "@then .x should",
    ".a { @on &.click { $x <- ",
    "@import \"",
    "%macro x {",
    "@data fetch x T : \"",
    ".x { color: }",
    "{{{{{{{{{{",
    "}}}}}}}}}}",
    "@assert (((((((((((",
    ".a{.b{.c{.d{.e{",
    "@behavior .m machine(initial:",
    "$\u{0000}",
    "@then .x { state == ",
    "/* unclosed comment",
    "\\\\\\\\",
    "@assert (1 === 2) \"msg\" extra trailing tokens here",
];

#[test]
fn parse_never_panics_on_adversarial_corpus() {
    for input in ADVERSARIAL {
        // Must not panic; Ok or Err are both acceptable.
        let _ = spacetime::parser::parse(input);
    }
}

proptest! {
    // Random byte strings (valid UTF-8) must never panic the parser.
    #![proptest_config(ProptestConfig::with_cases(2000))]
    #[test]
    fn parse_never_panics_on_random_utf8(s in ".{0,200}") {
        let _ = spacetime::parser::parse(&s);
    }

    // Random strings drawn from a Spacetime-ish alphabet (directives, delimiters,
    // operators) — denser coverage of the real grammar's panic surface.
    #[test]
    fn parse_never_panics_on_grammarish(s in "[@%${}().;:\"'<>= a-z0-9-]{0,160}") {
        let _ = spacetime::parser::parse(&s);
    }
}
