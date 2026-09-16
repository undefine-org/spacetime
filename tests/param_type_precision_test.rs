//! Why some `%form` params stay broad — the constraint, pinned.
//!
//! The cutover tightened every param it safely could: `easing:ident` -> `:easing`
//! (13 sites), `easing:string` -> `:easing` (@flip), `distance:string` -> `:length`
//! (@reveal). A sweep proposed several more. Most of those were WRONG, and the
//! reason is not a judgement call — it is measurable, so it is measured here.
//!
//! Two independent constraints stop a param from being tightened. Neither is
//! visible from the declaration alone, which is exactly why this file exists:
//! the next person to run that sweep will reach the same candidates, and needs
//! to find the answer already written down rather than re-derive it by breaking
//! the build.

use spacetime::syntax::events::capture_type_accepts;

/// CONSTRAINT 1 — a precise grammar refuses a QUOTED value.
///
/// `@cursor` declares `size: $size:string = "24px"`. The value is CSS, so
/// `:length` looks obviously right. But the default is a STRING LITERAL, and the
/// length grammar parses CSS, not a quoted CSS-shaped string:
///
///     length <- 24px    = true
///     length <- "24px"  = false
///
/// So the migration is not a type change, it is an UNQUOTING — every declaration
/// AND every call site has to move together, or the default itself becomes
/// invalid. Where a call site exists, that is real work with real risk; where the
/// param is quoted by contract (see constraint 2) it is impossible.
#[test]
fn a_precise_grammar_refuses_a_quoted_value() {
    assert!(capture_type_accepts("length", "24px"));
    assert!(capture_type_accepts("color", "#ff6b47"));
    assert!(capture_type_accepts("easing", "ease-out"));

    assert!(
        !capture_type_accepts("length", "\"24px\""),
        "if a quoted length starts passing, the quoting constraint is gone and \
         the `:string` params in cursor.st can finally be tightened"
    );
    assert!(
        !capture_type_accepts("color", "\"#ff6b47\""),
        "same for colour \u{2014} see magnetic-glow.st:24, particle-field.st:19"
    );
    assert!(!capture_type_accepts("easing", "\"ease-out\""));
}

/// CONSTRAINT 2 — a comma-bearing function value CANNOT be unquoted at all.
///
/// `@particle-field` declares `color: $color:string = "rgba(255,255,255,0.08)"`.
/// The colour grammar accepts `rgba(255,255,255,0.08)` perfectly well, so the
/// type looks weak. It is not. Measured through the real compiler:
///
///     @particle-field(count: 30, color: rgba(255,255,255,0.06))
///     -> E0946: Expected `$color:String`, got token "IDENT"
///
/// A `%form` param list is comma-separated, so the commas INSIDE `rgba(...)`
/// terminate the argument. The quotes are what make the value survive the
/// parameter grammar \u2014 they are load-bearing, not laziness.
///
/// This is the honest reason those params stay `:string`, and it is a GRAMMAR
/// limitation, not a typing one. Lifting it means teaching the param list to
/// balance parens (the same fix `balanced(')')` performs elsewhere), and only
/// then can the colour params be tightened. Filed rather than folded in: it is a
/// change to the form-matching grammar, which is a different subsystem from
/// value types.
#[test]
fn the_colour_grammar_accepts_what_the_param_list_cannot_carry() {
    // The value itself is a perfectly good colour...
    assert!(
        capture_type_accepts("color", "rgba(255,255,255,0.08)"),
        "the colour grammar handles functional notation"
    );
    assert!(capture_type_accepts("color", "currentColor"));

    // ...so if this test ever fails, it is the GRAMMAR that changed, and the
    // `:string` colour params should be revisited at the same time.
}

/// The tightenings that DID land, pinned so they cannot silently regress.
///
/// Each of these was verified against the corpus before being applied:
/// `@reveal`'s four real call sites pass `120%`, `50px`, `80px`, `60px`, and
/// `length` accepts all four plus the `100%` default.
#[test]
fn the_migrated_types_accept_every_corpus_value() {
    for v in ["100%", "120%", "50px", "80px", "60px"] {
        assert!(
            capture_type_accepts("length", v),
            "@reveal distance:length must accept the corpus value {v}"
        );
    }
    for v in ["ease-out", "linear", "cubic-bezier(.2, 0, 0, 1)"] {
        assert!(
            capture_type_accepts("easing", v),
            "@flip / @on / @fade-in easing:easing must accept {v}"
        );
    }
    assert!(
        !capture_type_accepts("easing", "not-a-curve"),
        "the point of :easing over :ident \u{2014} it can REFUSE"
    );
}
