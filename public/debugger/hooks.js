/**
 * Spacetime Debug Instrumentation Hooks
 *
 * These hooks call into `__ST_DEBUG__` when present, allowing the debug
 * runtime to track state changes without modifying the core runtime behavior.
 * Events are buffered if `__ST_DEBUG__` isn't ready yet, then replayed.
 */
window.__stDebugHook = (function() {
  // Buffer for events that occur before __ST_DEBUG__ is ready
  const buffer = [];
  let replayed = false;

  // Try to replay buffered events when __ST_DEBUG__ becomes available
  const tryReplay = () => {
    if (replayed || !window.__ST_DEBUG__) return;
    replayed = true;
    buffer.forEach(([method, args]) => {
      try {
        window.__ST_DEBUG__[method](...args);
      } catch (e) {
        console.warn('[ST_DEBUG] Replay error:', method, e);
      }
    });
    buffer.length = 0; // Clear buffer
  };

  // Check for __ST_DEBUG__ periodically until it's available
  const checkInterval = setInterval(() => {
    if (window.__ST_DEBUG__) {
      tryReplay();
      clearInterval(checkInterval);
    }
  }, 10);
  // Stop checking after 5 seconds
  setTimeout(() => clearInterval(checkInterval), 5000);

  return {
    timeline: (id, progress) => {
      if (window.__ST_DEBUG__) {
        window.__ST_DEBUG__.updateProgress(id, progress);
      } else if (!replayed) {
        buffer.push(['updateProgress', [id, progress]]);
      }
    },
    signal: (el, name, value) => {
      if (window.__ST_DEBUG__) {
        window.__ST_DEBUG__.setSignal(el, name, value);
      } else if (!replayed) {
        buffer.push(['setSignal', [el, name, value]]);
      }
    },
    state: (el, from, to, event) => {
      if (window.__ST_DEBUG__) {
        window.__ST_DEBUG__.setState(el, from, to, event);
      } else if (!replayed) {
        buffer.push(['setState', [el, from, to, event]]);
      }
    },
    registerTimeline: (id, config) => {
      if (window.__ST_DEBUG__) {
        window.__ST_DEBUG__.registerTimeline(id, config);
      } else if (!replayed) {
        buffer.push(['registerTimeline', [id, config]]);
      }
    },
    registerStateMachine: (el, initial, states) => {
      if (window.__ST_DEBUG__) {
        window.__ST_DEBUG__.registerStateMachine(el, initial, states);
      } else if (!replayed) {
        buffer.push(['registerStateMachine', [el, initial, states]]);
      }
    },
  };
})();
