//! FEAT-109 Tier 0, part 3 — the LSP swatch agrees with everyone else.
//!
//! W1b turned out smaller than planned, and the reason is worth recording.
//!
//! The plan assumed `src/lsp/colors.rs` held a hand-rolled colour RECOGNIZER to
//! delete. Reading it, the candidate scanner does not judge colours at all: it
//! finds `#`-runs and balanced `ident(...)` spans and hands the text to
//! `parse_css_color` for the verdict, deliberately keeping no list of function
//! names ("an unknown function simply fails to parse, which is one place fewer
//! for a list to go stale" — its own comment).
//!
//! So the duplication here was never the detection. It was the CONVERSION: this
//! file's `parse_css_color` was a verbatim twin of the one in
//! `src/analysis/visual_lint.rs`, and both bypassed the canonical parser in
//! `src/color/mod.rs`. W1c collapsed all three onto one, and these gates assert
//! the swatch behaviour that follows.
//!
//! The candidate SCAN stays, and should: the LSP runs on a live, frequently
//! invalid document where form matching may not produce a tree at all. Swapping
//! the scan for a compile would make swatches blink out mid-keystroke, which is
//! a regression dressed as a purification.

use spacetime::lsp::colors::provide_document_colors;
use spacetime::lsp::document::DocumentState;

fn swatch_count(src: &str) -> usize {
    let doc = DocumentState::new(src.to_string(), 0);
    provide_document_colors(&doc).len()
}

fn swatches(src: &str) -> Vec<(f32, f32, f32, f32)> {
    let doc = DocumentState::new(src.to_string(), 0);
    provide_document_colors(&doc)
        .into_iter()
        .map(|ci| (ci.color.red, ci.color.green, ci.color.blue, ci.color.alpha))
        .collect()
}

#[test]
fn every_hex_length_gets_a_swatch() {
    // 3/4/6/8 are the lengths the grammar accepts. Before Tier 0, the sync layer
    // recognised only 3 and 6 — this asserts the LSP does not have that gap.
    let src = "card { color: #abc; background: #abcd; border-color: #e8eef7; outline-color: #e8eef7ff; }";
    assert_eq!(swatch_count(src), 4, "expected a swatch per hex: {src}");
}

#[test]
fn modern_colour_functions_get_swatches() {
    for c in [
        "oklch(70% 0.1 200)",
        "oklab(59% 0.1 0.1)",
        "lab(50% 40 59)",
        "hwb(194 0% 0%)",
        "color-mix(in oklch, #fff, #000)",
    ] {
        let src = format!("card {{ color: {c}; }}");
        assert_eq!(swatch_count(&src), 1, "no swatch for {c}");
    }
}

#[test]
fn a_named_colour_gets_no_swatch_and_that_is_deliberate() {
    // The file's own comment: a bare word is as likely to be a class name or
    // prose as a colour, and a swatch on prose misleads. Hex and function forms
    // are unambiguous; identifiers are not.
    assert_eq!(swatch_count("card { color: chartreuse; }"), 0);
}

#[test]
fn prose_never_gets_a_swatch() {
    // gh-33. A colour-shaped token inside a comment or a string is text.
    assert_eq!(swatch_count("// use the #abc hex here"), 0);
    assert_eq!(swatch_count("card { content: \"#abc\"; }"), 0);
}

#[test]
fn a_swatch_carries_the_right_colour() {
    let got = swatches("card { color: #ff0000; }");
    assert_eq!(got.len(), 1);
    let (r, g, b, a) = got[0];
    assert!((r - 1.0).abs() < 0.01, "red channel was {r}");
    assert!(g.abs() < 0.01, "green channel was {g}");
    assert!(b.abs() < 0.01, "blue channel was {b}");
    assert!((a - 1.0).abs() < 0.01, "alpha was {a}");
}

#[test]
fn an_eight_digit_hex_carries_its_alpha() {
    // `#ff000080` is red at ~50% alpha. The old `Color::from_hex` in the
    // canonical parser handled 3 and 6 digits only, so this shape could not be
    // converted there at all.
    let got = swatches("card { color: #ff000080; }");
    assert_eq!(got.len(), 1);
    let (_, _, _, a) = got[0];
    assert!(
        a > 0.4 && a < 0.6,
        "expected ~0.5 alpha from #ff000080, got {a}"
    );
}

/// The convergence claim, stated where it can be seen to break: what the LSP
/// paints and what the contrast linter computes come from one parser now.
#[test]
fn the_swatch_matches_the_canonical_parser() {
    for hex in ["#abc", "#abcd", "#e8eef7", "#e8eef7ff", "#ff000080"] {
        let painted = swatches(&format!("card {{ color: {hex}; }}"));
        assert_eq!(painted.len(), 1, "no swatch for {hex}");
        let (r, g, b, a) = painted[0];

        let [cr, cg, cb, ca] = spacetime::color::parse_color_to_normalized_rgba(hex)
            .unwrap_or_else(|e| panic!("canonical parser refused {hex}: {e}"));

        assert!((r as f64 - cr).abs() < 0.01, "{hex} red: lsp {r} vs canonical {cr}");
        assert!((g as f64 - cg).abs() < 0.01, "{hex} green: lsp {g} vs canonical {cg}");
        assert!((b as f64 - cb).abs() < 0.01, "{hex} blue: lsp {b} vs canonical {cb}");
        assert!((a as f64 - ca).abs() < 0.01, "{hex} alpha: lsp {a} vs canonical {ca}");
    }
}
