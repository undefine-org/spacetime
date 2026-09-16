//! Comprehensive tests for the color module

use super::*;

// ============================================================
// Parsing Tests
// ============================================================

#[test]
fn test_parse_hex_3_digit() {
    let color = Color::from_hex("#fff").unwrap();
    match color {
        Color::RGB { r, g, b, a } => {
            assert_eq!(r, 255.0);
            assert_eq!(g, 255.0);
            assert_eq!(b, 255.0);
            assert_eq!(a, 1.0);
        }
        _ => panic!("Expected RGB color"),
    }
}

#[test]
fn test_parse_hex_6_digit() {
    let color = Color::from_hex("#ff0000").unwrap();
    match color {
        Color::RGB { r, g, b, a } => {
            assert_eq!(r, 255.0);
            assert_eq!(g, 0.0);
            assert_eq!(b, 0.0);
            assert_eq!(a, 1.0);
        }
        _ => panic!("Expected RGB color"),
    }
}

#[test]
fn test_parse_hex_8_digit() {
    let color = Color::from_hex("#ff000080").unwrap();
    match color {
        Color::RGB { r, g, b, a } => {
            assert_eq!(r, 255.0);
            assert_eq!(g, 0.0);
            assert_eq!(b, 0.0);
            assert!((a - 0.5019607843137255).abs() < 0.01); // 128/255
        }
        _ => panic!("Expected RGB color"),
    }
}

#[test]
fn test_parse_hex_without_hash() {
    let color = Color::from_hex("0000ff").unwrap();
    match color {
        Color::RGB { r, g, b, .. } => {
            assert_eq!(r, 0.0);
            assert_eq!(g, 0.0);
            assert_eq!(b, 255.0);
        }
        _ => panic!("Expected RGB color"),
    }
}

#[test]
fn test_parse_rgb() {
    let color = Color::from_rgb_string("rgb(255, 128, 0)").unwrap();
    match color {
        Color::RGB { r, g, b, a } => {
            assert_eq!(r, 255.0);
            assert_eq!(g, 128.0);
            assert_eq!(b, 0.0);
            assert_eq!(a, 1.0);
        }
        _ => panic!("Expected RGB color"),
    }
}

#[test]
fn test_parse_rgba() {
    let color = Color::from_rgb_string("rgba(255, 128, 0, 0.5)").unwrap();
    match color {
        Color::RGB { r, g, b, a } => {
            assert_eq!(r, 255.0);
            assert_eq!(g, 128.0);
            assert_eq!(b, 0.0);
            assert_eq!(a, 0.5);
        }
        _ => panic!("Expected RGB color"),
    }
}

#[test]
fn test_parse_hsl() {
    let color = Color::from_hsl_string("hsl(120, 50%, 50%)").unwrap();
    match color {
        Color::HSL { h, s, l, a } => {
            assert_eq!(h, 120.0);
            assert_eq!(s, 50.0);
            assert_eq!(l, 50.0);
            assert_eq!(a, 1.0);
        }
        _ => panic!("Expected HSL color"),
    }
}

#[test]
fn test_parse_hsla() {
    let color = Color::from_hsl_string("hsla(240, 100%, 50%, 0.75)").unwrap();
    match color {
        Color::HSL { h, s, l, a } => {
            assert_eq!(h, 240.0);
            assert_eq!(s, 100.0);
            assert_eq!(l, 50.0);
            assert_eq!(a, 0.75);
        }
        _ => panic!("Expected HSL color"),
    }
}

#[test]
fn test_parse_oklch() {
    let color = Color::from_oklch_string("oklch(0.5, 0.1, 180)").unwrap();
    match color {
        Color::OKLCH { l, c, h, a } => {
            assert_eq!(l, 0.5);
            assert_eq!(c, 0.1);
            assert_eq!(h, 180.0);
            assert_eq!(a, 1.0);
        }
        _ => panic!("Expected OKLCH color"),
    }
}

#[test]
fn test_parse_oklch_with_alpha() {
    let color = Color::from_oklch_string("oklch(0.7, 0.15, 45, 0.9)").unwrap();
    match color {
        Color::OKLCH { l, c, h, a } => {
            assert_eq!(l, 0.7);
            assert_eq!(c, 0.15);
            assert_eq!(h, 45.0);
            assert_eq!(a, 0.9);
        }
        _ => panic!("Expected OKLCH color"),
    }
}

#[test]
fn test_parse_invalid_hex() {
    assert!(Color::from_hex("#gg0000").is_err());
    assert!(Color::from_hex("#12").is_err());
    assert!(Color::from_hex("#12345").is_err());
}

// ============================================================
// Color Space Conversion Tests
// ============================================================

#[test]
fn test_rgb_to_hsl_black() {
    let color = Color::rgb(0.0, 0.0, 0.0);
    let hsl = color.to_hsl();
    match hsl {
        Color::HSL { h: _, s, l, .. } => {
            assert_eq!(s, 0.0);
            assert_eq!(l, 0.0);
        }
        _ => panic!("Expected HSL color"),
    }
}

#[test]
fn test_rgb_to_hsl_white() {
    let color = Color::rgb(255.0, 255.0, 255.0);
    let hsl = color.to_hsl();
    match hsl {
        Color::HSL { h: _, s, l, .. } => {
            assert_eq!(s, 0.0);
            assert!((l - 100.0).abs() < 0.01);
        }
        _ => panic!("Expected HSL color"),
    }
}

#[test]
fn test_rgb_to_hsl_red() {
    let color = Color::rgb(255.0, 0.0, 0.0);
    let hsl = color.to_hsl();
    match hsl {
        Color::HSL { h, s, l, .. } => {
            assert!((h - 0.0).abs() < 0.01);
            assert!((s - 100.0).abs() < 0.01);
            assert!((l - 50.0).abs() < 0.01);
        }
        _ => panic!("Expected HSL color"),
    }
}

#[test]
fn test_rgb_to_hsl_green() {
    let color = Color::rgb(0.0, 255.0, 0.0);
    let hsl = color.to_hsl();
    match hsl {
        Color::HSL { h, s, l, .. } => {
            assert!((h - 120.0).abs() < 0.01);
            assert!((s - 100.0).abs() < 0.01);
            assert!((l - 50.0).abs() < 0.01);
        }
        _ => panic!("Expected HSL color"),
    }
}

#[test]
fn test_rgb_to_hsl_blue() {
    let color = Color::rgb(0.0, 0.0, 255.0);
    let hsl = color.to_hsl();
    match hsl {
        Color::HSL { h, s, l, .. } => {
            assert!((h - 240.0).abs() < 0.01);
            assert!((s - 100.0).abs() < 0.01);
            assert!((l - 50.0).abs() < 0.01);
        }
        _ => panic!("Expected HSL color"),
    }
}

#[test]
fn test_hsl_to_rgb_red() {
    let color = Color::hsl(0.0, 100.0, 50.0);
    let rgb = color.to_rgb();
    match rgb {
        Color::RGB { r, g, b, .. } => {
            assert!((r - 255.0).abs() < 1.0);
            assert!(g.abs() < 1.0);
            assert!(b.abs() < 1.0);
        }
        _ => panic!("Expected RGB color"),
    }
}

#[test]
fn test_hsl_to_rgb_green() {
    let color = Color::hsl(120.0, 100.0, 50.0);
    let rgb = color.to_rgb();
    match rgb {
        Color::RGB { r, g, b, .. } => {
            assert!(r.abs() < 1.0);
            assert!((g - 255.0).abs() < 1.0);
            assert!(b.abs() < 1.0);
        }
        _ => panic!("Expected RGB color"),
    }
}

#[test]
fn test_hsl_to_rgb_blue() {
    let color = Color::hsl(240.0, 100.0, 50.0);
    let rgb = color.to_rgb();
    match rgb {
        Color::RGB { r, g, b, .. } => {
            assert!(r.abs() < 1.0);
            assert!(g.abs() < 1.0);
            assert!((b - 255.0).abs() < 1.0);
        }
        _ => panic!("Expected RGB color"),
    }
}

#[test]
fn test_rgb_to_oklch_red() {
    let color = Color::rgb(255.0, 0.0, 0.0);
    let oklch = color.to_oklch();
    match oklch {
        Color::OKLCH { l, c, h, .. } => {
            // Red in OKLCH has a hue around 29 degrees
            assert!(l > 0.4 && l < 0.7, "Expected l around 0.6, got {}", l);
            assert!(c > 0.2, "Expected c > 0.2, got {}", c);
            assert!(h > 20.0 && h < 40.0, "Expected h around 29, got {}", h);
        }
        _ => panic!("Expected OKLCH color"),
    }
}

#[test]
fn test_oklch_to_rgb_basic() {
    let color = Color::oklch(0.5, 0.1, 180.0);
    let rgb = color.to_rgb();
    match rgb {
        Color::RGB { r, g, b, .. } => {
            // Verify it's within valid RGB range
            assert!(r >= 0.0 && r <= 255.0);
            assert!(g >= 0.0 && g <= 255.0);
            assert!(b >= 0.0 && b <= 255.0);
        }
        _ => panic!("Expected RGB color"),
    }
}

// ============================================================
// Round-trip Conversion Tests
// ============================================================

#[test]
fn test_rgb_to_hsl_to_rgb_roundtrip() {
    let original = Color::rgb(128.0, 64.0, 192.0);
    let hsl = original.to_hsl();
    let back_to_rgb = hsl.to_rgb();

    match (original, back_to_rgb) {
        (
            Color::RGB {
                r: r1,
                g: g1,
                b: b1,
                ..
            },
            Color::RGB {
                r: r2,
                g: g2,
                b: b2,
                ..
            },
        ) => {
            assert!((r1 - r2).abs() < 1.0, "Red mismatch: {} vs {}", r1, r2);
            assert!((g1 - g2).abs() < 1.0, "Green mismatch: {} vs {}", g1, g2);
            assert!((b1 - b2).abs() < 1.0, "Blue mismatch: {} vs {}", b1, b2);
        }
        _ => panic!("Expected RGB colors"),
    }
}

#[test]
fn test_rgb_to_oklch_to_rgb_roundtrip() {
    let original = Color::rgb(128.0, 64.0, 192.0);
    let oklch = original.to_oklch();
    let back_to_rgb = oklch.to_rgb();

    match (original, back_to_rgb) {
        (
            Color::RGB {
                r: r1,
                g: g1,
                b: b1,
                ..
            },
            Color::RGB {
                r: r2,
                g: g2,
                b: b2,
                ..
            },
        ) => {
            assert!((r1 - r2).abs() < 2.0, "Red mismatch: {} vs {}", r1, r2);
            assert!((g1 - g2).abs() < 2.0, "Green mismatch: {} vs {}", g1, g2);
            assert!((b1 - b2).abs() < 2.0, "Blue mismatch: {} vs {}", b1, b2);
        }
        _ => panic!("Expected RGB colors"),
    }
}

#[test]
fn test_hsl_to_rgb_to_hsl_roundtrip() {
    let original = Color::hsl(200.0, 75.0, 60.0);
    let rgb = original.to_rgb();
    let back_to_hsl = rgb.to_hsl();

    match (original, back_to_hsl) {
        (
            Color::HSL {
                h: h1,
                s: s1,
                l: l1,
                ..
            },
            Color::HSL {
                h: h2,
                s: s2,
                l: l2,
                ..
            },
        ) => {
            assert!((h1 - h2).abs() < 1.0, "Hue mismatch: {} vs {}", h1, h2);
            assert!(
                (s1 - s2).abs() < 1.0,
                "Saturation mismatch: {} vs {}",
                s1,
                s2
            );
            assert!(
                (l1 - l2).abs() < 1.0,
                "Lightness mismatch: {} vs {}",
                l1,
                l2
            );
        }
        _ => panic!("Expected HSL colors"),
    }
}

// ============================================================
// Interpolation Tests
// ============================================================

#[test]
fn test_interpolate_rgb_midpoint() {
    let red = Color::rgb(255.0, 0.0, 0.0);
    let blue = Color::rgb(0.0, 0.0, 255.0);
    let mid = Color::interpolate(&red, &blue, 0.5, ColorSpace::RGB);

    match mid {
        Color::RGB { r, g, b, .. } => {
            assert!((r - 127.5).abs() < 1.0);
            assert!(g.abs() < 1.0);
            assert!((b - 127.5).abs() < 1.0);
        }
        _ => panic!("Expected RGB color"),
    }
}

#[test]
fn test_interpolate_rgb_start() {
    let red = Color::rgb(255.0, 0.0, 0.0);
    let blue = Color::rgb(0.0, 0.0, 255.0);
    let start = Color::interpolate(&red, &blue, 0.0, ColorSpace::RGB);

    match start {
        Color::RGB { r, g, b, .. } => {
            assert_eq!(r, 255.0);
            assert_eq!(g, 0.0);
            assert_eq!(b, 0.0);
        }
        _ => panic!("Expected RGB color"),
    }
}

#[test]
fn test_interpolate_rgb_end() {
    let red = Color::rgb(255.0, 0.0, 0.0);
    let blue = Color::rgb(0.0, 0.0, 255.0);
    let end = Color::interpolate(&red, &blue, 1.0, ColorSpace::RGB);

    match end {
        Color::RGB { r, g, b, .. } => {
            assert_eq!(r, 0.0);
            assert_eq!(g, 0.0);
            assert_eq!(b, 255.0);
        }
        _ => panic!("Expected RGB color"),
    }
}

#[test]
fn test_interpolate_hsl_hue_shortest_path() {
    // From red (0°) to yellow (60°) - should go forward
    let red = Color::hsl(0.0, 100.0, 50.0);
    let yellow = Color::hsl(60.0, 100.0, 50.0);
    let mid = Color::interpolate(&red, &yellow, 0.5, ColorSpace::HSL);

    match mid {
        Color::HSL { h, .. } => {
            assert!((h - 30.0).abs() < 1.0, "Expected h=30, got {}", h);
        }
        _ => panic!("Expected HSL color"),
    }
}

#[test]
fn test_interpolate_hsl_hue_wraps_around() {
    // From red (0°) to magenta (300°) - should go backwards through 330°
    let red = Color::hsl(0.0, 100.0, 50.0);
    let magenta = Color::hsl(300.0, 100.0, 50.0);
    let mid = Color::interpolate(&red, &magenta, 0.5, ColorSpace::HSL);

    match mid {
        Color::HSL { h, .. } => {
            // Shortest path is backwards: 0 -> 330 (midpoint)
            assert!((h - 330.0).abs() < 1.0, "Expected h=330, got {}", h);
        }
        _ => panic!("Expected HSL color"),
    }
}

#[test]
fn test_interpolate_oklch_midpoint() {
    let color1 = Color::oklch(0.3, 0.1, 0.0);
    let color2 = Color::oklch(0.7, 0.2, 120.0);
    let mid = Color::interpolate(&color1, &color2, 0.5, ColorSpace::OKLCH);

    match mid {
        Color::OKLCH { l, c, h, .. } => {
            assert!((l - 0.5).abs() < 0.01, "Expected l=0.5, got {}", l);
            assert!((c - 0.15).abs() < 0.01, "Expected c=0.15, got {}", c);
            assert!((h - 60.0).abs() < 1.0, "Expected h=60, got {}", h);
        }
        _ => panic!("Expected OKLCH color"),
    }
}

#[test]
fn test_interpolate_alpha() {
    let c1 = Color::rgba(255.0, 0.0, 0.0, 0.2);
    let c2 = Color::rgba(0.0, 0.0, 255.0, 0.8);
    let mid = Color::interpolate(&c1, &c2, 0.5, ColorSpace::RGB);

    assert!((mid.alpha() - 0.5).abs() < 0.01);
}

#[test]
fn test_interpolate_different_color_spaces() {
    // Interpolate between HSL and RGB colors
    let hsl_color = Color::hsl(120.0, 100.0, 50.0);
    let rgb_color = Color::rgb(255.0, 0.0, 0.0);

    // Should work in any color space
    let mid_rgb = Color::interpolate(&hsl_color, &rgb_color, 0.5, ColorSpace::RGB);
    let mid_hsl = Color::interpolate(&hsl_color, &rgb_color, 0.5, ColorSpace::HSL);
    let mid_oklch = Color::interpolate(&hsl_color, &rgb_color, 0.5, ColorSpace::OKLCH);

    // All should produce valid colors
    assert!(mid_rgb.to_rgb().alpha() >= 0.0);
    assert!(mid_hsl.to_hsl().alpha() >= 0.0);
    assert!(mid_oklch.to_oklch().alpha() >= 0.0);
}

// ============================================================
// CSS Output Tests
// ============================================================

#[test]
fn test_to_css_rgb() {
    let color = Color::rgb(255.0, 128.0, 0.0);
    assert_eq!(color.to_css(), "rgb(255, 128, 0)");
}

#[test]
fn test_to_css_rgba() {
    let color = Color::rgba(255.0, 128.0, 0.0, 0.5);
    assert_eq!(color.to_css(), "rgba(255, 128, 0, 0.5)");
}

#[test]
fn test_to_css_hsl() {
    let color = Color::hsl(120.0, 50.0, 75.0);
    assert_eq!(color.to_css(), "hsl(120, 50%, 75%)");
}

#[test]
fn test_to_css_hsla() {
    let color = Color::hsla(240.0, 100.0, 50.0, 0.75);
    assert_eq!(color.to_css(), "hsla(240, 100%, 50%, 0.75)");
}

#[test]
fn test_to_css_oklch() {
    let color = Color::oklch(0.5, 0.1, 180.0);
    assert_eq!(color.to_css(), "oklch(0.5 0.1 180)");
}

#[test]
fn test_to_css_oklch_with_alpha() {
    let color = Color::oklcha(0.7, 0.15, 45.0, 0.9);
    assert_eq!(color.to_css(), "oklch(0.7 0.15 45 / 0.9)");
}

// ============================================================
// Edge Cases and Special Values
// ============================================================

#[test]
fn test_hue_interpolation_across_0_degrees() {
    // From 350° to 10° - should cross 0°
    let c1 = Color::hsl(350.0, 100.0, 50.0);
    let c2 = Color::hsl(10.0, 100.0, 50.0);
    let mid = Color::interpolate(&c1, &c2, 0.5, ColorSpace::HSL);

    match mid {
        Color::HSL { h, .. } => {
            // Midpoint should be 0° (or 360°)
            assert!(
                h.abs() < 1.0 || (h - 360.0).abs() < 1.0,
                "Expected h near 0°, got {}",
                h
            );
        }
        _ => panic!("Expected HSL color"),
    }
}

#[test]
fn test_grayscale_colors() {
    let gray = Color::rgb(128.0, 128.0, 128.0);
    let hsl = gray.to_hsl();
    let oklch = gray.to_oklch();

    // Should convert without errors
    match hsl {
        Color::HSL { s, .. } => {
            assert_eq!(s, 0.0, "Grayscale should have 0% saturation");
        }
        _ => panic!("Expected HSL color"),
    }

    match oklch {
        Color::OKLCH { c, .. } => {
            assert!(
                c < 0.01,
                "Grayscale should have near-zero chroma, got {}",
                c
            );
        }
        _ => panic!("Expected OKLCH color"),
    }
}

#[test]
fn test_alpha_channel_preservation() {
    let color = Color::rgba(255.0, 0.0, 0.0, 0.5);
    let hsl = color.to_hsl();
    let oklch = color.to_oklch();
    let back_to_rgb = hsl.to_rgb();

    assert_eq!(color.alpha(), 0.5);
    assert_eq!(hsl.alpha(), 0.5);
    assert_eq!(oklch.alpha(), 0.5);
    assert_eq!(back_to_rgb.alpha(), 0.5);
}

#[test]
fn test_interpolate_quarter_and_three_quarters() {
    let red = Color::rgb(255.0, 0.0, 0.0);
    let blue = Color::rgb(0.0, 0.0, 255.0);

    let quarter = Color::interpolate(&red, &blue, 0.25, ColorSpace::RGB);
    let three_quarters = Color::interpolate(&red, &blue, 0.75, ColorSpace::RGB);

    match (quarter, three_quarters) {
        (Color::RGB { r: r1, b: b1, .. }, Color::RGB { r: r2, b: b2, .. }) => {
            assert!((r1 - 191.25).abs() < 1.0);
            assert!((b1 - 63.75).abs() < 1.0);
            assert!((r2 - 63.75).abs() < 1.0);
            assert!((b2 - 191.25).abs() < 1.0);
        }
        _ => panic!("Expected RGB colors"),
    }
}

// ============================================================
// Normalized RGBA Tests
// ============================================================

#[test]
fn test_to_normalized_rgba() {
    let color = Color::rgb(255.0, 128.0, 0.0);
    let [r, g, b, a] = color.to_normalized_rgba();
    assert!((r - 1.0).abs() < 0.001);
    assert!((g - 128.0 / 255.0).abs() < 0.001);
    assert!((b - 0.0).abs() < 0.001);
    assert!((a - 1.0).abs() < 0.001);
}

#[test]
fn test_parse_color_to_normalized_rgba_transparent() {
    let result = parse_color_to_normalized_rgba("transparent").unwrap();
    assert_eq!(result, [0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn test_parse_color_to_normalized_rgba_hex() {
    let [r, g, b, a] = parse_color_to_normalized_rgba("#E85D4A").unwrap();
    assert!((r - 232.0 / 255.0).abs() < 0.001);
    assert!((g - 93.0 / 255.0).abs() < 0.001);
    assert!((b - 74.0 / 255.0).abs() < 0.001);
    assert!((a - 1.0).abs() < 0.001);
}

#[test]
fn test_parse_color_to_normalized_rgba_hex3() {
    let [r, g, b, a] = parse_color_to_normalized_rgba("#F80").unwrap();
    assert!((r - 1.0).abs() < 0.001);
    assert!((g - 136.0 / 255.0).abs() < 0.001); // 0x88 = 136
    assert!((b - 0.0).abs() < 0.001);
    assert!((a - 1.0).abs() < 0.001);
}

#[test]
fn test_parse_color_to_normalized_rgba_hex8() {
    let [_r, _g, _b, a] = parse_color_to_normalized_rgba("#E85D4A80").unwrap();
    assert!((a - 128.0 / 255.0).abs() < 0.01); // 0x80 = 128, 128/255 ≈ 0.502
}

#[test]
fn test_parse_color_to_normalized_rgba_named() {
    let white = parse_color_to_normalized_rgba("white").unwrap();
    assert_eq!(white, [1.0, 1.0, 1.0, 1.0]);

    let black = parse_color_to_normalized_rgba("black").unwrap();
    assert_eq!(black, [0.0, 0.0, 0.0, 1.0]);

    let green = parse_color_to_normalized_rgba("green").unwrap();
    assert!((green[0] - 0.0).abs() < 0.001);
    assert!((green[1] - 128.0 / 255.0).abs() < 0.001);
    assert!((green[2] - 0.0).abs() < 0.001);
    assert!((green[3] - 1.0).abs() < 0.001);
}

#[test]
fn test_parse_color_to_normalized_rgba_invalid() {
    // Not a colour by any reading.
    assert!(parse_color_to_normalized_rgba("notacolor").is_err());
    assert!(parse_color_to_normalized_rgba("").is_err());
    assert!(parse_color_to_normalized_rgba("1px solid red").is_err());

    // A REFERENCE is not resolvable here, and that is a permanent property
    // rather than a limitation: `var(--primary)` names a value the cascade
    // supplies at render time, so there is nothing to compute with at build
    // time. This is the same boundary `infer_value_type` draws when it answers
    // `Reference` instead of a scalar.
    assert!(parse_color_to_normalized_rgba("var(--primary)").is_err());
}

/// FEAT-109 Tier 0. `color-mix(...)` USED to be asserted as an error here,
/// alongside `var()` — but the two are not alike, and grouping them hid that.
///
/// `var()` cannot be resolved at compile time because its value does not exist
/// yet. `color-mix()` is a closed computation over literal colours, and
/// lightningcss evaluates it: `color-mix(in oklch, red, blue)` is
/// `[0.714, 0.0, 0.741, 1.0]`, measured. The old assertion was pinning a
/// limitation of the eleven-colour hand-written table, not a property of the
/// language, and it would have blocked exactly the improvement that removed it.
#[test]
fn modern_colour_syntax_resolves_at_compile_time() {
    let mixed = parse_color_to_normalized_rgba("color-mix(in oklch, red, blue)")
        .expect("color-mix over two literals is computable");
    for c in mixed {
        assert!((0.0..=1.0).contains(&c), "component {c} out of range");
    }

    // The 148 named colours, not just the eleven the old table listed.
    let chartreuse = parse_color_to_normalized_rgba("chartreuse").expect("a named colour");
    assert!((chartreuse[1] - 1.0).abs() < 0.01, "chartreuse is full green");

    // 4- and 8-digit hex carry alpha; the old `from_hex` handled neither.
    let with_alpha = parse_color_to_normalized_rgba("#abcd").expect("4-digit hex");
    assert!(
        with_alpha[3] < 1.0,
        "#abcd has alpha {} but should be translucent",
        with_alpha[3]
    );
}
