/**
 * Value Functions - Spacetime Runtime Module
 *
 * Dynamic value functions for procedural animation values.
 * Used for wave patterns, randomness, noise, and parallax effects.
 */

(function() {
  'use strict';

  const ST = window.ST;
  if (!ST) {
    console.error('[ST] Value Functions requires ST to be defined first');
    return;
  }

  /**
   * Wave function - oscillating value based on progress
   * @param {number} progress - Animation progress (0-1 or unbounded for loops)
   * @param {string} type - Wave type: 'sin' or 'cos'
   * @param {number} freq - Frequency multiplier
   * @param {number} amp - Amplitude
   * @param {number} phase - Phase offset (0-1)
   * @returns {number} Wave value
   */
  ST.stWave = function(progress, type, freq, amp, phase) {
    const fn = type === 'cos' ? Math.cos : Math.sin;
    freq = freq || 1;
    amp = amp || 1;
    phase = phase || 0;
    return fn((progress * freq + phase) * Math.PI * 2) * amp;
  };

  /**
   * Random function - seeded pseudo-random value in range
   * Uses seed for consistent randomness per element
   * @param {number} min - Minimum value
   * @param {number} max - Maximum value
   * @param {number} seed - Seed value (typically element index)
   * @returns {number} Random value in range
   */
  ST.stRandom = function(min, max, seed) {
    seed = seed || 0;
    // Simple hash function for seeded randomness
    const x = Math.sin(seed * 12.9898) * 43758.5453;
    const rand = x - Math.floor(x);
    return min + rand * (max - min);
  };

  /**
   * Simplex-like noise function (1D)
   * @param {number} freq - Noise frequency
   * @param {number} amp - Noise amplitude
   * @param {number} idx - Position/index value
   * @returns {number} Noise value
   */
  ST.stNoise = function(freq, amp, idx) {
    freq = freq || 0.5;
    amp = amp || 1;
    idx = idx || 0;
    // Offset by frequency so different frequencies produce different patterns
    // even at idx=0. The offset ensures freq is part of the noise seed.
    const x = idx * freq + freq * 0.7071;
    // Layered sine waves approximation of noise
    const n = Math.sin(x * 1.0) * 0.5 +
              Math.sin(x * 2.3) * 0.25 +
              Math.sin(x * 4.1) * 0.125;
    return n * amp;
  };

  /**
   * Parallax function - offset based on scroll/mouse progress
   * @param {number} range - Total movement range
   * @param {number} progress - Progress value (e.g., scroll position 0-1)
   * @returns {number} Parallax offset
   */
  ST.stParallax = function(range, progress) {
    return (range || 0) * (progress || 0);
  };

  /**
   * Spring physics simulation
   * @param {number} current - Current value
   * @param {number} target - Target value
   * @param {Object} config - Spring config {stiffness, damping, mass}
   * @param {Object} state - Velocity state object {v}
   * @returns {number} New value
   */
  ST.stSpring = function(current, target, config, state) {
    const { stiffness = 100, damping = 10, mass = 1 } = config || {};
    state = state || { v: 0 };

    const springForce = -stiffness * (current - target);
    const dampingForce = -damping * state.v;
    const acceleration = (springForce + dampingForce) / mass;

    state.v += acceleration * 0.016; // ~60fps timestep
    const next = current + state.v * 0.016;

    // Stop when close enough
    if (Math.abs(next - target) < 0.001 && Math.abs(state.v) < 0.001) {
      state.v = 0;
      return target;
    }

    return next;
  };

  /**
   * Velocity function - maps scroll velocity to a scaled offset
   * @param {number} velocity - Current scroll velocity in px/s (from $velocity binding)
   * @param {number} scale - Multiplier (default 1)
   * @returns {number} Scaled velocity value
   */
  ST.stVelocity = function(velocity, scale) {
    velocity = velocity || 0;
    scale = scale || 1;
    return velocity * scale;
  };

  /**
   * Resolve runtime function values marked with __st_fn_ prefix
   * These are placeholders set during compilation for dynamic values
   * @param {*} val - Value to resolve
   * @param {Object} ctx - Context {progress, elementIndex, elementRect}
   * @returns {*} Resolved value
   */
  ST.resolveValue = function(val, ctx) {
    if (typeof val !== 'string' || !val.startsWith('__st_fn_')) {
      return val;
    }

    // Parse function call: __st_fn_wave(sin,1,10px)
    const match = val.match(/^__st_fn_(\w+)\((.+)\)$/);
    if (!match) return val;

    const fnName = match[1];
    const args = match[2].split(',').map(a => {
      const trimmed = a.trim();
      const num = parseFloat(trimmed);
      return isNaN(num) ? trimmed : num;
    });

    return ST.evaluateRuntimeFunction(fnName, args, ctx);
  };

  /**
   * Evaluate a runtime function by name
   * @param {string} fnName - Function name
   * @param {Array} args - Function arguments
   * @param {Object} ctx - Context {progress, elementIndex, elementRect}
   * @returns {*} Evaluated result
   */
  ST.evaluateRuntimeFunction = function(fnName, args, ctx) {
    const { progress, elementIndex, elementRect } = ctx || {};

    switch (fnName) {
      case 'wave':
        return ST.stWave(progress, args[0], args[1], args[2], args[3]);
      case 'random':
        return ST.stRandom(args[0], args[1], elementIndex);
      case 'noise':
        return ST.stNoise(args[0], args[1], elementIndex);
      case 'parallax':
        return ST.stParallax(args[0], progress);
      case 'velocity':
        return ST.stVelocity(ctx.velocity || 0, args[0]);
      case 'spring':
        // Spring needs state management, return simple lerp for now
        return ST.lerp(args[0], args[1], progress);
      default:
        console.warn('[ST] Unknown runtime function:', fnName);
        return 0;
    }
  };

})();
