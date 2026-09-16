/**
 * Purity Caching System - Phase 1
 *
 * Tracks animation state and frame invalidation for cache management.
 * A selector is "pure" (stable) when it's not currently animating,
 * allowing computed values to be cached.
 */

(function() {
  // Ensure ST global exists
  if (typeof window.ST === 'undefined') {
    window.ST = {};
  }

  /**
   * Purity tracking module
   * Manages animation state and cache invalidation
   */
  ST._purity = ST._purity || {
    /**
     * Frame counter - incremented on resize or explicit invalidation
     * Used to bust caches when layout changes
     */
    frame: 0,

    /**
     * Set of currently animating selectors
     * Selectors in this set are "impure" and should not be cached
     */
    animating: new Set(),

    /**
     * Statistics for debugging and performance monitoring
     */
    _stats: {
      hits: 0,
      misses: 0,
      invalidations: 0,
      animationStarts: 0,
      animationEnds: 0
    },

    /**
     * Dependency map: selector -> [selectors that depend on it]
     * Generated at compile-time from ElementDepGraph analysis
     */
    dependents: {},

    /**
     * Get a copy of current statistics
     * @returns {Object} Copy of stats object
     */
    getStats: function() {
      return Object.assign({}, this._stats);
    },

    /**
     * Reset all statistics to zero
     */
    resetStats: function() {
      this._stats.hits = 0;
      this._stats.misses = 0;
      this._stats.invalidations = 0;
      this._stats.animationStarts = 0;
      this._stats.animationEnds = 0;
    },

    /**
     * Mark a selector as currently animating
     * @param {string} selector - CSS selector being animated
     */
    startAnimation: function(selector) {
      this.animating.add(selector);
      this._stats.animationStarts++;
    },

    /**
     * Mark a selector as no longer animating
     * Invalidates cache since element position may have changed during animation
     * @param {string} selector - CSS selector that finished animating
     */
    endAnimation: function(selector) {
      this.animating.delete(selector);
      this._stats.animationEnds++;
      // Invalidate caches since animated element is now at new position
      this.frame++;
    },

    /**
     * Check if a selector is stable (not animating)
     * Stable selectors can have their computed values cached
     * @param {string} selector - CSS selector to check
     * @returns {boolean} true if selector is not animating
     */
    isStable: function(selector) {
      return !this.animating.has(selector);
    },

    /**
     * Invalidate all caches by incrementing frame counter
     * Called automatically on window resize
     */
    invalidate: function() {
      this.frame++;
      this._stats.invalidations++;
    },

    /**
     * Create a cached getBoundingClientRect accessor for an element
     * Returns a function that caches rect calculations based on frame and animation state
     *
     * @param {Element} el - DOM element to get rect from
     * @param {string} selector - CSS selector for animation state tracking
     * @returns {Function} Accessor function that returns cached or fresh DOMRect
     */
    createCachedRect: function(el, selector) {
      const cache = { rect: null, frame: -1 };
      const purity = this;
      return function() {
        // If animating, always recalculate (element is changing)
        if (purity.animating.has(selector)) {
          purity._stats.misses++;
          return el.getBoundingClientRect();
        }
        // If frame changed (e.g., window resize), recalculate and cache
        if (cache.frame !== purity.frame) {
          purity._stats.misses++;
          cache.rect = el.getBoundingClientRect();
          cache.frame = purity.frame;
        } else {
          purity._stats.hits++;
        }
        return cache.rect;
      };
    },

    // =========================================================================
    // Dependency-Aware Methods (Phase 4)
    // =========================================================================

    /**
     * Set the dependents map from generated code
     * Called by codegen to initialize dependency relationships
     * @param {Object} deps - Map of selector -> [dependent selectors]
     */
    setDependents: function(deps) {
      this.dependents = deps;
    },

    /**
     * Get all selectors transitively affected when this selector animates
     * Uses BFS to traverse the dependency graph
     * @param {string} selector - The animating selector
     * @returns {Array<string>} All affected selectors (including the input)
     */
    getAffectedSelectors: function(selector) {
      const affected = new Set([selector]);
      const queue = [selector];
      while (queue.length > 0) {
        const current = queue.shift();
        const deps = this.dependents[current] || [];
        for (var i = 0; i < deps.length; i++) {
          var dep = deps[i];
          if (!affected.has(dep)) {
            affected.add(dep);
            queue.push(dep);
          }
        }
      }
      return Array.from(affected);
    },

    /**
     * Start animation with dependency awareness
     * Marks both the selector and all its dependents as animating
     * @param {string} selector - The selector starting animation
     * @returns {Array<string>} All affected selectors (for use with endAnimationWithDeps)
     */
    startAnimationWithDeps: function(selector) {
      const affected = this.getAffectedSelectors(selector);
      for (var i = 0; i < affected.length; i++) {
        this.animating.add(affected[i]);
      }
      return affected;
    },

    /**
     * End animation with dependency awareness
     * Clears animation state for all affected selectors
     * @param {string} selector - The selector ending animation (for logging)
     * @param {Array<string>} affected - The affected selectors from startAnimationWithDeps
     */
    endAnimationWithDeps: function(selector, affected) {
      for (var i = 0; i < affected.length; i++) {
        this.animating.delete(affected[i]);
      }
      // Invalidate caches since animated elements are now at new positions
      this.frame++;
    }
  };

  // Listen for window resize to invalidate caches
  window.addEventListener('resize', function() {
    ST._purity.invalidate();
  });

})();
