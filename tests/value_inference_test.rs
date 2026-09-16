//! FEAT-109 W0 — value-level type inference, recognition polarity.
//!
//! These gates were written BEFORE the implementation and seeded from a measured
//! probe (tests/infer_feasibility_probe.rs), which showed that naively reversing
//! the `%scalar_type` -> `%capture` join answers AMBIGUOUS for 17 of 19 sample
//! literals and returns `date` as the unique answer for `"garbage ~~~ nonsense"`.
//!
//! The cause is polarity. `capture_type_accepts` defaults to ACCEPT when it does
//! not know, because it answers "should I refuse this?" and a false refusal
//! blocks a build. Inference answers "what is this?" and must default to
//! ABSTAIN, because a false type is worse than no type. Same grammars, opposite
//! default. See FEAT-109's 2026-08-11 entry.
//!
//! Every test here is a BEHAVIOUR claim about the inferrer, not a claim about
//! emitted strings.

use spacetime::types::value_infer::{Inferred, infer_value_type};

// ---------------------------------------------------------------------------
// The core claim: unambiguous literals infer, with no annotation anywhere.
// ---------------------------------------------------------------------------

#[test]
fn a_hex_literal_infers_as_a_colour() {
    assert_eq!(infer_value_type("#FF0020"), Inferred::Scalar("color".into()));
    assert_eq!(infer_value_type("#e8eef7"), Inferred::Scalar("color".into()));
}

#[test]
fn a_colour_function_infers_as_a_colour() {
    assert_eq!(
        infer_value_type("oklch(70% 0.1 200)"),
        Inferred::Scalar("color".into())
    );
    assert_eq!(
        infer_value_type("rgb(1 2 3)"),
        Inferred::Scalar("color".into())
    );
}

#[test]
fn a_dimension_infers_as_a_length() {
    assert_eq!(infer_value_type("8px"), Inferred::Scalar("length".into()));
    assert_eq!(infer_value_type("1.5rem"), Inferred::Scalar("length".into()));
}

#[test]
fn a_time_literal_infers_as_a_duration() {
    assert_eq!(
        infer_value_type("600ms"),
        Inferred::Scalar("duration".into())
    );
    assert_eq!(infer_value_type("2s"), Inferred::Scalar("duration".into()));
}

#[test]
fn an_angle_infers_as_an_angle() {
    assert_eq!(infer_value_type("45deg"), Inferred::Scalar("angle".into()));
}

// ---------------------------------------------------------------------------
// The polarity claim. These are the measured WRONG answers from the probe: the
// naive reverse join reports `date` as the UNIQUE type of both of these, because
// `date` names no production and "unknown -> accept" applies. An inferrer that
// answers anything but Unknown here has the refusal polarity, and this is the
// sharpest gate in the file.
// ---------------------------------------------------------------------------

#[test]
fn nonsense_infers_as_nothing_at_all() {
    assert_eq!(infer_value_type("garbage ~~~ nonsense"), Inferred::Unknown);
}

#[test]
fn a_compound_value_is_not_a_scalar() {
    // `1px solid red` is a valid CSS shorthand and NOT a scalar of any kind.
    // The naive join answered `date`.
    assert_eq!(infer_value_type("1px solid red"), Inferred::Unknown);
}

#[test]
fn a_scalar_naming_no_production_never_wins() {
    // `date`, `url`, `string` have no `%capture_type` in stdlib, so
    // `capture_type_accepts` returns true for them on ANY input. Inference must
    // never report a scalar it cannot positively match.
    for probe in ["#FF0020", "8px", "600ms", "45deg", "garbage ~~~ nonsense"] {
        let got = infer_value_type(probe);
        assert_ne!(
            got,
            Inferred::Scalar("date".into()),
            "{probe:?} inferred as `date` — the accept-everything row won"
        );
        assert_ne!(
            got,
            Inferred::Scalar("url".into()),
            "{probe:?} inferred as `url` — the accept-everything row won"
        );
    }
}

#[test]
fn the_empty_value_infers_as_nothing() {
    assert_eq!(infer_value_type(""), Inferred::Unknown);
    assert_eq!(infer_value_type("   "), Inferred::Unknown);
}

// ---------------------------------------------------------------------------
// Specificity. `string` accepts everything that is text, so it must lose every
// contest it enters — otherwise every keyword in the language infers as a
// string and the feature is worthless.
// ---------------------------------------------------------------------------

#[test]
fn a_contextual_colour_keyword_prefers_colour_over_string() {
    // `currentColor` and `transparent` ARE in the colour grammar.
    assert_eq!(
        infer_value_type("currentColor"),
        Inferred::Scalar("color".into())
    );
    assert_eq!(
        infer_value_type("transparent"),
        Inferred::Scalar("color".into())
    );
}

#[test]
fn a_named_colour_does_not_infer_and_that_is_deliberate() {
    // The 148 named colours are ABSENT from the colour grammar on purpose
    // (stdlib/capture-types/css-values.st): a bare identifier in a value
    // position is ambiguous with every other keyword CSS defines, so admitting
    // them would make the grammar claim `bold` and `hidden` are colours.
    //
    // So `chartreuse` is Unknown, and this test exists to pin that as a KNOWN
    // COST rather than let it read as a gap. It is also the sharpest statement
    // of the module's polarity: abstaining is a legitimate answer, and this is
    // the case where abstaining is strictly better than the alternative.
    //
    // If named colours are ever wanted, they must arrive as a production the
    // grammar can disambiguate (property-contextual, per PLAN-122 tier (c)) —
    // never as a Rust list, and never by loosening `color_core`.
    // It does NOT come back Unknown, and that is the honest answer rather than a
    // near-miss: `chartreuse` really is a bare identifier, so the shape-scalars
    // match it and no meaning-scalar claims it. The gate is that `color` is
    // absent — the inferrer must not invent a colour from a name it has no
    // grammar for.
    for named in ["chartreuse", "rebeccapurple"] {
        match infer_value_type(named) {
            Inferred::Ambiguous(set) => assert!(
                !set.contains(&"color".to_string()),
                "{named:?} inferred as a colour, but named colours are not in the grammar: {set:?}"
            ),
            Inferred::Unknown => {}
            other => panic!("{named:?} inferred as {other:?}; expected no colour claim"),
        }
    }
}

#[test]
fn an_easing_keyword_prefers_easing_over_string() {
    assert_eq!(
        infer_value_type("ease-out"),
        Inferred::Scalar("easing".into())
    );
    assert_eq!(
        infer_value_type("cubic-bezier(.4,0,.2,1)"),
        Inferred::Scalar("easing".into())
    );
}

// ---------------------------------------------------------------------------
// References are their OWN answer, not a failure. Resolution takes over from
// here — see FEAT-109 consequence 3.
// ---------------------------------------------------------------------------

#[test]
fn a_token_reference_infers_as_a_reference() {
    assert_eq!(infer_value_type("--ink"), Inferred::Reference);
    assert_eq!(infer_value_type("var(--ink)"), Inferred::Reference);
    assert_eq!(infer_value_type("var(--ink, #fff)"), Inferred::Reference);
}

#[test]
fn a_binding_infers_as_a_reference() {
    assert_eq!(infer_value_type("$brand.ink"), Inferred::Reference);
}

#[test]
fn a_reference_is_never_reported_as_a_scalar() {
    // The probe measured `--ink` accepted by ALL THIRTEEN scalars, because a
    // scalar deliberately accepts a reference to itself. Under recognition
    // polarity that must collapse to one answer: Reference.
    for probe in ["--ink", "var(--ink)", "$brand.ink"] {
        match infer_value_type(probe) {
            Inferred::Reference => {}
            other => panic!("{probe:?} inferred as {other:?}, expected Reference"),
        }
    }
}

// ---------------------------------------------------------------------------
// Real ambiguity is REPORTED, never guessed. A bare number is the case that
// silently made every stagger 1000x too slow when something decided a number
// could be a time (PLAN-122 C7). The inferrer must refuse to pick.
// ---------------------------------------------------------------------------

#[test]
fn a_bare_number_infers_as_a_number_and_not_a_time() {
    // `600` is a number. It is NOT a duration — the unit belongs to the
    // declaration site, and inferring time here is the stagger incident.
    assert_eq!(infer_value_type("600"), Inferred::Scalar("number".into()));
    assert_eq!(infer_value_type("0"), Inferred::Scalar("number".into()));
}

#[test]
fn a_percentage_is_reported_as_ambiguous_between_its_two_true_types() {
    // `100%` really is both a length and a percentage — the length grammar
    // includes percentage by design. This is the one place the answer is
    // legitimately plural, and it must be VISIBLE rather than picked.
    match infer_value_type("100%") {
        Inferred::Ambiguous(set) => {
            assert!(set.contains(&"length".to_string()), "got {set:?}");
            assert!(set.contains(&"percentage".to_string()), "got {set:?}");
        }
        other => panic!("expected Ambiguous for `100%`, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// The table drives it. A scalar added to stdlib must be inferable with no Rust
// change — that is the acceptance test for the whole approach (PLAN-122 W4).
// ---------------------------------------------------------------------------

#[test]
fn inference_covers_every_scalar_that_names_a_real_production() {
    // Not a claim about any specific scalar: a claim that the inferrer's domain
    // IS the table, so nothing is hand-listed in Rust.
    let reg = spacetime::compiler::cached_stdlib_registry().0;
    let with_production: Vec<String> = reg
        .scalar_types()
        .filter(|r| !r.capture.is_empty())
        .map(|r| r.id.clone())
        .collect();
    assert!(
        with_production.len() >= 8,
        "expected the scalar table to be loaded, got {with_production:?}"
    );

    let inferrable = spacetime::types::value_infer::inferrable_scalars();
    for id in &with_production {
        // A scalar whose named production does not exist cannot be inferred, and
        // that is correct — but it must be EXCLUDED explicitly, not silently
        // winning every contest.
        if !inferrable.contains(id) {
            let row = reg.get_scalar_type(id).unwrap();
            assert!(
                !spacetime::types::value_infer::production_exists(&row.capture),
                "scalar `{id}` names a real production `{}` but is not inferrable",
                row.capture
            );
        }
    }
}

// ---------------------------------------------------------------------------
// FEAT-109 W5 — precedence is DATA. The `%loses_to` clause on a `%scalar_type`
// row replaced a `const LOSES_TO` table in Rust, so a scalar's place in the
// ordering is declared where the scalar is declared.
// ---------------------------------------------------------------------------

#[test]
fn precedence_comes_from_the_scalar_table_not_from_rust() {
    let reg = spacetime::compiler::cached_stdlib_registry().0;

    // The shape-scalars must declare that they lose to the meaning-scalars. If
    // this is empty, precedence has silently moved back into Rust or the clause
    // stopped parsing — and the symptom would be every keyword in the language
    // coming back ambiguous with `string`.
    let string_row = reg.get_scalar_type("string").expect("a string scalar");
    assert!(
        string_row.loses_to.contains(&"color".to_string()),
        "`string` must declare it loses to `color`, got {:?}",
        string_row.loses_to
    );
    assert!(
        string_row.loses_to.contains(&"easing".to_string()),
        "`string` must declare it loses to `easing`, got {:?}",
        string_row.loses_to
    );

    // And the behaviour that depends on it.
    assert_eq!(
        infer_value_type("currentColor"),
        Inferred::Scalar("color".into())
    );
}

#[test]
fn every_declared_precedence_names_a_real_scalar() {
    // A `%loses_to` naming a scalar that does not exist is a silent no-op: the
    // pair simply never matches, and the symptom is an ambiguity nobody can
    // explain. Cheap to assert, impossible to notice otherwise.
    let reg = spacetime::compiler::cached_stdlib_registry().0;
    let known: Vec<String> = reg.scalar_types().map(|r| r.id.clone()).collect();

    for row in reg.scalar_types() {
        for winner in &row.loses_to {
            assert!(
                known.contains(winner),
                "`{}` declares it loses to `{winner}`, which is not a scalar",
                row.id
            );
            assert_ne!(
                &row.id, winner,
                "`{}` declares it loses to itself",
                row.id
            );
        }
    }
}
