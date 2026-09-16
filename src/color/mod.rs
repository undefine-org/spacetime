//! Color module for Spacetime
//!
//! Provides color parsing, conversion, and interpolation across multiple color spaces:
//! - RGB (sRGB) - Standard RGB color space
//! - HSL - Hue, Saturation, Lightness
//! - OKLCH - Perceptually uniform Oklab-based cylindrical color space
//!
//! # Color Spaces
//!
//! - **RGB**: Traditional RGB color space. Simple but not perceptually uniform.
//! - **HSL**: Cylindrical representation of RGB. Good for hue rotation but not perceptually uniform.
//! - **OKLCH**: Perceptually uniform color space based on Oklab. Best for smooth gradients.
//!
//! # Examples
//!
//! ```
//! use spacetime::color::{Color, ColorSpace};
//!
//! // Parse hex colors
//! let red = Color::from_hex("#ff0000").unwrap();
//! let blue = Color::from_hex("#0000ff").unwrap();
//!
//! // Interpolate in different color spaces
//! let mid_rgb = Color::interpolate(&red, &blue, 0.5, ColorSpace::RGB);
//! let mid_hsl = Color::interpolate(&red, &blue, 0.5, ColorSpace::HSL);
//! let mid_oklch = Color::interpolate(&red, &blue, 0.5, ColorSpace::OKLCH);
//! ```

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

/// The color space to use for interpolation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorSpace {
    RGB,
    HSL,
    OKLCH,
}

/// A color value that can represent multiple color spaces
#[derive(Debug, Clone, PartialEq)]
pub enum Color {
    RGB { r: f64, g: f64, b: f64, a: f64 },
    HSL { h: f64, s: f64, l: f64, a: f64 },
    OKLCH { l: f64, c: f64, h: f64, a: f64 },
}

impl Color {
    // ============================================================
    // Construction
    // ============================================================

    /// Create an RGB color (values 0-255 for r,g,b and 0-1 for alpha)
    pub fn rgb(r: f64, g: f64, b: f64) -> Self {
        Color::RGB { r, g, b, a: 1.0 }
    }

    /// Create an RGBA color (values 0-255 for r,g,b and 0-1 for alpha)
    pub fn rgba(r: f64, g: f64, b: f64, a: f64) -> Self {
        Color::RGB { r, g, b, a }
    }

    /// Create an HSL color (h: 0-360, s: 0-100, l: 0-100, a: 0-1)
    pub fn hsl(h: f64, s: f64, l: f64) -> Self {
        Color::HSL { h, s, l, a: 1.0 }
    }

    /// Create an HSLA color (h: 0-360, s: 0-100, l: 0-100, a: 0-1)
    pub fn hsla(h: f64, s: f64, l: f64, a: f64) -> Self {
        Color::HSL { h, s, l, a }
    }

    /// Create an OKLCH color (l: 0-1, c: 0-0.4, h: 0-360, a: 0-1)
    pub fn oklch(l: f64, c: f64, h: f64) -> Self {
        Color::OKLCH { l, c, h, a: 1.0 }
    }

    /// Create an OKLCH color with alpha (l: 0-1, c: 0-0.4, h: 0-360, a: 0-1)
    pub fn oklcha(l: f64, c: f64, h: f64, a: f64) -> Self {
        Color::OKLCH { l, c, h, a }
    }

    // ============================================================
    // Parsing
    // ============================================================

    /// Parse a hex color string (#RGB, #RRGGBB, #RRGGBBAA)
    pub fn from_hex(hex: &str) -> Result<Self, String> {
        let hex = hex.trim_start_matches('#');

        let (r, g, b, a) = match hex.len() {
            3 => {
                // #RGB -> #RRGGBB
                let r = u8::from_str_radix(&hex[0..1].repeat(2), 16)
                    .map_err(|_| "Invalid hex digit")?;
                let g = u8::from_str_radix(&hex[1..2].repeat(2), 16)
                    .map_err(|_| "Invalid hex digit")?;
                let b = u8::from_str_radix(&hex[2..3].repeat(2), 16)
                    .map_err(|_| "Invalid hex digit")?;
                (r as f64, g as f64, b as f64, 1.0)
            }
            6 => {
                // #RRGGBB
                let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| "Invalid hex digit")?;
                let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| "Invalid hex digit")?;
                let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| "Invalid hex digit")?;
                (r as f64, g as f64, b as f64, 1.0)
            }
            8 => {
                // #RRGGBBAA
                let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| "Invalid hex digit")?;
                let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| "Invalid hex digit")?;
                let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| "Invalid hex digit")?;
                let a = u8::from_str_radix(&hex[6..8], 16).map_err(|_| "Invalid hex digit")?;
                (r as f64, g as f64, b as f64, a as f64 / 255.0)
            }
            _ => return Err(format!("Invalid hex color length: {}", hex.len())),
        };

        Ok(Color::RGB { r, g, b, a })
    }

    /// Parse rgb() or rgba() function syntax
    pub fn from_rgb_string(s: &str) -> Result<Self, String> {
        let s = s.trim();
        let (prefix, content) = if s.starts_with("rgba(") && s.ends_with(')') {
            ("rgba", &s[5..s.len() - 1])
        } else if s.starts_with("rgb(") && s.ends_with(')') {
            ("rgb", &s[4..s.len() - 1])
        } else {
            return Err("Invalid rgb/rgba format".to_string());
        };

        let parts: Vec<&str> = content.split(',').map(|s| s.trim()).collect();

        if prefix == "rgb" && parts.len() != 3 {
            return Err("rgb() requires 3 values".to_string());
        }
        if prefix == "rgba" && parts.len() != 4 {
            return Err("rgba() requires 4 values".to_string());
        }

        let r = parts[0].parse::<f64>().map_err(|_| "Invalid r value")?;
        let g = parts[1].parse::<f64>().map_err(|_| "Invalid g value")?;
        let b = parts[2].parse::<f64>().map_err(|_| "Invalid b value")?;
        let a = if parts.len() == 4 {
            parts[3].parse::<f64>().map_err(|_| "Invalid alpha value")?
        } else {
            1.0
        };

        Ok(Color::RGB { r, g, b, a })
    }

    /// Parse hsl() or hsla() function syntax
    pub fn from_hsl_string(s: &str) -> Result<Self, String> {
        let s = s.trim();
        let (prefix, content) = if s.starts_with("hsla(") && s.ends_with(')') {
            ("hsla", &s[5..s.len() - 1])
        } else if s.starts_with("hsl(") && s.ends_with(')') {
            ("hsl", &s[4..s.len() - 1])
        } else {
            return Err("Invalid hsl/hsla format".to_string());
        };

        let parts: Vec<&str> = content.split(',').map(|s| s.trim()).collect();

        if prefix == "hsl" && parts.len() != 3 {
            return Err("hsl() requires 3 values".to_string());
        }
        if prefix == "hsla" && parts.len() != 4 {
            return Err("hsla() requires 4 values".to_string());
        }

        let h = parts[0]
            .trim_end_matches("deg")
            .parse::<f64>()
            .map_err(|_| "Invalid h value")?;
        let s = parts[1]
            .trim_end_matches('%')
            .parse::<f64>()
            .map_err(|_| "Invalid s value")?;
        let l = parts[2]
            .trim_end_matches('%')
            .parse::<f64>()
            .map_err(|_| "Invalid l value")?;
        let a = if parts.len() == 4 {
            parts[3].parse::<f64>().map_err(|_| "Invalid alpha value")?
        } else {
            1.0
        };

        Ok(Color::HSL { h, s, l, a })
    }

    /// Parse oklch() function syntax
    pub fn from_oklch_string(s: &str) -> Result<Self, String> {
        let s = s.trim();
        if !s.starts_with("oklch(") || !s.ends_with(')') {
            return Err("Invalid oklch format".to_string());
        }

        let content = &s[6..s.len() - 1];
        let parts: Vec<&str> = content.split(',').map(|s| s.trim()).collect();

        if parts.len() != 3 && parts.len() != 4 {
            return Err("oklch() requires 3 or 4 values".to_string());
        }

        let l = parts[0].parse::<f64>().map_err(|_| "Invalid l value")?;
        let c = parts[1].parse::<f64>().map_err(|_| "Invalid c value")?;
        let h = parts[2]
            .trim_end_matches("deg")
            .parse::<f64>()
            .map_err(|_| "Invalid h value")?;
        let a = if parts.len() == 4 {
            parts[3].parse::<f64>().map_err(|_| "Invalid alpha value")?
        } else {
            1.0
        };

        Ok(Color::OKLCH { l, c, h, a })
    }

    // ============================================================
    // Conversion to RGB
    // ============================================================

    /// Convert this color to RGB
    pub fn to_rgb(&self) -> Color {
        match self {
            Color::RGB { .. } => self.clone(),
            Color::HSL { h, s, l, a } => {
                let (r, g, b) = hsl_to_rgb(*h, *s, *l);
                Color::RGB { r, g, b, a: *a }
            }
            Color::OKLCH { l, c, h, a } => {
                let (r, g, b) = oklch_to_rgb(*l, *c, *h);
                Color::RGB { r, g, b, a: *a }
            }
        }
    }

    /// Convert this color to HSL
    pub fn to_hsl(&self) -> Color {
        match self {
            Color::HSL { .. } => self.clone(),
            Color::RGB { r, g, b, a } => {
                let (h, s, l) = rgb_to_hsl(*r, *g, *b);
                Color::HSL { h, s, l, a: *a }
            }
            Color::OKLCH { l, c, h, a } => {
                // Convert OKLCH -> RGB -> HSL
                let (r, g, b) = oklch_to_rgb(*l, *c, *h);
                let (h, s, l) = rgb_to_hsl(r, g, b);
                Color::HSL { h, s, l, a: *a }
            }
        }
    }

    /// Convert this color to OKLCH
    pub fn to_oklch(&self) -> Color {
        match self {
            Color::OKLCH { .. } => self.clone(),
            Color::RGB { r, g, b, a } => {
                let (l, c, h) = rgb_to_oklch(*r, *g, *b);
                Color::OKLCH { l, c, h, a: *a }
            }
            Color::HSL { h, s, l, a } => {
                // Convert HSL -> RGB -> OKLCH
                let (r, g, b) = hsl_to_rgb(*h, *s, *l);
                let (l, c, h) = rgb_to_oklch(r, g, b);
                Color::OKLCH { l, c, h, a: *a }
            }
        }
    }

    // ============================================================
    // Interpolation
    // ============================================================

    /// Interpolate between two colors in the specified color space
    /// t is the interpolation factor (0.0 = from, 1.0 = to)
    pub fn interpolate(from: &Color, to: &Color, t: f64, space: ColorSpace) -> Color {
        match space {
            ColorSpace::RGB => interpolate_rgb(from, to, t),
            ColorSpace::HSL => interpolate_hsl(from, to, t),
            ColorSpace::OKLCH => interpolate_oklch(from, to, t),
        }
    }

    // ============================================================
    // CSS Output
    // ============================================================

    /// Convert to CSS color string
    pub fn to_css(&self) -> String {
        match self {
            Color::RGB { r, g, b, a } => {
                if *a >= 1.0 {
                    format!("rgb({}, {}, {})", r.round(), g.round(), b.round())
                } else {
                    format!("rgba({}, {}, {}, {})", r.round(), g.round(), b.round(), a)
                }
            }
            Color::HSL { h, s, l, a } => {
                if *a >= 1.0 {
                    format!("hsl({}, {}%, {}%)", h.round(), s.round(), l.round())
                } else {
                    format!("hsla({}, {}%, {}%, {})", h.round(), s.round(), l.round(), a)
                }
            }
            Color::OKLCH { l, c, h, a } => {
                if *a >= 1.0 {
                    format!("oklch({} {} {})", l, c, h.round())
                } else {
                    format!("oklch({} {} {} / {})", l, c, h.round(), a)
                }
            }
        }
    }

    /// Get the alpha channel value
    pub fn alpha(&self) -> f64 {
        match self {
            Color::RGB { a, .. } | Color::HSL { a, .. } | Color::OKLCH { a, .. } => *a,
        }
    }

    /// Convert this color to normalized RGBA values (all in 0-1 range)
    /// Returns [r/255, g/255, b/255, a] suitable for WebGL gl.clearColor()
    pub fn to_normalized_rgba(&self) -> [f64; 4] {
        let rgb = self.to_rgb();
        match rgb {
            Color::RGB { r, g, b, a } => [r / 255.0, g / 255.0, b / 255.0, a],
            _ => unreachable!(),
        }
    }
}

/// Parse a CSS color string to normalized RGBA values (all in 0-1 range) at compile time.
/// Supports: transparent, hex (#RGB, #RRGGBB, #RRGGBBAA), and common named colors.
/// Returns Err for colors that cannot be resolved at compile time (color-mix, var(), etc.).
pub fn parse_color_to_normalized_rgba(s: &str) -> Result<[f64; 4], String> {
    let s = s.trim();

    if s.is_empty() {
        return Err("empty value is not a colour".to_string());
    }

    // FEAT-109 Tier 0. This function used to be the FIFTH independent answer to
    // a colour question in this tree, and the weakest of them: `#RGB` and
    // `#RRGGBB` via `Color::from_hex`, plus a hand-written table of ELEVEN named
    // colours, with everything else — `#RGBA`, `#RRGGBBAA`, `rgb()`, `rgba()`,
    // `hsl()`, `oklch()`, and 137 of the 148 named colours — rejected as
    // "cannot resolve at compile time".
    //
    // Both consumers had already routed around it. `src/lsp/colors.rs` and
    // `src/analysis/visual_lint.rs` each carried their OWN twelve-line
    // `parse_css_color`, identical but for building an f32 colour versus an f64
    // one, and both called lightningcss directly — so the canonical parser was
    // canonical in name only, and a value could be a colour to the swatch
    // renderer and not a colour to the contrast linter.
    //
    // lightningcss is already a dependency, already parses every CSS colour
    // syntax, and is already what those consumers reached for. Vendor + own
    // behind a primitive: this is the primitive.
    //
    // NB the deliberate asymmetry with `infer_value_type`. The grammar refuses a
    // bare identifier (`chartreuse`) because in a value position it is ambiguous
    // with every other CSS keyword — a RECOGNITION question. Converting a value
    // already known to be a colour is a different question, and there
    // `chartreuse` is simply green. A contrast linter reading `color:
    // chartreuse` must compute with it, so this accepts strictly more than the
    // grammar does, on purpose.
    use lightningcss::traits::Parse;
    use lightningcss::values::color::{CssColor, RGBA};

    let parsed = CssColor::parse_string(s)
        .map_err(|_| format!("Cannot resolve color '{}' at compile time", s))?;
    let rgba: RGBA = RGBA::try_from(parsed)
        .map_err(|_| format!("Cannot resolve color '{}' to RGBA at compile time", s))?;

    Ok([
        rgba.red as f64 / 255.0,
        rgba.green as f64 / 255.0,
        rgba.blue as f64 / 255.0,
        rgba.alpha as f64 / 255.0,
    ])
}

// ============================================================
// Color Space Conversion Functions
// ============================================================

/// Convert HSL to RGB
/// h: 0-360, s: 0-100, l: 0-100 -> r,g,b: 0-255
fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (f64, f64, f64) {
    let h = h / 360.0;
    let s = s / 100.0;
    let l = l / 100.0;

    if s == 0.0 {
        let gray = (l * 255.0).round();
        return (gray, gray, gray);
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;

    let r = hue_to_rgb(p, q, h + 1.0 / 3.0) * 255.0;
    let g = hue_to_rgb(p, q, h) * 255.0;
    let b = hue_to_rgb(p, q, h - 1.0 / 3.0) * 255.0;

    (r, g, b)
}

fn hue_to_rgb(p: f64, q: f64, mut t: f64) -> f64 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 1.0 / 2.0 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

/// Convert RGB to HSL
/// r,g,b: 0-255 -> h: 0-360, s: 0-100, l: 0-100
fn rgb_to_hsl(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let r = r / 255.0;
    let g = g / 255.0;
    let b = b / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    if max == min {
        return (0.0, 0.0, l * 100.0);
    }

    let delta = max - min;
    let s = if l > 0.5 {
        delta / (2.0 - max - min)
    } else {
        delta / (max + min)
    };

    let h = if max == r {
        (g - b) / delta + (if g < b { 6.0 } else { 0.0 })
    } else if max == g {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    };

    (h * 60.0, s * 100.0, l * 100.0)
}

/// Convert RGB to OKLCH (via Oklab)
/// r,g,b: 0-255 -> l: 0-1, c: 0-0.4, h: 0-360
fn rgb_to_oklch(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    // First convert sRGB to linear RGB
    let r_lin = srgb_to_linear(r / 255.0);
    let g_lin = srgb_to_linear(g / 255.0);
    let b_lin = srgb_to_linear(b / 255.0);

    // Convert linear RGB to Oklab
    let l = 0.4122214708 * r_lin + 0.5363325363 * g_lin + 0.0514459929 * b_lin;
    let m = 0.2119034982 * r_lin + 0.6806995451 * g_lin + 0.1073969566 * b_lin;
    let s = 0.0883024619 * r_lin + 0.2817188376 * g_lin + 0.6299787005 * b_lin;

    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();

    let oklab_l = 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_;
    let oklab_a = 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_;
    let oklab_b = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_;

    // Convert Oklab to OKLCH
    let c = (oklab_a * oklab_a + oklab_b * oklab_b).sqrt();
    let h = oklab_b.atan2(oklab_a).to_degrees();
    let h = if h < 0.0 { h + 360.0 } else { h };

    (oklab_l, c, h)
}

/// Convert OKLCH to RGB (via Oklab)
/// l: 0-1, c: 0-0.4, h: 0-360 -> r,g,b: 0-255
fn oklch_to_rgb(l: f64, c: f64, h: f64) -> (f64, f64, f64) {
    // Convert OKLCH to Oklab
    let h_rad = h.to_radians();
    let oklab_a = c * h_rad.cos();
    let oklab_b = c * h_rad.sin();

    // Convert Oklab to linear RGB
    let l_ = l + 0.3963377774 * oklab_a + 0.2158037573 * oklab_b;
    let m_ = l - 0.1055613458 * oklab_a - 0.0638541728 * oklab_b;
    let s_ = l - 0.0894841775 * oklab_a - 1.2914855480 * oklab_b;

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    let r_lin = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
    let g_lin = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
    let b_lin = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s;

    // Convert linear RGB to sRGB
    let r = linear_to_srgb(r_lin) * 255.0;
    let g = linear_to_srgb(g_lin) * 255.0;
    let b = linear_to_srgb(b_lin) * 255.0;

    // Clamp to valid range
    (
        r.max(0.0).min(255.0),
        g.max(0.0).min(255.0),
        b.max(0.0).min(255.0),
    )
}

/// Convert sRGB value to linear RGB
fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Convert linear RGB to sRGB
fn linear_to_srgb(c: f64) -> f64 {
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

// ============================================================
// WCAG 2.1 Contrast Utilities
// ============================================================

/// Compute relative luminance per WCAG 2.1
/// https://www.w3.org/TR/WCAG21/#dfn-relative-luminance
pub fn relative_luminance(color: &Color) -> f64 {
    let rgb = color.to_rgb();
    match rgb {
        Color::RGB { r, g, b, .. } => {
            let r_lin = srgb_to_linear(r / 255.0);
            let g_lin = srgb_to_linear(g / 255.0);
            let b_lin = srgb_to_linear(b / 255.0);
            0.2126 * r_lin + 0.7152 * g_lin + 0.0722 * b_lin
        }
        _ => unreachable!(),
    }
}

/// Compute contrast ratio per WCAG 2.1
/// Returns ratio >= 1.0 (always lighter/darker ordered)
pub fn contrast_ratio(color1: &Color, color2: &Color) -> f64 {
    let l1 = relative_luminance(color1);
    let l2 = relative_luminance(color2);
    let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (lighter + 0.05) / (darker + 0.05)
}

// ============================================================
// Interpolation Functions
// ============================================================

/// Linear interpolation in RGB space
fn interpolate_rgb(from: &Color, to: &Color, t: f64) -> Color {
    let from_rgb = from.to_rgb();
    let to_rgb = to.to_rgb();

    match (from_rgb, to_rgb) {
        (
            Color::RGB {
                r: r1,
                g: g1,
                b: b1,
                a: a1,
            },
            Color::RGB {
                r: r2,
                g: g2,
                b: b2,
                a: a2,
            },
        ) => Color::RGB {
            r: lerp(r1, r2, t),
            g: lerp(g1, g2, t),
            b: lerp(b1, b2, t),
            a: lerp(a1, a2, t),
        },
        _ => unreachable!(),
    }
}

/// Hue-aware interpolation in HSL space (shortest path)
fn interpolate_hsl(from: &Color, to: &Color, t: f64) -> Color {
    let from_hsl = from.to_hsl();
    let to_hsl = to.to_hsl();

    match (from_hsl, to_hsl) {
        (
            Color::HSL {
                h: h1,
                s: s1,
                l: l1,
                a: a1,
            },
            Color::HSL {
                h: h2,
                s: s2,
                l: l2,
                a: a2,
            },
        ) => {
            // Interpolate hue along shortest path
            let h = lerp_hue(h1, h2, t);
            Color::HSL {
                h,
                s: lerp(s1, s2, t),
                l: lerp(l1, l2, t),
                a: lerp(a1, a2, t),
            }
        }
        _ => unreachable!(),
    }
}

/// Perceptually uniform interpolation in OKLCH space
fn interpolate_oklch(from: &Color, to: &Color, t: f64) -> Color {
    let from_oklch = from.to_oklch();
    let to_oklch = to.to_oklch();

    match (from_oklch, to_oklch) {
        (
            Color::OKLCH {
                l: l1,
                c: c1,
                h: h1,
                a: a1,
            },
            Color::OKLCH {
                l: l2,
                c: c2,
                h: h2,
                a: a2,
            },
        ) => {
            // Interpolate hue along shortest path
            let h = lerp_hue(h1, h2, t);
            Color::OKLCH {
                l: lerp(l1, l2, t),
                c: lerp(c1, c2, t),
                h,
                a: lerp(a1, a2, t),
            }
        }
        _ => unreachable!(),
    }
}

/// Linear interpolation
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Interpolate hue along shortest circular path
fn lerp_hue(h1: f64, h2: f64, t: f64) -> f64 {
    let mut delta = h2 - h1;

    // Take shortest path around the color wheel
    if delta > 180.0 {
        delta -= 360.0;
    } else if delta < -180.0 {
        delta += 360.0;
    }

    let mut result = h1 + delta * t;

    // Normalize to 0-360
    if result < 0.0 {
        result += 360.0;
    } else if result >= 360.0 {
        result -= 360.0;
    }

    result
}
