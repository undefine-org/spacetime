/// Dev Hooks & Debug Runtime Primitive
///
/// Defines two global objects that form the debug instrumentation layer:
///
///   1. window.__stDebugHook — Lightweight hooks called by the page's compiled
///      runtime (timelines.js, state machines, signals). Buffers events if the
///      debug runtime hasn't loaded yet, then replays them.
///
///   2. window.__ST_DEBUG__ — Full debug runtime with Maps for timelines,
///      signals, state machines; arrays for errors and history; a listener
///      system for the dev panel to subscribe to changes.
///
/// This primitive MUST run before all other dev primitives so that:
///   - __stDebugHook is available when page JS calls registerTimeline()
///   - __ST_DEBUG__ is available when dev-inspect polls for data
///
/// No %exports — these are fire-and-forget globals.

%primitive dev-hooks() {
  %emit js {
    // =========================================================================
    // Part 1: Debug Instrumentation Hooks (event buffering)
    // =========================================================================
    window.__stDebugHook = (function() {
      var buffer = [];
      var replayed = false;

      var tryReplay = function() {
        if (replayed || !window.__ST_DEBUG__) return;
        replayed = true;
        buffer.forEach(function(entry) {
          try {
            window.__ST_DEBUG__[entry[0]].apply(window.__ST_DEBUG__, entry[1]);
          } catch (e) {
            console.warn('[ST_DEBUG] Replay error:', entry[0], e);
          }
        });
        buffer.length = 0;
      };

      // Poll for __ST_DEBUG__ availability
      var checkInterval = setInterval(function() {
        if (window.__ST_DEBUG__) {
          tryReplay();
          clearInterval(checkInterval);
        }
      }, 10);
      setTimeout(function() { clearInterval(checkInterval); }, 5000);

      return {
        timeline: function(id, progress) {
          if (window.__ST_DEBUG__) {
            window.__ST_DEBUG__.updateProgress(id, progress);
          } else if (!replayed) {
            buffer.push(['updateProgress', [id, progress]]);
          }
        },
        signal: function(el, name, value) {
          if (window.__ST_DEBUG__) {
            window.__ST_DEBUG__.setSignal(el, name, value);
          } else if (!replayed) {
            buffer.push(['setSignal', [el, name, value]]);
          }
        },
        state: function(el, from, to, event) {
          if (window.__ST_DEBUG__) {
            window.__ST_DEBUG__.setState(el, from, to, event);
          } else if (!replayed) {
            buffer.push(['setState', [el, from, to, event]]);
          }
        },
        registerTimeline: function(id, config) {
          if (window.__ST_DEBUG__) {
            window.__ST_DEBUG__.registerTimeline(id, config);
          } else if (!replayed) {
            buffer.push(['registerTimeline', [id, config]]);
          }
        },
        registerStateMachine: function(el, initial, states) {
          if (window.__ST_DEBUG__) {
            window.__ST_DEBUG__.registerStateMachine(el, initial, states);
          } else if (!replayed) {
            buffer.push(['registerStateMachine', [el, initial, states]]);
          }
        }
      };
    })();

    // =========================================================================
    // Part 2: Debug Runtime (__ST_DEBUG__)
    // =========================================================================
    (function() {
      'use strict';

      window.__ST_DEBUG__ = {
        version: '1.0.0',

        config: {
          recordHistory: true,
          maxHistoryEntries: 1000
        },

        // Data stores
        timelines: new Map(),
        signals: new Map(),
        states: new Map(),
        errors: [],
        history: [],
        panel: null,
        listeners: new Set(),

        // =====================================================================
        // Timeline Methods
        // =====================================================================

        registerTimeline: function(id, config) {
          this.timelines.set(id, {
            id: id,
            type: config.type || 'unknown',
            progress: 0,
            scope: config.scope || null,
            range: config.range || [0, 1],
            createdAt: Date.now()
          });
          this._recordHistory('timeline.register', { id: id, config: config });
          this._notifyListeners('timeline.register', { id: id });
        },

        updateProgress: function(id, progress) {
          var timeline = this.timelines.get(id);
          if (timeline) {
            var oldProgress = timeline.progress;
            timeline.progress = Math.max(0, Math.min(1, progress));
            timeline.lastUpdate = Date.now();
            if (Math.abs(oldProgress - progress) > 0.001) {
              this._recordHistory('timeline.progress', {
                id: id,
                from: oldProgress,
                to: timeline.progress
              });
              this._notifyListeners('timeline.progress', { id: id, progress: timeline.progress });
            }
          }
        },

        getTimelines: function() {
          return Array.from(this.timelines.values());
        },

        // =====================================================================
        // Signal Methods
        // =====================================================================

        setSignal: function(el, name, value) {
          var elId = typeof el === 'string' ? el : this._getElementId(el);
          var key = elId + ':' + name;
          var existing = this.signals.get(key);
          var oldValue = existing ? existing.value : undefined;

          this.signals.set(key, {
            element: elId,
            name: name,
            value: value,
            previousValue: oldValue,
            changedAt: Date.now(),
            changeCount: existing ? existing.changeCount + 1 : 1
          });

          if (oldValue !== value) {
            this._recordHistory('signal.change', {
              element: elId,
              name: name,
              from: oldValue,
              to: value
            });
            this._notifyListeners('signal.change', { element: elId, name: name, value: value });
          }
        },

        getSignal: function(el, name) {
          var elId = typeof el === 'string' ? el : this._getElementId(el);
          var key = elId + ':' + name;
          var signal = this.signals.get(key);
          return signal ? signal.value : undefined;
        },

        getSignals: function() {
          return Array.from(this.signals.values());
        },

        // =====================================================================
        // State Machine Methods
        // =====================================================================

        registerStateMachine: function(el, initialState, states) {
          var elId = typeof el === 'string' ? el : this._getElementId(el);
          this.states.set(elId, {
            element: elId,
            current: initialState,
            previous: null,
            validStates: states || [],
            transitions: [],
            createdAt: Date.now()
          });
          this._recordHistory('state.register', {
            element: elId,
            initial: initialState,
            states: states
          });
          this._notifyListeners('state.register', { element: elId, state: initialState });
        },

        setState: function(el, from, to, event) {
          var elId = typeof el === 'string' ? el : this._getElementId(el);
          var sm = this.states.get(elId);
          if (!sm) {
            sm = {
              element: elId,
              current: to,
              previous: from,
              validStates: [],
              transitions: [],
              createdAt: Date.now()
            };
            this.states.set(elId, sm);
          }
          sm.previous = from;
          sm.current = to;
          sm.lastTransition = {
            from: from,
            to: to,
            event: event || 'unknown',
            timestamp: Date.now()
          };
          sm.transitions.push(sm.lastTransition);
          if (sm.transitions.length > 50) {
            sm.transitions = sm.transitions.slice(-50);
          }
          this._recordHistory('state.transition', {
            element: elId,
            from: from,
            to: to,
            event: event
          });
          this._notifyListeners('state.transition', { element: elId, from: from, to: to, event: event });
        },

        getStateMachines: function() {
          return Array.from(this.states.values());
        },

        // =====================================================================
        // Error Tracking
        // =====================================================================

        recordError: function(source, error, context) {
          var entry = {
            id: Date.now() + '-' + Math.random().toString(36).substr(2, 9),
            source: source,
            message: error.message || String(error),
            stack: error.stack || null,
            context: context || {},
            timestamp: Date.now()
          };
          this.errors.push(entry);
          this._recordHistory('error', entry);
          this._notifyListeners('error', entry);
          if (this.panel && typeof this.panel.onError === 'function') {
            this.panel.onError(entry);
          }
        },

        getErrors: function() {
          return this.errors.slice();
        },

        clearErrors: function() {
          this.errors = [];
          this._notifyListeners('errors.clear', null);
        },

        // =====================================================================
        // History
        // =====================================================================

        getHistory: function(limit) {
          if (limit && limit > 0) {
            return this.history.slice(-limit);
          }
          return this.history.slice();
        },

        clearHistory: function() {
          this.history = [];
          this._notifyListeners('history.clear', {});
        },

        // =====================================================================
        // Snapshot & Export
        // =====================================================================

        getSnapshot: function() {
          return {
            timestamp: Date.now(),
            timelines: this.getTimelines(),
            signals: this.getSignals(),
            stateMachines: this.getStateMachines(),
            historyLength: this.history.length
          };
        },

        exportJSON: function() {
          return JSON.stringify(this.getSnapshot(), null, 2);
        },

        // =====================================================================
        // Listener System
        // =====================================================================

        subscribe: function(callback) {
          this.listeners.add(callback);
          return function() { this.listeners.delete(callback); }.bind(this);
        },

        // =====================================================================
        // Internal Methods
        // =====================================================================

        _recordHistory: function(type, data) {
          if (!this.config.recordHistory) return;
          var entry = {
            type: type,
            data: data,
            timestamp: Date.now()
          };
          this.history.push(entry);
          if (this.config.maxHistoryEntries > 0 &&
              this.history.length > this.config.maxHistoryEntries) {
            this.history = this.history.slice(-this.config.maxHistoryEntries);
          }
        },

        _notifyListeners: function(type, data) {
          this.listeners.forEach(function(listener) {
            try {
              listener(type, data);
            } catch (e) {
              console.error('[ST_DEBUG] Listener error:', e);
            }
          });
          if (this.panel && typeof this.panel.onDebugEvent === 'function') {
            try {
              this.panel.onDebugEvent(type, data);
            } catch (e) {
              console.error('[ST_DEBUG] Panel notification error:', e);
            }
          }
        },

        _getElementId: function(el) {
          if (!el) return 'unknown';
          if (el.id) return '#' + el.id;
          if (el.dataset && el.dataset.stId) return '[data-st-id="' + el.dataset.stId + '"]';
          var classNameStr = typeof el.className === 'string' ? el.className : (el.className && el.className.baseVal);
          if (classNameStr) {
            var classes = classNameStr.split(' ').filter(function(c) { return c; }).slice(0, 2).join('.');
            if (classes) return el.tagName.toLowerCase() + '.' + classes;
          }
          return el.tagName ? el.tagName.toLowerCase() : 'unknown';
        }
      };

      // Replay any buffered events from __stDebugHook
      if (window.__stDebugHook && typeof window.__stDebugHook._replay === 'function') {
        window.__stDebugHook._replay();
      }

      // Global error handlers
      window.addEventListener('error', function(event) {
        window.__ST_DEBUG__.recordError('uncaught', event.error || new Error(event.message), {
          filename: event.filename,
          lineno: event.lineno,
          colno: event.colno
        });
      });

      window.addEventListener('unhandledrejection', function(event) {
        var error = event.reason instanceof Error
          ? event.reason
          : new Error(String(event.reason));
        window.__ST_DEBUG__.recordError('unhandled-promise', error, {});
      });
    })();
  }
}
