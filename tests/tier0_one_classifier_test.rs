//! FEAT-109 Tier 0 — four classifiers become one.
//!
//! Four subsystems each answer "what kind of value is this?" independently:
//! `src/sync/protocol.rs::classify_value`, `src/lsp/colors.rs`,
//! `src/analysis/visual_lint.rs::parse_css_color`, and the detector half of
//! `src/color/mod.rs`. Nobody chose that; each was written because at that point
//! in the code the type was not available — it had been discarded at capture.
//!
//! Tier 1 stopped the discard. This tier makes the consumers ask.
//!
//! The gates below are AGREEMENT gates. They do not assert any particular
//! classification — they assert that the subsystems cannot disagree, which is
//! the property four hand-rolled detectors can never have and one shared
//! grammar has by construction.

use spacetime::sync::protocol::{ControlHint, ValueType, classify_value};

fn is_colour(v: &ValueType) -> bool {
    matches!(v, ValueType::Color(_))
}

// ---------------------------------------------------------------------------
// The drift, as a gate. MEASURED before the change: sync tested
// `len() == 7 || len() == 4`, so an 8-digit or 4-digit hex was not a colour to
// it, while `hex_color` in stdlib/capture-types/css-values.st accepts 3/4/6/8
// and the LSP accepts all four lengths. Three detectors, three answers.
// ---------------------------------------------------------------------------

#[test]
fn every_hex_length_the_grammar_accepts_is_a_colour() {
    for hex in ["#abc", "#abcd", "#e8eef7", "#e8eef7ff"] {
        let (vtype, hint) = classify_value(hex);
        assert!(
            is_colour(&vtype),
            "{hex} did not classify as a colour: {vtype:?}"
        );
        assert!(
            matches!(hint, ControlHint::ColorPicker),
            "{hex} did not get a colour picker: {hint:?}"
        );
    }
}

#[test]
fn a_hex_the_grammar_refuses_is_not_a_colour() {
    // 5 and 7 digits are not legal hex colours. The old scanner accepted any
    // `#`-prefixed run of length 4 or 7 without checking the digits at all, so
    // `#zzz` was a colour and `#abcde` was not — for the same wrong reason.
    for bad in ["#abcde", "#e8eef7f", "#zzz", "#"] {
        let (vtype, _) = classify_value(bad);
        assert!(
            !is_colour(&vtype),
            "{bad} classified as a colour but is not one: {vtype:?}"
        );
    }
}

#[test]
fn modern_colour_syntax_is_recognised() {
    // The old scanner tested `starts_with("rgb") || starts_with("hsl")`, so
    // every colour syntax CSS gained after 2019 was invisible to the admin.
    for c in [
        "oklch(70% 0.1 200)",
        "oklab(59% 0.1 0.1)",
        "color-mix(in oklch, #fff, #000)",
        "lab(50% 40 59)",
        "hwb(194 0% 0%)",
    ] {
        let (vtype, hint) = classify_value(c);
        assert!(is_colour(&vtype), "{c} was not a colour: {vtype:?}");
        assert!(
            matches!(hint, ControlHint::ColorPicker),
            "{c} got {hint:?}, not a colour picker"
        );
    }
}

// ---------------------------------------------------------------------------
// Agreement. The point of Tier 0 is not any single answer — it is that one
// grammar now decides, so the subsystems CANNOT drift apart again.
// ---------------------------------------------------------------------------

#[test]
fn sync_agrees_with_the_inferrer_on_every_probe() {
    use spacetime::types::value_infer::{Inferred, infer_value_type};

    let probes = [
        "#abc",
        "#abcd",
        "#e8eef7",
        "#e8eef7ff",
        "oklch(70% 0.1 200)",
        "rgb(1 2 3)",
        "currentColor",
        "8px",
        "1.5rem",
        "100%",
        "600ms",
        "2s",
        "45deg",
        "ease-out",
        "cubic-bezier(.4,0,.2,1)",
        "linear",
        "600",
        "3.14",
        "calc(100% - 20px)",
        "var(--my-color)",
        "garbage ~~~ nonsense",
    ];

    for p in probes {
        let (vtype, _) = classify_value(p);
        let inferred = infer_value_type(p);

        // The one property that matters: sync says "colour" exactly when the
        // grammar does. Everything else is a presentation choice.
        let sync_says_colour = is_colour(&vtype);
        let grammar_says_colour = matches!(&inferred, Inferred::Scalar(s) if s == "color");
        assert_eq!(
            sync_says_colour, grammar_says_colour,
            "{p:?}: sync said colour={sync_says_colour}, grammar said colour={grammar_says_colour} ({inferred:?})"
        );
    }
}

// ---------------------------------------------------------------------------
// The existing contract must not move. These mirror the 15 unit tests already
// in src/sync/protocol.rs; they are restated here because a delegation that
// silently changes an answer is a regression wearing a refactor's clothes.
// ---------------------------------------------------------------------------

#[test]
fn the_existing_contract_is_unchanged() {
    let cases: &[(&str, &str)] = &[
        ("#ff0000", "color"),
        ("rgb(255, 0, 0)", "color"),
        ("1200ms", "duration"),
        ("1.5s", "duration"),
        ("40px", "dimension"),
        ("2.5em", "dimension"),
        ("50%", "dimension"),
        ("ease-out", "easing"),
        ("cubic-bezier(0.25, 0.1, 0.25, 1)", "easing"),
        ("linear", "easing"),
        ("42", "number"),
        ("3.14", "number"),
        ("some-identifier", "unknown"),  // abstention, not a string claim
        ("", "unknown"),
        ("   ", "unknown"),
        ("calc(100% - 20px)", "unknown"),
        ("var(--my-color)", "unknown"),
    ];

    for (input, want) in cases {
        let (vtype, _) = classify_value(input);
        let got = match vtype {
            ValueType::Color(_) => "color",
            ValueType::Duration { .. } => "duration",
            ValueType::Dimension { .. } => "dimension",
            ValueType::Easing(_) => "easing",
            ValueType::Number(_) => "number",
            ValueType::Str(_) | ValueType::Identifier(_) => "string",
            ValueType::Unknown(_) => "unknown",
        };
        assert_eq!(got, *want, "{input:?} classified as {got}, expected {want}");
    }
}

#[test]
fn durations_still_carry_milliseconds() {
    match classify_value("1200ms").0 {
        ValueType::Duration { ms } => assert_eq!(ms, 1200.0),
        other => panic!("expected Duration, got {other:?}"),
    }
    match classify_value("1.5s").0 {
        ValueType::Duration { ms } => assert_eq!(ms, 1500.0),
        other => panic!("expected Duration, got {other:?}"),
    }
}

#[test]
fn dimensions_still_carry_value_and_unit() {
    match classify_value("40px").0 {
        ValueType::Dimension { value, unit } => {
            assert_eq!(value, 40.0);
            assert_eq!(unit, "px");
        }
        other => panic!("expected Dimension, got {other:?}"),
    }
    match classify_value("50%").0 {
        ValueType::Dimension { value, unit } => {
            assert_eq!(value, 50.0);
            assert_eq!(unit, "%");
        }
        other => panic!("expected Dimension, got {other:?}"),
    }
}
