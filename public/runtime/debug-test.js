/**
 * Spacetime Debug API Test Suite
 *
 * Automated tests to verify debug functionality.
 * Run in browser console with: loadScript('debug-test.js')
 */

(function() {
  'use strict';

  const tests = [];
  const results = { passed: 0, failed: 0, total: 0 };

  function test(name, fn) {
    tests.push({ name, fn });
  }

  function assert(condition, message) {
    if (!condition) {
      throw new Error(message || 'Assertion failed');
    }
  }

  function assertEqual(actual, expected, message) {
    if (actual !== expected) {
      throw new Error(`${message || 'Values not equal'}: expected ${expected}, got ${actual}`);
    }
  }

  function runTests() {
    console.log('%c🧪 Running Spacetime Debug API Tests', 'font-size: 16px; font-weight: bold; color: #667eea;');
    console.log('');

    tests.forEach(({ name, fn }) => {
      results.total++;
      try {
        fn();
        results.passed++;
        console.log(`%c✓ ${name}`, 'color: #00aa00;');
      } catch (error) {
        results.failed++;
        console.error(`%c✗ ${name}`, 'color: #ff0000;');
        console.error('  ', error.message);
      }
    });

    console.log('');
    console.log('%c' + '='.repeat(50), 'color: #666;');
    console.log(`Tests: ${results.total}, Passed: ${results.passed}, Failed: ${results.failed}`);

    if (results.failed === 0) {
      console.log('%c✓ All tests passed!', 'color: #00aa00; font-weight: bold;');
    } else {
      console.log(`%c✗ ${results.failed} test(s) failed`, 'color: #ff0000; font-weight: bold;');
    }

    return results;
  }

  // Tests

  test('ST namespace exists', () => {
    assert(typeof ST !== 'undefined', 'ST namespace not found');
    assert(typeof ST.debug !== 'undefined', 'ST.debug not found');
  });

  test('Debug API methods exist', () => {
    assert(typeof ST.debug.enable === 'function', 'enable() not found');
    assert(typeof ST.debug.disable === 'function', 'disable() not found');
    assert(typeof ST.debug.listTimelines === 'function', 'listTimelines() not found');
    assert(typeof ST.debug.inspectTimeline === 'function', 'inspectTimeline() not found');
    assert(typeof ST.debug.logDrivers === 'function', 'logDrivers() not found');
    assert(typeof ST.debug.snapshot === 'function', 'snapshot() not found');
  });

  test('Debug overlay methods exist', () => {
    assert(typeof ST.debugOverlay !== 'undefined', 'ST.debugOverlay not found');
    assert(typeof ST.debugOverlay.show === 'function', 'showOverlay() not found');
    assert(typeof ST.debugOverlay.hideAll === 'function', 'hideAll() not found');
  });

  test('Enable/disable debugging', () => {
    ST.debug.disable();
    assertEqual(ST.debug.isEnabled(), false, 'Debug should be disabled');

    ST.debug.enable();
    assertEqual(ST.debug.isEnabled(), true, 'Debug should be enabled');
  });

  test('Timeline registration', () => {
    if (!ST._debugInternal) {
      throw new Error('ST._debugInternal not available');
    }

    const testElement = document.createElement('div');
    testElement.className = 'test-element';
    document.body.appendChild(testElement);

    ST._debugInternal.registerTimeline('test-timeline', 'scroll', '.test-element', {
      start: 0,
      end: 1
    }, [testElement]);

    const timelines = ST.debug.getTimelines();
    assert(timelines.has('test-timeline'), 'Timeline not registered');

    const timeline = timelines.get('test-timeline');
    assertEqual(timeline.driver, 'scroll', 'Driver type mismatch');
    assertEqual(timeline.selector, '.test-element', 'Selector mismatch');

    document.body.removeChild(testElement);
  });

  test('Driver registration and progress updates', () => {
    if (!ST._debugInternal) {
      throw new Error('ST._debugInternal not available');
    }

    const testElement = document.createElement('div');
    document.body.appendChild(testElement);

    ST._debugInternal.registerDriver(testElement, 'scroll', { start: 0, end: 1 });

    const driverInfo = ST.debug.getDriverInfo(testElement);
    assert(driverInfo !== null, 'Driver not registered');
    assertEqual(driverInfo.type, 'scroll', 'Driver type mismatch');
    assertEqual(driverInfo.progress, 0, 'Initial progress should be 0');

    ST._debugInternal.updateDriverProgress(testElement, 0.5);

    const updatedInfo = ST.debug.getDriverInfo(testElement);
    assertEqual(updatedInfo.progress, 0.5, 'Progress not updated');

    document.body.removeChild(testElement);
  });

  test('listTimelines returns array', () => {
    const timelines = ST.debug.listTimelines();
    assert(Array.isArray(timelines), 'listTimelines should return array');
  });

  test('snapshot returns valid structure', () => {
    const snapshot = ST.debug.snapshot();
    assert(typeof snapshot === 'object', 'Snapshot should be object');
    assert(typeof snapshot.timestamp === 'string', 'Snapshot should have timestamp');
    assert(typeof snapshot.timelineCount === 'number', 'Snapshot should have timelineCount');
    assert(Array.isArray(snapshot.timelines), 'Snapshot should have timelines array');
    assert(Array.isArray(snapshot.elements), 'Snapshot should have elements array');
  });

  test('inspectTimeline returns null for non-existent timeline', () => {
    const result = ST.debug.inspectTimeline('non-existent-timeline-12345');
    assertEqual(result, null, 'Should return null for non-existent timeline');
  });

  test('showOverlay returns count', () => {
    const testElement = document.createElement('div');
    testElement.className = 'overlay-test';
    document.body.appendChild(testElement);

    const count = ST.debugOverlay.show('.overlay-test');
    assertEqual(count, 1, 'Should show 1 overlay');

    ST.debugOverlay.hideAll();
    document.body.removeChild(testElement);
  });

  test('showOverlay handles non-existent selector', () => {
    const count = ST.debugOverlay.show('.non-existent-selector-12345');
    assertEqual(count, 0, 'Should return 0 for non-existent selector');
  });

  test('logDrivers with filter', () => {
    ST.debug.logDrivers('.test-filter');
    // No assertions, just verify it doesn't throw
    ST.debug.stopLogging();
  });

  test('logDrivers with function filter', () => {
    ST.debug.logDrivers((el, info) => info.type === 'scroll');
    // No assertions, just verify it doesn't throw
    ST.debug.stopLogging();
  });

  test('Multiple overlays on same element', () => {
    const testElement = document.createElement('div');
    testElement.className = 'multi-overlay-test';
    document.body.appendChild(testElement);

    const count1 = ST.debugOverlay.show('.multi-overlay-test');
    assertEqual(count1, 1, 'First overlay should be shown');

    const count2 = ST.debugOverlay.show('.multi-overlay-test');
    assertEqual(count2, 0, 'Should skip if overlay already exists');

    ST.debugOverlay.hideAll();
    document.body.removeChild(testElement);
  });

  test('Timeline with multiple elements', () => {
    if (!ST._debugInternal) {
      throw new Error('ST._debugInternal not available');
    }

    const elements = [];
    for (let i = 0; i < 3; i++) {
      const el = document.createElement('div');
      el.className = 'multi-element-test';
      document.body.appendChild(el);
      elements.push(el);
    }

    ST._debugInternal.registerTimeline('multi-element', 'scroll', '.multi-element-test', {}, elements);

    const timeline = ST.debug.inspectTimeline('multi-element');
    assert(timeline !== null, 'Timeline should exist');
    assertEqual(timeline.elements.length, 3, 'Should have 3 elements');

    elements.forEach(el => document.body.removeChild(el));
  });

  test('Animation state tracking', () => {
    if (!ST._debugInternal) {
      throw new Error('ST._debugInternal not available');
    }

    const testElement = document.createElement('div');
    document.body.appendChild(testElement);

    ST._debugInternal.setAnimationState(testElement, {
      properties: { opacity: 0.5, transform: 'translateX(10px)' }
    });

    const state = ST.debug.getAnimationState(testElement);
    assert(state !== null, 'Animation state should exist');
    assert(typeof state.timestamp === 'number', 'Should have timestamp');
    assert(state.properties.opacity === 0.5, 'Should track opacity');

    document.body.removeChild(testElement);
  });

  test('Cleanup on element removal', () => {
    if (!ST._debugInternal) {
      throw new Error('ST._debugInternal not available');
    }

    const testElement = document.createElement('div');
    testElement.className = 'cleanup-test';
    document.body.appendChild(testElement);

    ST._debugInternal.registerDriver(testElement, 'scroll', {});

    const beforeRemoval = ST.debug.getDriverInfo(testElement);
    assert(beforeRemoval !== null, 'Driver should exist before removal');

    document.body.removeChild(testElement);

    // WeakMap should still hold reference until GC
    // This test just verifies no errors occur
  });

  // Run all tests
  window.runDebugTests = runTests;

  // Auto-run if not in silent mode
  if (!window.location.search.includes('silent')) {
    setTimeout(runTests, 100);
  } else {
    console.log('Debug tests loaded. Run with: runDebugTests()');
  }

})();
