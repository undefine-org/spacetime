//! A misspelled directive property says what you probably meant.
//!
//! PLAN-136 W7. The derived map knows every property each directive declares,
//! so it can answer "did you mean?" without a list — the same fact serving a
//! second question.
//!
//! WHAT THIS REPLACES. `src/diagnostics/suggestions.rs` carried
//! `CSS_PROPERTIES`: 98 hand-written property names, documented as "not
//! exhaustive", with no type information. Its only consumer was
//! `suggest_css_property`, and THAT had no consumers at all — a hardcoded list
//! maintained by hand for a function nobody called. Measured, not assumed:
//! `grep -rn suggest_css_property src/` returns only its own definition.
//!
//! So this wave deletes it rather than wiring it up. A list that is stale by
//! construction (it can never keep pace with CSS) and dead by measurement is
//! not a feature to preserve; the derived map answers the question that
//! actually gets asked, which is about SPACETIME properties — the ones no
//! upstream parser knows and where a typo therefore costs the most.

use spacetime::syntax::property_types::suggest_property;

/// The headline: a typo in a declared property is named.
#[test]
fn a_misspelled_property_is_corrected() {
    let s = suggest_property("fade-in-up", "easng");
    assert_eq!(
        s.as_deref(),
        Some("easing"),
        "@fade-in-up declares `easing`; `easng` is one deletion away"
    );
}

/// Suggestions are SCOPED to the directive, because the properties are. This is
/// the same reason the map is keyed by (directive, property): `@object` has a
/// `size` and `@fade-in-up` does not, and offering one inside the other would
/// be confidently wrong.
#[test]
fn suggestions_come_from_the_directive_that_was_used() {
    // `distance` belongs to @fade-in-up.
    assert_eq!(
        suggest_property("fade-in-up", "distanc").as_deref(),
        Some("distance")
    );
    // …and not to @object, which must not offer it.
    assert_ne!(
        suggest_property("object", "distanc").as_deref(),
        Some("distance"),
        "@object does not declare `distance`; suggesting it would send the \
         author to a property that does not exist there"
    );
}

/// Nonsense gets no suggestion. A did-you-mean that always fires is noise, and
/// noise trains authors to ignore the one that matters.
#[test]
fn an_unrecognizable_name_gets_no_suggestion() {
    assert_eq!(suggest_property("fade-in-up", "zzzzqqqq"), None);
}

/// An unknown directive has no properties to suggest from.
#[test]
fn an_unknown_directive_suggests_nothing() {
    assert_eq!(suggest_property("no-such-directive", "easing"), None);
}

/// An exact match is not a typo. Suggesting the word the author already wrote
/// would be absurd, and it is the kind of thing an untested edit-distance
/// helper does happily.
#[test]
fn an_exact_match_is_not_suggested() {
    assert_eq!(suggest_property("fade-in-up", "easing"), None);
}
