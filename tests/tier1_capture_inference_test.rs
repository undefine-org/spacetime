//! FEAT-109 Tier 1 — the keystone: stop discarding type evidence at capture.
//!
//! `convert_property_value` (src/syntax/events/form_compiler.rs) matches on the
//! DECLARED capture type and ends `_ => CapturedValue::Expr(...)`. A literal
//! whose type nobody wrote down therefore reifies to an opaque `Expr`, and every
//! consumer downstream — admin widgets, LSP swatches, value-level type checking
//! — has to guess the type back from the string. That is why four independent
//! colour detectors exist in this tree.
//!
//! The `CapturedValue` enum ALREADY has `Color`, `Time` and `Length` variants.
//! They are simply unreachable without a declaration. Tier 1 makes them
//! reachable by asking the FEAT-109 inferrer when — and only when — the declared
//! type does not answer.
//!
//! These gates are written BEFORE the change and must be seen to fail.

use spacetime::syntax::events::convert_property_value_public as convert;
use spacetime::syntax::CapturedValue;
use spacetime::parser::meta_ast::CaptureType;

// ---------------------------------------------------------------------------
// The keystone claim: an UNDECLARED literal keeps its type.
// ---------------------------------------------------------------------------

#[test]
fn an_undeclared_hex_literal_captures_as_a_colour() {
    // `Custom("value")` stands for "some capture whose type says nothing about
    // the value's shape" — the overwhelmingly common case in the corpus, and the
    // one that falls to `_ => Expr` today.
    let got = convert(&CaptureType::Custom("value".into()), "#FF0020");
    assert_eq!(got, CapturedValue::Color("#FF0020".into()));
}

#[test]
fn an_undeclared_colour_function_captures_as_a_colour() {
    let got = convert(&CaptureType::Custom("value".into()), "oklch(70% 0.1 200)");
    assert_eq!(got, CapturedValue::Color("oklch(70% 0.1 200)".into()));
}

#[test]
fn an_undeclared_duration_captures_as_a_time() {
    // `600ms` is 600 milliseconds. The Time variant carries ms, matching the
    // declared path and the param-default path (both emit Time(ms)).
    let got = convert(&CaptureType::Custom("value".into()), "600ms");
    assert_eq!(got, CapturedValue::Time(600));
}

#[test]
fn an_undeclared_second_duration_converts_to_milliseconds() {
    let got = convert(&CaptureType::Custom("value".into()), "2s");
    assert_eq!(got, CapturedValue::Time(2000));
}

// ---------------------------------------------------------------------------
// The safety claim. Inference must NEVER override a declaration, and must never
// fire where the declared type already has an answer — otherwise Tier 1 is a
// silent re-typing of the whole corpus rather than a gap-filler.
// ---------------------------------------------------------------------------

#[test]
fn a_declared_string_is_never_re_inferred() {
    // A `string` capture holding something colour-shaped stays a String. The
    // author declared what they wanted; inference does not second-guess it.
    let got = convert(&CaptureType::String, "#FF0020");
    assert_eq!(got, CapturedValue::String("#FF0020".into()));
}

#[test]
fn a_declared_ident_is_never_re_inferred() {
    let got = convert(&CaptureType::Ident, "ease-out");
    assert_eq!(got, CapturedValue::Ident("ease-out".into()));
}

#[test]
fn a_declared_number_still_wins_over_inference() {
    let got = convert(&CaptureType::Number, "600");
    assert_eq!(got, CapturedValue::Number(600.0));
}

// ---------------------------------------------------------------------------
// The abstention claim. Everything the inferrer cannot name must reach `Expr`
// exactly as it does today — this is what keeps Tier 1 a strict addition.
// ---------------------------------------------------------------------------

#[test]
fn a_compound_value_still_reaches_expr() {
    let got = convert(&CaptureType::Custom("value".into()), "1px solid red");
    assert_eq!(got, CapturedValue::Expr("1px solid red".into()));
}

#[test]
fn a_reference_still_reaches_expr() {
    // A reference is not a literal — its type is a resolution question. It must
    // NOT become a Color/Length/Time, and it must not change shape at all.
    for r in ["--ink", "var(--ink)", "$brand.ink"] {
        assert_eq!(
            convert(&CaptureType::Custom("value".into()), r),
            CapturedValue::Expr(r.to_string()),
            "{r:?} changed shape"
        );
    }
}

#[test]
fn nonsense_still_reaches_expr() {
    let got = convert(&CaptureType::Custom("value".into()), "garbage ~~~ nonsense");
    assert_eq!(got, CapturedValue::Expr("garbage ~~~ nonsense".into()));
}

#[test]
fn a_bare_number_does_not_become_a_time() {
    // The stagger incident, as a gate. `600` with no declared type is a number,
    // never a duration — the unit lives at the declaration site.
    let got = convert(&CaptureType::Custom("value".into()), "600");
    assert_ne!(got, CapturedValue::Time(600));
}

#[test]
fn an_ambiguous_value_reaches_expr_rather_than_being_picked() {
    // `chartreuse` ties between the shape-scalars and no meaning-scalar claims
    // it. A tie must NOT be broken here — Expr is the honest capture.
    let got = convert(&CaptureType::Custom("value".into()), "chartreuse");
    assert_eq!(got, CapturedValue::Expr("chartreuse".into()));
}

// ---------------------------------------------------------------------------
// FEAT-109 Tier 2 — the DECLARED slot supplies what the literal cannot.
// ---------------------------------------------------------------------------

#[test]
fn a_bare_number_in_a_duration_slot_captures_as_a_time() {
    // `$reveal:duration` with a value of `600` means 600 milliseconds. Nothing
    // in `600` says so; the declaration does. This is the unit problem, solved
    // where the information actually lives.
    let got = convert(&CaptureType::Custom("duration".into()), "600");
    assert_eq!(got, CapturedValue::Time(600));
}

#[test]
fn a_bare_number_in_an_unrelated_slot_is_not_a_time() {
    // A slot may supply a UNIT the spelling omits. It may not supply a KIND the
    // value contradicts.
    for slot in ["color", "easing", "string"] {
        let got = convert(&CaptureType::Custom(slot.into()), "600");
        assert_eq!(
            got,
            CapturedValue::Expr("600".into()),
            "slot {slot} re-typed a bare number"
        );
    }
}

#[test]
fn a_colour_in_a_duration_slot_stays_a_colour() {
    // A type error for a caller to report — never a value to reinterpret.
    let got = convert(&CaptureType::Custom("duration".into()), "#FF0020");
    assert_eq!(got, CapturedValue::Color("#FF0020".into()));
}
