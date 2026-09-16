/**
 * Stagger Calculation - Spacetime Runtime Module
 *
 * Calculate stagger delays for coordinated animations across multiple elements.
 * Supports various stagger patterns: first, last, center, random, grid.
 */

(function() {
  'use strict';

  const ST = window.ST;
  if (!ST) {
    console.error('[ST] Stagger requires ST to be defined first');
    return;
  }

  /**
   * Calculate stagger delay for an element
   * @param {number} index - Element index in collection
   * @param {number} total - Total elements in collection
   * @param {Object} stagger - Stagger configuration
   * @param {number} stagger.delay - Base delay between elements (ms)
   * @param {string|number} stagger.from - Start point: 'first', 'last', 'center', 'random', or index
   * @param {Array} stagger.grid - Grid dimensions [cols, rows] for radial stagger
   * @returns {number} Delay in ms for this element
   */
  ST.calculateStagger = function(index, total, stagger) {
    if (!stagger || !stagger.delay) return 0;

    const { delay, from, grid } = stagger;
    let position;

    // Grid-based stagger (radial from center)
    if (grid) {
      const [cols, rows] = grid;
      const x = index % cols;
      const y = Math.floor(index / cols);
      const centerX = (cols - 1) / 2;
      const centerY = (rows - 1) / 2;

      // Distance from center
      position = Math.sqrt(Math.pow(x - centerX, 2) + Math.pow(y - centerY, 2));
      const maxDist = Math.sqrt(Math.pow(centerX, 2) + Math.pow(centerY, 2));
      position = maxDist > 0 ? position / maxDist : 0;
    } else {
      // Linear stagger patterns
      const denominator = total - 1 || 1;

      switch (from) {
        case 'first':
          position = index / denominator;
          break;
        case 'last':
          position = (total - 1 - index) / denominator;
          break;
        case 'center':
          position = Math.abs(index - (total - 1) / 2) / ((total - 1) / 2 || 1);
          break;
        case 'random':
          position = Math.random();
          break;
        default:
          // Numeric from value - distance from that index
          if (typeof from === 'number') {
            position = Math.abs(index - from) / denominator;
          } else {
            // Default to 'first'
            position = index / denominator;
          }
      }
    }

    return position * delay;
  };

  /**
   * Create a stagger configuration object
   * @param {number} delay - Delay between elements (ms)
   * @param {Object} options - Stagger options
   * @returns {Object} Stagger configuration
   */
  ST.stagger = function(delay, options = {}) {
    return {
      delay,
      from: options.from || 'first',
      grid: options.grid || null
    };
  };

})();
