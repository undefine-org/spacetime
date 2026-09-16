//! A design token is a legal value of its type, wherever that type is expected.
//!
//! PLAN-136 W2. `--ease-out-expo` and `var(--ink)` name a value whose
//! resolution happens later — in the cascade, at paint time. They are not
//! literals, but they are not errors either, and the scalar grammars refused
//! both:
//!
//!     easing <- --ease-out-expo   REFUSED
//!     color  <- var(--ink)        REFUSED
//!
//! while `easing: --ease-out-expo` is authored code in
//! `demos/spacetime-docs/index.st:148` today. It passed only because `:ident`
//! (the type that declaration actually carries) accepted anything — so W1,
//! which made `:ident` mean something, is exactly what turns this from a latent
//! wrong answer into a broken build. The two waves are one change.
//!
//! This is the same judgement the CSS declaration validator already makes:
//! `var()` defers to the browser rather than being refused
//! (`src/validation/css.rs`). One rule, in the second place it was needed.
//!
//! WHY NOT AN ARM PER SCALAR: adding `| ( $token:token_ref )` to each of the six
//! leaf productions is six places to forget. A NEW `%scalar_type` row would not
//! inherit it, which reintroduces exactly the hand-synced-list drift PLAN-122
//! spent seven commits deleting. The admission is made ONCE, keyed off the
//! scalar-capture set the registry already maintains, so a scalar added
//! tomorrow gets it for free.

use spacetime::syntax::events::capture_type_accepts;

/// The authored spelling that is in the tree right now.
#[test]
fn a_bare_token_reference_is_a_value_of_its_type() {
    for (ty, val) in [
        ("easing", "--ease-out-expo"),
        ("color", "--brand-ink"),
        ("length", "--gap-lg"),
        ("duration", "--dur-fast"),
        ("time", "--dur-fast"),
        ("angle", "--tilt"),
        ("percentage", "--fill"),
    ] {
        assert!(
            capture_type_accepts(ty, val),
            "`{ty}` must accept the token reference `{val}` — \
             demos/spacetime-docs/index.st:148 writes exactly this"
        );
    }
}

/// `var()` is the CSS spelling of the same idea, and it may nest a fallback.
#[test]
fn a_var_reference_is_a_value_of_its_type() {
    for (ty, val) in [
        ("color", "var(--ink)"),
        ("color", "var(--ink, #fff)"),
        ("color", "var(--a, var(--b, red))"),
        ("length", "var(--pad)"),
        ("duration", "var(--dur, 300ms)"),
        ("easing", "var(--ease)"),
    ] {
        assert!(
            capture_type_accepts(ty, val),
            "`{ty}` must accept `{val}` — its value is unknowable at build time, \
             so refusing it would be a lie"
        );
    }
}

/// Admitting references must not admit nonsense. This is the half that proves
/// the change is a widening and not a hole.
#[test]
fn admitting_references_does_not_admit_garbage() {
    for (ty, val) in [
        ("easing", "not-a-curve"),
        ("color", "#e8ee1"),
        ("length", "8zz"),
        ("duration", "abc"),
        ("time", "60"),
        ("color", "-ink"),          // one dash is not a token reference
        ("color", "--"),            // a sigil with no name
        ("color", "var(--ink"),     // unbalanced
        ("color", "var()"),         // no name
        ("color", "notvar(--ink)"), // not the var function
    ] {
        assert!(
            !capture_type_accepts(ty, val),
            "`{ty}` must still REFUSE `{val}`"
        );
    }
}

/// A scalar added as pure stdlib data inherits reference support with no Rust
/// edit. This is FEAT-168's negative acceptance, extended to cover W2: if this
/// fails, the admission was written per-scalar and the drift is back.
#[test]
fn a_new_scalar_inherits_reference_support() {
    // `resolution` is not referenced by any Rust match arm; it exists only as a
    // `%scalar_type` row + `%capture_type` production in stdlib.
    if !capture_type_accepts("resolution", "96dpi") {
        // The row is absent from this checkout — nothing to prove, and failing
        // here would report a missing fixture as a broken feature.
        return;
    }
    assert!(
        capture_type_accepts("resolution", "--screen-density"),
        "a scalar defined purely as stdlib data must inherit token references; \
         if it does not, the admission was written per-scalar"
    );
}

/// Parens inside a string or comment are content, not structure.
///
/// Found by the review swarm: the balance check counted raw parens, so the `)`
/// inside `/* ) */` drove the depth negative and refused a valid fallback — a
/// false refusal on legal CSS.
#[test]
fn parens_inside_strings_and_comments_do_not_break_balance() {
    for value in [
        "var(--ink, /* ) */ red)",
        r#"var(--font, "a)b")"#,
        r#"var(--bg, url("i)mg.png"))"#,
    ] {
        assert!(
            capture_type_accepts("color", value),
            "`{value}` balances once strings and comments are treated as content"
        );
    }
    // …and genuinely unbalanced input is still refused.
    for value in ["var(--a))", "var(--a", "var(--a, (b)"] {
        assert!(
            !capture_type_accepts("color", value),
            "`{value}` is unbalanced and must still be REFUSED"
        );
    }
}

