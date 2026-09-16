//! Tests for LSP document color provider.

use spacetime::lsp::{DocumentState, provide_document_colors};
use tower_lsp::lsp_types::Color;

fn document_from(content: &str) -> DocumentState {
    DocumentState::new(content.to_string(), 1)
}

/// Helper to assert a Color's RGBA values within tolerance.
fn assert_color_approx(color: &Color, r: f32, g: f32, b: f32, a: f32) {
    let tol = 0.02;
    assert!(
        (color.red - r).abs() < tol,
        "red: expected {r}, got {}",
        color.red
    );
    assert!(
        (color.green - g).abs() < tol,
        "green: expected {g}, got {}",
        color.green
    );
    assert!(
        (color.blue - b).abs() < tol,
        "blue: expected {b}, got {}",
        color.blue
    );
    assert!(
        (color.alpha - a).abs() < tol,
        "alpha: expected {a}, got {}",
        color.alpha
    );
}

// =============================================================================
// Hex color detection
// =============================================================================

#[test]
fn hex_colors_detected() {
    let doc = document_from("color: #FFFFFF; bg: #1B2D4F;");
    let colors = provide_document_colors(&doc);
    assert_eq!(colors.len(), 2, "expected 2 hex colors, got {:?}", colors);
}

// =============================================================================
// rgba() color detection
// =============================================================================

#[test]
fn rgba_integer_color() {
    let doc = document_from("rgba(232,93,74,0)");
    let colors = provide_document_colors(&doc);
    assert_eq!(colors.len(), 1, "expected 1 rgba color, got {:?}", colors);
    // 232/255 ~ 0.91, 93/255 ~ 0.36, 74/255 ~ 0.29, alpha = 0.0
    assert_color_approx(&colors[0].color, 0.91, 0.36, 0.29, 0.0);
}

#[test]
fn rgba_float_alpha() {
    let doc = document_from("rgba(255,255,255,0.7)");
    let colors = provide_document_colors(&doc);
    assert_eq!(colors.len(), 1, "expected 1 rgba color, got {:?}", colors);
    assert_color_approx(&colors[0].color, 1.0, 1.0, 1.0, 0.7);
}

#[test]
fn rgba_with_spaces() {
    let doc = document_from("rgba( 232, 93, 74, 0 )");
    let colors = provide_document_colors(&doc);
    assert_eq!(
        colors.len(),
        1,
        "expected 1 rgba color with spaces, got {:?}",
        colors
    );
    assert_color_approx(&colors[0].color, 0.91, 0.36, 0.29, 0.0);
}

// =============================================================================
// Mixed color detection
// =============================================================================

#[test]
fn mixed_hex_and_rgba() {
    let doc = document_from("#FFFFFF -> rgba(92,92,92,1)");
    let colors = provide_document_colors(&doc);
    assert_eq!(
        colors.len(),
        2,
        "expected 2 colors (1 hex + 1 rgba), got {:?}",
        colors
    );
}

// =============================================================================
// Negative / edge cases
// =============================================================================

#[test]
fn no_colors_in_directives() {
    let doc = document_from("@scroll(start: 0.15)");
    let colors = provide_document_colors(&doc);
    assert!(
        colors.is_empty(),
        "expected no colors in directive, got {:?}",
        colors
    );
}
