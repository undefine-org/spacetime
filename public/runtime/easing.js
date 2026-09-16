/**
 * Easing Functions - Spacetime Runtime Module
 *
 * 28 easing functions for smooth animations.
 * Based on Robert Penner's easing equations.
 */

(function() {
  'use strict';

  const ST = window.ST;
  if (!ST) {
    console.error('[ST] Easing requires ST to be defined first');
    return;
  }

  const easings = {
    // Linear
    linear: t => t,

    // Quadratic
    easeInQuad: t => t * t,
    easeOutQuad: t => t * (2 - t),
    easeInOutQuad: t => t < 0.5 ? 2 * t * t : -1 + (4 - 2 * t) * t,

    // Cubic
    easeInCubic: t => t * t * t,
    easeOutCubic: t => (--t) * t * t + 1,
    easeInOutCubic: t => t < 0.5 ? 4 * t * t * t : (t - 1) * (2 * t - 2) * (2 * t - 2) + 1,

    // Quartic
    easeInQuart: t => t * t * t * t,
    easeOutQuart: t => 1 - (--t) * t * t * t,
    easeInOutQuart: t => t < 0.5 ? 8 * t * t * t * t : 1 - 8 * (--t) * t * t * t,

    // Quintic
    easeInQuint: t => t * t * t * t * t,
    easeOutQuint: t => 1 + (--t) * t * t * t * t,
    easeInOutQuint: t => t < 0.5 ? 16 * t * t * t * t * t : 1 + 16 * (--t) * t * t * t * t,

    // Sine
    easeInSine: t => 1 - Math.cos(t * Math.PI / 2),
    easeOutSine: t => Math.sin(t * Math.PI / 2),
    easeInOutSine: t => -(Math.cos(Math.PI * t) - 1) / 2,

    // Exponential
    easeInExpo: t => t === 0 ? 0 : Math.pow(2, 10 * (t - 1)),
    easeOutExpo: t => t === 1 ? 1 : 1 - Math.pow(2, -10 * t),
    easeInOutExpo: t => {
      if (t === 0 || t === 1) return t;
      return t < 0.5 ? Math.pow(2, 20 * t - 10) / 2 : (2 - Math.pow(2, -20 * t + 10)) / 2;
    },

    // Circular
    easeInCirc: t => 1 - Math.sqrt(1 - t * t),
    easeOutCirc: t => Math.sqrt(1 - (--t) * t),
    easeInOutCirc: t => t < 0.5
      ? (1 - Math.sqrt(1 - 4 * t * t)) / 2
      : (Math.sqrt(1 - Math.pow(-2 * t + 2, 2)) + 1) / 2,

    // Back (overshoot)
    easeInBack: t => 2.70158 * t * t * t - 1.70158 * t * t,
    easeOutBack: t => 1 + 2.70158 * Math.pow(t - 1, 3) + 1.70158 * Math.pow(t - 1, 2),
    easeInOutBack: t => {
      const c = 1.70158 * 1.525;
      return t < 0.5
        ? (Math.pow(2 * t, 2) * ((c + 1) * 2 * t - c)) / 2
        : (Math.pow(2 * t - 2, 2) * ((c + 1) * (t * 2 - 2) + c) + 2) / 2;
    },

    // Elastic
    easeInElastic: t => {
      if (t === 0 || t === 1) return t;
      return -Math.pow(2, 10 * t - 10) * Math.sin((t * 10 - 10.75) * (2 * Math.PI / 3));
    },
    easeOutElastic: t => {
      if (t === 0 || t === 1) return t;
      return Math.pow(2, -10 * t) * Math.sin((t * 10 - 0.75) * (2 * Math.PI / 3)) + 1;
    },
    easeInOutElastic: t => {
      if (t === 0 || t === 1) return t;
      return t < 0.5
        ? -(Math.pow(2, 20 * t - 10) * Math.sin((20 * t - 11.125) * (2 * Math.PI / 4.5))) / 2
        : (Math.pow(2, -20 * t + 10) * Math.sin((20 * t - 11.125) * (2 * Math.PI / 4.5))) / 2 + 1;
    },

    // Bounce
    easeInBounce: t => 1 - easings.easeOutBounce(1 - t),
    easeOutBounce: t => {
      const n1 = 7.5625, d1 = 2.75;
      if (t < 1 / d1) return n1 * t * t;
      if (t < 2 / d1) return n1 * (t -= 1.5 / d1) * t + 0.75;
      if (t < 2.5 / d1) return n1 * (t -= 2.25 / d1) * t + 0.9375;
      return n1 * (t -= 2.625 / d1) * t + 0.984375;
    },
    easeInOutBounce: t => t < 0.5
      ? (1 - easings.easeOutBounce(1 - 2 * t)) / 2
      : (1 + easings.easeOutBounce(2 * t - 1)) / 2,

    // Spring (damped harmonic oscillator)
    springGentle: function(t) {
      var w = Math.sqrt(170 - 169);
      return 1 - Math.exp(-13 * t) * Math.cos(w * t);
    },
    springBouncy: function(t) {
      var w = Math.sqrt(300 - 25);
      return 1 - Math.exp(-5 * t) * Math.cos(w * t);
    },
    springStiff: function(t) {
      var w = Math.sqrt(400 - 196);
      return 1 - Math.exp(-14 * t) * Math.cos(w * t);
    }
  };

  // Aliases for common CSS naming conventions
  easings['ease'] = easings.easeInOutQuad;
  easings['ease-in'] = easings.easeInQuad;
  easings['ease-out'] = easings.easeOutQuad;
  easings['ease-in-out'] = easings.easeInOutQuad;

  // Also support camelCase with lowercase start
  easings['easeIn'] = easings.easeInQuad;
  easings['easeOut'] = easings.easeOutQuad;
  easings['easeInOut'] = easings.easeInOutQuad;

  // BUG-297: the dev server loads SEVERAL bundles that each vendor this file
  // and share one window.ST (site runtime, then comments/migrations/inspect/
  // host). A late copy re-assigning ST.easings wholesale WIPES custom curves
  // a page registered via ST.registerEasing — silently back to linear. Merge
  // instead: this copy's stdlib definitions win for the names it knows;
  // anything it does NOT define (page-registered custom curves) survives.
  var prior = ST.easings;
  if (prior) {
    for (var priorName in prior) {
      if (!(priorName in easings)) easings[priorName] = prior[priorName];
    }
  }
  ST.easings = easings;

  /**
   * CSS cubic-bezier solver — the factory a custom `@form easing --name
   * { cubic-bezier(a, b, c, d) }` declaration registers through (BUG-297).
   * Newton-Raphson with a bisection fallback, the standard CSS algorithm.
   */
  ST.cubicBezier = function(p1x, p1y, p2x, p2y) {
    var cx = 3 * p1x, bx = 3 * (p2x - p1x) - cx, ax = 1 - cx - bx;
    var cy = 3 * p1y, by = 3 * (p2y - p1y) - cy, ay = 1 - cy - by;
    function sampleX(t) { return ((ax * t + bx) * t + cx) * t; }
    function sampleY(t) { return ((ay * t + by) * t + cy) * t; }
    function sampleDX(t) { return (3 * ax * t + 2 * bx) * t + cx; }
    function solveX(x) {
      var t = x;
      for (var i = 0; i < 8; i++) {
        var err = sampleX(t) - x;
        if (Math.abs(err) < 1e-6) return t;
        var d = sampleDX(t);
        if (Math.abs(d) < 1e-6) break;
        t -= err / d;
      }
      var lo = 0, hi = 1;
      t = x;
      while (lo < hi) {
        var m = (lo + hi) / 2;
        if (Math.abs(sampleX(m) - x) < 1e-6) return m;
        if (sampleX(m) < x) lo = m; else hi = m;
      }
      return t;
    }
    return function(x) {
      if (x <= 0) return 0;
      if (x >= 1) return 1;
      return sampleY(solveX(x));
    };
  };

  /**
   * Damped-harmonic spring factory — a custom `@form easing --name
   * { spring(stiffness, damping, mass) }` registers through here (BUG-297).
   * Underdamped canonical form over normalized progress t in [0,1]; the
   * exponent settles the curve to ~1 by t=1 for any physical parameter set.
   * Critically/overdamped sets use the critical form (no overshoot).
   */
  ST.springCurve = function(stiffness, damping, mass) {
    var m = mass > 0 ? mass : 1;
    var omega0 = Math.sqrt(stiffness / m);
    var zeta = damping / (2 * Math.sqrt(stiffness * m));
    if (zeta < 1) {
      var omegad = omega0 * Math.sqrt(1 - zeta * zeta);
      var zw = zeta * omega0;
      return function(t) {
        if (t <= 0) return 0;
        if (t >= 1) return 1;
        return 1 - Math.exp(-zw * t) * (Math.cos(omegad * t) + (zw / omegad) * Math.sin(omegad * t));
      };
    }
    return function(t) {
      if (t <= 0) return 0;
      if (t >= 1) return 1;
      return 1 - Math.exp(-omega0 * t) * (1 + omega0 * t);
    };
  };

  /**
   * Register a custom easing curve under its form name (BUG-297). The
   * compiler emits one registration per `@form easing` declaration in the
   * page's merged AST, so getEasing resolves the AUTHOR'S curve — never the
   * silent linear fallback that made BUG-297 invisible.
   */
  ST.registerEasing = function(name, fn) {
    easings[name] = fn;
  };

  /**
   * Get an easing function by name
   * Supports camelCase (easeOutExpo) and kebab-case (ease-out-expo) naming.
   * @param {string} name - Easing function name
   * @returns {Function} Easing function (defaults to linear)
   */
  ST.getEasing = function(name) {
    if (typeof name === 'function') return name;
    if (easings[name]) return easings[name];
    // PLAN-150 W1: `steps(N)` — a stepped curve of N equal jumps (the
    // typewriter / frame-counter easing of docs/language/film.st.md §2).
    // A jump-end staircase: progress advances only at the end of each of the
    // N segments, matching CSS `steps(N, jump-end)`. Parsed here so a literal
    // step count needs no per-instance registration.
    if (typeof name === 'string') {
      const m = name.match(/^steps\((\d+)\)$/);
      if (m) {
        const n = Math.max(1, parseInt(m[1], 10));
        return function(t) {
          if (t <= 0) return 0;
          if (t >= 1) return 1;
          return Math.floor(t * n) / n;
        };
      }
    }
    // Convert kebab-case to camelCase: "ease-out-expo" -> "easeOutExpo"
    if (typeof name === 'string' && name.includes('-')) {
      const camel = name.replace(/-([a-z])/g, function(_, c) { return c.toUpperCase(); });
      if (easings[camel]) return easings[camel];
    }
    return easings.linear;
  };

  // Also expose on ST.ease for backwards compatibility
  ST.ease = {
    linear: easings.linear,
    easeIn: easings.easeInQuad,
    easeOut: easings.easeOutQuad,
    easeInOut: easings.easeInOutQuad,
    easeInCubic: easings.easeInCubic,
    easeOutCubic: easings.easeOutCubic,
    easeInOutCubic: easings.easeInOutCubic
  };

})();
