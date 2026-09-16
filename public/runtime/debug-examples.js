/**
 * Spacetime Debug API - Example Usage
 *
 * Copy-paste examples for common debugging scenarios.
 * Load this file in the console or include in your page.
 */

const DebugExamples = {
  /**
   * Basic Setup - Enable debugging
   */
  basicSetup() {
    console.log('=== BASIC SETUP ===');
    ST.debug.enable();
    console.log('Debugging enabled. Try: ST.debug.listTimelines()');
  },

  /**
   * List all active timelines
   */
  listAllTimelines() {
    console.log('=== ALL TIMELINES ===');
    const timelines = ST.debug.listTimelines();
    console.log('Found ' + timelines.length + ' timeline(s)');
    return timelines;
  },

  /**
   * Monitor scroll animations
   */
  monitorScroll() {
    console.log('=== MONITOR SCROLL ===');
    ST.debug.enable();
    ST.debug.logDrivers((el, info) => {
      return info.type === 'scroll';
    });
    console.log('Scroll the page to see driver updates...');
  },

  /**
   * Monitor hover animations
   */
  monitorHover() {
    console.log('=== MONITOR HOVER ===');
    ST.debug.enable();
    ST.debug.logDrivers((el, info) => {
      return info.type === 'hover';
    });
    console.log('Hover over elements to see driver updates...');
  },

  /**
   * Show overlays on all cards
   */
  showCardOverlays() {
    console.log('=== SHOW CARD OVERLAYS ===');
    const count = ST.debug.showOverlay('.card, .demo-card, .ik-project');
    console.log('Showing ' + count + ' overlay(s)');
    return count;
  },

  /**
   * Inspect hero animation
   */
  inspectHero() {
    console.log('=== INSPECT HERO ===');

    // Find timelines with 'hero' in the name
    const timelines = ST.debug.listTimelines();
    const heroTimelines = timelines.filter(t =>
      t.name.toLowerCase().includes('hero')
    );

    console.log('Found ' + heroTimelines.length + ' hero timeline(s)');

    if (heroTimelines.length > 0) {
      const timeline = ST.debug.inspectTimeline(heroTimelines[0].name);
      console.log('Hero timeline:', timeline);
      return timeline;
    }

    return null;
  },

  /**
   * Performance check
   */
  performanceCheck() {
    console.log('=== PERFORMANCE CHECK ===');
    const snapshot = ST.debug.snapshot();

    console.log('Performance Overview:');
    console.log('- Timelines:', snapshot.timelineCount);
    console.log('- Active elements:', snapshot.activeElements);
    console.log('- Timestamp:', snapshot.timestamp);

    // Count by driver type
    const byType = {};
    snapshot.elements.forEach(el => {
      if (el.driver) {
        byType[el.driver.type] = (byType[el.driver.type] || 0) + 1;
      }
    });

    console.log('- By driver type:', byType);

    return snapshot;
  },

  /**
   * Export state to localStorage
   */
  exportState() {
    console.log('=== EXPORT STATE ===');
    const state = ST.debug.snapshot();
    const json = JSON.stringify(state, null, 2);

    try {
      localStorage.setItem('spacetime-debug-snapshot', json);
      console.log('State exported to localStorage');
      console.log('Retrieve with: JSON.parse(localStorage.getItem("spacetime-debug-snapshot"))');
      return state;
    } catch (e) {
      console.error('Failed to export state:', e.message);
      console.log('State too large for localStorage. Logging to console instead:');
      console.log(state);
      return state;
    }
  },

  /**
   * Find slow animations (> 1 second)
   */
  findSlowAnimations() {
    console.log('=== FIND SLOW ANIMATIONS ===');
    const timelines = ST.debug.listTimelines();

    const slow = timelines.filter(t => {
      return t.config && t.config.duration > 1000;
    });

    console.log('Found ' + slow.length + ' slow animation(s):');
    console.table(slow.map(t => ({
      name: t.name,
      duration: (t.config ? t.config.duration : 0) + 'ms',
      driver: t.driver
    })));

    return slow;
  },

  /**
   * Track animation updates for 5 seconds
   */
  trackUpdates(duration = 5000) {
    console.log('=== TRACK UPDATES (' + duration + 'ms) ===');

    let updateCount = 0;
    const startTime = Date.now();

    const originalUpdate = ST._debugInternal && ST._debugInternal.updateDriverProgress;
    if (!originalUpdate) {
      console.error('Debug internal API not available');
      return;
    }

    ST._debugInternal.updateDriverProgress = function(...args) {
      updateCount++;
      return originalUpdate.apply(this, args);
    };

    console.log('Tracking started. Interact with the page...');

    setTimeout(() => {
      ST._debugInternal.updateDriverProgress = originalUpdate;

      const elapsed = (Date.now() - startTime) / 1000;
      const fps = updateCount / elapsed;

      console.log('Tracking complete:');
      console.log('- Total updates: ' + updateCount);
      console.log('- Duration: ' + elapsed.toFixed(2) + 's');
      console.log('- Average FPS: ' + fps.toFixed(2));

      return {
        updates: updateCount,
        duration: elapsed,
        fps: fps
      };
    }, duration);
  },

  /**
   * Debug specific element
   */
  debugElement(selector) {
    console.log('=== DEBUG ELEMENT: ' + selector + ' ===');

    const element = document.querySelector(selector);
    if (!element) {
      console.error('Element not found: ' + selector);
      return null;
    }

    const driverInfo = ST.debug.getDriverInfo(element);
    const animState = ST.debug.getAnimationState(element);
    const signals = ST.signals.get(element);

    console.log('Element:', element);
    console.log('Driver:', driverInfo);
    console.log('Animation state:', animState);
    console.log('Signals:', signals);

    // Show overlay
    ST.debug.showOverlay(selector);

    return {
      element: element,
      driver: driverInfo,
      animation: animState,
      signals: signals
    };
  },

  /**
   * Compare before/after state
   */
  compareStates(before, after) {
    console.log('=== STATE COMPARISON ===');

    if (!before || !after) {
      console.error('Need two snapshots to compare');
      console.log('Usage: const before = ST.debug.snapshot()');
      console.log('       ... interact with page ...');
      console.log('       const after = ST.debug.snapshot()');
      console.log('       DebugExamples.compareStates(before, after)');
      return;
    }

    console.log('Timeline count:', before.timelineCount, '->', after.timelineCount);
    console.log('Active elements:', before.activeElements, '->', after.activeElements);

    // Find new timelines
    const beforeNames = new Set(before.timelines.map(t => t.name));
    const afterNames = new Set(after.timelines.map(t => t.name));

    const added = after.timelines.filter(t => !beforeNames.has(t.name));
    const removed = before.timelines.filter(t => !afterNames.has(t.name));

    if (added.length > 0) {
      console.log('Added timelines:', added.map(t => t.name));
    }
    if (removed.length > 0) {
      console.log('Removed timelines:', removed.map(t => t.name));
    }

    return { added: added, removed: removed };
  },

  /**
   * Run all examples
   */
  runAll() {
    console.clear();
    console.log('%c🎬 SPACETIME DEBUG EXAMPLES', 'font-size: 20px; font-weight: bold; color: #667eea;');
    console.log('');

    this.basicSetup();
    console.log('');

    this.listAllTimelines();
    console.log('');

    this.performanceCheck();
    console.log('');

    console.log('Other examples available:');
    console.log('- DebugExamples.monitorScroll()');
    console.log('- DebugExamples.monitorHover()');
    console.log('- DebugExamples.showCardOverlays()');
    console.log('- DebugExamples.inspectHero()');
    console.log('- DebugExamples.debugElement(selector)');
    console.log('- DebugExamples.exportState()');
    console.log('- DebugExamples.trackUpdates(5000)');
  }
};

// Make available globally
window.DebugExamples = DebugExamples;

// Auto-run basic setup if debug is enabled
if (typeof ST !== 'undefined' && ST.debug && ST.debug.isEnabled && ST.debug.isEnabled()) {
  console.log('%c📚 Debug Examples loaded!', 'color: #00aa00; font-weight: bold;');
  console.log('Try: DebugExamples.runAll()');
} else {
  console.log('%c📚 Debug Examples loaded!', 'color: #666;');
  console.log('Enable debugging first: ST.debug.enable()');
  console.log('Then try: DebugExamples.runAll()');
}
