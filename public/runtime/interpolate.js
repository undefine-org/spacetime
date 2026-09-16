/**
 * Interpolation - Spacetime Runtime Module
 *
 * Linear interpolation and keyframe-based animation value calculation.
 * Handles numeric values, colors, and CSS units.
 */

(function() {
  'use strict';

  const ST = window.ST;
  if (!ST) {
    console.error('[ST] Interpolate requires ST to be defined first');
    return;
  }

  /**
   * Linear interpolation between two values
   * @param {number} a - Start value
   * @param {number} b - End value
   * @param {number} t - Progress (0-1)
   * @returns {number} Interpolated value
   */
  ST.lerp = function(a, b, t) {
    return a + (b - a) * t;
  };

  /**
   * Interpolate through keyframes
   * @param {Array} keyframes - Array of {at, value, easing?} objects
   * @param {number} progress - Progress (0-1)
   * @param {string} easing - Default easing function name
   * @param {string} colorSpace - Color interpolation space (unused, future)
   * @returns {*} Interpolated value
   */
  ST.interpolate = function(keyframes, progress, easing, colorSpace) {
    if (!keyframes || keyframes.length === 0) return null;
    if (keyframes.length === 1) return keyframes[0].value;

    // Find surrounding keyframes
    let prev = keyframes[0];
    let next = keyframes[keyframes.length - 1];

    for (let i = 0; i < keyframes.length - 1; i++) {
      if (progress >= keyframes[i].at && progress <= keyframes[i + 1].at) {
        prev = keyframes[i];
        next = keyframes[i + 1];
        break;
      }
    }

    // Clamp to boundaries
    if (progress <= prev.at) return prev.value;
    if (progress >= next.at) return next.value;

    // Calculate local progress
    const localProgress = (progress - prev.at) / (next.at - prev.at);
    const easeFn = ST.getEasing(prev.easing || easing);
    const easedProgress = easeFn(localProgress);

    // Try color interpolation first
    if (ST.parseColor && ST.interpolateColor) {
      const color1 = ST.parseColor(prev.value);
      const color2 = ST.parseColor(next.value);
      if (color1 && color2) {
        return ST.interpolateColor(color1, color2, easedProgress);
      }
    }

    // Numeric interpolation (handles units like px, em, %, deg, etc.)
    const parseNumeric = v => {
      if (typeof v === 'number') return { num: v, unit: '' };
      const match = String(v).match(/^(-?\d*\.?\d+)(.*)$/);
      return match ? { num: parseFloat(match[1]), unit: match[2] } : null;
    };

    const prevParsed = parseNumeric(prev.value);
    const nextParsed = parseNumeric(next.value);

    if (prevParsed && nextParsed) {
      const interpolated = ST.lerp(prevParsed.num, nextParsed.num, easedProgress);
      const unit = nextParsed.unit || prevParsed.unit;
      return unit ? interpolated + unit : interpolated;
    }

    // Discrete interpolation for non-numeric values
    return easedProgress < 0.5 ? prev.value : next.value;
  };

  /**
   * Apply an animated property to an element
   * Handles transforms, CSS variables, and regular properties.
   * @param {Element} el - Target element
   * @param {string} property - Property name (CSS or transform)
   * @param {*} value - Value to apply
   */
  ST.applyProperty = function(el, property, value) {
    // CSS custom properties (check before normalization to preserve --)
    if (property.startsWith('--')) {
      el.style.setProperty(property, value);
      return;
    }

    // Normalize property name (kebab-case to camelCase)
    const prop = property.replace(/-([a-z])/g, (_, c) => c.toUpperCase());

    // Transform functions (compose into single transform)
    const transformFns = [
      'translateX', 'translateY', 'translateZ', 'translate',
      'rotate', 'rotateX', 'rotateY', 'rotateZ',
      'scale', 'scaleX', 'scaleY', 'scaleZ',
      'skewX', 'skewY',
      'perspective'
    ];

    if (transformFns.includes(prop)) {
      const transforms = el._stTransforms || {};
      transforms[prop] = value;
      el._stTransforms = transforms;
      el.style.transform = Object.entries(transforms)
        .map(([k, v]) => {
          // Add default units based on transform type
          if (k.startsWith('translate')) {
            return `${k}(${typeof v === 'number' ? v + 'px' : v})`;
          }
          if (k.startsWith('rotate') || k.startsWith('skew')) {
            return `${k}(${typeof v === 'number' ? v + 'deg' : v})`;
          }
          if (k === 'perspective') {
            return `${k}(${typeof v === 'number' ? v + 'px' : v})`;
          }
          return `${k}(${v})`;
        })
        .join(' ');
      return;
    }

    // Regular CSS properties. Coerce to string to match the real CSSOM contract
    // (a browser stringifies `el.style.opacity = 0.5` to '0.5' on read). LinkeDOM
    // stores the raw value, so without this an assertion reading back the value
    // sees a Number; the cast is a no-op semantically (CSS values are strings).
    const cssValue = typeof value === 'number'
      ? (prop === 'opacity' || prop === 'zIndex' || prop === 'fontWeight' ? String(value) : value + 'px')
      : value;
    el.style[prop] = cssValue;
  };

  /**
   * Set a specific transform function value
   * @param {Element} el - Target element
   * @param {string} fn - Transform function name
   * @param {*} value - Value (with or without unit)
   */
  ST.setTransform = function(el, fn, value) {
    ST.applyProperty(el, fn, value);
  };

  /**
   * Convert a Spacetime `$body:keyframes` capture into WAAPI keyframes.
   *
   * A `keyframes` capture arrives as `{ properties: [...] }` or, when the body
   * used scope blocks (`& { ... }`), as `{ scopes: [{ selector, properties }] }`.
   * Each property carries `{ property, keyframes: [{ at, value }] }`. WAAPI wants
   * the transpose: one frame per OFFSET, carrying every property at that offset.
   *
   * Shared because two engines need the identical transform: `reveal-engine`
   * (per split unit) and `fade-in-engine` (whole element). It lived inlined in
   * reveal; a second copy would be a parallel implementation of one rule, and
   * the two would drift the first time a keyframe shape changed.
   *
   * @param {object|null} data - the raw keyframes capture
   * @returns {Array<object>|null} WAAPI keyframes, or null when there is no body
   */
  ST.toWaapiKeyframes = function(data) {
    if (!data) return null;
    var props = data.properties || [];
    var scopes = data.scopes || [];
    if (scopes.length > 0) {
      // `& { ... }` addresses the element itself; fall back to the first scope
      // so a body that used some other selector still contributes something
      // rather than silently producing nothing.
      var self = scopes.find(function(s) {
        return s.selector === '&' || s.selector === '&self';
      });
      props = (self ? self.properties : scopes[0].properties) || [];
    }
    if (!props.length) return null;

    var offsets = [];
    props.forEach(function(p) {
      (p.keyframes || []).forEach(function(k) {
        if (offsets.indexOf(k.at) === -1) offsets.push(k.at);
      });
    });
    offsets.sort(function(a, b) { return a - b; });

    return offsets.map(function(at) {
      var frame = { offset: at };
      props.forEach(function(p) {
        var kf = (p.keyframes || []).find(function(k) { return k.at === at; });
        if (kf) frame[p.property] = kf.value;
      });
      return frame;
    });
  };

})();
