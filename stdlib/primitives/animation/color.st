/// Color Interpolation Primitive
///
/// Provides color parsing and interpolation in multiple color spaces.
/// Supports RGB, HSL, and OKLCH color spaces.
///
/// @yields $parseColor - Parse color string to internal format
/// @yields $interpolateColor - Interpolate between two colors
/// @yields $colorToString - Convert internal format to CSS string

%primitive color-interpolate() {
  %emit js {
    // Parse color string to internal representation
    function parseColor(str) {
      if (!str || typeof str !== 'string') return null;
      const s = str.trim();

      // Hex colors
      if (s.startsWith('#')) {
        const hex = s.slice(1);
        let r, g, b, a = 1;

        if (hex.length === 3) {
          r = parseInt(hex[0] + hex[0], 16);
          g = parseInt(hex[1] + hex[1], 16);
          b = parseInt(hex[2] + hex[2], 16);
        } else if (hex.length === 6) {
          r = parseInt(hex.slice(0, 2), 16);
          g = parseInt(hex.slice(2, 4), 16);
          b = parseInt(hex.slice(4, 6), 16);
        } else if (hex.length === 8) {
          r = parseInt(hex.slice(0, 2), 16);
          g = parseInt(hex.slice(2, 4), 16);
          b = parseInt(hex.slice(4, 6), 16);
          a = parseInt(hex.slice(6, 8), 16) / 255;
        } else {
          return null;
        }
        return { type: 'rgb', r, g, b, a };
      }

      // rgb() or rgba()
      const rgbMatch = s.match(/rgba?\s*\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*(?:,\s*([\d.]+)\s*)?\)/);
      if (rgbMatch) {
        return {
          type: 'rgb',
          r: parseFloat(rgbMatch[1]),
          g: parseFloat(rgbMatch[2]),
          b: parseFloat(rgbMatch[3]),
          a: rgbMatch[4] ? parseFloat(rgbMatch[4]) : 1
        };
      }

      // hsl() or hsla()
      const hslMatch = s.match(/hsla?\s*\(\s*([\d.]+)(?:deg)?\s*,\s*([\d.]+)%\s*,\s*([\d.]+)%\s*(?:,\s*([\d.]+)\s*)?\)/);
      if (hslMatch) {
        return {
          type: 'hsl',
          h: parseFloat(hslMatch[1]),
          s: parseFloat(hslMatch[2]),
          l: parseFloat(hslMatch[3]),
          a: hslMatch[4] ? parseFloat(hslMatch[4]) : 1
        };
      }

      // oklch()
      const oklchMatch = s.match(/oklch\s*\(\s*([\d.]+)\s+([\d.]+)\s+([\d.]+)(?:deg)?\s*(?:\/\s*([\d.]+)\s*)?\)/);
      if (oklchMatch) {
        return {
          type: 'oklch',
          l: parseFloat(oklchMatch[1]),
          c: parseFloat(oklchMatch[2]),
          h: parseFloat(oklchMatch[3]),
          a: oklchMatch[4] ? parseFloat(oklchMatch[4]) : 1
        };
      }

      return null;
    }

    // Color space conversions
    function hslToRgb(h, s, l) {
      h = h / 360;
      s = s / 100;
      l = l / 100;

      if (s === 0) {
        const gray = Math.round(l * 255);
        return { r: gray, g: gray, b: gray };
      }

      const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
      const p = 2 * l - q;

      const hueToRgb = (p, q, t) => {
        if (t < 0) t += 1;
        if (t > 1) t -= 1;
        if (t < 1/6) return p + (q - p) * 6 * t;
        if (t < 1/2) return q;
        if (t < 2/3) return p + (q - p) * (2/3 - t) * 6;
        return p;
      };

      return {
        r: Math.round(hueToRgb(p, q, h + 1/3) * 255),
        g: Math.round(hueToRgb(p, q, h) * 255),
        b: Math.round(hueToRgb(p, q, h - 1/3) * 255)
      };
    }

    function rgbToHsl(r, g, b) {
      r /= 255; g /= 255; b /= 255;
      const max = Math.max(r, g, b);
      const min = Math.min(r, g, b);
      const l = (max + min) / 2;

      if (max === min) return { h: 0, s: 0, l: l * 100 };

      const delta = max - min;
      const s = l > 0.5 ? delta / (2 - max - min) : delta / (max + min);

      let h;
      if (max === r) h = (g - b) / delta + (g < b ? 6 : 0);
      else if (max === g) h = (b - r) / delta + 2;
      else h = (r - g) / delta + 4;
      h *= 60;

      return { h, s: s * 100, l: l * 100 };
    }

    function oklchToRgb(l, c, h) {
      const hRad = h * Math.PI / 180;
      const a = c * Math.cos(hRad);
      const b = c * Math.sin(hRad);

      const l_ = l + 0.3963377774 * a + 0.2158037573 * b;
      const m_ = l - 0.1055613458 * a - 0.0638541728 * b;
      const s_ = l - 0.0894841775 * a - 1.2914855480 * b;

      const lCube = l_ * l_ * l_;
      const mCube = m_ * m_ * m_;
      const sCube = s_ * s_ * s_;

      let rLin = 4.0767416621 * lCube - 3.3077115913 * mCube + 0.2309699292 * sCube;
      let gLin = -1.2684380046 * lCube + 2.6097574011 * mCube - 0.3413193965 * sCube;
      let bLin = -0.0041960863 * lCube - 0.7034186147 * mCube + 1.7076147010 * sCube;

      const linearToSrgb = (c) => c <= 0.0031308 ? c * 12.92 : 1.055 * Math.pow(c, 1/2.4) - 0.055;

      return {
        r: Math.max(0, Math.min(255, Math.round(linearToSrgb(rLin) * 255))),
        g: Math.max(0, Math.min(255, Math.round(linearToSrgb(gLin) * 255))),
        b: Math.max(0, Math.min(255, Math.round(linearToSrgb(bLin) * 255)))
      };
    }

    function rgbToOklch(r, g, b) {
      r /= 255; g /= 255; b /= 255;
      const srgbToLinear = (c) => c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);

      const rLin = srgbToLinear(r);
      const gLin = srgbToLinear(g);
      const bLin = srgbToLinear(b);

      const l = 0.4122214708 * rLin + 0.5363325363 * gLin + 0.0514459929 * bLin;
      const m = 0.2119034982 * rLin + 0.6806995451 * gLin + 0.1073969566 * bLin;
      const s = 0.0883024619 * rLin + 0.2817188376 * gLin + 0.6299787005 * bLin;

      const l_ = Math.cbrt(l);
      const m_ = Math.cbrt(m);
      const s_ = Math.cbrt(s);

      const oklabL = 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_;
      const oklabA = 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_;
      const oklabB = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_;

      const c = Math.sqrt(oklabA * oklabA + oklabB * oklabB);
      let h = Math.atan2(oklabB, oklabA) * 180 / Math.PI;
      if (h < 0) h += 360;

      return { l: oklabL, c, h };
    }

    function lerpHue(h1, h2, t) {
      let delta = h2 - h1;
      if (delta > 180) delta -= 360;
      else if (delta < -180) delta += 360;

      let result = h1 + delta * t;
      if (result < 0) result += 360;
      else if (result >= 360) result -= 360;

      return result;
    }

    // Interpolate between two colors
    function interpolateColor(c1, c2, t, space = 'rgb') {
      if (!c1 || !c2) return null;
      const lerp = (a, b, t) => a + (b - a) * t;

      if (space === 'hsl') {
        const hsl1 = c1.type === 'hsl' ? c1 : rgbToHsl(c1.r, c1.g, c1.b);
        const hsl2 = c2.type === 'hsl' ? c2 : rgbToHsl(c2.r, c2.g, c2.b);

        const h = lerpHue(hsl1.h, hsl2.h, t);
        const s = lerp(hsl1.s, hsl2.s, t);
        const l = lerp(hsl1.l, hsl2.l, t);
        const a = lerp(c1.a, c2.a, t);

        const rgb = hslToRgb(h, s, l);
        return `rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, ${a})`;
      } else if (space === 'oklch') {
        const oklch1 = c1.type === 'oklch' ? c1 : rgbToOklch(c1.r, c1.g, c1.b);
        const oklch2 = c2.type === 'oklch' ? c2 : rgbToOklch(c2.r, c2.g, c2.b);

        const l = lerp(oklch1.l, oklch2.l, t);
        const c = lerp(oklch1.c, oklch2.c, t);
        const h = lerpHue(oklch1.h, oklch2.h, t);
        const a = lerp(c1.a, c2.a, t);

        const rgb = oklchToRgb(l, c, h);
        return `rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, ${a})`;
      } else {
        // RGB interpolation (default)
        const rgb1 = c1.type === 'rgb' ? c1 : hslToRgb(c1.h, c1.s, c1.l);
        const rgb2 = c2.type === 'rgb' ? c2 : hslToRgb(c2.h, c2.s, c2.l);

        const r = Math.round(lerp(rgb1.r, rgb2.r, t));
        const g = Math.round(lerp(rgb1.g, rgb2.g, t));
        const b = Math.round(lerp(rgb1.b, rgb2.b, t));
        const a = lerp(c1.a, c2.a, t);

        return a >= 1 ? `rgb(${r}, ${g}, ${b})` : `rgba(${r}, ${g}, ${b}, ${a})`;
      }
    }

    %yield parseColor -> $parseColor;
    %yield interpolateColor -> $interpolateColor;
  }

  %exports {
    $parseColor: fn(color: string) ~> object
    $interpolateColor: fn(from: object, to: object, t: number, mode: string?) ~> string
  }
}
