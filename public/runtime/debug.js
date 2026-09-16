/**
 * Spacetime Debug API
 *
 * Browser debugging tools for inspecting timelines, drivers, and animation state.
 *
 * Usage:
 *   window.ST.debug.enable()          // Enable all debugging
 *   window.ST.debug.listTimelines()   // List active timelines
 *   window.ST.debug.inspectTimeline('nav') // Inspect specific timeline
 *   window.ST.debug.logDrivers()      // Log driver signal changes
 *   window.ST.debug.snapshot()        // Get current state snapshot
 */

(function() {
  'use strict';

  // Guard against multiple initialization
  if (typeof ST === 'undefined') {
    console.error('[ST.debug] Spacetime runtime not found. Load st.js first.');
    return;
  }

  if (ST.debug) {
    console.warn('[ST.debug] Debug API already initialized.');
    return;
  }

  // Debug state
  let debugEnabled = false;
  let logDriverChanges = false;
  let driverLogFilter = null;

  // Timeline tracking
  const timelines = new Map(); // name -> { driver, selector, elements, config }
  const driverInstances = new WeakMap(); // element -> driver info
  const animationStates = new WeakMap(); // element -> animation state

  /**
   * Register a timeline for debugging
   * @param {string} name - Timeline identifier
   * @param {string} driver - Driver type (scroll, time, event, load, etc.)
   * @param {string} selector - CSS selector
   * @param {Object} config - Timeline configuration
   * @param {Element[]} elements - Target elements
   */
  function registerTimeline(name, driver, selector, config, elements) {
    timelines.set(name, {
      driver,
      selector,
      config,
      elements: elements || [],
      registeredAt: Date.now()
    });

    if (debugEnabled) {
      console.log(`[ST:timeline] Registered "${name}"`, {
        driver,
        selector,
        elementCount: elements?.length || 0,
        config
      });
    }
  }

  /**
   * Register a driver instance on an element
   * @param {Element} element
   * @param {string} driver - Driver type
   * @param {Object} config - Driver configuration
   */
  function registerDriver(element, driver, config) {
    driverInstances.set(element, {
      type: driver,
      config,
      progress: 0,
      lastUpdate: Date.now()
    });
  }

  /**
   * Update driver progress (called from driver primitives)
   * @param {Element} element
   * @param {number} progress - Current progress (0-1)
   */
  function updateDriverProgress(element, progress) {
    const info = driverInstances.get(element);
    if (!info) return;

    const oldProgress = info.progress;
    info.progress = progress;
    info.lastUpdate = Date.now();

    if (logDriverChanges) {
      const shouldLog = !driverLogFilter ||
        (typeof driverLogFilter === 'function' && driverLogFilter(element, info)) ||
        (typeof driverLogFilter === 'string' && element.matches(driverLogFilter));

      if (shouldLog) {
        const selector = getElementSelector(element);
        console.log(
          `[ST:${info.type}-driver] ${selector} $progress = ${progress.toFixed(3)}`,
          `(Δ ${(progress - oldProgress).toFixed(3)})`
        );
      }
    }
  }

  /**
   * Track animation state for an element
   * @param {Element} element
   * @param {Object} state - Animation state
   */
  function setAnimationState(element, state) {
    animationStates.set(element, {
      ...state,
      timestamp: Date.now()
    });
  }

  /**
   * Get a simple selector string for an element
   * @param {Element} element
   * @returns {string}
   */
  function getElementSelector(element) {
    if (element.id) return `#${element.id}`;
    if (element.className && typeof element.className === 'string') {
      const classes = element.className.trim().split(/\s+/);
      if (classes.length > 0 && classes[0]) return `.${classes[0]}`;
    }
    return element.tagName.toLowerCase();
  }

  /**
   * Get all elements with active drivers
   * @returns {Element[]}
   */
  function getActiveElements() {
    const elements = [];
    document.querySelectorAll('*').forEach(el => {
      if (driverInstances.has(el)) {
        elements.push(el);
      }
    });
    return elements;
  }

  // Public Debug API
  const DebugAPI = {
    /**
     * Enable all debugging features
     */
    enable() {
      debugEnabled = true;
      logDriverChanges = true;
      console.log('[ST.debug] Debugging enabled');
      console.log('[ST.debug] Available commands:',
        'enable(), disable(), listTimelines(), inspectTimeline(name), logDrivers(filter), snapshot(), showOverlay(selector), purity(), showPurityOverlay()'
      );
      return 'Debugging enabled';
    },

    /**
     * Disable all debugging features
     */
    disable() {
      debugEnabled = false;
      logDriverChanges = false;
      driverLogFilter = null;
      console.log('[ST.debug] Debugging disabled');
      return 'Debugging disabled';
    },

    /**
     * List all registered timelines
     * @returns {Array}
     */
    listTimelines() {
      const list = [];
      timelines.forEach((info, name) => {
        const progress = info.elements[0] ?
          (driverInstances.get(info.elements[0])?.progress ?? 0) : 0;

        list.push({
          name,
          driver: info.driver,
          selector: info.selector,
          elementCount: info.elements.length,
          progress: parseFloat(progress.toFixed(3)),
          config: info.config
        });
      });

      console.table(list.map(item => ({
        name: item.name,
        driver: item.driver,
        selector: item.selector,
        elements: item.elementCount,
        progress: item.progress
      })));

      return list;
    },

    /**
     * Inspect a specific timeline in detail
     * @param {string} name - Timeline name
     * @returns {Object}
     */
    inspectTimeline(name) {
      const timeline = timelines.get(name);
      if (!timeline) {
        console.error(`[ST.debug] Timeline "${name}" not found`);
        console.log('Available timelines:', Array.from(timelines.keys()));
        return null;
      }

      const elements = timeline.elements.map(el => {
        const driver = driverInstances.get(el);
        const anim = animationStates.get(el);
        return {
          element: el,
          selector: getElementSelector(el),
          driver: driver ? {
            type: driver.type,
            progress: driver.progress,
            lastUpdate: new Date(driver.lastUpdate).toISOString()
          } : null,
          animation: anim || null
        };
      });

      const result = {
        name,
        driver: timeline.driver,
        selector: timeline.selector,
        config: timeline.config,
        registeredAt: new Date(timeline.registeredAt).toISOString(),
        elements
      };

      console.group(`[ST.debug] Timeline: ${name}`);
      console.log('Driver:', timeline.driver);
      console.log('Selector:', timeline.selector);
      console.log('Config:', timeline.config);
      console.log('Elements:', elements.length);
      if (elements.length > 0) {
        console.log('First Element:', elements[0]);
      }
      console.groupEnd();

      return result;
    },

    /**
     * Enable driver change logging with optional filter
     * @param {string|function} filter - CSS selector or filter function
     */
    logDrivers(filter = null) {
      logDriverChanges = true;
      driverLogFilter = filter;

      if (filter) {
        console.log(`[ST.debug] Driver logging enabled with filter:`, filter);
      } else {
        console.log('[ST.debug] Driver logging enabled for all drivers');
      }

      return 'Driver logging enabled';
    },

    /**
     * Stop driver change logging
     */
    stopLogging() {
      logDriverChanges = false;
      driverLogFilter = null;
      console.log('[ST.debug] Driver logging disabled');
      return 'Driver logging disabled';
    },

    /**
     * Get a snapshot of current animation state
     * @returns {Object}
     */
    snapshot() {
      const activeElements = getActiveElements();

      const snapshot = {
        timestamp: new Date().toISOString(),
        timelineCount: timelines.size,
        activeElements: activeElements.length,
        timelines: [],
        elements: []
      };

      // Capture timeline state
      timelines.forEach((info, name) => {
        snapshot.timelines.push({
          name,
          driver: info.driver,
          selector: info.selector,
          elementCount: info.elements.length
        });
      });

      // Capture element state
      activeElements.forEach(el => {
        const driver = driverInstances.get(el);
        const anim = animationStates.get(el);

        snapshot.elements.push({
          selector: getElementSelector(el),
          driver: driver ? {
            type: driver.type,
            progress: driver.progress,
            config: driver.config
          } : null,
          animation: anim || null,
          signals: ST.signals.get(el) || null
        });
      });

      console.group('[ST.debug] State Snapshot');
      console.log('Timestamp:', snapshot.timestamp);
      console.log('Timelines:', snapshot.timelineCount);
      console.log('Active Elements:', snapshot.activeElements);
      console.log('Full State:', snapshot);
      console.groupEnd();

      return snapshot;
    },

    /**
     * Show debug overlay for an element
     * @param {string} selector - CSS selector
     */
    showOverlay(selector) {
      if (typeof ST.debugOverlay === 'undefined') {
        console.error('[ST.debug] Debug overlay not loaded. Include debug-overlay.js');
        return null;
      }

      return ST.debugOverlay.show(selector);
    },

    /**
     * Hide all debug overlays
     */
    hideOverlay() {
      if (typeof ST.debugOverlay === 'undefined') {
        console.error('[ST.debug] Debug overlay not loaded. Include debug-overlay.js');
        return;
      }

      ST.debugOverlay.hideAll();
    },

    /**
     * Show purity cache statistics
     * @returns {Object} Current statistics
     */
    purity() {
      if (typeof ST._purity === 'undefined') {
        console.error('[ST.debug] Purity system not loaded. Include purity.js');
        return null;
      }

      const stats = ST._purity.getStats();
      const total = stats.hits + stats.misses;
      const hitRate = total > 0 ? (stats.hits / total * 100).toFixed(1) : 'N/A';

      console.group('[ST.debug] Purity Cache Statistics');
      console.table({
        'Cache Hits': stats.hits,
        'Cache Misses': stats.misses,
        'Hit Rate': hitRate + '%',
        'Invalidations': stats.invalidations,
        'Animation Starts': stats.animationStarts,
        'Animation Ends': stats.animationEnds,
        'Currently Animating': ST._purity.animating.size > 0
          ? [...ST._purity.animating].join(', ')
          : 'none'
      });
      console.groupEnd();

      return stats;
    },

    /**
     * Reset purity cache statistics
     */
    resetPurity() {
      if (typeof ST._purity === 'undefined') {
        console.error('[ST.debug] Purity system not loaded. Include purity.js');
        return;
      }

      ST._purity.resetStats();
      console.log('[ST.debug] Purity statistics reset');
    },

    /**
     * Show purity stats overlay
     */
    showPurityOverlay() {
      if (typeof ST.debugOverlay === 'undefined') {
        console.error('[ST.debug] Debug overlay not loaded. Include debug-overlay.js');
        return null;
      }

      return ST.debugOverlay.showPurity();
    },

    /**
     * Hide purity stats overlay
     */
    hidePurityOverlay() {
      if (typeof ST.debugOverlay === 'undefined') {
        console.error('[ST.debug] Debug overlay not loaded. Include debug-overlay.js');
        return;
      }

      ST.debugOverlay.hidePurity();
    },

    /**
     * Get driver info for an element
     * @param {Element} element
     * @returns {Object}
     */
    getDriverInfo(element) {
      return driverInstances.get(element) || null;
    },

    /**
     * Get animation state for an element
     * @param {Element} element
     * @returns {Object}
     */
    getAnimationState(element) {
      return animationStates.get(element) || null;
    },

    /**
     * Get all registered timelines
     * @returns {Map}
     */
    getTimelines() {
      return new Map(timelines);
    },

    /**
     * Check if debugging is enabled
     * @returns {boolean}
     */
    isEnabled() {
      return debugEnabled;
    }
  };

  // Internal API for runtime integration
  const DebugInternal = {
    registerTimeline,
    registerDriver,
    updateDriverProgress,
    setAnimationState
  };

  // Attach to ST namespace
  ST.debug = DebugAPI;
  ST._debugInternal = DebugInternal;

  // Auto-enable if URL contains ?debug
  if (typeof window !== 'undefined' && window.location.search.includes('debug')) {
    DebugAPI.enable();
    console.log('[ST.debug] Auto-enabled via ?debug query parameter');
  }

  console.log('[ST.debug] Debug API loaded. Call ST.debug.enable() to start debugging.');

})();
