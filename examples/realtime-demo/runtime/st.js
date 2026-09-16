/**
 * Spacetime Minimal Reactive Runtime
 *
 * A tiny (~50 lines) reactive core for Spacetime primitives.
 * Replaces the old complex runtime with a simple signal-based system.
 *
 * Features:
 * - Per-element signal storage (WeakMap for auto-cleanup)
 * - Batched updates via microtasks
 * - Automatic cleanup on element removal
 * - Minimal API: set, get, watch, onCleanup
 */

const ST = {
  // Signal storage per element (auto-cleanup when element is GC'd)
  signals: new WeakMap(),

  // Cleanup functions per element
  cleanup: new WeakMap(),

  // Timeline registry for @after chaining (global, not per-element)
  timelines: new Map(),

  /**
   * Register a timeline by name
   * @param {string} name - Unique timeline identifier
   * @param {Element} el - Element the timeline is attached to
   * @returns {Object} Timeline handle with complete() method
   */
  registerTimeline(name, el) {
    if (!this.timelines.has(name)) {
      this.timelines.set(name, {
        el,
        completed: false,
        listeners: new Set()
      });
    }
    return {
      complete: () => this.notifyComplete(name)
    };
  },

  /**
   * Notify that a timeline has completed
   * @param {string} name - Timeline identifier
   */
  notifyComplete(name) {
    const timeline = this.timelines.get(name);
    if (timeline) {
      timeline.completed = true;
      for (const fn of timeline.listeners) {
        try {
          fn();
        } catch (e) {
          console.error('[ST] Timeline complete callback error:', e);
        }
      }
      // Clear listeners after notifying (one-shot)
      timeline.listeners.clear();
    }
  },

  /**
   * Subscribe to timeline completion
   * @param {string} name - Timeline identifier to wait for
   * @param {Function} fn - Callback when timeline completes
   * @returns {Function} Unsubscribe function
   */
  onComplete(name, fn) {
    let timeline = this.timelines.get(name);

    // If timeline doesn't exist yet, create a placeholder
    if (!timeline) {
      timeline = { el: null, completed: false, listeners: new Set() };
      this.timelines.set(name, timeline);
    }

    // If already completed, call immediately
    if (timeline.completed) {
      queueMicrotask(() => fn());
      return () => {};
    }

    // Otherwise, add to listeners
    timeline.listeners.add(fn);
    return () => timeline.listeners.delete(fn);
  },

  /**
   * Check if a timeline has completed
   * @param {string} name - Timeline identifier
   * @returns {boolean}
   */
  isTimelineComplete(name) {
    const timeline = this.timelines.get(name);
    return timeline ? timeline.completed : false;
  },

  /**
   * Get or create signal store for element
   * @param {Element} el
   * @returns {Object} Signal store
   */
  store(el) {
    let s = this.signals.get(el);
    if (!s) {
      s = {};
      this.signals.set(el, s);
    }
    return s;
  },

  /**
   * Set signal value (batched updates via microtask)
   * @param {Element} el - Target element
   * @param {string} name - Signal name (without $)
   * @param {any} value - New value
   */
  set(el, name, value) {
    const s = this.store(el);
    if (!s[name]) {
      s[name] = { v: value, d: new Set() };
    } else if (s[name].v === value) {
      return; // No change, skip update
    } else {
      s[name].v = value;
    }
    // Batch updates via microtask
    for (const fn of s[name].d) {
      queueMicrotask(() => fn(value));
    }
  },

  /**
   * Get current signal value
   * @param {Element} el - Target element
   * @param {string} name - Signal name (without $)
   * @returns {any} Current value
   */
  get(el, name) {
    return this.store(el)[name]?.v;
  },

  /**
   * Watch signal changes
   * @param {Element} el - Target element
   * @param {string} name - Signal name (without $)
   * @param {Function} fn - Callback function
   * @returns {Function} Unsubscribe function
   */
  watch(el, name, fn) {
    const s = this.store(el);
    if (!s[name]) {
      s[name] = { v: undefined, d: new Set() };
    }
    s[name].d.add(fn);
    // Call immediately with current value if defined
    if (s[name].v !== undefined) {
      fn(s[name].v);
    }
    // Return unsubscribe function
    return () => s[name].d.delete(fn);
  },

  /**
   * Watch multiple signals
   * @param {Element} el - Target element
   * @param {string[]} names - Signal names
   * @param {Function} fn - Callback receiving all values
   * @returns {Function} Unsubscribe function
   */
  watchAll(el, names, fn) {
    const unsubs = [];
    const values = {};
    let initialized = 0;

    for (const name of names) {
      unsubs.push(
        this.watch(el, name, (v) => {
          values[name] = v;
          initialized++;
          // Only call after all signals have initial values
          if (initialized >= names.length) {
            fn(values);
          }
        })
      );
    }

    return () => unsubs.forEach((u) => u());
  },

  /**
   * Register cleanup function for element
   * @param {Element} el - Target element
   * @param {Function} fn - Cleanup function
   */
  onCleanup(el, fn) {
    let fns = this.cleanup.get(el);
    if (!fns) {
      fns = [];
      this.cleanup.set(el, fns);
    }
    fns.push(fn);
  },

  /**
   * Run all cleanup functions for element
   * @param {Element} el - Target element
   */
  dispose(el) {
    const fns = this.cleanup.get(el);
    if (fns) {
      for (const fn of fns) {
        try {
          fn();
        } catch (e) {
          console.error('[ST] Cleanup error:', e);
        }
      }
      this.cleanup.delete(el);
    }
    this.signals.delete(el);
  },

  /**
   * Initialize element with primitive setup function
   * @param {Element} el - Target element
   * @param {Function} setupFn - Setup function that returns cleanup
   */
  init(el, setupFn) {
    const cleanup = setupFn(el);
    if (typeof cleanup === 'function') {
      this.onCleanup(el, cleanup);
    }
  },

  /**
   * Apply primitive to all matching elements
   * @param {string} selector - CSS selector
   * @param {Function} setupFn - Setup function
   */
  apply(selector, setupFn) {
    document.querySelectorAll(selector).forEach((el) => {
      this.init(el, setupFn);
    });
  },

  /**
   * Derive a computed signal from dependencies
   * @param {Element} el - Target element
   * @param {string} name - Derived signal name
   * @param {string[]} deps - Dependency signal names
   * @param {Function} compute - Compute function
   */
  derive(el, name, deps, compute) {
    this.watchAll(el, deps, (values) => {
      const result = compute(values);
      this.set(el, name, result);
    });
  },

  /**
   * Bind signal to state attribute
   * @param {Element} el - Target element
   * @param {string} signalName - Signal to watch
   * @param {string} stateName - State name for data-st-state
   */
  bindState(el, signalName, stateName) {
    this.watch(el, signalName, (v) => {
      if (v) {
        el.dataset.stState = stateName;
      } else if (el.dataset.stState === stateName) {
        el.dataset.stState = '';
      }
    });
  },

  /**
   * Bind signal to CSS property
   * @param {Element} el - Target element
   * @param {string} signalName - Signal to watch
   * @param {string} property - CSS property name
   * @param {Function} [transform] - Optional transform function
   */
  bindStyle(el, signalName, property, transform) {
    this.watch(el, signalName, (v) => {
      el.style[property] = transform ? transform(v) : v;
    });
  }
};

// Auto-cleanup on element removal
if (typeof MutationObserver !== 'undefined' && typeof document !== 'undefined') {
  const observer = new MutationObserver((mutations) => {
    for (const m of mutations) {
      for (const node of m.removedNodes) {
        if (node.nodeType === Node.ELEMENT_NODE) {
          ST.dispose(node);
          // Also dispose descendants
          if (node.querySelectorAll) {
            node.querySelectorAll('*').forEach((child) => ST.dispose(child));
          }
        }
      }
    }
  });

  // Start observing when DOM is ready
  if (document.body) {
    observer.observe(document.body, { childList: true, subtree: true });
  } else {
    document.addEventListener('DOMContentLoaded', () => {
      observer.observe(document.body, { childList: true, subtree: true });
    });
  }
}

// Export for module systems
if (typeof module !== 'undefined' && module.exports) {
  module.exports = ST;
}
