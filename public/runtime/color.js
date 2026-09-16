/**
 * Color Utilities - Spacetime Runtime Module
 *
 * Parse and interpolate colors with OKLCH-first approach.
 * OKLCH provides perceptually uniform color interpolation.
 * Supports oklch(), hex, rgb(), rgba(), hsl(), hsla() formats.
 */

(function() {
  'use strict';

  const ST = window.ST;
  if (!ST) {
    console.error('[ST] Color requires ST to be defined first');
    return;
  }

  /**
   * Parse a color string into OKLCH components
   * @param {string} str - Color string (oklch, hex, rgb, rgba, hsl, hsla)
   * @returns {Object|null} Color object {l, c, h, a} in OKLCH space, or null
   */
  ST.parseColor = function(str) {
    if (!str || typeof str !== 'string') return null;
    const s = str.trim().toLowerCase();

    // OKLCH colors (native)
    const oklchMatch = s.match(/oklch\s*\(\s*([\d.]+%?)\s+([\d.]+)\s+([\d.]+)\s*(?:\/\s*([\d.]+%?)\s*)?\)/);
    if (oklchMatch) {
      const l = parseFloat(oklchMatch[1]) / (oklchMatch[1].includes('%') ? 100 : 1);
      return {
        l: Math.min(1, Math.max(0, l)),
        c: parseFloat(oklchMatch[2]),
        h: parseFloat(oklchMatch[3]),
        a: oklchMatch[4] ? parseFloat(oklchMatch[4]) / (oklchMatch[4].includes('%') ? 100 : 1) : 1
      };
    }

    // Hex colors -> convert to OKLCH
    if (s.startsWith('#')) {
      const rgb = hexToRgb(s);
      return rgb ? rgbToOklch(rgb) : null;
    }

    // RGB/RGBA colors -> convert to OKLCH
    const rgbMatch = s.match(/rgba?\s*\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*(?:,\s*([\d.]+)\s*)?\)/);
    if (rgbMatch) {
      const rgb = {
        r: parseFloat(rgbMatch[1]),
        g: parseFloat(rgbMatch[2]),
        b: parseFloat(rgbMatch[3]),
        a: rgbMatch[4] ? parseFloat(rgbMatch[4]) : 1
      };
      return rgbToOklch(rgb);
    }

    // HSL/HSLA colors -> convert to RGB -> convert to OKLCH
    const hslMatch = s.match(/hsla?\s*\(\s*(\d+)\s*,\s*(\d+)%?\s*,\s*(\d+)%?\s*(?:,\s*([\d.]+)\s*)?\)/);
    if (hslMatch) {
      const h = parseFloat(hslMatch[1]);
      const sat = parseFloat(hslMatch[2]) / 100;
      const l = parseFloat(hslMatch[3]) / 100;
      const a = hslMatch[4] ? parseFloat(hslMatch[4]) : 1;
      const rgb = hslToRgb(h, sat, l);
      rgb.a = a;
      return rgbToOklch(rgb);
    }

    return null;
  };

  /**
   * Parse hex color to RGB
   */
  function hexToRgb(hex) {
    const h = hex.slice(1);
    let r, g, b, a = 1;
    if (h.length === 3) {
      r = parseInt(h[0] + h[0], 16);
      g = parseInt(h[1] + h[1], 16);
      b = parseInt(h[2] + h[2], 16);
    } else if (h.length === 4) {
      r = parseInt(h[0] + h[0], 16);
      g = parseInt(h[1] + h[1], 16);
      b = parseInt(h[2] + h[2], 16);
      a = parseInt(h[3] + h[3], 16) / 255;
    } else if (h.length === 6) {
      r = parseInt(h.slice(0, 2), 16);
      g = parseInt(h.slice(2, 4), 16);
      b = parseInt(h.slice(4, 6), 16);
    } else if (h.length === 8) {
      r = parseInt(h.slice(0, 2), 16);
      g = parseInt(h.slice(2, 4), 16);
      b = parseInt(h.slice(4, 6), 16);
      a = parseInt(h.slice(6, 8), 16) / 255;
    } else {
      return null;
    }
    return { r, g, b, a };
  }

  /**
   * Convert HSL to RGB
   */
  function hslToRgb(h, s, l) {
    h = h / 360;
    let r, g, b;

    if (s === 0) {
      r = g = b = l;
    } else {
      const hue2rgb = (p, q, t) => {
        if (t < 0) t += 1;
        if (t > 1) t -= 1;
        if (t < 1/6) return p + (q - p) * 6 * t;
        if (t < 1/2) return q;
        if (t < 2/3) return p + (q - p) * (2/3 - t) * 6;
        return p;
      };

      const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
      const p = 2 * l - q;
      r = hue2rgb(p, q, h + 1/3);
      g = hue2rgb(p, q, h);
      b = hue2rgb(p, q, h - 1/3);
    }

    return {
      r: Math.round(r * 255),
      g: Math.round(g * 255),
      b: Math.round(b * 255)
    };
  }

  /**
   * Convert sRGB to linear RGB
   */
  function srgbToLinear(c) {
    c = c / 255;
    return c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
  }

  /**
   * Convert linear RGB to sRGB
   */
  function linearToSrgb(c) {
    c = c <= 0.0031308 ? 12.92 * c : 1.055 * Math.pow(c, 1/2.4) - 0.055;
    return Math.round(Math.min(255, Math.max(0, c * 255)));
  }

  /**
   * Convert RGB to OKLCH
   * Uses OKLab as intermediate (perceptually uniform)
   */
  function rgbToOklch(rgb) {
    // sRGB to linear RGB
    const lr = srgbToLinear(rgb.r);
    const lg = srgbToLinear(rgb.g);
    const lb = srgbToLinear(rgb.b);

    // Linear RGB to OKLab
    const l_ = 0.4122214708 * lr + 0.5363325363 * lg + 0.0514459929 * lb;
    const m_ = 0.2119034982 * lr + 0.6806995451 * lg + 0.1073969566 * lb;
    const s_ = 0.0883024619 * lr + 0.2817188376 * lg + 0.6299787005 * lb;

    const l = Math.cbrt(l_);
    const m = Math.cbrt(m_);
    const s = Math.cbrt(s_);

    const L = 0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s;
    const a = 1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s;
    const b = 0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s;

    // OKLab to OKLCH
    const C = Math.sqrt(a * a + b * b);
    let H = Math.atan2(b, a) * 180 / Math.PI;
    if (H < 0) H += 360;

    return {
      l: L,
      c: C,
      h: H,
      a: rgb.a !== undefined ? rgb.a : 1
    };
  }

  /**
   * Convert OKLCH to RGB
   */
  function oklchToRgb(oklch) {
    const { l: L, c: C, h: H, a: alpha } = oklch;

    // OKLCH to OKLab
    const hRad = H * Math.PI / 180;
    const a = C * Math.cos(hRad);
    const b = C * Math.sin(hRad);

    // OKLab to linear RGB
    const l_ = L + 0.3963377774 * a + 0.2158037573 * b;
    const m_ = L - 0.1055613458 * a - 0.0638541728 * b;
    const s_ = L - 0.0894841775 * a - 1.2914855480 * b;

    const l = l_ * l_ * l_;
    const m = m_ * m_ * m_;
    const s = s_ * s_ * s_;

    const lr = +4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
    const lg = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
    const lb = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s;

    return {
      r: linearToSrgb(lr),
      g: linearToSrgb(lg),
      b: linearToSrgb(lb),
      a: alpha
    };
  }

  /**
   * Interpolate between two colors in OKLCH space
   * OKLCH provides perceptually uniform interpolation
   * @param {Object} c1 - Start color (OKLCH)
   * @param {Object} c2 - End color (OKLCH)
   * @param {number} t - Progress (0-1)
   * @returns {string} Interpolated color as oklch() string
   */
  ST.interpolateColor = function(c1, c2, t) {
    if (!c1 || !c2) return null;

    const lerp = (a, b, t) => a + (b - a) * t;

    // Handle hue interpolation (shortest path around the circle)
    let h1 = c1.h, h2 = c2.h;
    const hDiff = h2 - h1;
    if (Math.abs(hDiff) > 180) {
      if (hDiff > 0) {
        h1 += 360;
      } else {
        h2 += 360;
      }
    }

    // Interpolate in OKLCH space
    const l = lerp(c1.l, c2.l, t);
    const c = lerp(c1.c, c2.c, t);
    let h = lerp(h1, h2, t);
    if (h >= 360) h -= 360;
    if (h < 0) h += 360;
    const a = lerp(c1.a, c2.a, t);

    // Return as oklch() for modern browsers, with rgb fallback
    if (a >= 1) {
      return `oklch(${(l * 100).toFixed(2)}% ${c.toFixed(4)} ${h.toFixed(2)})`;
    } else {
      return `oklch(${(l * 100).toFixed(2)}% ${c.toFixed(4)} ${h.toFixed(2)} / ${a.toFixed(3)})`;
    }
  };

  /**
   * Interpolate and return as RGB (for older browser fallback)
   * @param {Object} c1 - Start color (OKLCH)
   * @param {Object} c2 - End color (OKLCH)
   * @param {number} t - Progress (0-1)
   * @returns {string} Interpolated color as rgb()/rgba() string
   */
  ST.interpolateColorRgb = function(c1, c2, t) {
    if (!c1 || !c2) return null;

    const lerp = (a, b, t) => a + (b - a) * t;

    // Handle hue interpolation (shortest path)
    let h1 = c1.h, h2 = c2.h;
    const hDiff = h2 - h1;
    if (Math.abs(hDiff) > 180) {
      if (hDiff > 0) h1 += 360;
      else h2 += 360;
    }

    const interpolated = {
      l: lerp(c1.l, c2.l, t),
      c: lerp(c1.c, c2.c, t),
      h: lerp(h1, h2, t) % 360,
      a: lerp(c1.a, c2.a, t)
    };
    if (interpolated.h < 0) interpolated.h += 360;

    const rgb = oklchToRgb(interpolated);
    return rgb.a >= 1
      ? `rgb(${rgb.r}, ${rgb.g}, ${rgb.b})`
      : `rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, ${rgb.a.toFixed(3)})`;
  };

  /**
   * Check if a string is a color value
   * @param {string} str - Value to check
   * @returns {boolean} True if it's a color
   */
  ST.isColor = function(str) {
    return ST.parseColor(str) !== null;
  };

  // Export conversion utilities for testing/debugging
  ST._colorUtils = ST._colorUtils || {
    rgbToOklch,
    oklchToRgb,
    hexToRgb,
    hslToRgb
  };

})();
