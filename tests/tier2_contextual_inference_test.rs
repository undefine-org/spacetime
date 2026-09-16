//! FEAT-109 W3 / Tier 2 — inference in CONTEXT, and the unit problem.
//!
//! Recognition is local: it reads a literal and names its shape. It therefore
//! cannot know that `600` in a `duration:` slot means 600 milliseconds, because
//! nothing in `600` says so. That is not a defect in recognition — it is the
//! boundary where CONTEXT takes over, and the context already exists: the form
//! declared `$duration:duration` before the value was ever read.
//!
//! This wave threads the declared type into inference so an ambiguous or bare
//! answer can be NARROWED by the slot. Narrowing only, never widening:
//!
//!   `600` in a `duration` slot   -> a time. The SLOT said so.
//!   `600` with no slot            -> a number. Nothing said otherwise.
//!   `#FF0020` in a `duration` slot -> NOT a time. The slot is not a licence to
//!                                     reinterpret a value that already has an
//!                                     unambiguous shape.
//!
//! That last rule is the one with a body count. A blanket `number <= time`
//! widening once made every staggered animation in the corpus 1000x too slow,
//! across twenty files, silently, with a fully green build.

use spacetime::types::value_infer::{Inferred, infer_value_type, infer_value_type_in_context};

// ---------------------------------------------------------------------------
// Narrowing: the slot answers what the literal cannot.
// ---------------------------------------------------------------------------

#[test]
fn a_bare_number_in_a_duration_slot_is_a_duration() {
    assert_eq!(
        infer_value_type_in_context("600", Some("duration")),
        Inferred::Scalar("duration".into())
    );
}

#[test]
fn a_bare_number_with_no_slot_is_still_a_number() {
    // The stagger incident, as a gate. Without a slot there is no licence.
    assert_eq!(
        infer_value_type_in_context("600", None),
        Inferred::Scalar("number".into())
    );
    assert_eq!(infer_value_type("600"), Inferred::Scalar("number".into()));
}

#[test]
fn a_bare_number_in_a_number_slot_is_a_number() {
    assert_eq!(
        infer_value_type_in_context("600", Some("number")),
        Inferred::Scalar("number".into())
    );
}

#[test]
fn an_ambiguous_value_is_narrowed_by_its_slot() {
    // `100%` genuinely is both a length and a percentage — the length grammar
    // includes percentage by design. Unresolvable locally; trivial with a slot.
    assert!(matches!(
        infer_value_type("100%"),
        Inferred::Ambiguous(_)
    ));
    assert_eq!(
        infer_value_type_in_context("100%", Some("length")),
        Inferred::Scalar("length".into())
    );
    assert_eq!(
        infer_value_type_in_context("100%", Some("percentage")),
        Inferred::Scalar("percentage".into())
    );
}

#[test]
fn a_bare_identifier_is_narrowed_by_its_slot() {
    // `chartreuse` ties across the shape-scalars with no meaning-scalar claiming
    // it. A `string` slot resolves that tie honestly.
    assert_eq!(
        infer_value_type_in_context("chartreuse", Some("string")),
        Inferred::Scalar("string".into())
    );
}

// ---------------------------------------------------------------------------
// NEVER widening. A slot narrows a set that already contains the answer; it can
// never overrule a literal whose shape is unambiguous.
// ---------------------------------------------------------------------------

#[test]
fn a_colour_in_a_duration_slot_is_still_a_colour() {
    // This is a TYPE ERROR the caller should report — not an invitation to
    // reinterpret `#FF0020` as a time because the slot wanted one.
    assert_eq!(
        infer_value_type_in_context("#FF0020", Some("duration")),
        Inferred::Scalar("color".into())
    );
}

#[test]
fn a_duration_in_a_number_slot_is_still_a_duration() {
    assert_eq!(
        infer_value_type_in_context("600ms", Some("number")),
        Inferred::Scalar("duration".into())
    );
}

#[test]
fn a_slot_cannot_conjure_a_type_from_nonsense() {
    assert_eq!(
        infer_value_type_in_context("garbage ~~~ nonsense", Some("color")),
        Inferred::Unknown
    );
    assert_eq!(
        infer_value_type_in_context("1px solid red", Some("color")),
        Inferred::Unknown
    );
}

#[test]
fn a_reference_is_a_reference_whatever_the_slot_says() {
    for r in ["--ink", "var(--ink)", "$brand.ink"] {
        assert_eq!(
            infer_value_type_in_context(r, Some("color")),
            Inferred::Reference,
            "{r} was reinterpreted by its slot"
        );
    }
}

#[test]
fn an_unknown_slot_name_changes_nothing() {
    // A slot naming a type the language does not have must not silently license
    // anything. It degrades to context-free inference.
    assert_eq!(
        infer_value_type_in_context("600", Some("notatype")),
        Inferred::Scalar("number".into())
    );
    assert_eq!(
        infer_value_type_in_context("#FF0020", Some("notatype")),
        Inferred::Scalar("color".into())
    );
}

#[test]
fn context_free_inference_is_unchanged_by_this_wave() {
    // Every existing caller passes no context. `infer_value_type` must remain
    // exactly what it was — this wave adds a door, it does not move the house.
    for probe in [
        "#FF0020", "8px", "600ms", "45deg", "600", "--ink", "1px solid red",
        "garbage ~~~ nonsense", "",
    ] {
        assert_eq!(
            infer_value_type(probe),
            infer_value_type_in_context(probe, None),
            "{probe:?} differs with and without an explicit None context"
        );
    }
}

// ---------------------------------------------------------------------------
// The two gates below were added AFTER mutation testing showed the mechanisms
// they cover were unproven: deleting `slot_supplies_a_unit` (so every slot
// widens) and deleting the unknown-slot guard both left the suite fully green.
// A mechanism no test can see is not yet load-bearing, however correct it reads.
// ---------------------------------------------------------------------------

#[test]
fn a_unitless_slot_does_not_absorb_a_bare_number() {
    // `color` and `string` are real scalars, so the unknown-slot guard does not
    // catch them — only `slot_supplies_a_unit` stands between `600` and being
    // called a colour. A slot may supply a UNIT the spelling omits; it may not
    // supply a KIND the value contradicts.
    assert_eq!(
        infer_value_type_in_context("600", Some("color")),
        Inferred::Scalar("number".into()),
        "a colour slot must not swallow a bare number"
    );
    assert_eq!(
        infer_value_type_in_context("600", Some("easing")),
        Inferred::Scalar("number".into()),
        "an easing slot must not swallow a bare number"
    );
    assert_eq!(
        infer_value_type_in_context("600", Some("string")),
        Inferred::Scalar("number".into()),
        "a string slot must not swallow a bare number"
    );
}

#[test]
fn an_unknown_slot_cannot_widen_a_bare_number() {
    // Without the `inferrable_scalars` guard, a typo'd or invented slot name
    // reaches the widening branch. `slot_supplies_a_unit` returns false for a
    // name with no scalar row, so the number survives — but only because BOTH
    // checks exist. This pins the outer one directly.
    for bogus in ["notatype", "durationn", "Color", ""] {
        assert_eq!(
            infer_value_type_in_context("600", Some(bogus)),
            Inferred::Scalar("number".into()),
            "slot {bogus:?} changed a bare number"
        );
        assert_eq!(
            infer_value_type_in_context("100%", Some(bogus)),
            infer_value_type("100%"),
            "slot {bogus:?} narrowed a genuine ambiguity"
        );
    }
}
