//! A property whose type the forms declare must have its value checked.
//!
//! PLAN-136 W4. W3 derived `(directive, property) -> capture type` from the
//! registered forms. This wave spends it: when a directive param's value fails
//! the type its own form declares, that is a diagnostic, not a shrug.
//!
//! # What this wave deliberately does NOT judge
//!
//! A dry run over the corpus before enforcing found 25 declarations the naive
//! rule would refuse, and every one was the RULE being wrong:
//!
//!     stagger: 0.05 first          a compound value — number + origin
//!     stagger: 0.003 grid(13 13) center
//!     easing: cubic-bezier(0.34, 1.56, 0.64, 1)
//!
//! `stagger: 0.05 first` is not a number that happens to be followed by junk;
//! it is a value with its own shape (delay + origin + optional grid), and no
//! `%capture_type` describes it. The map has no entry for it, so W4 says
//! nothing — which is the correct answer, not a gap being tolerated.
//!
//! That is the whole discipline here: enforce exactly what a form DECLARED, and
//! stay silent everywhere else. A validator that guesses is worse than one that
//! waits, because a false refusal blocks a build over a value that was always
//! legal — the same reasoning that exempted function-shaped CSS in FUP-177.

use spacetime::validation::css::directive_property_diagnostics;

/// The headline: a value that fails its declared type is refused.
#[test]
fn a_value_that_fails_its_declared_type_is_refused() {
    // @fade-in-up declares `distance: $distance:length` (fade-in.st:53).
    //
    // NB `duration` is deliberately not used here: on this form it is a
    // POSITIONAL capture (`$duration:time`, no label), so there is no property
    // name to key it by and the map correctly holds no entry. A property is
    // enforceable only where a form gave it a NAME.
    let diags = directive_property_diagnostics("fade-in-up", &[("distance", "banana")]);
    assert_eq!(
        diags.len(),
        1,
        "`distance: banana` must be refused against the declared `length`"
    );
    assert!(
        diags[0].message.contains("length"),
        "the diagnostic must name the type that was expected, got: {}",
        diags[0].message
    );
}

/// The other half: a valid value passes. Without this, a validator that refuses
/// everything would score as working.
#[test]
fn a_value_that_satisfies_its_declared_type_passes() {
    for (directive, property, value) in [
        ("fade-in-up", "distance", "30px"),
        ("fade-in-up", "easing", "ease-out"),
        ("fade-in-up", "easing", "cubic-bezier(.2, 0, 0, 1)"),
        // W2: a token reference is a value of its type.
        ("fade-in-up", "easing", "--ease-out-expo"),
        ("fade-in-up", "distance", "var(--gap)"),
    ] {
        let diags = directive_property_diagnostics(directive, &[(property, value)]);
        assert!(
            diags.is_empty(),
            "@{directive} `{property}: {value}` must PASS, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}

/// A property no form declares is not ours to judge. This is what keeps
/// `stagger: 0.05 first` — a compound value with no grammar — from being
/// refused by a rule that does not understand it.
#[test]
fn a_property_no_form_declares_is_not_judged() {
    let diags = directive_property_diagnostics("fade-in-up", &[("no-such-property", "anything")]);
    assert!(
        diags.is_empty(),
        "an undeclared property must be silent, not refused"
    );

    let diags = directive_property_diagnostics("no-such-directive", &[("duration", "banana")]);
    assert!(
        diags.is_empty(),
        "an unknown directive has no declarations to enforce"
    );
}

/// Values that are not literals at all — bindings, holes, element refs — are
/// resolved elsewhere and must never be measured against a value grammar.
#[test]
fn non_literal_values_are_left_alone() {
    for value in ["$speed", "`hole`", "&element", "0 -> 1", "$brand.ink"] {
        let diags = directive_property_diagnostics("fade-in-up", &[("distance", value)]);
        assert!(
            diags.is_empty(),
            "`{value}` is not a literal; it must not be checked as one"
        );
    }
}

/// A type with no grammar behind it cannot enforce anything, and pretending
/// otherwise would refuse valid values. `expr` means "hand this to JS";
/// `array`/`object` are structural shapes, not values.
#[test]
fn types_that_resolve_to_nothing_enforce_nothing() {
    // @data fetch declares `src: $src:expr` — any expression is legal.
    let diags = directive_property_diagnostics("data", &[("src", "anything at all")]);
    assert!(
        diags.is_empty(),
        "`expr` is deliberately permissive; enforcing it would be enforcing nothing"
    );
}

/// The measured corpus reality this wave must not break: every real `.st` in
/// the tree still compiles. A gate that only tests synthetic inputs cannot see
/// a rule that is right in the small and wrong across 21,979 declarations.
#[test]
fn the_corpus_stays_clean() {
    // Cheap proxy for the full corpus run: the exact spellings the dry run
    // flagged, which must all still pass because no form declares them.
    for (directive, property, value) in [
        ("on", "stagger", "0.05 first"),
        ("on", "stagger", "0.003 grid(13 13) center"),
        ("on", "easing", "cubic-bezier(0.34, 1.56, 0.64, 1)"),
        ("on", "easing", "linear"),
        ("on", "easing", "--ease-out-expo"),
    ] {
        let diags = directive_property_diagnostics(directive, &[(property, value)]);
        assert!(
            diags.is_empty(),
            "@{directive} `{property}: {value}` is authored in the corpus today \
             and must not be refused, got: {:?}",
            diags.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}
