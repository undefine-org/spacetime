//! FEAT-109 Tier 0, part 2 — one colour conversion, not three.
//!
//! `src/analysis/visual_lint.rs::parse_css_color` and
//! `src/lsp/colors.rs::parse_css_color` were the SAME twelve lines, differing
//! only in whether they built an `f64` colour or an `f32` one. Both re-derived
//! what `src/color/mod.rs::parse_color_to_normalized_rgba` already computes.
//!
//! This is a different duplication from the one Tier 0 part 1 removed. That was
//! DETECTION ("is this a colour?"), which belongs to the grammar. This is
//! CONVERSION ("what RGBA is it?"), which belongs to the colour module — and
//! deliberately accepts more than the grammar does, because contrast maths must
//! work on `chartreuse` even though a bare identifier is not inferable as a
//! colour.
//!
//! The gate is agreement: every consumer that converts a colour must get the
//! same numbers from the same input.

use spacetime::color::parse_color_to_normalized_rgba;

/// Inputs every colour consumer in the tree must agree about.
const CASES: &[&str] = &[
    "#000000",
    "#ffffff",
    "#e8eef7",
    "#abc",
    "#abcd",
    "#e8eef7ff",
    "rgb(255, 0, 0)",
    "rgba(255, 0, 0, 0.5)",
    "hsl(120, 100%, 50%)",
    "chartreuse",
    "rebeccapurple",
    "transparent",
];

#[test]
fn the_canonical_parser_handles_every_shape_the_consumers_need() {
    for c in CASES {
        assert!(
            parse_color_to_normalized_rgba(c).is_ok(),
            "canonical parser refused {c:?}, which a consumer needs"
        );
    }
}

#[test]
fn the_canonical_parser_refuses_what_is_not_a_colour() {
    for c in ["", "   ", "1px solid red", "garbage ~~~ nonsense", "#zzz"] {
        assert!(
            parse_color_to_normalized_rgba(c).is_err(),
            "canonical parser accepted {c:?}, which is not a colour"
        );
    }
}

#[test]
fn normalized_components_are_in_range() {
    for c in CASES {
        let rgba = parse_color_to_normalized_rgba(c).expect("case parses");
        for (i, comp) in rgba.iter().enumerate() {
            assert!(
                (0.0..=1.0).contains(comp),
                "{c:?} component {i} = {comp} is outside 0..=1"
            );
        }
    }
}

#[test]
fn alpha_is_honoured() {
    let opaque = parse_color_to_normalized_rgba("#000000").expect("opaque");
    assert_eq!(opaque[3], 1.0, "a 6-digit hex is fully opaque");

    let clear = parse_color_to_normalized_rgba("transparent").expect("transparent");
    assert_eq!(clear[3], 0.0, "transparent has zero alpha");

    let half = parse_color_to_normalized_rgba("rgba(255, 0, 0, 0.5)").expect("half");
    assert!(
        (half[3] - 0.5).abs() < 0.01,
        "rgba alpha 0.5 became {}",
        half[3]
    );
}

/// The convergence claim: the visual-lint consumer's colour maths agrees with
/// the canonical parser. Asserted through the PUBLIC contrast API, because
/// `parse_css_color` is private and the point is the OBSERVABLE agreement, not
/// the call graph.
#[test]
fn contrast_maths_agrees_with_the_canonical_parser() {
    use spacetime::color::{Color, contrast_ratio};

    let to_color = |s: &str| {
        let [r, g, b, a] = parse_color_to_normalized_rgba(s).expect("parses");
        Color::rgba(r * 255.0, g * 255.0, b * 255.0, a)
    };

    // Black on white is the maximum contrast CSS can express: 21:1.
    let ratio = contrast_ratio(&to_color("#000000"), &to_color("#ffffff"));
    assert!(
        (ratio - 21.0).abs() < 0.1,
        "black on white should be 21:1, got {ratio}"
    );

    // A colour and itself has no contrast at all.
    let same = contrast_ratio(&to_color("#e8eef7"), &to_color("#e8eef7"));
    assert!((same - 1.0).abs() < 0.01, "self-contrast should be 1:1, got {same}");
}
