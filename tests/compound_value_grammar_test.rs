//! `stagger: 0.05 first` and `range: 0 to 0.5` are values with a shape.
//!
//! PLAN-136 W5. These two spellings are written 202 times across the corpus and
//! no grammar describes either, which is why W4's dry run refused 25 valid
//! declarations before the rule was scoped back. Until a production says what
//! they ARE, nothing can check them and nothing can tighten the declarations
//! that name them.
//!
//! # Why the declarations could not simply be tightened
//!
//! The obvious move was `%migration`: rewrite bare numbers to `ms` and tighten
//! `stagger: $stagger:number` to `:time`. Measured, that would corrupt output:
//!
//!   - `stagger: 0.05 first` is a COMPOUND value (delay + origin), so `:time`
//!     rejects it outright.
//!   - the bare number is SECONDS here (`0.05`), while `duration:number = 600`
//!     in reveal.st is MILLISECONDS and `duration:number = 1.2` in
//!     smooth-scroll.st is seconds again. The unit is per-declaration, not
//!     per-property, so a blanket `N -> Nms` rewrite makes staggers 1000x too
//!     slow in 20 files, silently, with a green build.
//!
//! So the grammar comes first and asserts only what is actually true: a delay is
//! a number OR a duration, and its unit is the runtime's business.

use spacetime::syntax::events::capture_type_accepts;

/// Every stagger spelling the corpus actually writes.
#[test]
fn stagger_accepts_what_the_corpus_writes() {
    for value in [
        "0",                        // 110 uses — the default
        "30",                       // bare number, unit is the runtime's
        "0.05",                     // seconds, per apply-animations.st
        "50ms",                     // explicit duration
        "0.1 first",                // 23 uses — delay + origin
        "0.15 last",
        "0.08 center",
        "0.003 grid(13 13) center", // 3 uses — delay + grid + origin
        "0.004 grid(10 10) center",
        "0.08 grid(3 2) center",
    ] {
        assert!(
            capture_type_accepts("stagger_value", value),
            "`stagger: {value}` is authored in the corpus and must be accepted"
        );
    }
}

/// A widening is only honest if it still refuses what is wrong.
#[test]
fn stagger_refuses_nonsense() {
    for value in [
        "first",            // an origin with no delay
        "0.05 sideways",    // not an origin
        "0.05 grid(1)",     // a grid needs two dimensions
        "0.05 grid(a b)",   // dimensions are numbers
        "banana",
        "#e8eef7",
        "0.05 first extra", // trailing junk
    ] {
        assert!(
            !capture_type_accepts("stagger_value", value),
            "`stagger: {value}` must be REFUSED"
        );
    }
}

/// `range: 0 to 0.5` — 49 uses, one shape.
#[test]
fn range_accepts_a_start_and_an_end() {
    for value in ["0 to 0.5", "0.1 to 0.5", "0 to 1", "0.3 to 0.8"] {
        assert!(
            capture_type_accepts("range_value", value),
            "`range: {value}` is authored in the corpus and must be accepted"
        );
    }
}

#[test]
fn range_refuses_nonsense() {
    for value in ["0", "0 to", "to 1", "0 1", "0 through 1", "a to b"] {
        assert!(
            !capture_type_accepts("range_value", value),
            "`range: {value}` must be REFUSED"
        );
    }
}

/// The grammars are stdlib DATA, not Rust. If adding them required a Rust edit
/// the metasystem would not be self-describing — the same negative acceptance
/// FEAT-168 shipped and W2 extended.
#[test]
fn the_new_grammars_are_stdlib_data() {
    let file = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/stdlib/capture-types/spacetime-values.st"
    );
    let src = std::fs::read_to_string(file).expect("the productions live in stdlib, as data");
    for production in [
        "%capture_type stagger_value",
        "%capture_type stagger_origin",
        "%capture_type stagger_grid",
        "%capture_type range_value",
    ] {
        assert!(
            src.contains(production),
            "`{production}` must be declared in stdlib, not in a Rust match arm"
        );
    }
}
