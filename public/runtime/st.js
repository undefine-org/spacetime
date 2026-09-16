/**
 * Spacetime Core Runtime
 *
 * The minimal reactive core for Spacetime primitives.
 * This module must be loaded first - other runtime modules extend it.
 *
 * Features:
 * - Per-element signal storage (WeakMap for auto-cleanup)
 * - Batched updates via microtasks
 * - Automatic cleanup on element removal
 * - Core API: set, get, watch, watchAll, derive, onCleanup, dispose
 */

(function(global) {
  'use strict';

  // Polyfill queueMicrotask for environments that don't have it (e.g., Boa)
  if (typeof global.queueMicrotask !== 'function') {
    global.queueMicrotask = function(callback) {
      Promise.resolve().then(callback).catch(function(e) {
        setTimeout(function() { throw e; }, 0);
      });
    };
  }

  // Create or get existing ST namespace
  const ST = global.ST || {};

  // ==========================================================================
  // Core Signal Storage
  // ==========================================================================

  // Signal storage per element (auto-cleanup when element is GC'd).
  // IDEMPOTENT (BUG: second-bundle clobber): a later runtime bundle on the same
  // page (host/migrations dev widgets embed a full runtime copy) reuses ST, so
  // stateful registries MUST be reused — resetting ST.signals orphans every
  // element's watcher store and freezes in-flight animations.
  ST.signals = ST.signals || new WeakMap();

  // Signal publishers by name. Strong element references are intentionally pruned
  // by consumers that need discovery; writes stay one Map/Set lookup plus add.
  ST._published = ST._published || new Map();

  // Cleanup functions per element
  ST.cleanups = ST.cleanups || new WeakMap();

  // Bridged exports (BUG-069): names that an element-scoped signal (ST.set) must
  // also mirror into the body-scope `_localState` plane so file-scope bindings
  // (`text: $lines`) — which read `SpacetimeLocal[name]` and listen for
  // `local:<name>:updated` — observe a primitive's `%yield $name`. Only names a
  // binding actually consumes are registered (via `ST._bridgeExport`), so the
  // two reactive planes stay disjoint for the common element-only case and we
  // never pollute the global namespace with per-element signals like $progress.
  ST._bridgedExports = ST._bridgedExports || new Set();
  ST._bridgeExport = function(name) { this._bridgedExports.add(name); };

  // Signal-call registry (PLAN-053): the by-NAME invocation rail for a
  // `@data signal`/`@data stream` callable. The signal-call primitive yields a
  // `fire` fn; registering it here lets a mutation/effect body invoke the signal
  // with the documented surface `@on click { $sig(args) }` (which lowers to a
  // bare `$sig(args)` statement run through ST.runMutations). Without this rail
  // a `$sig(...)` statement matched no runMutations branch and silently dropped.
  ST._signalCalls = ST._signalCalls || {};
  ST.registerSignalCall = function(name, fn) {
    if (!name || typeof fn !== 'function') return;
    this._signalCalls[String(name).replace(/^\$/, '')] = fn;
  };
  ST.getSignalCall = function(name) {
    return this._signalCalls[String(name).replace(/^\$/, '')] || null;
  };

  // Canonical body-scope (`SpacetimeLocal`) Proxy factory. The reactive write-trap
  // dispatches `local:<name>:updated`, unwraps binding objects, and mirrors a
  // CSS var — identical semantics to stdlib `local-state-impl`, kept in ONE place
  // so the export bridge (BUG-069) and local-state never create divergent proxies.
  // Idempotent: if a proxy already exists it is left untouched.
  ST._ensureLocalProxy = function() {
    if (typeof window === 'undefined') return;
    if (!window._localState) window._localState = {};
    if (window.SpacetimeLocal && window.SpacetimeLocal.__stProxy) return;
    // Migrate any plain object a non-proxy writer left behind.
    if (window.SpacetimeLocal && !window.SpacetimeLocal.__stProxy) {
      for (const k in window.SpacetimeLocal) {
        if (Object.prototype.hasOwnProperty.call(window.SpacetimeLocal, k)) {
          window._localState[k] = window.SpacetimeLocal[k];
        }
      }
    }
    window.SpacetimeLocal = new Proxy(window._localState, {
      set(target, prop, value) {
        const current = target[prop];
        if (current && current.__stBinding) { current.set(value); return true; }
        target[prop] = value;
        document.dispatchEvent(new CustomEvent('local:' + String(prop) + ':updated', {
          detail: value, bubbles: true, composed: true
        }));
        document.dispatchEvent(new CustomEvent('local:updated', {
          detail: { name: prop, value }, bubbles: true, composed: true
        }));
        document.documentElement.style.setProperty(
          '--st-' + String(prop), Array.isArray(value) ? value.length : value
        );
        return true;
      },
      get(target, prop) {
        if (prop === '__stProxy') return true;
        const val = target[prop];
        if (val && val.__stBinding) return val.get();
        return val;
      }
    });
  };

  // Timelines are PUBLISHED PROGRESS SIGNALS (PLAN-126): drivers publish
  // progress under their name into scope; read via ST.resolve(el, name).
  // The string-keyed ST.timelines registry is deleted.

  // Data registry for @data/@each wiring (mirrors timeline pattern).
  // IDEMPOTENT: `const ST = global.ST || {}` reuses an existing ST, so when a SECOND
  // runtime bundle loads on the same page (e.g. the dev-tools runtime after the page
  // runtime, --debug), these registries must NOT be reset — doing so wipes data sources
  // and selector-inits the first bundle already registered (the on-page @richtext src
  // for FUP-047 lived here and was being clobbered). Reuse the existing map/array/set.
  ST._dataRegistry = ST._dataRegistry || new Map();  // name -> { data, loaded, listeners: Set }

  // Selector-based initializer registry for dynamic elements (@each/@template)
  ST._selectorInitializers = ST._selectorInitializers || [];

  // Pending-init queue for batched MutationObserver processing
  ST._pendingInit = ST._pendingInit || new Set();
  if (typeof ST._initScheduled !== 'boolean') ST._initScheduled = false;

  // Debug mode
  ST.debug = false;

  // ==========================================================================
  // Global Cleanup Infrastructure
  // ==========================================================================

  ST._cleanup = ST._cleanup || {
    controller: null,
    handlers: [],
    destroyed: false
  };

  /**
   * Initialize cleanup controller (for AbortSignal-based cleanup)
   * @returns {AbortSignal} Abort signal for event listeners
   */
  ST.initCleanup = function() {
    if (!this._cleanup.controller) {
      this._cleanup.controller = new AbortController();
    }
    return this._cleanup.controller.signal;
  };

  /**
   * Register a global cleanup handler
   * @param {Function} fn - Cleanup function
   */
  ST.addCleanupHandler = function(fn) {
    this._cleanup.handlers.push(fn);
  };

  /**
   * Destroy all timelines and run global cleanup
   */
  ST.destroy = function() {
    if (this._cleanup.destroyed) return;
    this._cleanup.destroyed = true;

    // Abort all signal-based listeners
    if (this._cleanup.controller) {
      this._cleanup.controller.abort();
    }

    // Run registered cleanup handlers
    this._cleanup.handlers.forEach(fn => {
      try { fn(); } catch (e) { console.error('[ST] Cleanup error:', e); }
    });

    this._cleanup.handlers = [];
  };

  // ==========================================================================
  // Selector-Based Dynamic Element Initialization
  // ==========================================================================

  /**
   * Register a selector-based initializer for dynamic elements.
   * Used by generated code to support @each/@template created elements.
   * @param {string} selector - CSS selector to match
   * @param {Function} init - Initializer function(el)
   */
  ST.registerSelectorInit = function(selector, init) {
    this._selectorInitializers.push({ selector, init });
  };

  /**
   * Flush all pending element initializations.
   * Iterates each selector once across all pending nodes instead of
   * re-running querySelectorAll per selector per node.
   * @private
   */
  ST._flushInit = function() {
    const pending = ST._pendingInit;
    ST._pendingInit = new Set();

    if (pending.size === 0) {
      ST._initScheduled = false;
      return;
    }

    // Deduplicate pending roots: skip nodes that are descendants of another pending
    // node — the ancestor's querySelectorAll will cover them. This avoids redundant
    // subtree traversals when large sections (hero, tabs) are added at once.
    // O(n²) on pending count but pending is typically < 20 nodes per flush.
    const roots = [];
    for (const node of pending) {
      let dominated = false;
      for (const other of pending) {
        if (other !== node && other.contains && other.contains(node)) {
          dominated = true;
          break;
        }
      }
      if (!dominated) roots.push(node);
    }

    for (const { selector, init } of ST._selectorInitializers) {
      for (const root of roots) {
        if (root.matches && root.matches(selector)) {
          try {
            init(root);
          } catch (e) {
            // An init that throws must not die silently: the page keeps
            // "working" with a piece of behavior simply absent — the runtime
            // shape of the banned silent-drop class (BUG-271: a missing
            // AbortController killed a driver subscription invisibly).
            console.error('[ST] Init error for', selector, e);
          }
        }
        if (root.querySelectorAll) {
          root.querySelectorAll(selector).forEach(child => {
            try {
              init(child);
            } catch (e) {
              // Same rule: init errors are always reported, never swallowed.
              console.error('[ST] Init error for', selector, e);
            }
          });
        }
      }
    }

    // Set false AFTER processing so that any DOM mutations triggered by init
    // callbacks (which cause _scheduleInit to run) accumulate in _pendingInit
    // rather than spawning a concurrent flush microtask.
    ST._initScheduled = false;
    if (ST._pendingInit.size > 0) {
      ST._initScheduled = true;
      queueMicrotask(function() { ST._flushInit(); });
    }
  };

  /**
   * Schedule a node for batched initialization via microtask.
   * Multiple nodes queued within the same microtask checkpoint are flushed
   * together, reducing selector queries from O(N×M) to O(M) per flush.
   * @param {Element} node - Element to initialize
   */
  ST._scheduleInit = function(node) {
    if (!node || !node.nodeType || node.nodeType !== Node.ELEMENT_NODE) return;
    this._pendingInit.add(node);
    if (!this._initScheduled) {
      this._initScheduled = true;
      queueMicrotask(function() { ST._flushInit(); });
    }
  };

  /**
   * Run all matching initializers on an element and its descendants.
   * Delegates to the batch scheduler; kept public for backward compatibility.
   * @param {Element} el - Element to initialize
   */
  ST.initElement = function(el) {
    if (!el || !el.nodeType || el.nodeType !== Node.ELEMENT_NODE) return;
    this._scheduleInit(el);
  };

  // ==========================================================================
  // Signal Store
  // ==========================================================================

  /**
   * Get or create signal store for element
   * @param {Element} el - Target element
   * @returns {Object} Signal store
   */
  ST.store = function(el) {
    if (!el) {
      if (this.debug) console.warn('[ST] store() called with null element');
      return {};
    }
    let s = this.signals.get(el);
    if (!s) {
      s = {};
      this.signals.set(el, s);
    }
    return s;
  };

  /**
   * Set signal value (batched updates via microtask)
   * @param {Element} el - Target element
   * @param {string} name - Signal name (without $)
   * @param {any} value - New value
   */
  ST.set = function(el, name, value) {
    if (!el) {
      if (this.debug) console.warn('[ST] set() called with null element for signal:', name);
      return;
    }

    // Dev-mode export enforcement
    if (window.__ST_DEV && el.__stExports !== undefined) {
      if (!(name in el.__stExports)) {
        console.error('[ST] Dev: Cannot access "' + name + '" — not exported by template. Available exports:', Object.keys(el.__stExports));
        return;
      }
      if (!el.__stExports[name].mutable) {
        console.error('[ST] Dev: Cannot write "' + name + '" — exported as read-only');
        return;
      }
    }
    let publishers = this._published.get(name);
    if (!publishers) {
      publishers = new Set();
      this._published.set(name, publishers);
    }
    publishers.add(el);

    const s = this.store(el);
    // Is this the first DEFINED value for the name? `ST.watch` leaves `{v: undefined, d}`
    // placeholder slots up the ancestor chain, so "entry exists" is not the same question
    // as "a value exists" — ST.resolve draws the same distinction when it refuses to let a
    // bare watcher slot shadow a real value further out.
    const isSeed = s[name] === undefined || s[name].v === undefined;
    if (!s[name]) {
      s[name] = { v: value, d: new Set() };
    } else if (s[name].v === value) {
      return; // No change, skip update
    } else {
      s[name].v = value;
    }

    // Also set CSS variable for CSS-based animations
    if (typeof value === 'number' || typeof value === 'string') {
      el.style.setProperty('--st-' + name, value);
    }

    // For typed union signals (objects with .type), set data attribute for CSS pattern matching
    // This enables selectors like [data-st-ws-type="Connected"]
    if (value && typeof value === 'object' && value.type) {
      el.setAttribute('data-st-' + name + '-type', value.type);
    }

    // Notify debug hooks (skip internal _progress signals to reduce noise)
    if (typeof __stDebugHook !== 'undefined' && !name.endsWith('_progress')) {
      __stDebugHook.signal(el, name, value);
    }

    // Bridge to the body-scope plane (BUG-069) when a file-scope binding consumes
    // this export. Mirror the value into _localState and dispatch the same
    // `local:<name>:updated` event the SpacetimeLocal proxy would, so bindings
    // that read `SpacetimeLocal[name]` re-render. Guarded by _bridgedExports so
    // only consumed names cross planes.
    if (this._bridgedExports.has(name) && typeof window !== 'undefined') {
      // Write through the body-scope surface the binding reads. Ensure the
      // SpacetimeLocal proxy exists first (a page may consume a bridged export
      // without declaring any local-state/@data of its own, in which case the
      // proxy that local-state-impl normally creates is absent and a bare
      // `SpacetimeLocal[name]` read would throw). Writing through the proxy
      // dispatches `local:<name>:updated` for us.
      if (!window.SpacetimeLocal || !window.SpacetimeLocal.__stProxy) {
        ST._ensureLocalProxy();
      }
      if (window.SpacetimeLocal[name] !== value) {
        window.SpacetimeLocal[name] = value;
      }
    }

    // A SEED is not a CHANGE (BUG-349).
    //
    // `ST.watch` fires the callback SYNCHRONOUSLY when a value is already present
    // ("call immediately with current value"). So a watcher that subscribes AFTER
    // the value lands is painted at once, while one that subscribes BEFORE waited a
    // microtask for the batch below. Same author code, same end state, different
    // observable timing decided purely by which happened first.
    //
    // That ordering is not something an author controls. A `@template` builder wires
    // its holes (`ST.watchScoped`) while constructing the element, and the caller
    // seeds the params (`ST.set`) only once it holds that element — watch ALWAYS
    // precedes set. So every hole in a named template rendered blank until a
    // microtask elapsed: the row existed, carried its `--st-i` custom property, and
    // had empty text. Assertions on element count or attributes passed; only text
    // saw it.
    //
    // Fix the invariant rather than the symptom: the FIRST time a name acquires a
    // value, notify synchronously, matching what `ST.watch` already promises the
    // other way round. The two orderings now agree by construction.
    //
    // Genuine CHANGES stay batched — that is what the batching is for (N dependents
    // queue 1 microtask, not N, avoiding storms when many signals move at once). A
    // seed transition happens at most once per (element, name), so it cannot storm.
    const deps = Array.from(s[name].d);
    if (deps.length > 0) {
      const notify = () => {
        for (const fn of deps) {
          try { fn(value); } catch(e) {
            if (ST.debug) console.error('[ST] Dependent error for', name, e);
          }
        }
      };
      if (isSeed) notify(); else queueMicrotask(notify);
    }
  };

  /**
   * Get current signal value
   * @param {Element} el - Target element
   * @param {string} name - Signal name (without $)
   * @returns {any} Current value
   */
  ST.get = function(el, name) {
    if (!el) return undefined;
    // Dev-mode export enforcement (warning only for reads)
    if (window.__ST_DEV && el.__stExports !== undefined && !(name in el.__stExports)) {
      console.warn('[ST] Dev: Reading "' + name + '" — not in @exports. Available exports:', Object.keys(el.__stExports));
    }
    const sig = this.store(el)[name];
    if (sig !== undefined) return sig.v;
    // Fallback: check SpacetimeLocal (body-scoped state)
    if (typeof SpacetimeLocal !== 'undefined' && name in (window._localState || {})) {
      return SpacetimeLocal[name];
    }
    return undefined;
  };

  /**
   * Lexical scope resolution: read a signal NAME as visible from `el`, walking the
   * element + its ancestors (the DOM parent chain IS the signal scope chain) before
   * falling back to global page state (SpacetimeLocal). This is the runtime analogue of
   * compile-time scope-matching: a construct inside a `@template` instance (or any nested
   * scope) resolves `$name` against its instance first, then outward, then the page.
   *
   * `ST.get` stays ELEMENT-LOCAL (one element's own signal) — one job; `ST.resolve` is
   * NAME-IN-SCOPE (lexical). Primitives that need scope resolution call this ONCE instead
   * of each hand-rolling a `while (node) { ST.get(node, name); node = node.parentElement }`
   * walk (the rot this replaces: editable resolveBoundDoc, each-with-templates source).
   * @param {Element} el - Resolution origin (walk starts here)
   * @param {string} name - Signal name (without $)
   * @returns {any} The resolved value, or undefined when unbound in any scope.
   */
  // `.prev` (PLAN-126 D31-ii): the previous value of a signal. Lazily
  // subscribes per (node, name); at mount the previous value IS the initial
  // value (the arms primed-guard convention), every change shifts cur -> prev.
  ST._prevTrackers = ST._prevTrackers || new WeakMap();
  ST.prev = function(el, name) {
    let perNode = this._prevTrackers.get(el);
    if (!perNode) {
      perNode = {};
      this._prevTrackers.set(el, perNode);
    }
    if (!(name in perNode)) {
      const tracker = { cur: this.resolve(el, name), prev: undefined };
      tracker.prev = tracker.cur;
      perNode[name] = tracker;
      this.watch(el, name, function(next) {
        tracker.prev = tracker.cur;
        tracker.cur = next;
      });
    }
    return perNode[name].prev;
  };

  ST.resolve = function(el, name) {
    let node = el;
    while (node) {
      const s = this.signals.get(node);
      // Match a node only if it holds a DEFINED VALUE (not a bare watcher slot: ST.watch
      // creates `{v:undefined, d}` placeholders up the chain via watchScoped, which must
      // NOT shadow a real value owned further out).
      if (s && s[name] !== undefined && s[name].v !== undefined) return s[name].v;
      node = node.parentElement || null;
    }
    // Global fallback: page-scope state lives in SpacetimeLocal (the local-state /
    // @data plane). Read it directly when the name is present there — covers both the
    // `_localState`-backed proxy and a bare SpacetimeLocal object.
    if (typeof SpacetimeLocal !== 'undefined' && SpacetimeLocal && name in SpacetimeLocal) {
      return SpacetimeLocal[name];
    }
    return undefined;
  };

  /**
   * Lexical scope resolution of a dotted PATH (`"f.value"`): resolve the head name in
   * scope (via ST.resolve), then traverse the remaining segments as plain property
   * accesses. Returns undefined if the head is unbound or any segment is null.
   * @param {Element} el - Resolution origin
   * @param {string} path - Dotted path, e.g. "f" or "post.body"
   * @returns {any}
   */
  ST.resolvePath = function(el, path) {
    const parts = String(path).split('.');
    let v = this.resolve(el, parts[0]);
    for (let i = 1; i < parts.length && v != null; i++) v = v[parts[i]];
    return v;
  };

  /**
   * Watch a signal NAME as visible from `el`'s scope: attach a watcher on every node from
   * `el` up the ancestor chain, so the callback fires wherever in scope the name is (or
   * later becomes) set. ST.watch fires immediately when a value is already present, so this
   * covers BOTH the already-seeded case and the seed-after-init race uniformly — the single
   * home for the "re-render when my scoped source lands" pattern (replaces the per-primitive
   * ancestor-watch loops). Returns a combined unsubscribe.
   * @param {Element} el - Resolution origin
   * @param {string} name - Signal name (without $)
   * @param {Function} fn - Callback receiving the value
   * @returns {Function} Unsubscribe all
   */
  ST.watchScoped = function(el, name, fn) {
    const unsubs = [];
    let node = el;
    while (node) {
      unsubs.push(this.watch(node, name, fn));
      node = node.parentElement || null;
    }
    // FUP-094: ST.resolve falls through to the GLOBAL page store (SpacetimeLocal)
    // when no element scope owns `name`; the watch side must mirror that, or a
    // template hole that reads an OUTER signal renders the initial value but never
    // reacts. Subscribe to the global update channel (`local:<name>:updated`, the
    // SignalScope::Global emit path) so an outer-signal change reaches `fn` too.
    const chan = 'local:' + name + ':updated';
    const onGlobal = function() { fn(typeof SpacetimeLocal !== 'undefined' && SpacetimeLocal ? SpacetimeLocal[name] : undefined); };
    if (typeof document !== 'undefined' && document.addEventListener) {
      document.addEventListener(chan, onGlobal);
      unsubs.push(function() { document.removeEventListener(chan, onGlobal); });
    }
    return () => unsubs.forEach((u) => u());
  };

  /**
   * Watch a signal for GENUINE CHANGES, in the scope the author's reads use.
   *
   * This is the "a value moved" subscription — the one an animation, an effect,
   * or a replay wants — as opposed to ST.watch's "tell me the value, including
   * right now" and ST.watchScoped's "tell me from any scope, once per scope".
   *
   * Three primitives (signal-arms, the change driver, effect) each hand-rolled
   * this and each got it wrong the same two ways:
   *
   *  - they subscribed with a bare ST.watch, which sees only THIS element's
   *    store, while the author's read resolves by nearest owner (element,
   *    ancestor, or global). A signal declared at file scope never fired.
   *    (FUP-094 watch-mirrors-resolve; stated at stdlib/enum/dispatch.st:192.)
   *
   *  - they primed by CALL COUNT (`if (!primed) { primed = true; return; }`),
   *    assuming a subscription always opens with a synchronous current-value
   *    call. ST.watch makes that call only when this element's store already
   *    holds the name — so for an ancestor- or globally-owned signal the first
   *    callback IS the first change, and the guard ate it.
   *
   * Priming here is STATE-based: read the current value up front, then deliver
   * only when the resolved value actually differs. That is correct for both
   * ownership shapes, and it also absorbs watchScoped's per-scope fan-out (one
   * write can arrive once per node in the chain; a change is still one change).
   *
   * @param {Element} el - Resolution origin (scope node)
   * @param {string} name - Signal name (without $)
   * @param {Function} fn - Called (next, prev) on each genuine change
   * @returns {Function} Unsubscribe
   */
  ST.watchChanges = function(el, name, fn) {
    if (!el) return function() {};
    const read = () => this.resolve(el, name);
    let prev = read();
    const subscribe = this.watchScoped ? this.watchScoped : this.watch;
    return subscribe.call(this, el, name, (delivered) => {
      // Re-derive through resolve — the SAME lookup the author's expression
      // uses — so a fan-out delivery from a non-owning scope cannot report a
      // value the author would never read.
      let next = read();
      if (next === undefined) next = delivered;
      if (next === prev) return;
      const was = prev;
      prev = next;
      fn(next, was);
    });
  };

  /**
   * Find the nearest node (el or an ancestor) whose OWN signal store holds `name` — the
   * write target for a scoped mutation. Returns null when no element scope owns it (the
   * signal is global page state, written through SpacetimeLocal). Pairs with ST.resolve
   * (read) so a mutation inside a `@template` instance writes the INSTANCE signal, while a
   * file-scope mutation falls through to the global path.
   * @param {Element} el - Resolution origin
   * @param {string} name - Signal name (without $)
   * @returns {Element|null}
   */
  ST.resolveOwner = function(el, name) {
    let node = el;
    while (node) {
      const s = this.signals.get(node);
      // A real owner holds a DEFINED VALUE — skip bare watcher slots (see ST.resolve).
      if (s && s[name] !== undefined && s[name].v !== undefined) return node;
      node = node.parentElement || null;
    }
    return null;
  };

  /**
   * Watch signal changes
   * @param {Element} el - Target element
   * @param {string} name - Signal name (without $)
   * @param {Function} fn - Callback function
   * @returns {Function} Unsubscribe function
   */
  ST.watch = function(el, name, fn) {
    if (!el) return () => {};
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
  };

  /**
   * Watch multiple signals
   * @param {Element} el - Target element
   * @param {string[]} names - Signal names
   * @param {Function} fn - Callback receiving all values as object
   * @returns {Function} Unsubscribe function
   */
  ST.watchAll = function(el, names, fn) {
    if (!el) return () => {};
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
  };

  /**
   * Derive a computed signal from dependencies
   * @param {Element} el - Target element
   * @param {string} name - Derived signal name
   * @param {string[]} deps - Dependency signal names
   * @param {Function} compute - Compute function receiving values object
   */
  ST.derive = function(el, name, deps, compute) {
    if (!el) return;
    const unsub = this.watchAll(el, deps, (values) => {
      const result = compute(values);
      this.set(el, name, result);
    });
    // Tear the dependency watchers down when the element is disposed, so a
    // re-rendered node (e.g. @each) does not accumulate stale derive watchers.
    this.onCleanup(el, unsub);
  };


  // ==========================================================================
  // Element Cleanup
  // ==========================================================================

  /**
   * Register cleanup function for element
   * @param {Element} el - Target element
   * @param {Function} fn - Cleanup function
   */
  ST.onCleanup = function(el, fn) {
    if (!el) return;
    let fns = this.cleanups.get(el);
    if (!fns) {
      fns = [];
      this.cleanups.set(el, fns);
    }
    fns.push(fn);
  };

  /**
   * Split a semicolon-joined mutation body into statements, at TOP LEVEL only.
   *
   * The compiler joins a driver body with `"; "` (drivers.rs `mutations.join`),
   * so the runtime has to undo exactly that — without cutting a `;` that lives
   * inside a string literal or a bracketed group. Naive `String.split(';')`
   * would tear `$msg <- "a; b"` and `$rows.insert({ a: 1; b: 2 })` in half.
   *
   * Mirrors the nesting/quote discipline of `splitArgs` below; kept separate
   * because that one splits on `,` within ONE call's arg run.
   *
   * @param {string} src - semicolon-joined statement text
   * @returns {string[]} statements, trimmed, empties dropped
   */
  function splitStatements(src) {
    const out = [];
    let depth = 0, quote = null, buf = '';
    for (let i = 0; i < src.length; i++) {
      const c = src[i];
      if (quote) {
        buf += c;
        // A backslash escapes the next char, so `"a\"b"` stays one string.
        if (c === '\\' && i + 1 < src.length) { buf += src[++i]; continue; }
        if (c === quote) quote = null;
        continue;
      }
      if (c === '"' || c === "'" || c === '`') { quote = c; buf += c; continue; }
      if (c === '(' || c === '[' || c === '{') depth++;
      else if (c === ')' || c === ']' || c === '}') depth--;
      else if (c === ';' && depth <= 0) { out.push(buf); buf = ''; continue; }
      buf += c;
    }
    out.push(buf);
    return out.map((s) => s.trim()).filter((s) => s !== '');
  }
  ST._splitStatements = splitStatements;

  /**
   * Execute a captured mutation/effect body against a target element.
   * Shared by @on mutation handlers (on-mutation-handler) and @effect (effect-runner)
   * so the mutation semantics live in ONE place. `body` is the captured
   * mutation_actions: { js_statements: string[], macro_calls: [{__macro,name}] }.
   * Statement forms (order matters): `&.attr <- expr` (attr on targetEl),
   * `&name.attr <- expr` (attr on a named ref), `$var <- expr` (signal mutation).
   * @param {Element} targetEl - element context for `&.` attrs and @emit source
   * @param {Object} body - { js_statements, macro_calls }
   */
  ST.runMutations = function(targetEl, body, locals, params) {
    if (!body) return;
    const $ = targetEl;
    // `locals` (optional) — a { name: value } map of lexically-bound payloads, e.g.
    // the `todo` carried by a decoded signal variant in `@handle`'s
    // `Created(todo) => { … }` arm. Bare identifiers in the mutation RHS that name
    // a local resolve to the JSON-encoded value (so an object/array survives the
    // eval), mirroring the `$signal` substitution. PLAN-038 FUP-080: handle dispatch.
    //
    // `params` (optional) — a { name: value } map of $-SIGIL signal-call params,
    // e.g. `{ card, to }` from a `$move(card, to)` invocation. Referenced in the
    // mutation as `$card`/`$to` and substituted anchored to the `$` sigil, so an
    // object key `{ to: … }` or member `.to` is never clobbered. Kept SEPARATE
    // from `locals` because the two binding conventions (bare receive-arm carry
    // vs $-sigil signal param) must not cross-substitute.
    const __locals = locals || null;
    const __params = params || null;
    // BUG-119(3): guarantee SpacetimeLocal is a defined proxy before any mutation
    // eval. A `$x <- expr` whose RHS references an as-yet-undeclared local signal
    // lowers to `SpacetimeLocal.x`; if SpacetimeLocal were undefined the eval would
    // throw ReferenceError, the catch would yield undefined, and the write (plus its
    // `local:x:updated` event) would be silently skipped — so an undeclared toggle
    // like `$open <- !$open` did nothing. Ensuring the proxy here makes the write
    // path total: the name resolves to `undefined` (not a throw), the mutation
    // applies, and the proxy dispatches the update event that reactive bindings await.
    if (typeof ST !== 'undefined' && ST._ensureLocalProxy) ST._ensureLocalProxy();
    // A PLAIN-STRING body may hold MANY statements. Every compiler path passes
    // `{ js_statements: [...] }`, but the public contract (`ST.runMutations`
    // is the shared @on/@effect mutation rail) also accepts a raw string —
    // and until this normalization it fell through EVERY branch below and
    // silently ran nothing (the banned silent-drop class, runtime edition).
    //
    // The string arrives semicolon-JOINED (drivers.rs `mutations.join("; ")`),
    // so treating it as ONE statement silently dropped every statement after
    // the first: `@on &.click { $a <- 1; $b <- 2; }` set only `$a`, because the
    // `$x <- expr` regex is greedy and swallowed the rest as `$a`'s RHS. Split
    // it here, at the one place a string body becomes statements.
    //
    // Split only at TOP-LEVEL `;` — a semicolon inside a string literal or
    // inside (), [], {} belongs to its statement (`$rows.insert({a: 1; b: 2})`,
    // `$msg <- "a; b"`).
    if (typeof body === 'string') body = { js_statements: splitStatements(body) };
    if (body.js_statements) {
      for (const stmt of body.js_statements) {
        if (typeof stmt !== 'string') {
          console.warn('[ST] runMutations: expected string statement, got', typeof stmt, stmt);
          continue;
        }
        // Resolve `$name` reads LEXICALLY from the mutation target's scope: an instance
        // signal (a `@template` body's `$open`/param, owned by an ancestor) wins over
        // global page state, and a name owned by no element scope falls back to
        // SpacetimeLocal (file-scope state) — the SAME ST.resolve precedence as reads
        // everywhere. JSON-encode the resolved value so it inlines into the eval'd RHS
        // (objects/strings/numbers/bools all survive). A name resolving to undefined
        // keeps the SpacetimeLocal.<name> form so genuinely-global reads still work.
        const transformExpr = (expr) => {
          let out = expr;
          // (1) $-sigil signal params FIRST: substitute `$card`/`$to` (a signal
          // call's named payload, bound via the `params` channel) BEFORE the
          // generic `$name`->SpacetimeLocal rewrite, so a param wins over a
          // nonexistent global. Without this an @handle optimistic clause like
          // `$cards.map(c => c.id == $card ? …)` resolved `$card` to
          // `SpacetimeLocal.card` (undefined) and matched nothing (drag-drop
          // move silently no-op'd). Anchored to the `$` sigil, so an object key
          // `{ to: … }` or member access `.to` is never touched.
          if (__params) {
            for (const pname in __params) {
              if (!Object.prototype.hasOwnProperty.call(__params, pname)) continue;
              let enc;
              try { enc = JSON.stringify(__params[pname]); } catch (e) { enc = 'undefined'; }
              if (enc === undefined) enc = 'undefined';
              const esc = pname.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
              out = out.replace(new RegExp('\\$' + esc + '(?![\\w])', 'g'), enc);
            }
          }
          // (2) generic $name -> scoped value or SpacetimeLocal.<name>.
          out = out.replace(/\$(\w+)/g, (_, name) => {
            const scoped = (typeof ST !== 'undefined' && ST.resolveOwner) ? ST.resolveOwner($, name) : null;
            if (scoped) {
              try { return JSON.stringify(ST.get(scoped, name)); } catch (e) { /* fall through */ }
            }
            return `SpacetimeLocal.${name}`;
          });
          // (3) BARE-NAME lexical binds (a receive-arm carry, e.g. `Created(todo)`
          // used as `todo`/`todo.id`). The lookbehind keeps it off member
          // accesses (`obj.todo`) and the $-rewritten `SpacetimeLocal.todo`.
          if (__locals) {
            for (const lname in __locals) {
              if (!Object.prototype.hasOwnProperty.call(__locals, lname)) continue;
              const re = new RegExp('(?<![\\w.$])' + lname.replace(/[.*+?^${}()|[\]\\]/g, '\\$&') + '(?![\\w])', 'g');
              let encoded;
              try { encoded = JSON.stringify(__locals[lname]); } catch (e) { encoded = 'undefined'; }
              if (encoded === undefined) encoded = 'undefined';
              out = out.replace(re, encoded);
            }
          }
          return out;
        };

        // &.attr <- expr (attribute on targetEl)
        const currentAttrMatch = stmt.match(/^&\.(\w+)\s*<-\s*(.+)$/s);
        if (currentAttrMatch) {
          const [, attrName, expr] = currentAttrMatch;
          try {
            const value = eval(transformExpr(expr));
            if (value !== undefined && $) $.setAttribute(attrName, String(value));
          } catch (err) { console.error('[ST] Attribute mutation error:', err); }
          continue;
        }

        // &name.attr <- expr (attribute on a named element ref)
        const namedAttrMatch = stmt.match(/^&(\w+)\.(\w+)\s*<-\s*(.+)$/s);
        if (namedAttrMatch) {
          const [, refName, attrName, expr] = namedAttrMatch;
          try {
            const value = eval(transformExpr(expr));
            const el = window.ST?.refs?.[refName];
            if (el && value !== undefined) el.setAttribute(attrName, String(value));
          } catch (err) { console.error('[ST] Named ref attribute mutation error:', err); }
          continue;
        }

        // Shared argument evaluator for call-shaped statements ($coll.method(args)
        // and $signal(args)). Splits top-level comma-separated args (brace/bracket
        // aware) and evals each through the same transformExpr rail as a mutation
        // RHS, honoring a trailing Spacetime filter pipe (`expr | filterName`) via
        // ST.filter rather than letting `|` eval as bitwise-OR.
        const splitArgs = (s) => {
          const out = []; let depth = 0, last = 0;
          for (let i = 0; i < s.length; i++) {
            const c = s[i];
            if (c === '(' || c === '[' || c === '{') depth++;
            else if (c === ')' || c === ']' || c === '}') depth--;
            else if (c === ',' && depth === 0) { out.push(s.slice(last, i)); last = i + 1; }
          }
          if (s.trim() !== '') out.push(s.slice(last));
          return out;
        };
        const evalCallArg = (raw) => {
          let expr = raw.trim();
          let filter = null;
          const pipe = expr.indexOf('|');
          if (pipe !== -1 && expr[pipe + 1] !== '|') {
            filter = expr.slice(pipe + 1).trim();
            expr = expr.slice(0, pipe).trim();
          }
          let v = eval('(' + transformExpr(expr) + ')');
          if (filter && typeof ST !== 'undefined' && ST.filter) v = ST.filter(filter, v);
          else if (filter === 'int') v = parseInt(v, 10);
          else if (filter === 'number') v = Number(v);
          return v;
        };

        // $coll.method(args) — a collection mutator call (@data collection, FEAT-075).
        // Dispatches to ST._collections[name][method](...args); the collection primitive
        // registered insert/delete/reorder there. Unknown name/method is a no-op + warn.
        const callMatch = stmt.match(/^\$([\w-]+)\.(\w+)\s*\((.*)\)\s*$/s);
        if (callMatch) {
          const [, collName, method, rawArgs] = callMatch;
          const coll = (typeof ST !== 'undefined' && ST._collections) ? ST._collections[collName] : null;
          if (coll && typeof coll[method] === 'function') {
            try {
              const args = rawArgs.trim() === '' ? [] : splitArgs(rawArgs).map(evalCallArg);
              coll[method].apply(coll, args);
            } catch (err) { console.error('[ST] collection call error (' + collName + '.' + method + '):', err); }
          } else if (typeof ST !== 'undefined' && ST._collections && Object.keys(ST._collections).length > 0) {
            // Only warn when collections exist at all (else this is a non-collection
            // signal_call — e.g. @realtime/@presence — which is a legitimate no-op here).
            console.warn('[ST] collection mutator not found: $' + collName + '.' + method + '()');
          }
          continue;
        }

        // $signal(args) — fire a `@data signal`/`@data stream` callable by NAME
        // (PLAN-053). This is the documented `@on click { $sig(args) }` surface:
        // the signal-call primitive registers its fire fn via ST.registerSignalCall,
        // and we invoke it here with the evaluated argument(s). An unregistered name
        // is a benign no-op (the signal may not exist on this page).
        const signalCallMatch = stmt.match(/^\$([\w-]+)\s*\((.*)\)\s*$/s);
        if (signalCallMatch) {
          const [, sigName, rawArgs] = signalCallMatch;
          const fire = (typeof ST !== 'undefined' && ST.getSignalCall) ? ST.getSignalCall(sigName) : null;
          if (typeof fire === 'function') {
            try {
              const args = rawArgs.trim() === '' ? [] : splitArgs(rawArgs).map(evalCallArg);
              fire.apply(null, args);
            } catch (err) { console.error('[ST] signal call error ($' + sigName + '):', err); }
          }
          continue;
        }

        // $var <- expr (signal mutation)
        const varMatch = stmt.match(/^\$([\w-]+)\s*<-\s*(.+)$/s);
        if (varMatch) {
          const [, varName, expr] = varMatch;
          try {
            let jsExpr = transformExpr(expr);
            // Parenthesize an object-literal RHS so `eval('{...}')` isn't a block.
            if (jsExpr.trimStart().startsWith('{')) { jsExpr = '(' + jsExpr + ')'; }
            const value = (function() {
              try { return eval(jsExpr); }
              catch (e) { console.error('[ST] Mutation eval:', e, jsExpr); return undefined; }
            })();
            if (value !== undefined) {
              // Scope-first write: if an element scope (the mutation target or an
              // ancestor — e.g. a `@template` instance root) OWNS this signal, write THE
              // INSTANCE signal via ST.set. This is the write half of the lexical scope
              // model (read half = transformExpr/ST.resolve above), and is what lets a
              // `@on` inside a template body drive per-instance state. Falls through to
              // the global SpacetimeLocal path when no element scope owns the name.
              const owner = (typeof ST !== 'undefined' && ST.resolveOwner) ? ST.resolveOwner($, varName) : null;
              if (owner) {
                ST.set(owner, varName, value);
              } else if (window.SpacetimeLocal && varName in SpacetimeLocal) {
                SpacetimeLocal[varName] = value;
                if (!window.SpacetimeLocal.__stProxy) {
                  document.dispatchEvent(new CustomEvent(`local:${varName}:updated`, { detail: value }));
                }
              } else if (window.ST?.signals?.[varName]) {
                ST.signals[varName].set(value);
              } else {
                window._localState = window._localState || {};
                window._localState[varName] = value;
                if (!window.SpacetimeLocal) window.SpacetimeLocal = window._localState;
                document.dispatchEvent(new CustomEvent(`local:${varName}:updated`, { detail: value }));
              }
            }
          } catch (err) { console.error('[ST] Mutation error:', err); }
        }
      }
    }

    // @emit events bubble so parent/sibling state machines can receive them.
    if (body.macro_calls && Array.isArray(body.macro_calls)) {
      for (const call of body.macro_calls) {
        if (call.__macro === 'emit' && call.name) {
          document.dispatchEvent(new CustomEvent(call.name, { bubbles: true, detail: { source: targetEl } }));
        }
      }
    }
  };

  /**
   * Refcounted body scroll-lock for @portal modals (FEAT-074). The FIRST lock hides
   * body overflow (saving the prior value); the lock is released only when the LAST
   * portal unlocks — so stacked modals don't prematurely restore scrolling.
   */
  ST._scrollLockCount = 0;
  ST._scrollLockPrev = '';
  ST.lockScroll = function() {
    if (typeof document === 'undefined') return;
    if (this._scrollLockCount === 0) {
      this._scrollLockPrev = document.body.style.overflow || '';
      document.body.style.overflow = 'hidden';
    }
    this._scrollLockCount++;
  };
  ST.unlockScroll = function() {
    if (typeof document === 'undefined') return;
    if (this._scrollLockCount === 0) return;
    this._scrollLockCount--;
    if (this._scrollLockCount === 0) {
      document.body.style.overflow = this._scrollLockPrev;
    }
  };

  /**
   * Stack of open @portal modals (FEAT-074), topmost last. A single document-level
   * keydown handler closes the TOPMOST modal on Escape (registered lazily on first push).
   */
  ST._portalStack = ST._portalStack || [];
  ST._portalEscBound = ST._portalEscBound || false;
  ST.pushPortal = function(entry) {
    this._portalStack.push(entry);
    if (!this._portalEscBound && typeof document !== 'undefined') {
      this._portalEscBound = true;
      document.addEventListener('keydown', (e) => {
        if (e.key !== 'Escape' || ST._portalStack.length === 0) return;
        const top = ST._portalStack[ST._portalStack.length - 1];
        if (top && typeof top.close === 'function') { e.stopPropagation(); top.close(); }
      });
    }
  };
  ST.removePortal = function(entry) {
    const i = this._portalStack.indexOf(entry);
    if (i !== -1) this._portalStack.splice(i, 1);
  };

  /**
   * Run all cleanup functions for element
   * @param {Element} el - Target element
   */
  /**
   * Dispose a node reported as removed by the MutationObserver — BUT only if it
   * was GENUINELY removed, not merely MOVED (re-parented, e.g. @portal relocating
   * a subtree to <body>). A moved node emits a removedNodes record for its old
   * slot, yet is already re-attached by the time the observer microtask runs, so
   * it reads isConnected===true and must be PRESERVED — disposing it would wipe
   * its (and its descendants') reactive signal store while still live (BUG-078).
   * A truly removed node is disconnected (isConnected===false) and is disposed,
   * along with its still-disconnected descendants. Extracted so the cleanup logic
   * is unit-testable without a live MutationObserver.
   */
  ST._handleRemovedNode = function(node) {
    if (!node || node.nodeType !== Node.ELEMENT_NODE || node.isConnected) return;
    this.dispose(node);
    if (node.querySelectorAll) {
      node.querySelectorAll('*').forEach((child) => {
        if (!child.isConnected) this.dispose(child);
      });
    }
  };

  ST.dispose = function(el) {
    if (!el) return;
    const fns = this.cleanups.get(el);
    if (fns) {
      for (const fn of fns) {
        try {
          fn();
        } catch (e) {
          console.error('[ST] Cleanup error:', e);
        }
      }
      this.cleanups.delete(el);
    }
    this.signals.delete(el);
  };

  // Alias for backwards compatibility
  ST.cleanup = ST.dispose;

  // ==========================================================================
  // Initialization Helpers
  // ==========================================================================

  /**
   * Initialize element with primitive setup function
   * @param {Element} el - Target element
   * @param {Function} setupFn - Setup function that returns cleanup
   */
  ST.init = function(el, setupFn) {
    if (!el) return;
    const cleanup = setupFn(el);
    if (typeof cleanup === 'function') {
      this.onCleanup(el, cleanup);
    }
  };

  /**
   * Apply primitive to all matching elements
   * @param {string} selector - CSS selector
   * @param {Function} setupFn - Setup function
   */
  ST.apply = function(selector, setupFn) {
    document.querySelectorAll(selector).forEach((el) => {
      this.init(el, setupFn);
    });
  };

  // ==========================================================================
  // State Binding Helpers
  // ==========================================================================

  /**
   * Bind signal to state attribute
   * @param {Element} el - Target element
   * @param {string} signalName - Signal to watch
   * @param {string} stateName - State name for data-st-state
   */
  ST.bindState = function(el, signalName, stateName) {
    if (!el) return;
    // Normalize a leading `$` sigil: the state codegen emits the gate as the
    // authored token (`$over`, `$active`), but signals are stored under the bare
    // name (`over`, `active`) via ST.set. Without stripping, ST.watch keys on
    // `$over` while ST.set notifies `over` — the watcher never fires and the
    // state attribute never applies. (Single-signal gate; compound conditions
    // are pre-resolved to one signal by the macro, e.g. dnd's `$over`.)
    if (typeof signalName === 'string' && signalName.charCodeAt(0) === 36) {
      signalName = signalName.slice(1);
    }
    // Track registered states per element for debugger
    if (!el.__stBoundStates) {
      el.__stBoundStates = new Set();
    }
    el.__stBoundStates.add(stateName);

    // --- instrumentation state (ported from the retired %primitive state) --
    el.__stStateHistory = el.__stStateHistory || [];
    el.__stStateEntries = el.__stStateEntries || [];
    el.__stStateExits = el.__stStateExits || [];
    el.__stTransitionCount = el.__stTransitionCount || 0;
    el.__stStateDurations = el.__stStateDurations || {};
    el.__stTransitionTimes = el.__stTransitionTimes || {};
    el.__stStateStartTime = el.__stStateStartTime || Date.now();
    el.__stLastTransitionTime = el.__stLastTransitionTime || Date.now();
    el.__stLastTransition = el.__stLastTransition || null;

    // Register with debugger if available
    if (typeof __stDebugHook !== 'undefined') {
      const states = Array.from(el.__stBoundStates);
      const initial = el.dataset.stState || '';
      __stDebugHook.registerStateMachine(el, initial, states);
    }

    // THE CANONICAL state-reflect semantics (PLAN-077 W6/W7: ONE writer for
    // @state AND the %states clause — state.st's inline copy was folded back
    // into this function at GATE 7):
    //  - the degenerate-union read: union objects reflect `.type`; EVERY other
    //    truthy value gates on the declaration's own state name (the legacy
    //    bindState semantics for `@state(when: "loading")` + a bool signal).
    //  - multi-gate carrier sharing: a FALSY clear may only remove THIS
    //    declaration's own state (two gates share one data-st-state carrier).
    const stateNameOf = (v) => {
      if (v && typeof v === 'object' && v.type) return v.type;
      return v ? stateName : '';
    };

    const reflect = (v) => {
      const name = stateNameOf(v);
      const oldState = el.dataset.stState || '';
      if (oldState === name) return;
      // A falsy clear may only remove THIS declaration's own state — an
      // unguarded clear let one gate's false transition wipe another gate's
      // active state on a shared element (GATE 6 P1).
      if (!name && oldState !== stateName) return;

      const now = Date.now();
      // --- instrumentation: durations, transitions, exits ------------------
      if (el.__stStateStartTime && oldState) {
        el.__stStateDurations[oldState] = now - el.__stStateStartTime;
      }
      const transitionKey = oldState + '->' + name;
      el.__stTransitionTimes[transitionKey] = now - (el.__stLastTransitionTime || now);
      if (oldState) {
        el.__stStateExits.push(oldState);
        el.dispatchEvent(new CustomEvent('state.exit', {
          detail: { from: oldState, to: name },
          bubbles: false
        }));
      }

      // --- the write: unified carrier + namespaced compat carrier ----------
      el.dataset.stState = name;
      el.setAttribute('data-st-' + signalName + '-type', name);
      // Union payload destructure (the watchTypedUnion contract): extract
      // every payload field into element scope.
      if (v && typeof v === 'object' && v.type && typeof ST !== 'undefined' && ST.set) {
        Object.keys(v).forEach((k) => {
          if (k !== 'type') ST.set(el, k, v[k]);
        });
      }

      // --- instrumentation: entries, history, transitions, debug hook ------
      if (name) {
        el.__stStateHistory.push(name);
        el.__stStateEntries.push(name);
        el.__stTransitionCount++;
        el.__stLastTransition = { from: oldState, to: name, time: now };
        el.__stStateStartTime = now;
        el.__stLastTransitionTime = now;
        el.dispatchEvent(new CustomEvent('state.enter', {
          detail: { from: oldState, to: name },
          bubbles: false
        }));
      }
      if (typeof __stDebugHook !== 'undefined' && oldState !== name) {
        __stDebugHook.state(el, oldState, name, signalName);
      }
    };

    // FUP-094-complete subscription: host scope + ancestors + the GLOBAL
    // channel — the orphan fix (a GLOBAL-channel write reaches an element
    // whose scope never owned the signal). The global leg only hears FUTURE
    // `local:<sig>:updated` events, so pair it with the initial reflect below.
    const unwatch = (typeof ST !== 'undefined' && ST.watchScoped)
      ? ST.watchScoped(el, signalName, reflect)
      : this.watch(el, signalName, reflect);

    // Initial reflection: ST.resolve walks element scope → ancestors → the
    // global store, covering the already-published case uniformly. reflect is
    // idempotent (oldState === name early-return), so a later event replaying
    // the same value is a no-op.
    if (typeof ST !== 'undefined' && ST.resolve) reflect(ST.resolve(el, signalName));

    if (typeof ST !== 'undefined' && ST.onCleanup) {
      ST.onCleanup(el, () => {
        if (unwatch) unwatch();
        delete el.__stStateHistory;
        delete el.__stStateEntries;
        delete el.__stStateExits;
        delete el.__stTransitionCount;
        delete el.__stStateDurations;
        delete el.__stTransitionTimes;
        delete el.__stStateStartTime;
        delete el.__stLastTransitionTime;
        delete el.__stLastTransition;
      });
    }
  };

  /**
   * Run a callback when a signal TRANSITIONS to a target value (PLAN-053).
   *
   * Backs the macro `%on $var -> value { actions }` clause (e.g. @drag's
   * `on-drop`, which fires when the gesture's `$active` goes true -> false).
   * Fires `fn(newVal, oldVal)` only on the EDGE into `targetValue` — not on every
   * update, and not for the initial value. `targetValue` is the literal from the
   * clause (`"false"`, `"true"`, or any token); it is coerced to match common JS
   * value shapes (booleans, numbers) so `-> false` matches the boolean `false`.
   * @param {Element} el - Target element owning the signal
   * @param {string} signalName - Signal to watch (no `$`)
   * @param {string} targetValue - The value whose arrival fires the callback
   * @param {Function} fn - Callback run on the transition edge
   */
  ST.onTransition = function(el, signalName, targetValue, fn) {
    if (!el || typeof fn !== 'function') return;
    // Coerce the clause token to the JS value it represents so `-> false`
    // matches a boolean false, `-> 0` a numeric 0, etc. Unknown tokens compare
    // as strings.
    const coerce = (t) => {
      if (t === 'true') return true;
      if (t === 'false') return false;
      if (t === 'null') return null;
      if (t !== '' && !isNaN(Number(t))) return Number(t);
      return t;
    };
    const target = coerce(String(targetValue).trim());
    let prev;
    let seeded = false;
    const unsub = this.watch(el, signalName, (v) => {
      const was = prev;
      const had = seeded;
      prev = v;
      seeded = true;
      // Fire only on the EDGE into the target: the new value equals target AND
      // either we have a prior value that differed, or this is a genuine change.
      // Skip the very first observation (no transition has occurred yet).
      if (v === target && had && was !== target) {
        try { fn(v, was); } catch (e) { console.error('[ST] onTransition handler error:', e); }
      }
    });
    // Drop the transition watcher when the element is disposed (re-render safety).
    this.onCleanup(el, unsub);
  };

  /**
   * Bind signal to CSS property
   * @param {Element} el - Target element
   * @param {string} signalName - Signal to watch
   * @param {string} property - CSS property name
   * @param {Function} [transform] - Optional transform function
   */
  ST.bindStyle = function(el, signalName, property, transform) {
    if (!el) return;
    this.watch(el, signalName, (v) => {
      const value = transform ? transform(v) : v;
      if (property.startsWith('--')) {
        el.style.setProperty(property, value);
      } else {
        // Coerce to string to match the real CSSOM contract (a browser stringifies
        // `el.style.opacity = 1` to '1' on read). Semantic no-op; CSS values are strings.
        el.style[property] = typeof value === 'number' ? String(value) : value;
      }
    });
  };

  /**
   * Resolve the DROP TARGET under a drag's release point (PLAN-055).
   *
   * `@drag` moves an element with a CSS transform (it is NOT reparented), so the
   * dragged node stays a child of its source container — `el.closest(selector)`
   * at drop time returns the SOURCE, not where it was dropped. This hit-tests the
   * release point (the gesture's `$dropX`/`$dropY` signals) with the dragged
   * element temporarily made pointer-transparent, then returns the nearest
   * ancestor of the topmost element that matches `selector` (or that element
   * itself). Returns null when the release is over nothing matching.
   *
   * Usage in a drag on-drop action:
   *   on-drop: $move({ card: $.dataset.id, to: ST.dropTarget($, '.col')?.dataset.col })
   *
   * @param {Element} el - The dragged element (its $dropX/$dropY hold the point)
   * @param {string} selector - The drop-zone selector to resolve to
   * @returns {Element|null}
   */
  ST.dropTarget = function(el, selector) {
    if (!el || typeof document === 'undefined' || !document.elementFromPoint) return null;
    const x = this.get(el, 'dropX');
    const y = this.get(el, 'dropY');
    if (x == null || y == null) return null;
    // Make the dragged element (and any drag clone) ignore hit-testing so the
    // element BELOW it at the release point is found.
    const prevPE = el.style.pointerEvents;
    el.style.pointerEvents = 'none';
    let stack;
    try {
      // Walk the WHOLE hit-test stack, not just the topmost element: a fixed
      // overlay (the dev panel `#__spacetime-dev-panel`, a presence cursor, a
      // toast) can sit ON TOP of a drop zone and would otherwise shadow it,
      // making the drop resolve to null. `elementsFromPoint` returns front-to-
      // back; the first entry that resolves to a `selector` match is the real
      // drop target beneath the overlays.
      stack = (typeof document.elementsFromPoint === 'function')
        ? document.elementsFromPoint(x, y)
        : [document.elementFromPoint(x, y)];
    } finally {
      el.style.pointerEvents = prevPE;
    }
    if (!stack || !stack.length) return null;
    if (!selector) return stack[0] || null;
    for (const node of stack) {
      if (node && typeof node.closest === 'function') {
        const match = node.closest(selector);
        if (match) return match;
      }
    }
    return null;
  };

  /**
   * getBoundingClientRect of an element, null-safe. Backs `&self.rect` /
   * `&$bounds.rect` in lowered %derives expressions (PLAN-054). A null/absent
   * element yields a zero rect so a guarded bounds branch never throws.
   */
  ST.rectOf = function(el) {
    if (el && typeof el.getBoundingClientRect === 'function') {
      return el.getBoundingClientRect();
    }
    return { x: 0, y: 0, width: 0, height: 0, top: 0, left: 0, right: 0, bottom: 0 };
  };

  /**
   * Resolve a named element ref (`&name`) to its DOM element via the
   * `__stRefs` registry the element-ref primitive populates. Backs `$bounds` in
   * lowered `%derives` expressions (PLAN-057, the @drag bounds fix): an `&name`
   * Element capture lowers to `ST.ref("name")`, so `$bounds != none` becomes a
   * real `ST.ref(...) != null` test and `&$bounds.rect` reads the live rect.
   * Returns null when no such ref is registered (the bounds branch stays
   * guarded-false, never throwing).
   */
  ST.ref = function(name) {
    if (typeof window === 'undefined' || !window.__stRefs) return null;
    return window.__stRefs[name] || null;
  };

  /**
   * Resolve an assertion/probe SUBJECT to an element.
   *
   * A subject is usually a CSS selector, but `&name` is how Spacetime names an
   * element everywhere else (`@on &item.click`, `&box <div>` in markup), so an
   * author reasonably writes `@then &box .num should have_text "01"`. That went
   * straight into `document.querySelector("&box .num")` — not valid CSS, always
   * null, reported as `got "undefined"`.
   *
   * Accepted-then-ignored is the worst failure direction: the test ran and
   * looked like a real assertion. With a `not_exist` expectation it would even
   * PASS, silently asserting nothing.
   *
   * Forms:
   *   `.foo .bar`      plain CSS, unchanged
   *   `&box`           the named element itself
   *   `&box .num`      a CSS descendant, scoped to the named element
   *
   * Returns null when the ref is unregistered or the descendant is absent — the
   * same contract `querySelector` has, so every existing assertion keeps its
   * null handling.
   */
  ST.subject = function(sel, root) {
    const parts = ST._refSubject(sel);
    const scope = root || (typeof document !== 'undefined' ? document : null);
    if (!parts) return scope ? scope.querySelector(sel) : null;
    const el = ST.ref(parts.name);
    if (!el) return null;
    return parts.rest ? el.querySelector(parts.rest) : el;
  };

  /**
   * Split a `&name rest` subject, or null when `sel` is not one.
   *
   * A ref NAME is a plain identifier. `&cards[] .list` is a COLLECTION ref — a
   * different construct with its own block syntax — and `&:hover`, `&.active`
   * are CSS nesting selectors. Treating any of those as a ref name corrupted the
   * selector: `&cards[]` was read as the name `cards[]`, and the leftover `]`
   * reached the CSS parser as `Expected name, found ]`.
   *
   * So the name is matched EXPLICITLY rather than by "everything up to the first
   * space" — anything that is not `&<ident>` is left alone as plain CSS.
   */
  ST._refSubject = function(sel) {
    if (typeof sel !== 'string') return null;
    const m = /^&([A-Za-z_][A-Za-z0-9_-]*)(?![\w[\]-])\s*(.*)$/.exec(sel);
    if (!m) return null;
    return { name: m[1], rest: m[2].trim() };
  };

  /** Plural form of `ST.subject`, for assertions that count matches. */
  ST.subjectAll = function(sel, root) {
    const parts = ST._refSubject(sel);
    const scope = root || (typeof document !== 'undefined' ? document : null);
    if (!parts) return scope ? scope.querySelectorAll(sel) : [];
    const el = ST.ref(parts.name);
    if (!el) return [];
    return parts.rest ? el.querySelectorAll(parts.rest) : [el];
  };

  /**
   * Clamp a number to [lo, hi]. Used by lowered %derives bound expressions
   * (e.g. @drag bounds constraints). PLAN-054.
   */
  ST.clamp = function(v, lo, hi) {
    v = Number(v);
    if (lo != null && v < lo) v = lo;
    if (hi != null && v > hi) v = hi;
    return v;
  };

  /**
   * Reactively compose CSS `transform` from one or more animated channels and
   * apply it to the element (PLAN-054, backs the macro `%animates` clause).
   *
   * Each channel is `{ fn, signal }`: `fn(value)` returns a transform FUNCTION
   * token (e.g. `translateX(40px)`, `scale(1.04)`), and `signal` is the element
   * signal whose live value drives it (typically a derived signal from a lowered
   * `%derives`). All channels recompose into ONE `transform` string on any
   * channel update, so translate-x + translate-y + scale combine instead of
   * clobbering each other. Channels with no current value contribute nothing.
   *
   * @param {Element} el - Target element
   * @param {Array<{fn: function, signal: string}>} channels
   */
  ST.bindTransform = function(el, channels) {
    if (!el || !channels || !channels.length) return;
    const recompute = () => {
      const parts = [];
      for (const ch of channels) {
        const v = this.get(el, ch.signal);
        if (v === undefined || v === null) continue;
        const token = ch.fn(v);
        if (token != null && token !== '') parts.push(token);
      }
      el.style.transform = parts.join(' ');
    };
    const unsubs = [];
    for (const ch of channels) {
      unsubs.push(this.watch(el, ch.signal, recompute));
    }
    // Drop the channel watchers when the element is disposed (re-render safety).
    this.onCleanup(el, () => unsubs.forEach((u) => u()));
  };

  /**
   * Watch a typed union signal for pattern matching
   * Sets data-st-{signal}-type attribute when variant matches
   * @param {Element} el - Target element
   * @param {string} signalName - Signal name (without $)
   * @param {string} variant - Variant to match (e.g., 'Connected')
   * @param {string[]} bindings - Binding names to extract when matched
   */
  ST.watchTypedUnion = function(el, signalName, variant, bindings) {
    if (!el) return;
    const attrName = `data-st-${signalName}-type`;
    const apply = (state) => {
      if (state && typeof state === 'object' && state.type) {
        // Always update the attribute to reflect current state
        el.setAttribute(attrName, state.type);

        // Extract bindings when variant matches
        if (state.type === variant && bindings.length > 0) {
          bindings.forEach(binding => {
            if (state[binding] !== undefined) {
              this.set(el, binding, state[binding]);
            }
          });
        }
      }
    };
    this.watch(el, signalName, apply);
    // PLAN-077 (gate-2 fix): typed unions are ALSO published on the GLOBAL
    // channel (`@data derive … : @match` uses the same SpacetimeLocal +
    // local:<name>:updated channel every derive uses), which the element-local
    // ST.watch above never observes. Mirror ST.resolve's global fallthrough
    // (FUP-094 doctrine: the watch side must mirror the read side) — apply
    // global updates only when NO element scope owns the name, so a scoped
    // signal always wins over a global one (nearest-owner precedence).
    const chan = 'local:' + signalName + ':updated';
    const readGlobal = () => (typeof SpacetimeLocal !== 'undefined' && SpacetimeLocal)
      ? SpacetimeLocal[signalName]
      : undefined;
    const onGlobal = () => {
      if (this.resolveOwner(el, signalName)) return;
      apply(readGlobal());
    };
    if (typeof document !== 'undefined' && document.addEventListener) {
      document.addEventListener(chan, onGlobal);
      this.onCleanup && this.onCleanup(el, () => document.removeEventListener(chan, onGlobal));
      // Initial: a global value published before this element mounted.
      if (!this.resolveOwner(el, signalName)) {
        const v = readGlobal();
        if (v !== undefined) apply(v);
      }
    }
  };

  // ==========================================================================
  // Data Source Registry (for @data/@each coordination)
  // ==========================================================================

  /**
   * Set data for a named source (producer side)
   * Notifies all waiting subscribers and dispatches event for backwards compatibility
   * @param {string} name - Data source name
   * @param {any} data - Data to store
   */
  /**
   * Read the current value of a data source (or undefined if unset). Used by @data
   * collection mutators to snapshot the array before an optimistic apply (FEAT-075).
   */
  ST.getData = function(name) {
    const entry = this._dataRegistry.get(name);
    return entry ? entry.data : undefined;
  };

  /**
   * Enrich a collection's entries with a resolved display `_label` for the CMS
   * admin's list rows. The admin is a meta-renderer: the title FIELD varies per
   * type, so we resolve it from the type's display.title role (types[type].
   * display.title), falling back to common human fields, then the id. Returns a
   * new array of shallow-cloned entries each carrying `_label`.
   */
  /**
   * Pick a default active collection file when none is selected yet, so the
   * admin center isn't blank on load. Returns the current selection if one
   * exists, else the first collection's file, else ''.
   */
  /**
   * Parse an HTML fragment into its first element, NAMESPACE-AWARE (BUG-082).
   * `containerEl` supplies the namespace context: when it lives in the SVG
   * namespace, the fragment is wrap-parsed inside <svg>…</svg> so the parser
   * assigns the SVG namespace (a bare `<circle>` parsed via template.innerHTML
   * becomes an HTMLUnknownElement that never paints). HTML containers keep the
   * plain <template> parse. The shared row/fragment parser for @each and
   * template instantiation.
   */
  var SVG_NS = 'http://www.w3.org/2000/svg';

  // Parse a duration (a ms number, or a duration string like "1500ms"/"2s"/"1m")
  // into milliseconds. The `:time`/`:duration` scalar's value IS its source text
  // ("1500ms"), so JS-numeric consumers (`@wait`, `@wait_for_signal`, `@wait_until`,
  // …) must convert before setTimeout/Date arithmetic. See BUG-317.
  ST.parseMs = function(v) {
    if (v == null) return 0;
    if (typeof v === 'number') return v;
    var s = String(v).trim();
    var m = s.match(/^([\d.]+)\s*(ms|s|m)?$/);
    if (!m) return parseFloat(s) || 0;
    var n = parseFloat(m[1]);
    if (m[2] === 's') return n * 1000;
    if (m[2] === 'm') return n * 60000;
    return n;
  };

  ST.parseHtml = function(html, containerEl) {
    const tpl = document.createElement('template');
    const inSvg = !!(containerEl && containerEl.namespaceURI === SVG_NS &&
      String(containerEl.tagName).toLowerCase() !== 'foreignobject');
    if (inSvg) {
      tpl.innerHTML = '<svg>' + html + '</svg>';
      const wrap = tpl.content.firstElementChild;
      return wrap ? wrap.firstElementChild : null;
    }
    tpl.innerHTML = html;
    return tpl.content.firstElementChild;
  };

  /**
   * Resolve a single template/​@match invocation ARG against a data object.
   * Mirrors the dotted-path walk already used for @match SUBJECTS and template
   * NAMES (BUG-074): `$a` → data.a, `$a.b.c` → data.a.b.c (deep walk, undefined
   * collapses to the literal). Quoted string literals are unquoted; everything
   * else passes through verbatim. One shared resolver so arg/name/subject all
   * agree on dotted-path semantics.
   */
  ST.resolveTemplateArg = function(arg, data) {
    if (typeof arg !== 'string') return arg;
    if (arg.startsWith('$')) {
      const path = arg.slice(1).split('.');
      let v = data ? data[path[0]] : undefined;
      for (let i = 1; i < path.length && v != null; i++) v = v[path[i]];
      return v !== undefined ? v : arg;
    }
    if ((arg.startsWith('"') && arg.endsWith('"')) || (arg.startsWith("'") && arg.endsWith("'"))) {
      return arg.slice(1, -1);
    }
    return arg;
  };

  /**
   * Build the argument list for a template factory call from a ref's structured
   * args (BUG-131). `args[i]` is the raw value expression; `argNames[i]` (when
   * present) is the parameter NAME for a named invocation (`&card(title: "A")`).
   * Each value is resolved through `resolveTemplateArg` (dotted-path + unquote),
   * then:
   *  - POSITIONAL slots (name == null) become ordered factory args.
   *  - NAMED slots collapse into ONE branded `{__stNamedArgs: {name: value}}`
   *    object appended last; the factory binds those by name, order-independent.
   * When no slot is named the result is the plain positional list (zero overhead,
   * identical to the pre-BUG-131 behaviour).
   */
  /**
   * Resolve a RAW invocation arg list (BUG-131, selector-init path). Unlike
   * `buildTemplateArgs` (which receives args + parallel argNames already split at
   * compile), this path — the `invoke-template` stdlib primitive — receives the
   * raw source arg strings, where a named arg is the single token `name: value`.
   * Split each on the first top-level `:` (a `:` inside quotes/brackets is part of
   * the value), then delegate to `buildTemplateArgs` so the named-vs-positional
   * branding + `$`-resolution live in ONE place. No name present anywhere -> the
   * plain positional list (identical to the pre-BUG-131 behaviour).
   */
  ST.resolveRawInvocationArgs = function(rawArgs, data) {
    const list = Array.isArray(rawArgs) ? rawArgs : [];
    const values = [];
    const names = [];
    list.forEach(function (raw) {
      if (typeof raw !== 'string') { values.push(raw); names.push(null); return; }
      const s = raw.trim();
      let depth = 0, inStr = null, cut = -1;
      for (let i = 0; i < s.length; i++) {
        const c = s[i];
        if (inStr) {
          if (c === '\\') { i++; continue; }
          if (c === inStr) inStr = null;
        } else if (c === '"' || c === "'") {
          inStr = c;
        } else if (c === '(' || c === '[' || c === '{') {
          depth++;
        } else if (c === ')' || c === ']' || c === '}') {
          depth--;
        } else if (c === ':' && depth === 0) {
          cut = i; break;
        }
      }
      if (cut > 0) {
        const name = s.slice(0, cut).trim();
        if (name && /^[A-Za-z0-9_-]+$/.test(name)) {
          names.push(name);
          values.push(s.slice(cut + 1).trim());
          return;
        }
      }
      names.push(null);
      values.push(s);
    });
    return ST.buildTemplateArgs(values, names, data);
  };

  ST.buildTemplateArgs = function(args, argNames, data) {
    const rawArgs = args || [];
    const names = argNames || [];
    const hasNamed = names.some(function (n) { return n != null && n !== ''; });
    if (!hasNamed) {
      return rawArgs.map(function (a) { return ST.resolveTemplateArg(a, data); });
    }
    const positional = [];
    const named = {};
    rawArgs.forEach(function (a, i) {
      const value = ST.resolveTemplateArg(a, data);
      const name = names[i];
      if (name != null && name !== '') {
        named[name] = value;
      } else {
        positional.push(value);
      }
    });
    positional.push({ __stNamedArgs: named });
    return positional;
  };

  ST.adAutoSelect = function(collections, current) {
    if (current) return current;
    const list = Array.isArray(collections) ? collections : [];
    return list.length ? list[0].file : '';
  };

  /** The given field of the auto-selected (or current) collection row. */
  ST.adAutoSelectField = function(collections, current, field) {
    const list = Array.isArray(collections) ? collections : [];
    const file = current || (list.length ? list[0].file : '');
    const row = list.filter(function (c) { return c.file === file; })[0];
    return row ? (row[field] || '') : '';
  };

  /**
   * Reflect the active collection as a class on the rail nav buttons: the button
   * whose data-file matches `activeFile` gets `ad-nav-item--active`, the rest
   * lose it. Returns '' (so it can be assigned to a throwaway signal in an
   * @effect body). DOM-touch is idempotent.
   */
  ST.adMarkActiveNav = function(activeFile) {
    const btns = document.querySelectorAll('.ad-nav-item[data-file]');
    btns.forEach(function (b) {
      b.classList.toggle('ad-nav-item--active', b.getAttribute('data-file') === activeFile);
    });
    return '';
  };

  /**
   * Flatten a type schema + an entry value into an ordered list of field
   * descriptors the admin drawer @each-renders. Each descriptor:
   *   { name, label, widget, value, path, depth, enumvals, ref }
   * Nested objects (widget 'fieldset') and object arrays (widget 'list') are
   * flattened DEPTH-FIRST with a `depth` marker + group header descriptors, so a
   * flat @each can render an arbitrarily-deep form without recursive templates.
   * The admin is a meta-renderer: widget kinds come from types.json (server),
   * never re-derived here.
   */
  // ── Admin CRUD over the ACTIVE collection (FEAT-076 W3) ───────────────────
  // The admin is a meta-renderer: the active collection is whichever file the
  // rail selected ($activeFile), its rows the $entries signal. These mutators
  // mirror the FEAT-075 @data-collection optimistic-CRUD pattern (snapshot →
  // optimistic apply → EditJsonArray/EditJson via window.__stDevWs → revert on
  // reject), but parameterized by the runtime-active file instead of a static
  // `@data collection` name. Each republishes $entries via ST.setData so the
  // list @each + drawer re-render immediately.

  ST._adReadEntries = function() {
    // Prefer the live SpacetimeLocal signal (what the @effect/adFormFields read),
    // falling back to the @data registry. Both are kept in sync by _adPublishEntries.
    const sl = (typeof window !== 'undefined') ? window.SpacetimeLocal : null;
    if (sl && Array.isArray(sl.entries)) return sl.entries;
    const v = (this.getData) ? this.getData('entries') : null;
    return Array.isArray(v) ? v : [];
  };
  // Publish the entries array through BOTH reactive channels (the @data registry
  // AND the SpacetimeLocal signal + local:entries:updated event), exactly like
  // the reactive-source primitive's publish() — so the list @each, the drawer
  // form, and every imperative SpacetimeLocal.entries reader all see the update.
  ST._adPublishEntries = function(arr) {
    if (this.setData) this.setData('entries', arr);
    if (typeof window !== 'undefined') {
      window._localState = window._localState || {};
      window._localState['entries'] = arr;
      if (window.SpacetimeLocal) window.SpacetimeLocal['entries'] = arr;
      document.dispatchEvent(new CustomEvent('local:entries:updated', { detail: arr }));
    }
  };
  ST._adFile = function() {
    const sl = (typeof window !== 'undefined') ? window.SpacetimeLocal : null;
    return (sl && sl.activeFile) ? sl.activeFile : '';
  };
  // Send an EditJsonArray op for the active file + register revert-on-reject.
  // The revert is bound to the file CAPTURED at send time: if the user switches
  // collections while the op is in flight, a late reject must NOT mutate the new
  // collection's entries (cross-collection corruption). The guard no-ops the
  // revert when the active file has changed.
  ST._adSendArray = function(op, revert) {
    const ws = (typeof window !== 'undefined') ? window.__stDevWs : null;
    if (!ws || !ws.connected) return; // no transport: optimistic apply stands
    const capturedFile = ST._adFile();
    const opId = 'ad_' + Date.now() + '_' + Math.random().toString(36).substr(2, 6);
    ws.send({ type: 'EditJsonArray', file: capturedFile, path: '', op: op, op_id: opId });
    if (ws.onReject) ws.onReject(opId, (reason) => {
      if (ST._adFile() !== capturedFile) return; // collection switched — don't corrupt the other
      revert(reason);
    });
  };

  /**
   * Two-click delete arm. First click arms (returns true → the button reflects an
   * armed state via $delArmed); a second click confirms: deletes the active
   * entry, closes the drawer, disarms. A 3s auto-disarm timer prevents a stale
   * arm from persisting (which would turn a later single click into an
   * unconfirmed delete — a data-loss footgun). The arm is also reset whenever the
   * drawer opens on a new entry (see the @effect in drawer.st). Returns the new
   * armed state.
   */
  ST.adArmDelete = function(armed, index) {
    if (!armed) {
      // Arm + schedule a 3s auto-disarm.
      if (typeof window !== 'undefined') {
        if (this._adDisarmTimer) clearTimeout(this._adDisarmTimer);
        this._adDisarmTimer = setTimeout(() => {
          if (window.SpacetimeLocal && window.SpacetimeLocal.delArmed) {
            window.SpacetimeLocal.delArmed = false;
            document.dispatchEvent(new CustomEvent('local:delArmed:updated', { detail: false }));
          }
        }, 3000);
      }
      return true;
    }
    // confirm: delete + close + disarm
    if (this._adDisarmTimer) { clearTimeout(this._adDisarmTimer); this._adDisarmTimer = null; }
    this.adDelete(index);
    if (typeof window !== 'undefined' && window.SpacetimeLocal) {
      window.SpacetimeLocal.drawerOpen = false;
      window.SpacetimeLocal.activeIndex = -1;
    }
    return false;
  };

  /** Append a blank entry built from the active type's schema defaults. */
  ST.adInsert = function(item) {
    const arr = this._adReadEntries().slice();
    const at = arr.length;
    arr.push(item);
    this._adPublishEntries(arr);
    this._adSendArray({ op: 'Insert', item: item, index: null }, () => {
      const cur = this._adReadEntries().slice();
      const i = cur.indexOf(item);
      if (i !== -1) cur.splice(i, 1); else if (at < cur.length) cur.splice(at, 1);
      this._adPublishEntries(cur);
    });
    return '';
  };

  /** Build a blank entry from a type schema: each property gets a typed empty. */
  ST.adBlankEntry = function(schema) {
    const out = {};
    const props = (schema && schema.properties) || {};
    Object.keys(props).forEach(function(name) {
      const node = props[name] || {};
      const t = node.type;
      if (t === 'boolean') out[name] = false;
      else if (t === 'number' || t === 'integer') out[name] = 0;
      else if (t === 'array') out[name] = [];
      else if (t === 'object') out[name] = {};
      else out[name] = '';
    });
    return out;
  };

  /** Delete the entry at index (optimistic; re-insert on reject). */
  ST.adDelete = function(index) {
    const i = Number(index);
    const arr = this._adReadEntries().slice();
    if (isNaN(i) || i < 0 || i >= arr.length) return '';
    const removed = arr[i];
    arr.splice(i, 1);
    this._adPublishEntries(arr);
    this._adSendArray({ op: 'Delete', index: i }, () => {
      const cur = this._adReadEntries().slice();
      cur.splice(Math.min(i, cur.length), 0, removed);
      this._adPublishEntries(cur);
    });
    return '';
  };

  /** Move the entry from→to (optimistic; move back on reject). */
  ST.adReorder = function(from0, to0) {
    const from = Number(from0), to = Number(to0);
    const arr = this._adReadEntries().slice();
    if (isNaN(from) || isNaN(to) || from < 0 || from >= arr.length || to < 0 || to >= arr.length) return '';
    const moved = arr.splice(from, 1)[0];
    arr.splice(to, 0, moved);
    this._adPublishEntries(arr);
    this._adSendArray({ op: 'Reorder', from_index: from, to_index: to }, () => {
      const cur = this._adReadEntries().slice();
      if (to < 0 || to >= cur.length) return; // bounds guard (concurrent op shrank the array)
      const m = cur.splice(to, 1)[0];
      cur.splice(Math.min(from, cur.length), 0, m);
      this._adPublishEntries(cur);
    });
    return '';
  };

  /**
   * Keep the open drawer pinned to its entry across a reorder: if the moved row
   * (from) or the row it displaces (to) is the active entry, shift $activeIndex
   * so a subsequent field edit still targets the SAME entry (not whatever now
   * sits at the old index). Without this, reordering while the drawer is open
   * silently writes edits into the wrong entry.
   */
  ST._adReindexAfterReorder = function(from, to) {
    if (typeof window === 'undefined' || !window.SpacetimeLocal) return;
    const sl = window.SpacetimeLocal;
    const a = Number(sl.activeIndex);
    if (isNaN(a) || a < 0) return;
    let na = a;
    if (a === from) na = to;
    else if (from < a && to >= a) na = a - 1;
    else if (from > a && to <= a) na = a + 1;
    if (na !== a) { sl.activeIndex = na; document.dispatchEvent(new CustomEvent('local:activeIndex:updated', { detail: na })); }
  };

  /** Move the entry at index one position up (toward 0). */
  ST.adReorderUp = function(index) {
    const i = Number(index);
    if (i > 0) { this.adReorder(i, i - 1); this._adReindexAfterReorder(i, i - 1); }
    return '';
  };
  /** Move the entry at index one position down. */
  ST.adReorderDown = function(index) {
    const i = Number(index);
    const n = this._adReadEntries().length;
    if (i >= 0 && i < n - 1) { this.adReorder(i, i + 1); this._adReindexAfterReorder(i, i + 1); }
    return '';
  };

  /** The delete button's label, reflecting the two-click armed state. */
  ST.adDeleteLabel = function(armed) { return armed ? 'Confirm delete' : 'Delete'; };

  // ---- Media library glue (FEAT-092 W2) -----------------------------------
  // The modal/grid/search are fully declarative (media.st). These helpers are
  // network + presentation glue only: thumbnail URLs, overlay-click detection,
  // the pick→persist routing, and the multipart upload POST.

  /** Reflect data-thumb paths into background-image URLs (CSS can't read attrs). */
  ST.adReflectThumbs = function() {
    if (typeof document === 'undefined') return '';
    document.querySelectorAll('.ad-medialib-thumb[data-thumb]').forEach(function(el) {
      el.style.backgroundImage = 'url("' + encodeURI(el.dataset.thumb) + '")';
    });
    return '';
  };

  /** Pick an asset for the field the library was opened for → standard persist arc. */
  ST.adPickAsset = function(assetPath, activeIndex, fieldPath) {
    if (!assetPath || !fieldPath) return '';
    this.adEditField(activeIndex, fieldPath, assetPath);
    // Reflect into the visible drawer input + preview immediately (the form
    // rebuild only runs on selection change, not on every field persist).
    if (typeof document !== 'undefined') {
      const inp = document.querySelector('.ad-media-url[data-path="' + fieldPath + '"]');
      if (inp) inp.value = assetPath;
      const prev = document.querySelector('[data-media-prev="' + fieldPath + '"]');
      if (prev) prev.style.backgroundImage = 'url("' + encodeURI(assetPath) + '")';
    }
    return '';
  };

  /**
   * Upload a new asset: hidden file input → multipart POST /__spacetime/dev/upload
   * → the server returns the (content-hash-deduped) path → persist it into the
   * active media field + refresh the catalog so the grid shows the new asset.
   */
  ST.adUpload = function(activeIndex, fieldPath) {
    if (typeof document === 'undefined') return '';
    const self = this;
    const fi = document.createElement('input');
    fi.type = 'file';
    fi.accept = 'image/*';
    fi.style.display = 'none';
    document.body.appendChild(fi);
    fi.addEventListener('change', function() {
      const file = fi.files && fi.files[0];
      if (!file) { fi.remove(); return; }
      const fd = new FormData();
      fd.append('file', file);
      fetch('/__spacetime/dev/upload', { method: 'POST', body: fd })
        .then(function(r) { if (!r.ok) throw new Error('upload ' + r.status); return r.json(); })
        .then(function(data) {
          const path = '/' + String(data.path || '').replace(/^\//, '');
          if (fieldPath) self.adPickAsset(path, activeIndex, fieldPath);
          return self.adRefreshAssets();
        })
        .catch(function() {})
        .then(function() { fi.remove(); });
    });
    fi.click();
    return '';
  };

  /** Re-fetch the asset catalog + republish so the $assets derive re-renders. */
  ST.adRefreshAssets = function() {
    const self = this;
    return fetch('/__spacetime/dev/assets.json')
      .then(function(r) { return r.json(); })
      .then(function(d) {
        if (self.setData) self.setData('assetDoc', d);
        if (typeof window !== 'undefined' && window.SpacetimeLocal) {
          window.SpacetimeLocal['assetDoc'] = d;
        }
        if (typeof document !== 'undefined') {
          document.dispatchEvent(new CustomEvent('local:assetDoc:updated', { detail: d }));
        }
        return '';
      })
      .catch(function() { return ''; });
  };

  // ---- Relation graph layout (FEAT-092 W3) --------------------------------
  // Pure data-shaping (the accepted glue seam, like adFormFields): turn the
  // active collection's entries + the type schemas into positioned graph nodes
  // and cubic-bezier edges. The RENDERING is fully declarative SVG (graph.st
  // @each over these descriptors) — this only computes geometry. Bipartite:
  // source entries on the left, their deduped relation TARGETS on the right.

  /** The relation fields of a type: [{field, target, multi}] from the schema. */
  ST._adRelationFields = function(typeDef) {
    const props = (typeDef && typeDef.properties) || {};
    const out = [];
    Object.keys(props).forEach(function(k) {
      const p = props[k] || {};
      if (p.widget === 'relation') out.push({ field: k, target: p['x-st-ref'] || '', multi: false });
      else if (p.widget === 'relation-multi') out.push({ field: k, target: (p.items && p.items['x-st-ref']) || '', multi: true });
    });
    return out;
  };

  /** Whether the active type has any relation fields (drives the Graph toggle). */
  ST.adTypeHasRelations = function(types, activeType) {
    return ST._adRelationFields((types && types[activeType]) || {}).length > 0;
  };

  /** A label for a related target entry (its type's title role), via the rel cache. */
  ST._adTargetLabel = function(target, id) {
    const c = ST._adRelCache[target] || { rows: [], titleField: '' };
    const row = c.rows.find(function(e){ return String((e && (e.id != null && e.id !== '' ? e.id : e.slug)) || '') === String(id); });
    if (!row) return String(id);
    return String((c.titleField && row[c.titleField]) || row.title || row.name || row.label || id);
  };

  /**
   * Bipartite layout for the active collection's relation graph. Returns
   * { nodes:[{x,y,tx,ty,label,index,side}], edges:[{d}], h, hasEdges, legend:[{field,target,multi}] }.
   * Source entries get index (→ drawer open); target nodes have index -1.
   */
  ST.adGraphLayout = function(entries, types, activeType) {
    const src = Array.isArray(entries) ? entries : [];
    const rels = ST._adRelationFields((types && types[activeType]) || {});
    const titleField = ((types && types[activeType] && types[activeType].display && types[activeType].display.title) || '');
    const srcLabel = function(e, i) {
      return String((titleField && e[titleField]) || e.title || e.name || e.label || e.heading || e.id || ('Entry ' + (i + 1)));
    };
    // Collect deduped targets + edges.
    const targets = {}; // 'Type:id' -> {label}
    const tkeys = [];
    const rawEdges = []; // {fromIdx, tkey}
    src.forEach(function(e, i) {
      rels.forEach(function(r) {
        const ids = r.multi
          ? (Array.isArray(e[r.field]) ? e[r.field] : [])
          : (e[r.field] ? [e[r.field]] : []);
        ids.forEach(function(rid) {
          if (rid == null || rid === '') return;
          const tkey = r.target + ':' + rid;
          if (!(tkey in targets)) {
            targets[tkey] = { label: ST._adTargetLabel(r.target, rid) };
            tkeys.push(tkey);
          }
          rawEdges.push({ fromIdx: i, tkey: tkey });
        });
      });
    });
    const W = 760, rowH = 58, padY = 42, lx = 150, rx = W - 150;
    const h = Math.max(src.length, tkeys.length, 1) * rowH + padY * 2;
    const sy = function(i) { return padY + i * rowH + 20; };
    const nodes = [];
    src.forEach(function(e, i) {
      nodes.push({ x: lx, y: sy(i), tx: lx - 14, ty: sy(i) + 4, label: srcLabel(e, i), index: i, side: 'src' });
    });
    tkeys.forEach(function(tk, ti) {
      nodes.push({ x: rx, y: sy(ti), tx: rx + 14, ty: sy(ti) + 4, label: targets[tk].label, index: -1, side: 'tgt' });
    });
    const midx = (lx + rx) / 2;
    const edges = rawEdges.map(function(ed) {
      const ti = tkeys.indexOf(ed.tkey);
      const y1 = sy(ed.fromIdx), y2 = sy(ti);
      return { d: 'M ' + lx + ' ' + y1 + ' C ' + midx + ' ' + y1 + ', ' + midx + ' ' + y2 + ', ' + rx + ' ' + y2 };
    });
    return {
      nodes: nodes,
      edges: edges,
      h: h,
      hasEdges: edges.length > 0,
      legend: rels.map(function(r){ return { field: r.field, target: r.target, multi: r.multi }; })
    };
  };

  /** A graph node's entry index (≥ 0 for source nodes, -1 for targets). */
  ST.adGraphNodeIndex = function(raw) {
    const i = parseInt(raw, 10);
    return isNaN(i) ? -1 : i;
  };
  /** Whether a clicked graph node opens a drawer (source nodes only). */
  ST.adGraphNodeOpens = function(raw) {
    return ST.adGraphNodeIndex(raw) >= 0;
  };

  // ── Tool panes: Theme · Components · Motion (FEAT-076 W4) ─────────────────
  const adEsc = (s) => String(s == null ? '' : s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/"/g, '&quot;');

  // ---- Tool-pane data shaping (FEAT-092 W4) -------------------------------
  // The theme/palette/motion panes are now fully DECLARATIVE (tools.st: @data
  // fetch → derive → @each + templates). These helpers only SHAPE the fetched
  // JSON into flat render descriptors (the accepted data-shaping seam, like
  // adFormFields) — zero innerHTML. Each descriptor carries `tpl` = its widget
  // template name for the panes' dynamic `&$item.tpl($item)` dispatch.

  // Tool-pane header strings keyed on $pane. Kept in the shaping seam (vs an
  // inline nested-ternary derive) — presentation strings, and it sidesteps the
  // nested-ternary-in-parens derive grammar gap (BUG-084).
  ST.adToolTitle = function(pane) {
    return pane === 'brand' ? 'Brand' : (pane === 'palette' ? 'Components' : 'Motion');
  };
  ST.adToolSub = function(pane) {
    return pane === 'brand' ? 'design tokens'
      : (pane === 'palette' ? 'the project\u2019s templates' : 'declarative animations');
  };
  ST.adToolEmptyText = function(pane) {
    return pane === 'palette' ? 'No @templates in this project.'
      : (pane === 'motion' ? 'No @reveal/@scroll motion in this project.' : 'No brand singleton (@data inline + @cms editable: inline) in this project.');
  };
  /** Whether the active tool pane's descriptor list is empty (empty-state hint). */
  ST.adToolEmpty = function(pane, brandFields, componentCards, motionCards) {
    const len = function(a) { return Array.isArray(a) ? a.length : 0; };
    if (pane === 'palette') return len(componentCards) === 0;
    if (pane === 'motion') return len(motionCards) === 0;
    if (pane === 'brand') return len(brandFields) === 0;
    return false;
  };

  // Persist a brand-token edit to the .st SINGLETON literal via EditAst (PLAN-034
  // Wave B). The brand singleton lives in code as `@data inline $brand Brand :
  // { ... }`; editing a field rewrites that literal's matching key. This is the
  // cardinality-keyed write channel: singletons → EditAst (the .st value),
  // collections → EditJson (the data file). Debounced so typing a hex doesn't
  // rewrite the .st per keystroke. (Replaced ST.adEditToken / the CSS-custom-prop
  // scan — AP2.) Components + motions render directly from their server contracts
  // (sig/label delivered server-side) — no client shaper (Wave C, AP2 removal).
  ST._adBrandTimers = ST._adBrandTimers || {};
  ST.adBrandInput = function(el, name, file) {
    if (typeof window === 'undefined' || !el || !el.dataset) return '';
    // Skip motion param fields (they carry a motion address) — adMotionInput owns them.
    if (el.dataset.motionSel) return '';
    // The descriptor path is `.field`; the singleton key is the leaf token name.
    const path = el.dataset.path || '';
    const key = path.replace(/^\./, '').split('.')[0].split('[')[0];
    if (!key) return '';
    const value = el.value;
    const timerKey = (file || '') + '\u0000' + key;
    if (this._adBrandTimers[timerKey]) clearTimeout(this._adBrandTimers[timerKey]);
    this._adBrandTimers[timerKey] = setTimeout(() => {
      delete this._adBrandTimers[timerKey];
      const ws = window.__stDevWs;
      if (!ws || !ws.connected) return;
      const opId = 'adbrand_' + Date.now() + '_' + Math.random().toString(36).substr(2, 6);
      const patch = {}; patch[key] = value;
      // File-scope binding selector: `$<name> §data` → the @data inline singleton.
      ws.send({ type: 'EditAst', file: file, selector: '$' + name + ' §data', patch: patch, op_id: opId });
    }, 400);
    return '';
  };

  // FEAT-105 live preview: push a token value into the preview <iframe> WITHOUT
  // a disk write or reload. The iframe's LIVE_RELOAD_SCRIPT listens for
  // `{__spacetime:'token-preview',binding,field,value}` and writes the page's
  // reactive SpacetimeLocal[binding], so every $binding.field repaints in one
  // frame (BUG-091). Optimistic only — persistence is adBrandInput's EditAst.
  // `binding` is the singleton name ($brand → 'brand'); `field` the token leaf.
  ST.adTokenPreview = function(el, binding) {
    if (typeof window === 'undefined' || !el || !el.dataset || !binding) return '';
    const path = el.dataset.path || '';
    const field = path.replace(/^\./, '').split('.')[0].split('[')[0];
    if (!field) return '';
    const frame = document.querySelector('.ad-preview-frame');
    if (!frame || !frame.contentWindow) return '';
    frame.contentWindow.postMessage(
      { __spacetime: 'token-preview', binding: binding, field: field, value: el.value },
      window.location.origin
    );
    return '';
  };

  // FEAT-106: flatten the motion inventory (server contract) into ONE flat field
  // list the shared form-renderer @each-renders: a group header per motion
  // directive, then its widget-typed param fields. Each field is tagged with its
  // motion's selector+kind (the EditAst address) via motionSel/motionKind so the
  // persist dispatcher (adMotionInput) can route a param edit to the right scope.
  // FEAT-108: resolve a derived-token formula against the brand literal values,
  // using the SAME ST.filters the runtime derive uses. The admin page is a
  // separate document from the live preview, so it can't read the project's
  // derived signals — instead it evaluates the formula itself from the values it
  // already fetched. `$brand.x` resolves to values[x]; pipes desugar to
  // ST.filters calls; bare filter calls resolve too. Returns '' on any error.
  ST.adResolveDerived = function(formula, values) {
    if (!formula) return '';
    const vals = values || {};
    // Desugar the left-insertion pipe a|f(b) -> f(a,b) (top-level single |).
    const desugar = function(text) {
      const segs = []; let depth = 0, last = 0;
      for (let i = 0; i < text.length; i++) {
        const c = text[i];
        if (c === '"' || c === "'" || c === '`') { const q = c; i++; while (i < text.length) { if (text[i] === '\\') { i += 2; continue; } if (text[i] === q) break; i++; } continue; }
        if (c === '(' || c === '[' || c === '{') depth++;
        else if (c === ')' || c === ']' || c === '}') depth--;
        else if (c === '|' && depth === 0 && text[i-1] !== '|' && text[i+1] !== '|') { segs.push(text.slice(last, i)); last = i + 1; }
      }
      segs.push(text.slice(last));
      if (segs.length === 1) return text;
      return segs.reduce(function(acc, raw, idx) {
        if (idx === 0) return raw.trim();
        const seg = raw.trim();
        const call = /^([A-Za-z_][A-Za-z0-9_]*)\s*\((.*)\)$/.exec(seg);
        if (call) { const args = call[2].trim(); return call[1] + '(' + acc + (args ? ', ' + args : '') + ')'; }
        return seg + '(' + acc + ')';
      }, '');
    };
    try {
      let expr = desugar(formula);
      // ORDER MATTERS (FEAT-108 review P2): rewrite bare filter calls in the RAW
      // formula FIRST — they're real calls and no values are present yet to be
      // mis-matched. Doing it after $brand substitution would let a brand VALUE
      // containing a filter-name + '(' (e.g. a token literally `mix(a,b)`) get
      // rewritten inside its own string literal. The dot-guard (prev char != '.')
      // leaves member calls alone.
      expr = expr.replace(/(\.?)([A-Za-z_][A-Za-z0-9_]*)\s*\(/g, function(m, dot, name) {
        if (dot) return m;
        return (ST.filters && typeof ST.filters[name] === 'function') ? ("ST.filters['" + name + "'](") : m;
      });
      // Then substitute $brand.field -> JSON string of values[field]. JSON.stringify
      // keeps a crafted value inside a string literal (no code injection).
      expr = expr.replace(/\$[A-Za-z_][A-Za-z0-9_]*\.([A-Za-z_][A-Za-z0-9_]*)/g, function(_m, f) {
        return JSON.stringify(vals[f] == null ? '' : vals[f]);
      });
      const v = (new Function('ST', 'return (' + expr + ');'))(ST);
      return v == null ? '' : String(v);
    } catch (e) { return ''; }
  };

  // Build the WHOLE Brand-pane field list: the editable token literals followed
  // by the read-only derived chips, concatenated so a SINGLE @each renders both
  // (sequential @each blocks in one scope body don't both render — BUG-085).
  ST.adBrandPane = function(brand) {
    const b = brand || {};
    ST._adBrandDerived = Array.isArray(b.derived) ? b.derived : [];
    const editable = ST.adFormFields(b.schema, b.values, '');
    const derived = ST.adDerivedFields(b.derived, b.values);
    return editable.concat(derived);
  };

  // FEAT-108: re-resolve the derived-token chips IN PLACE from the brand pane's
  // LIVE input values (not the fetched doc, which is stale mid-edit). Called on
  // every brand-token keystroke so a derived swatch updates the instant its
  // upstream literal changes — without re-rendering the field list (which would
  // steal focus). Reads each editable color/value input by its data-path leaf,
  // recomputes each formula, and patches the chip's swatch + value text.
  ST.adRefreshDerived = function() {
    if (typeof document === 'undefined') return '';
    const derived = ST._adBrandDerived || [];
    if (!derived.length) return '';
    // Gather current literal values from the editable inputs (skip motion/derived).
    // A color field has TWO inputs sharing one data-path (the native swatch +
    // the hex text); the hex TEXT input is authoritative (it's what the user
    // types and what persists), so a `.st-fld-color-text` value overrides the swatch.
    const values = {};
    document.querySelectorAll('.ad-brand-grid .st-fld-field [data-path]').forEach((el) => {
      const key = (el.dataset.path || '').replace(/^\./, '').split('.')[0];
      if (!key || key.indexOf('__') === 0 || typeof el.value !== 'string') return;
      const isText = el.classList && el.classList.contains('st-fld-color-text');
      if (values[key] === undefined || isText) values[key] = el.value;
    });
    for (let i = 0; i < derived.length; i++) {
      const d = derived[i] || {};
      const resolved = ST.adResolveDerived(d.formula || '', values);
      const row = document.querySelector('.ad-field--derived[data-path="__derived_' + d.name + '"]');
      if (!row) continue;
      const sw = row.querySelector('.ad-derived-swatch');
      const vt = row.querySelector('.ad-derived-val');
      if (sw) sw.style.background = resolved;
      if (vt) vt.textContent = resolved;
    }
    return '';
  };

  // Build the DERIVED-token field list for the Brand pane. Each derive becomes a
  // read-only `&fld-derived` chip carrying its formula + a swatch resolved from
  // the brand literal values (see adResolveDerived). A leading group header
  // separates derived from editable tokens. Empty when no derives.
  ST.adDerivedFields = function(derived, values) {
    const out = [];
    if (!Array.isArray(derived) || derived.length === 0) return out;
    out.push({ name: '__derived', label: 'Derived', widget: 'group', tpl: 'fld-group',
               value: null, path: '__derived', depth: 0 });
    for (let i = 0; i < derived.length; i++) {
      const d = derived[i] || {};
      const resolved = ST.adResolveDerived(d.formula || '', values);
      out.push({
        name: d.name, label: d.name, widget: 'derived', tpl: 'fld-derived',
        formula: d.formula || '', value: resolved,
        path: '__derived_' + d.name
      });
    }
    return out;
  };


  // THE ONE PLACE a widget kind becomes a template name.
  //
  // Two descriptor builders feed the admin's dynamic `&$f.tpl($f)` dispatch
  // (`adFormFields` for schema/brand forms, `adMotionFields` for the motion
  // pane), and a third surface — the coming Inspector pill — will feed a fourth.
  // Encoding the shared-widget set per builder is drift bait: a widget that
  // resolves to a template nobody registered renders NOTHING, silently (the
  // failure mode PLAN-112 W2 already hit once). So every builder calls this.
  //
  // - KNOWN: widgets a template exists for. Anything else falls back rather than
  //   being dropped — a field must never vanish without a label or a warning.
  // - SHARED: widgets hoisted into `stdlib/fields`, dispatched DIRECTLY as
  //   `field-<widget>`. A thin `fld-*` wrapper delegating to the shared template
  //   cannot work: a template body that is a lone ref compiles to `html: ""`,
  //   and the runtime only renders body refs once it produced a root element.
  ST.adFieldTemplate = function(widget) {
    const KNOWN = { text:1, textarea:1, number:1, toggle:1, readonly:1, media:1, select:1, chips:1, relation:1, 'relation-multi':1, richtext:1, color:1, 'range-length':1, 'range-duration':1 };
    const FALLBACK = { json: 'textarea' };
    const SHARED = { text:1, number:1, toggle:1, color:1, 'range-length':1, 'range-duration':1 };
    // STRUCTURAL widgets are not input fields — they are the form's own scaffolding
    // (group headers for a nested object / list item, derived read-only chips).
    // They are admin-owned and must NOT fall back to a text input: routing `group`
    // through the KNOWN table would resolve it to `field-text` and replace every
    // section header with an empty textbox.
    const STRUCTURAL = { group:1, derived:1 };
    if (STRUCTURAL[widget]) return 'fld-' + widget;
    const kind = KNOWN[widget] ? widget : (FALLBACK[widget] || 'text');
    return (SHARED[kind] ? 'field-' : 'fld-') + kind;
  };

  ST.adMotionFields = function(motions) {
    const out = [];
    if (!Array.isArray(motions)) return out;
    for (let i = 0; i < motions.length; i++) {
      const m = motions[i] || {};
      out.push({ name: m.label, label: m.label, widget: 'group',
                 tpl: ST.adFieldTemplate('group'),
                 value: null, path: '__motion_' + i, depth: 0 });
      const fields = Array.isArray(m.fields) ? m.fields : [];
      for (let j = 0; j < fields.length; j++) {
        const f = fields[j] || {};
        out.push({
          // Resolve the template HERE, from the widget kind, rather than trusting
          // a `tpl` the server computed: one resolver, so the two surfaces cannot
          // disagree about which widgets are shared.
          name: f.key, label: f.label, widget: f.widget,
          tpl: ST.adFieldTemplate(f.widget),
          value: f.value, path: f.path,
          motionSel: m.selector || '', motionKind: m.kind || '', file: m.file || 'index.st'
        });
      }
    }
    return out;
  };
  ST._adMotionTimers = ST._adMotionTimers || {};

  // FEAT-106: persist a motion param edit to the .st SOURCE via the EXISTING
  // scoped EditAst path. The field carries data-motion-sel (the element scope)
  // and data-motion-kind (the directive); the selector `<sel> §<kind>` + patch
  // `{param: value}` is exactly what apply_text_patch handles for directive
  // props — no new write machinery. Debounced per (selector,kind,param).
  ST.adMotionInput = function(el, motions) {
    if (typeof window === 'undefined' || !el || !el.dataset) return '';
    const sel = el.dataset.motionSel;
    const kind = el.dataset.motionKind;
    if (!sel || !kind) return '';            // not a motion field
    const param = el.dataset.path || '';
    if (!param) return '';
    const file = el.dataset.motionFile || 'index.st';
    let value = el.value;
    if (el.type === 'checkbox') value = el.checked ? 'true' : 'false';
    const timerKey = file + '\u0000' + sel + '\u0000' + kind + '\u0000' + param;
    if (this._adMotionTimers[timerKey]) clearTimeout(this._adMotionTimers[timerKey]);
    this._adMotionTimers[timerKey] = setTimeout(() => {
      delete this._adMotionTimers[timerKey];
      const ws = window.__stDevWs;
      if (!ws || !ws.connected) return;
      const opId = 'admotion_' + Date.now() + '_' + Math.random().toString(36).substr(2, 6);
      const patch = {}; patch[param] = value;
      ws.send({ type: 'EditAst', file: file, selector: sel + ' \u00a7' + kind, patch: patch, op_id: opId });
    }, 400);
    return '';
  };

  // Relation cache: type name → that type's entries (for relation pickers).
  // The admin is a meta-renderer; a relation field references a target TYPE, so
  // the picker needs that type's collection rows. Preload every collection's data
  // keyed by type, then relation/relation-multi controls populate from it.
  ST._adRelCache = ST._adRelCache || {};
  ST.adLoadRelationData = function(collections, types) {
    const list = Array.isArray(collections) ? collections : [];
    list.forEach(function(c) {
      if (!c || !c.type || !c.file) return;
      const url = c.file[0] === '/' ? c.file : '/' + c.file;
      fetch(url).then(function(r){ return r.ok ? r.json() : []; }).then(function(data){
        ST._adRelCache[c.type] = { rows: Array.isArray(data) ? data : [], titleField: ((types && types[c.type] && types[c.type].display && types[c.type].display.title) || '') };
        // A relation fetch landed AFTER the form may have rendered with empty
        // pickers (cold open). Re-hydrate the open drawer's controls so the
        // pickers fill once their target type's rows arrive (BUG-080).
        if (typeof window !== 'undefined' && window.SpacetimeLocal && window.SpacetimeLocal.drawerOpen) {
          ST.adHydrateControls(window.SpacetimeLocal.formFields);
        }
      }).catch(function(){ ST._adRelCache[c.type] = { rows: [], titleField: '' }; });
    });
    return '';
  };
  // The display label + id for a relation option (target type's title role).
  ST._adRelOptions = function(refType) {
    const c = ST._adRelCache[refType] || { rows: [], titleField: '' };
    return c.rows.map(function(e){
      const id = (e && (e.id != null && e.id !== '' ? e.id : e.slug)) || '';
      const label = (c.titleField && e[c.titleField]) || e.title || e.name || e.label || id;
      return { id: String(id), label: String(label) };
    });
  };

  /**
   * Hydrate the rendered form controls from their field descriptors after the
   * @each builds them: populate <select> options from the field's enum, set the
   * current value on every control (selects, toggles, the descriptor value),
   * and render chips for string-array fields. Called from an @effect on
   * $formFields. The form @each builds the control SHELLS (the meta-renderer
   * can't know enum option lists at template-author time); this fills them from
   * the per-field server schema the descriptor carries.
   */
  ST.adHydrateControls = function(formFields) {
    if (typeof document === 'undefined') return '';
    const form = document.querySelector('.ad-form');
    if (!form) return '';
    const byPath = {};
    (Array.isArray(formFields) ? formFields : []).forEach(f => { if (f && f.path) byPath[f.path] = f; });
    // Selects: fill <option>s from enum + select the current value.
    form.querySelectorAll('select[data-widget="select"]').forEach(sel => {
      const f = byPath[sel.dataset.path];
      if (!f) return;
      const opts = Array.isArray(f.enumvals) ? f.enumvals : [];
      sel.innerHTML = opts.map(o =>
        '<option value="' + adEsc(o) + '"' + (String(o) === String(f.value) ? ' selected' : '') + '>' + adEsc(o) + '</option>'
      ).join('');
    });
    // Toggles: reflect the boolean value.
    form.querySelectorAll('input[data-widget="toggle"]').forEach(cb => {
      const f = byPath[cb.dataset.path];
      if (f) cb.checked = !!f.value;
    });
    // Chips: render string-array values as removable chips.
    form.querySelectorAll('[data-widget="chips"]').forEach(box => {
      const f = byPath[box.dataset.path];
      const arr = (f && Array.isArray(f.value)) ? f.value : [];
      box.innerHTML = arr.map(c =>
        '<span class="ad-chip">' + adEsc(c) + '</span>'
      ).join('') + '<input class="ad-chip-add" placeholder="Add…">';
    });
    // Single relation: a <select> over the target type's entries (by id),
    // labelled by the target's display title.
    form.querySelectorAll('select[data-widget="relation"]').forEach(sel => {
      const f = byPath[sel.dataset.path];
      if (!f) return;
      const opts = ST._adRelOptions(f.ref || sel.dataset.ref);
      // Keep the current link visible even if its target row isn't loaded: add a
      // fallback option for f.value when it matches no loaded option.
      const cur = String(f.value == null ? '' : f.value);
      const hasCur = cur === '' || opts.some(o => o.id === cur);
      sel.innerHTML = '<option value="">— none —</option>' +
        (hasCur ? '' : '<option value="' + adEsc(cur) + '" selected>' + adEsc(cur) + '</option>') +
        opts.map(o =>
          '<option value="' + adEsc(o.id) + '"' + (o.id === cur ? ' selected' : '') + '>' + adEsc(o.label) + '</option>'
        ).join('');
    });
    // Multi relation: chips of linked entries + a picker to add more.
    form.querySelectorAll('[data-widget="relation-multi"]').forEach(box => {
      const f = byPath[box.dataset.path];
      if (!f) return;
      const opts = ST._adRelOptions(f.ref || box.dataset.ref);
      const labelOf = (id) => { const o = opts.filter(x => x.id === String(id))[0]; return o ? o.label : String(id); };
      const linked = Array.isArray(f.value) ? f.value : [];
      box.innerHTML = linked.map(id =>
        '<span class="ad-chip ad-chip--rel">' + adEsc(labelOf(id)) + '</span>'
      ).join('') + '<select class="ad-input ad-rel-add"><option value="">+ link …</option>' +
        opts.map(o => '<option value="' + adEsc(o.id) + '">' + adEsc(o.label) + '</option>').join('') + '</select>';
    });
    return '';
  };

  /**
   * Handle a field-edit DOM event. The element `el` is the event target's nearest
   * [data-path] ancestor (the delegated handler resolves it). Only persist when
   * `el` is an ACTUAL widget control — it must carry data-widget. The injected
   * chip-add input / relation-multi picker have NO data-path/data-widget of their
   * own, so `closest('[data-path]')` would climb to the container <div> (value
   * undefined) and wipe the whole array — this guard makes that a no-op (the
   * add/remove of array items is handled by dedicated chip/link handlers, not the
   * generic field-edit path). BUG-080.
   */
  ST.adFieldInput = function(el, index) {
    if (!el || !el.dataset || !el.dataset.path) return '';
    // Richtext surface (PLAN-031): the @editable host stores its current document
    // AST on the element (el.__stDoc) and calls adFieldInput(host, activeIndex)
    // like any other field. Persist the AST OBJECT directly — the server validates
    // it against the editable schema before disk (FEAT-099). The richtext path is
    // chosen by the widget tag / the stored doc, not by reading el.value.
    if (el.dataset.widget === 'richtext' || el.__stDoc !== undefined) {
      var astDoc = (el.__stDoc !== undefined) ? el.__stDoc : null;
      return this.adEditField(index, el.dataset.path, astDoc);
    }
    if (!el.dataset.widget) return '';
    // Containers (chips/relation-multi) are not directly editable via this path;
    // their sub-controls add/remove items through their own handlers.
    if (el.dataset.widget === 'chips' || el.dataset.widget === 'relation-multi') return '';
    const raw = el.dataset.widget === 'toggle' ? el.checked : this.adCoerceValue(el.dataset.widget, el.value);
    return this.adEditField(index, el.dataset.path, raw);
  };


  /**
   * Persist a single field edit. `path` is the descriptor path (".field",
   * "[i].sub", ".obj.k"); the leading entry index is the active index. Updates
   * $entries[index]<path> optimistically + sends EditJson(file, fullPath, value).
   */
  ST.adEditField = function(index, path, value) {
    const i = Number(index);
    const arr = this._adReadEntries().slice();
    if (isNaN(i) || i < 0 || i >= arr.length) return '';
    // Apply value into a shallow-cloned entry by the descriptor path.
    const entry = Object.assign({}, arr[i]);
    const prev = this._adSetByPath(entry, path, value);
    arr[i] = entry;
    this._adPublishEntries(arr);
    const ws = (typeof window !== 'undefined') ? window.__stDevWs : null;
    if (ws && ws.connected) {
      const capturedFile = this._adFile();
      const opId = 'adf_' + Date.now() + '_' + Math.random().toString(36).substr(2, 6);
      // Full JSON path: the entry index + the descriptor path (normalized).
      const full = '[' + i + ']' + (path && path[0] === '.' ? path : (path ? '.' + path : ''));
      ws.send({ type: 'EditJson', file: capturedFile, path: full, value: value, op_id: opId });
      if (ws.onReject) ws.onReject(opId, () => {
        if (this._adFile() !== capturedFile) return; // collection switched
        const cur = this._adReadEntries().slice();
        if (i < cur.length) { const e2 = Object.assign({}, cur[i]); this._adSetByPath(e2, path, prev); cur[i] = e2; this._adPublishEntries(cur); }
      });
    }
    return '';
  };

  /**
   * Tokenize a descriptor path into property + array-index segments. Splits on
   * BOTH '.' and '[i]' so nested object-array paths like '.sections[0].heading'
   * descend correctly (string keys for objects, numbers for array indices) —
   * matching the server's parse_json_path. (A '.'-only split would create a junk
   * key literally named 'sections[0]'.)
   */
  ST._adPathTokens = function(path) {
    const toks = [];
    const re = /([^.\[\]]+)|\[(\d+)\]/g;
    let m;
    while ((m = re.exec(String(path || ''))) !== null) {
      toks.push(m[2] !== undefined ? Number(m[2]) : m[1]);
    }
    return toks;
  };

  /** Set a value into obj by a descriptor path; returns the previous value. */
  ST._adSetByPath = function(obj, path, value) {
    const parts = this._adPathTokens(path);
    if (!parts.length) return undefined;
    let cur = obj;
    for (let k = 0; k < parts.length - 1; k++) {
      const p = parts[k];
      const nextIsIndex = typeof parts[k + 1] === 'number';
      if (cur[p] == null || typeof cur[p] !== 'object') cur[p] = nextIsIndex ? [] : {};
      cur = cur[p];
    }
    const last = parts[parts.length - 1];
    const prev = cur[last];
    cur[last] = value;
    return prev;
  };

  /** The selected entry object from a collection's rows by index (null if oob). */
  ST.adActiveEntry = function(entries, index) {
    const list = Array.isArray(entries) ? entries : [];
    const i = Number(index);
    return (i >= 0 && i < list.length) ? list[i] : null;
  };

  /** The schema object for the active type (or an empty {properties:{}}). */
  ST.adActiveSchema = function(types, activeType) {
    return (types && types[activeType]) || { properties: {} };
  };

  /** A human title for the drawer head: the entry's display.title field, else id. */
  ST.adEntryTitle = function(entries, index, types, activeType) {
    const e = ST.adActiveEntry(entries, index);
    if (!e) return '';
    const tdef = (types && types[activeType]) || {};
    const f = (tdef.display && tdef.display.title) || '';
    return (f && e[f]) || e.title || e.name || e.label || e.heading || e.id || ('Entry ' + (Number(index) + 1));
  };

  ST.adFormFields = function(schema, entry, basePath) {
    const out = [];
    const props = (schema && schema.properties) || {};
    const walk = function(props, value, path, depth) {
      Object.keys(props).forEach(function(name) {
        const node = props[name] || {};
        const widget = node.widget || 'text';
        const v = value ? value[name] : undefined;
        const fieldPath = path + '.' + name;
        if (widget === 'fieldset') {
          out.push({ name: name, label: name, widget: 'group', tpl: 'fld-group', value: null, path: fieldPath, depth: depth });
          walk(node.properties || {}, v || {}, fieldPath, depth + 1);
        } else if (widget === 'list') {
          out.push({ name: name, label: name, widget: 'group', tpl: 'fld-group', value: null, path: fieldPath, depth: depth });
          const items = Array.isArray(v) ? v : [];
          const itemProps = (node.items && node.items.properties) || {};
          items.forEach(function(item, i) {
            out.push({ name: name + ' ' + (i + 1), label: name + ' ' + (i + 1), widget: 'group', tpl: 'fld-group', value: null, path: fieldPath + '[' + i + ']', depth: depth + 1 });
            walk(itemProps, item, fieldPath + '[' + i + ']', depth + 2);
          });
        } else {
          // Map the server widget to a registered field template. Unknown or
          // not-yet-implemented widgets (richtext, json, …) fall back to a known
          // template so a field is NEVER silently dropped (BUG: a missing
          // fld-<widget> template made invokeTemplate return null → the @each
          // skipped the field with no label/input/warning).
          const tpl = ST.adFieldTemplate(widget);
          // Preserve arrays for chips/relation-multi (the control consumes the
          // array). richtext keeps its value as the document AST OBJECT (the
          // @editable surface consumes the AST directly, PLAN-031). Everything
          // else stringifies a plain object only for the text/json fallback.
          const keepArray = (widget === 'chips' || widget === 'relation-multi');
          const keepObject = (widget === 'richtext');
          let val;
          if (v == null) val = keepArray ? [] : (keepObject ? null : '');
          else if (keepArray) val = Array.isArray(v) ? v : [];
          else if (keepObject) val = (typeof v === 'object') ? v : null;
          else if (typeof v === 'object') val = JSON.stringify(v);
          else val = v;
          // Range widgets (FEAT-107): pre-decompose the dimensioned value into
          // the slider number + unit + sane bounds, so the &fld-range template is
          // pure markup (no per-field JS). Bounds by kind: length 0–64, ms 0–2000.
          let rnum = 0, runit = '', rmin = 0, rmax = 100, rstep = 1;
          if (widget === 'range-length' || widget === 'range-duration') {
            const p = ST.adRangeParse(val);
            rnum = p.num; runit = p.unit || (widget === 'range-duration' ? 'ms' : 'px');
            if (widget === 'range-duration') { rmin = 0; rmax = 2000; rstep = 10; }
            else { rmin = 0; rmax = 64; rstep = 1; }
          }
          out.push({
            name: name,
            label: name,
            widget: widget,
            tpl: tpl,
            value: val,
            path: fieldPath,
            depth: depth,
            enumvals: node.enum || (node.items && node.items.enum) || null,
            ref: node['x-st-ref'] || (node.items && node.items['x-st-ref']) || '',
            rnum: rnum, runit: runit, rmin: rmin, rmax: rmax, rstep: rstep
          });
        }
      });
    };
    walk(props, entry, basePath || '', 0);
    return out;
  };

  ST.adLabelRows = function(entries, types, activeType) {
    const list = Array.isArray(entries) ? entries : [];
    const tdef = (types && types[activeType]) || {};
    const titleField = (tdef.display && tdef.display.title) || '';
    return list.map(function (e) {
      e = e || {};
      const label = (titleField && e[titleField]) ||
        e.title || e.name || e.label || e.heading || e.id || 'Untitled';
      const out = Object.assign({}, e);
      out._label = label;
      return out;
    });
  };

  // MCP kit (PLAN-044): gather every NAMED control inside a region root into
  // { name: value } — checkbox/radio yield boolean checked, an unselected radio
  // is skipped, everything else yields its string value. Lives here (not in the
  // primitive's %emit js) because the emit-js statement splitter mangles a
  // loop-bearing helper function; a single call expression in the primitive is
  // serialized intact. Mirrors the admin's querySelectorAll gather helpers.
  ST._mcpCollectValues = function(regionEl) {
    var values = {};
    if (!regionEl || !regionEl.querySelectorAll) return values;
    var controls = regionEl.querySelectorAll('input[name], select[name], textarea[name]');
    controls.forEach(function (c) {
      var name = c.getAttribute ? c.getAttribute('name') : '';
      if (!name) return;
      var t = (c.type || '').toLowerCase();
      var isToggle = (t === 'checkbox' || t === 'radio');
      if (t === 'radio' && !c.checked) return;
      values[name] = isToggle ? !!c.checked : c.value;
    });
    return values;
  };

  ST.setData = function(name, data) {
    let entry = this._dataRegistry.get(name);
    if (!entry) {
      entry = { data: null, loaded: false, listeners: new Set() };
      this._dataRegistry.set(name, entry);
    }

    entry.data = data;
    entry.loaded = true;

    // Notify all waiting subscribers
    for (const fn of entry.listeners) {
      try { fn(data); } catch (e) { console.error('[ST] Data callback error:', e); }
    }
    entry.listeners.clear();

    // Dispatch event for backwards compatibility
    if (typeof document !== 'undefined') {
      document.dispatchEvent(new CustomEvent(`data:${name}:loaded`, { detail: data }));
    }
  };

  /**
   * Subscribe to data source (consumer side)
   * If data already loaded, calls callback immediately via microtask
   * Otherwise, subscribes for future notification
   * @param {string} name - Data source name
   * @param {Function} fn - Callback receiving data
   * @returns {Function} Unsubscribe function
   */
  ST.afterData = function(name, fn) {
    let entry = this._dataRegistry.get(name);

    // Create placeholder if doesn't exist yet (handles out-of-order declaration)
    if (!entry) {
      entry = { data: null, loaded: false, listeners: new Set() };
      this._dataRegistry.set(name, entry);
    }

    // If already loaded, call immediately via microtask
    if (entry.loaded) {
      queueMicrotask(() => fn(entry.data));
      return () => {};  // No-op unsubscribe since already called
    }

    // Otherwise, subscribe for future notification
    entry.listeners.add(fn);
    return () => entry.listeners.delete(fn);
  };

  // ==========================================================================
  // Timing Instrumentation (for testing and performance analysis)
  // ==========================================================================

  ST._timing = ST._timing || {
    enabled: false,
    samples: [],
    lastTimestamp: null,
    driverUpdates: new Map(),

    enable() {
      this.enabled = true;
      this.samples = [];
      this.lastTimestamp = null;
      this.driverUpdates.clear();
    },

    disable() {
      this.enabled = false;
    },

    recordRAF(timestamp) {
      if (!this.enabled) return;
      if (this.lastTimestamp !== null) {
        const delta = timestamp - this.lastTimestamp;
        this.samples.push({
          timestamp,
          delta,
          fps: delta > 0 ? 1000 / delta : 0
        });
      }
      this.lastTimestamp = timestamp;
    },

    recordDriver(name, progress) {
      if (!this.enabled) return;
      if (!this.driverUpdates.has(name)) this.driverUpdates.set(name, []);
      this.driverUpdates.get(name).push({
        t: typeof performance !== 'undefined' ? performance.now() : Date.now(),
        p: progress
      });
    },

    report() {
      const deltas = this.samples.map(s => s.delta);
      const fps = this.samples.map(s => s.fps);

      const avg = arr => arr.length ? arr.reduce((a, b) => a + b, 0) / arr.length : 0;
      const min = arr => arr.length ? Math.min(...arr) : 0;
      const max = arr => arr.length ? Math.max(...arr) : 0;

      return {
        sampleCount: this.samples.length,
        avgDelta: avg(deltas),
        minDelta: min(deltas),
        maxDelta: max(deltas),
        avgFPS: avg(fps),
        minFPS: min(fps),
        maxFPS: max(fps),
        jitter: deltas.length > 0
          ? Math.sqrt(deltas.map(d => Math.pow(d - 16.67, 2)).reduce((a, b) => a + b, 0) / deltas.length)
          : 0,
        samples: this.samples.slice(),
        drivers: Object.fromEntries(this.driverUpdates)
      };
    }
  };

  // ==========================================================================
  // Fidelity Ladder (PLAN-027 W1) — refuse-to-fake gate
  // ==========================================================================
  //
  // A test (or assertion) declares the fidelity RUNG it needs. The active
  // backend advertises its max rung. An operation that needs more fidelity than
  // the backend provides MUST throw — never silently run against a faked
  // primitive (getComputedStyle = el.style || {}, synchronous rAF) and report a
  // meaningless pass. This is the runtime half of the gate; the Rust side
  // (src/rung.rs) owns the canonical model and selects the backend.
  ST._rung = ST._rung || {
    // Total order — mirror of Rung::ALL in src/rung.rs. Index = fidelity level.
    order: ['pure', 'logic', 'layout', 'timing', 'paint'],

    // Max rung this backend can faithfully provide. The runner sets this:
    //   V8 + LinkeDOM  -> 'logic'   (default)
    //   CDP / browser  -> 'paint'
    backendMax: 'logic',

    // The effective rung the current test was resolved to (set per-test by the
    // harness from the Rust-computed value). Informational; the gate uses
    // per-operation require() calls so an un-annotated test still can't fake.
    current: null,

    level(name) {
      const i = this.order.indexOf(name);
      return i < 0 ? 0 : i;
    },

    setBackend(name) {
      if (this.order.indexOf(name) >= 0) this.backendMax = name;
      return this.backendMax;
    },

    setCurrent(name) {
      this.current = name || null;
    },

    // True if the backend can satisfy `need`.
    satisfies(need) {
      return this.level(this.backendMax) >= this.level(need);
    },

    // The refuse-to-fake guard. An operation that genuinely needs `need`
    // fidelity calls this FIRST; if the backend can't provide it, throw a
    // tagged RungError so the test runner SKIPS it (deferred to a higher-fidelity
    // backend) instead of running against a fake. The error carries
    // `__stRungSkip` so the loop can distinguish a fidelity deferral from a real
    // assertion failure: a refusal on `logic` is NOT a failure, it is "runs on
    // CDP." `what` names the operation for the message.
    require(need, what) {
      if (!this.satisfies(need)) {
        const label = what ? what + ' ' : '';
        const err = new Error(
          `${label}needs '${need}' fidelity but the current backend tops out at ` +
          `'${this.backendMax}' — run on a higher-fidelity backend (e.g. CDP) ` +
          `instead of faking '${need}'`
        );
        err.__stRungSkip = true;
        err.__stRungNeed = need;
        throw err;
      }
    }
  };

  // ==========================================================================
  // Utility Functions
  // ==========================================================================

  /**
   * Global data store
   */
  ST.data = {};

  /**
   * Clone a template element by ID
   * @param {string} id - Template element ID
   * @returns {Element|null} Cloned element
   */
  ST.cloneTemplate = function(id) {
    const el = document.getElementById(id);
    if (!el) return null;
    return el.tagName === 'TEMPLATE'
      ? el.content.cloneNode(true).firstElementChild
      : el.cloneNode(true);
  };

  // ==========================================================================
  // Filter Registry (SC-003)
  // ==========================================================================

  /**
   * Built-in filter functions for content injection transforms.
   * Filters are applied via `text <- $var | filterName;` in template bodies.
   */
  // Color algebra helpers (FEAT-108). hex/rgb/hsl → {r,g,b,a} → hex, so design
  // tokens compose: `$brand.scarlet | darken(0.08)`. Pure functions — callable as
  // a pipe OR a bare fn (derived-signal resolves bare calls against ST.filters).
  // Operate in sRGB for darken/lighten/mix; alpha emits rgba() when a<1.
  function _stParseColor(str) {
    var s = String(str == null ? '' : str).trim();
    var m;
    if ((m = /^#([0-9a-fA-F]{3})$/.exec(s))) {
      return { r: parseInt(m[1][0] + m[1][0], 16), g: parseInt(m[1][1] + m[1][1], 16), b: parseInt(m[1][2] + m[1][2], 16), a: 1 };
    }
    if ((m = /^#([0-9a-fA-F]{6})$/.exec(s))) {
      return { r: parseInt(m[1].slice(0,2),16), g: parseInt(m[1].slice(2,4),16), b: parseInt(m[1].slice(4,6),16), a: 1 };
    }
    if ((m = /^#([0-9a-fA-F]{8})$/.exec(s))) {
      return { r: parseInt(m[1].slice(0,2),16), g: parseInt(m[1].slice(2,4),16), b: parseInt(m[1].slice(4,6),16), a: parseInt(m[1].slice(6,8),16)/255 };
    }
    if ((m = /^rgba?\(([^)]+)\)$/.exec(s))) {
      var p = m[1].split(',').map(function(x){ return x.trim(); });
      return { r: +p[0], g: +p[1], b: +p[2], a: p[3] !== undefined ? +p[3] : 1 };
    }
    return null;
  }
  function _stClamp(n) { return Math.max(0, Math.min(255, Math.round(n))); }
  function _stToHex(c) {
    var h = function(n) { return _stClamp(n).toString(16).padStart(2, '0'); };
    if (c.a != null && c.a < 1) {
      return 'rgba(' + _stClamp(c.r) + ', ' + _stClamp(c.g) + ', ' + _stClamp(c.b) + ', ' + (Math.round(c.a * 1000) / 1000) + ')';
    }
    return '#' + h(c.r) + h(c.g) + h(c.b);
  }
  // amount ∈ [0,1]: fraction toward black (darken) / white (lighten).
  function _stScale(v, amount, toward) { return v + (toward - v) * amount; }

  // RGB ↔ HSV for the token color picker (FEAT-107). h∈[0,360), s/v∈[0,100].
  function _stRgbToHsv(r, g, b) {
    r /= 255; g /= 255; b /= 255;
    var max = Math.max(r, g, b), min = Math.min(r, g, b), d = max - min;
    var h = 0;
    if (d !== 0) {
      if (max === r) h = ((g - b) / d) % 6;
      else if (max === g) h = (b - r) / d + 2;
      else h = (r - g) / d + 4;
      h *= 60; if (h < 0) h += 360;
    }
    var s = max === 0 ? 0 : d / max;
    return { h: h, s: s * 100, v: max * 100 };
  }
  function _stHsvToRgb(h, s, v) {
    h = ((h % 360) + 360) % 360; s /= 100; v /= 100;
    var c = v * s, x = c * (1 - Math.abs((h / 60) % 2 - 1)), m = v - c;
    var r = 0, g = 0, b = 0;
    if (h < 60) { r = c; g = x; }
    else if (h < 120) { r = x; g = c; }
    else if (h < 180) { g = c; b = x; }
    else if (h < 240) { g = x; b = c; }
    else if (h < 300) { r = x; b = c; }
    else { r = c; b = x; }
    return { r: (r + m) * 255, g: (g + m) * 255, b: (b + m) * 255 };
  }


  // Shared field compatibility surface. Admin aliases delegate here; field DOM controls
  // carry their own implementation through stdlib/fields primitives.
  ST.fieldCoerce = function(widget, raw) {
    if (widget === 'number') return raw === '' || raw == null ? null : Number(raw);
    if (widget === 'toggle') return !!raw;
    return raw;
  };
  ST.adCoerceValue = function(widget, raw) { return ST.fieldCoerce(widget, raw); };
  // Refresh the write CREDENTIALS on already-rendered editors, in place.
  //
  // Every accepted write changes the file, so the hash and span text the editors
  // are holding go stale immediately — the NEXT edit of the same node would be
  // refused by the server's staleness guard, silently, until the user re-selected
  // the node. (Observed live: edit the slider, then the toggle, and the toggle
  // does nothing.)
  //
  // This CANNOT be fixed by re-projecting the list: re-rendering the editors is
  // exactly what BUG-151 forbids, because it destroys a focused input mid-edit.
  // So the DOM stays untouched and only the address attributes are updated —
  // node identity is preserved, credentials are current.
  //
  // Values are deliberately NOT refreshed: the user's in-progress text must win
  // over the server's echo of what they just saved.
  ST.insRefreshCredentials = function(root, params, selectedNode) {
    if (!root || !Array.isArray(params)) return '';
    var fresh = {};
    params.forEach(function(p) {
      if (p && p.node_id === selectedNode) fresh[p.param] = p;
    });
    var controls = root.querySelectorAll('[data-addr-sel]');
    for (var i = 0; i < controls.length; i++) {
      var el = controls[i];
      var row = fresh[el.dataset.addrSel];
      if (row) {
        var writable = row.source_hash != null;
        el.dataset.addr = writable ? row.source_hash : '';
        el.dataset.addrKind = writable ? row.span_text : '';
        el.dataset.addrFile = row.file || '';
        el.dataset.addrStart = row.span_start;
        el.dataset.addrEnd = row.span_end;
      }
    }
    return '';
  };

  // Build ONE inspector write request from whichever control committed, or null.
  //
  // Returning null is the safety valve: the caller assigns the result straight to
  // the request signal, and the write primitive ignores an unaddressed request, so
  // every "this must not be written" case funnels through here.
  //
  // Per-widget value extraction matters — a control's `.value` is not always the
  // value the SOURCE wants:
  //   - toggle: an unchecked checkbox still reports `.value === 'on'`, so reading
  //     `.value` wrote the literal token `on` into a bool param. Read `.checked`.
  //   - number: an emptied field reports `''`, which the patcher would splice in
  //     as a BARE empty token — malformed source (`count: )`). Refuse it; the user
  //     can type a number or leave the field alone.
  //   - composite (range/colour): the control that FIRES is the slider, but the
  //     value + address live on its twin/hex partner, which is what the shared
  //     widget writes through. Resolve to that partner.
  ST.insWriteRequest = function(el) {
    if (!el || !el.dataset) return null;
    // A composite's slider carries no address; its twin (range) or hex text
    // (colour) does. Fall back to the addressed control within the same field.
    var source = el;
    if (!source.dataset.addrSel) {
      var field = el.closest ? el.closest('.st-fld-field') : null;
      source = (field && field.querySelector('[data-addr-sel]')) || el;
    }
    var d = source.dataset || {};
    // No verified snapshot → display-only row. `addr` (the hash) and `addrKind`
    // (the span text) are BOTH blanked by insEditorRows for such a row.
    if (!d.addrSel || !d.addr || !d.addrKind) return null;

    var widget = source.dataset.widget || '';
    var value;
    if (widget === 'toggle') {
      value = source.checked ? 'true' : 'false';
    } else {
      value = source.value;
    }
    if (value == null) return null;
    if (widget === 'number' && String(value).trim() === '') return null;

    return {
      file: d.addrFile || '',
      start: d.addrStart,
      end: d.addrEnd,
      param: d.addrSel,
      source_hash: d.addr,
      span_text: d.addrKind,
      value: value
    };
  };

    // Select a source invocation from either a tree row or a stamped page
    // instance. Keeping this state transition shared prevents pick mode from
    // drifting from the editor projection contract.
    ST.insSelectNode = function(root, selectedNode, params) {
      if (!root || !selectedNode) return '';
      ST.set(root, 'insSelectedNode', selectedNode);
      ST.set(root, 'insActiveParams', ST.insEditorRows(params, selectedNode));
      return '';
    };

    // Select a data record. Mirrors `insSelectNode` (the invocation path) so both
    // kinds of selection drive the SAME two signals and the panel needs no branch.
    ST.insSelectDataNode = function(root, addr) {
      if (!root || !addr) return '';
      var record = null;
      var entry = ST._dataRegistry && ST._dataRegistry.get(addr.source);
      var arr = entry && entry.data;
      if (!arr && typeof window !== 'undefined' && window.SpacetimeData) {
        arr = window.SpacetimeData[addr.source];
      }
      if (Array.isArray(arr)) record = arr[addr.index];
      // Fall back to the row's retained item (`__stItem`, kept by @each for
      // re-filtering) when the registry has not settled — the DOM is the same
      // data, and a selection that silently produced no editors would read as the
      // feature being broken.
      if (!record && addr.row && addr.row.__stItem) record = addr.row.__stItem;
      if (!record) return '';
      ST.set(root, 'insSelectedNode', ST.insDataNodeId(addr));
      ST.set(root, 'insActiveParams', ST.insDataRows(addr, record));
      return '';
    };

    // ---- DATA-ROW SELECTION (@each over @data) -------------------------------
    //
    // A template-less page — 12 of 23 projects in this repo — has no invocation
    // to select: its editable surface is the JSON records behind `@each`. These
    // helpers give that surface the same read→edit rail templates already have,
    // addressed by (source, index, field) rather than by a source byte span.

    // Resolve a clicked element to its data address. `field` is optional: clicking
    // a row's padding selects the whole record and offers every addressable field.
    ST.insDataAddress = function(el) {
      if (!el || !el.closest) return null;
      var row = el.closest('[data-st-source][data-st-index]');
      if (!row) return null;
      var fieldEl = el.closest('[data-st-field]');
      return {
        source: row.dataset.stSource,
        index: Number(row.dataset.stIndex),
        field: (fieldEl && row.contains(fieldEl)) ? fieldEl.dataset.stField : null,
        row: row
      };
    };

    // A stable id for a data selection, so the pill can hold ONE `insSelectedNode`
    // signal whichever kind of thing is selected.
    ST.insDataNodeId = function(addr) {
      return addr ? ('data:' + addr.source + '[' + addr.index + ']') : '';
    };

    // Project one record into editor descriptors. Every addressable field stamped
    // on the row is offered — not just the clicked one — because a record is the
    // unit a person edits; clicking a title and being unable to reach its body
    // would be a worse tool. Widget kind is inferred from the VALUE, since a
    // `@data` record carries no declared param types the way a template does.
    ST.insDataRows = function(addr, record) {
      if (!addr || !record) return [];
      var fields = [];
      var stamped = addr.row.querySelectorAll('[data-st-field]');
      for (var i = 0; i < stamped.length; i++) {
        var f = stamped[i].dataset.stField;
        if (f && fields.indexOf(f) === -1) fields.push(f);
      }
      // A field the row renders but did not stamp (ambiguous duplicate text, or a
      // piped hole) is deliberately absent: it has no unambiguous element, so an
      // edit could not be shown back to the user honestly.
      var out = [];
      for (var j = 0; j < fields.length; j++) {
        var name = fields[j];
        var value = record[name];
        if (value === undefined || value === null) value = '';
        if (typeof value === 'object') continue; // nested shapes are not a flat field edit
        var w = ST.insWidgetForValue(value);
        out.push({
          param: name,
          label: name,
          value: String(value),
          widget: w,
          tpl: ST.adFieldTemplate(w),
          node_id: ST.insDataNodeId(addr),
          // The data ADDRESS the widgets render into `data-data-*`, which
          // `ST.insDataWrite` reads back when the control commits.
          source: addr.source,
          index: addr.index,
          dataField: name,
          checked: (w === 'toggle' && (value === true || value === 'true')) ? 'checked' : false,
          // A data row has no source span or hash: it is addressed by
          // (source, index, field) into a JSON file, not by `.st` bytes. Blanking
          // these keeps `ST.insWriteRequest` from ever claiming this row — the two
          // write rails stay strictly separate.
          addr: '',
          addrSel: '',
          addrKind: '',
          addrFile: '',
          addrStart: '',
          addrEnd: ''
        });
      }
      return out;
    };

    // Commit a data-field edit over the EXISTING EditJson rail — the same one the
    // admin CMS uses (`ClientMessage::EditJson { file, path, value, op_id }`,
    // handled in src/sync/handlers.rs). This is deliberately NOT the pill's
    // source-span writer: a `@data` value lives in a JSON file, not in `.st`
    // bytes, so it has no span and no source hash to guard. Reusing the JSON rail
    // means the server-side validation, the ack/reject protocol, and the
    // write-back safety already in place all apply unchanged.
    //
    // The file comes from `ST._dataRegistry`, which `@data fetch`/`collection`
    // populates with each source's backing path at init.
    ST.insDataWrite = function(el) {
      if (!el || !el.dataset) return null;
      var source = el;
      if (!source.dataset.dataField) {
        var field = el.closest ? el.closest('.st-fld-field') : null;
        source = (field && field.querySelector('[data-data-field]')) || el;
      }
      var d = source.dataset || {};
      if (!d.dataField || !d.dataSource || d.dataIndex == null) return null;

      var widget = d.widget || '';
      var raw = (widget === 'toggle') ? (source.checked ? 'true' : 'false') : source.value;
      if (raw == null) return null;
      if (widget === 'number' && String(raw).trim() === '') return null;

      // Restore the JSON type the record actually holds, so an edit never changes
      // a number into a string (which would silently alter the data's shape).
      var value = raw;
      if (widget === 'number') {
        var n = Number(raw);
        if (!isFinite(n)) return null;
        value = n;
      } else if (widget === 'toggle') {
        value = (raw === 'true');
      }

      var file = ST.insDataFilePath(d.dataSource);
      if (!file) return null;

      return {
        file: file,
        path: '[' + d.dataIndex + '].' + d.dataField,
        value: value
      };
    };

    // Resolve a data source to its SITE-RELATIVE backing file.
    //
    // The registry stores the source's URL (`/data/pains.json`) because that is
    // what the fetch used. The EditJson rail addresses a file on disk relative to
    // the site root, so the leading slash must go — sending the URL verbatim gets
    // "File not found: /data/pains.json" and the edit is silently rejected.
    // Same rule as `resolveFilePath` in stdlib/__dev__/primitives/dev-save.st,
    // which is where this convention is already established.
    ST.insDataFilePath = function(source) {
      var entry = ST._dataRegistry && ST._dataRegistry.get(source);
      var src = entry && entry.src;
      if (!src || typeof src !== 'string') return null;
      return src.charAt(0) === '/' ? src.substring(1) : src;
    };

    // Infer a widget from a JSON value. Deliberately conservative: only shapes we
    // can round-trip losslessly get a rich control, everything else stays text.
    ST.insWidgetForValue = function(v) {
      if (typeof v === 'boolean') return 'toggle';
      if (typeof v === 'number') return 'number';
      if (typeof v === 'string' && /^#[0-9a-fA-F]{3,8}$/.test(v)) return 'color';
      return 'text';
    };

    // Count all mounted instances that map to one source invocation. `@each`
    // deliberately gives each iteration the same structural id: a source edit
    // changes all of them, so the pill must disclose that shared scope.
    // `poll` is a CHANGE TICKET, not data: the count is read from the live DOM,
    // so the caller passes a signal that changes on each poll to force a
    // recompute as instances mount/unmount.
    ST.insInstanceCount = function(selectedNode, poll) {
      void poll;
      if (!selectedNode || typeof document === 'undefined') return 0;
      var nodes = document.querySelectorAll('[data-st-node]');
      var count = 0;
      for (var i = 0; i < nodes.length; i++) {
        if (nodes[i].dataset && nodes[i].dataset.stNode === selectedNode) count++;
      }
      return count;
    };

    // Pick one root's walk out of the all-roots response.
    //
    // The server sends every root at once (`roots`) because a `@data fetch` URL is
    // a literal and cannot be re-pointed at a different `?template=`. When `root`
    // is empty the server's own chosen root applies, which is the top-level
    // `nodes`/`params` — so the default costs no lookup and an older response
    // shape (no `roots`) still renders instead of going blank.
    function insRootSlice(structure, root, key) {
      if (!structure) return [];
      if (root && structure.roots && structure.roots[root]) {
        return structure.roots[root][key] || [];
      }
      return structure[key] || [];
    }
    ST.insNodesForRoot = function(structure, root) {
      return insRootSlice(structure, root, 'nodes');
    };
    ST.insParamsForRoot = function(structure, root) {
      return insRootSlice(structure, root, 'params');
    };

    // Project the inspect rail's param rows for ONE selected node into descriptors
  // the shared `field-*` widgets can render (PLAN-112 W3).
  //
  // This is the pill's whole read→edit contract in one place. Each row must carry:
  //   - what to RENDER: `tpl` (via the single dispatch resolver), `label`, and a
  //     display `value` with its source quotes stripped;
  //   - for a range widget, the pre-split number/unit it tunes;
  //   - the WRITE ADDRESS: file + byte span, plus the two staleness credentials
  //     (`source_hash`, `span_text`) the server refuses a patch without.
  //
  // The address rides the widgets' generic `data-addr-*` pass-through, so the
  // pill's one delegated change handler reads it straight off the control that
  // fired — no wrapper element, and nothing for a row template to lose.
  // A row whose snapshot could NOT be verified (`source_hash: null`) is display
  // only: it is handed an EMPTY address, which the write rail rejects and the
  // stylesheet renders inert.
  // The shared editor-descriptor builder. The inspector supplies a source-token
  // display transform; the workbench keeps its already-evaluated values intact.
  // Bindings lack a server widget kind, so their declared type picks the same
  // shared-widget vocabulary before the one `ST.adFieldTemplate` dispatch.
  function fieldWidgetForType(typeRef) {
    var type = String(typeRef || '').toLowerCase();
    if (type.indexOf('bool') === 0) return 'toggle';
    if (type.indexOf('colo') === 0) return 'color';
    if (type.indexOf('int') === 0 || type.indexOf('uint') === 0 || type.indexOf('float') === 0 || type.indexOf('number') === 0) return 'number';
    return 'text';
  }

  ST.fieldEditorRows = function(rows, displayValueFn, paramKind) {
    if (!Array.isArray(rows)) return [];
    var kind = paramKind || 'node-param';
    return rows.filter(function(row) { return !!row; }).map(function(row) {
      var display = displayValueFn ? displayValueFn(row.value) : (row.value == null ? '' : String(row.value));
      var range = ST.fieldRangeParse(display);
      var writable = row.source_hash != null;
      var widget = row.widget || fieldWidgetForType(row.type);
      // Bounds are PER WIDGET KIND, matching the admin's own `adFormFields`
      // derivation. A single 0-200 range is wrong for both kinds it serves: the
      // native slider CLAMPS to its max, so a `600ms` duration snapped to 200ms
      // on first touch — the control silently destroyed the value it was editing.
      var bounds = (widget === 'range-duration')
        ? { min: 0, max: 2000, step: 10 }
        : { min: 0, max: 64, step: 1 };
      return Object.assign({}, row, {
        tpl: ST.adFieldTemplate(widget),
        label: row.param,
        value: display,
        rnum: range.num,
        runit: range.unit,
        rmin: bounds.min,
        rmax: bounds.max,
        rstep: bounds.step,
        addr: writable ? row.source_hash : '',
        addrSel: row.param,
        addrKind: writable ? row.span_text : '',
        addrFile: row.file || '',
        addrStart: row.span_start,
        addrEnd: row.span_end,
        widget: widget,
        // `checked` rides the SAME rendering pass as every other attribute (the
        // emit path drops a boolean attribute when the value is false), so a
        // toggle shows its true state on mount — no post-mount hydration, and no
        // dependence on when selector-init happens to run.
        checked: (function () {
          var v = display;
          return (v === true || v === 'true' || v === '1' || v === 'on' || v === 'yes') ? 'checked' : false;
        })(),
        spanStart: row.span_start == null ? '' : row.span_start,
        spanEnd: row.span_end == null ? '' : row.span_end,
        paramKind: kind
      });
    });
  };

  ST.insEditorRows = function(params, selectedNode) {
    if (!Array.isArray(params)) return [];
    return ST.fieldEditorRows(
      params.filter(function(p) { return p && p.node_id === selectedNode; }),
      ST.insDisplayValue
    );
  };

  ST.bindingEditorRows = function(bindings) {
    return ST.fieldEditorRows(bindings, null, 'binding');
  };

  // The inspector rail reports a param's value as its RAW SOURCE TOKEN — a string
  // literal arrives with its quotes (`"Welcome"`, `"#FF0020"`). A text input then
  // displays the quotes as content, and a native colour swatch cannot parse the
  // token at all and silently falls back to #000000. Strip ONE layer of matching
  // surrounding quotes for display.
  //
  // The write direction needs the same bare form: the server's patcher re-quotes
  // whatever it is given to match what the source had (`emit_patch_value`), so a
  // value that kept its quotes here would be written back double-quoted.
  // Non-string tokens (numbers, `$signal` refs) have no quotes and pass through.
  ST.insDisplayValue = function(value) {
    var s = (value == null) ? '' : String(value);
    if (s.length >= 2) {
      var head = s.charAt(0), tail = s.charAt(s.length - 1);
      if (head === '"' && tail === '"') {
        // DECODE, don't just unwrap. Stripping the outer quotes alone leaves the
        // source ESCAPES in the displayed text (`He said \\"hi\\"`), and sending
        // that back re-escapes the backslashes — so merely saving an untouched
        // value silently changed it (W3R review finding). Parsing the token gives
        // the real string; the server re-encodes it exactly once on write.
        try {
          return JSON.parse(s);
        } catch (e) {
          // Not valid JSON (a single-quoted or malformed literal): fall through
          // to the conservative unwrap rather than showing the raw token.
          return s.slice(1, -1);
        }
      }
      if (head === "'" && tail === "'") {
        return s.slice(1, -1);
      }
    }
    return s;
  };

  // Split a dimensioned token value (`8px`, `600ms`) into its number + unit, so a
  // range slider can tune the number while the unit is preserved on write-back.
  // PURE — the shared `fields` module's range widget and the admin's descriptor
  // builder (`adFormFields`, which pre-decomposes range values so the template
  // stays markup-only) both need it, so it lives here with the other shared
  // value helpers rather than inside either consumer.
  ST.fieldRangeParse = function(value) {
    var m = /^\s*(-?[0-9]*\.?[0-9]+)\s*([a-z%]*)\s*$/i.exec(String(value == null ? '' : value));
    if (!m) return { num: 0, unit: '' };
    return { num: parseFloat(m[1]), unit: m[2] || '' };
  };
  ST.adRangeParse = function(value) { return ST.fieldRangeParse(value); };
  ST.fieldColorToHsv = function(value) {
    var c = _stParseColor(value) || { r: 0, g: 0, b: 0, a: 1 }, hsv = _stRgbToHsv(c.r, c.g, c.b);
    return { h: hsv.h, s: hsv.s, v: hsv.v, a: c.a == null ? 1 : c.a };
  };
  ST.fieldHsvToHex = function(h, s, v, a) {
    var rgb = _stHsvToRgb(Number(h) || 0, Number(s) || 0, Number(v) || 0);
    return _stToHex({ r: rgb.r, g: rgb.g, b: rgb.b, a: (a == null || a === '') ? 1 : Math.max(0, Math.min(1, Number(a))) });
  };
  ST.adColorToHsv = function(value) { return ST.fieldColorToHsv(value); };
  ST.adHsvToHex = function(h, s, v, a) { return ST.fieldHsvToHex(h, s, v, a); };

  // --- FILTERS ---------------------------------------------------------------
  // SINGLE SOURCE (GH-11 / PLAN-138): the DECLARED filter surface is
  // stdlib/primitives/data/filters.st — 15 %primitive definitions. The
  // build-time SSG unroller evaluates those bodies directly
  // (src/html/ssg_filters.rs), so build output and hydrated output come from ONE
  // definition and cannot drift.
  //
  // This table is the RUNTIME half and is not yet generated from that registry.
  // Until it is, the invariant to preserve is: a name here MUST match the
  // %primitive of the same name in filters.st, and a name declared there but
  // missing here is a BUG, not a gap to route around.
  //
  // History, so this is not re-learned the hard way: there used to be a THIRD
  // table in public/runtime/data-binding.js (deleted - nothing bundled it) and
  // the three disagreed. capitalize was declared in stdlib and implemented in
  // neither runtime table, so it silently failed at build time AND at runtime.
  // If you are adding a filter, add the %primitive first.
  ST.filters = {
    uppercase: function(v) { return String(v).toUpperCase(); },
    lowercase: function(v) { return String(v).toLowerCase(); },
    // GH-11: capitalize was DECLARED in filters.st and implemented NOWHERE --
    // it failed at build time (leaked as literal pipe text) and at runtime (no
    // such filter, value passed through). Mirrors the %primitive exactly:
    // the FIRST character of the whole string, not each word.
    capitalize: function(v) {
      var str = String(v);
      return str.charAt(0).toUpperCase() + str.slice(1);
    },
    // BUG-188: the documented filter set used by the canonical data-binding
    // example (`| currency("$")`, `| truncate(100)`, `| if("a", "b")`) —
    // previously absent here, so ST.filter passed values through unfiltered.
    // currency mirrors stdlib %primitive currency (Intl, "$2,400.00");
    // truncate/if mirror the data-binding engine's SpacetimeFilters.
    currency: function(v, symbol) {
      var num = typeof v === 'number' ? v : (parseFloat(v) || 0);
      var cur = (symbol && symbol !== '$') ? symbol : 'USD';
      try {
        return new Intl.NumberFormat('en-US', { style: 'currency', currency: cur }).format(num);
      } catch (e) {
        return (symbol || '$') + num.toFixed(2);
      }
    },
    truncate: function(v, length) {
      var str = String(v == null ? '' : v);
      var len = parseInt(length, 10);
      if (isNaN(len)) len = 50;
      return str.length > len ? str.substring(0, len) + '...' : str;
    },
    'if': function(v, trueVal, falseVal) {
      return v ? trueVal : falseVal;
    },
    number: function(v, fmt) {
      var n = Number(v);
      if (isNaN(n)) return String(v);
      if (fmt) {
        var match = fmt.match(/\.(\d+)/);
        if (match) return n.toFixed(parseInt(match[1], 10));
      }
      return String(n);
    },
    darken: function(v, amount) {
      var c = _stParseColor(v); if (!c) return v;
      var a = Math.max(0, Math.min(1, Number(amount) || 0));
      return _stToHex({ r: _stScale(c.r, a, 0), g: _stScale(c.g, a, 0), b: _stScale(c.b, a, 0), a: c.a });
    },
    lighten: function(v, amount) {
      var c = _stParseColor(v); if (!c) return v;
      var a = Math.max(0, Math.min(1, Number(amount) || 0));
      return _stToHex({ r: _stScale(c.r, a, 255), g: _stScale(c.g, a, 255), b: _stScale(c.b, a, 255), a: c.a });
    },
    alpha: function(v, a) {
      var c = _stParseColor(v); if (!c) return v;
      return _stToHex({ r: c.r, g: c.g, b: c.b, a: Math.max(0, Math.min(1, Number(a) || 0)) });
    },
    mix: function(v, other, weight) {
      var c1 = _stParseColor(v), c2 = _stParseColor(other);
      if (!c1 || !c2) return v;
      var w = weight == null ? 0.5 : Math.max(0, Math.min(1, Number(weight)));
      return _stToHex({
        r: _stScale(c1.r, w, c2.r), g: _stScale(c1.g, w, c2.g),
        b: _stScale(c1.b, w, c2.b), a: _stScale(c1.a, w, c2.a)
      });
    }
  };

  /**
   * Apply a named filter to a value.
   * Parses filter strings like "uppercase" or "currency(\"$\")".
   * @param {string} name - Filter name, optionally with args in parens
   * @param {any} value - Value to filter
   * @param {...any} args - Additional arguments
   * @returns {any} Filtered value, or original if filter not found
   */
  ST.filter = function(name, value) {
    if (!name) return value;
    // Parse filter name and args from string like "currency(\"$\")" or "uppercase"
    var match = name.match(/^(\w+)(?:\((.+)\))?$/);
    if (!match) return value;
    var filterName = match[1];
    var filterArgs = match[2]
      ? match[2].split(',').map(function(a) { return a.trim().replace(/^["']|["']$/g, ''); })
      : [];
    var fn = ST.filters[filterName];
    return fn ? fn.apply(null, [value].concat(filterArgs)) : value;
  };

  // ==========================================================================
  // Auto-cleanup on element removal + dynamic element initialization
  // ==========================================================================

  if (typeof MutationObserver !== 'undefined' && typeof document !== 'undefined') {
    const observer = new MutationObserver((mutations) => {
      for (const m of mutations) {
        // Handle removed nodes (cleanup) — see ST._handleRemovedNode.
        for (const node of m.removedNodes) {
          ST._handleRemovedNode(node);
        }

        // Handle added nodes (initialize dynamic elements)
        for (const node of m.addedNodes) {
          if (node.nodeType === Node.ELEMENT_NODE) {
            ST._scheduleInit(node);
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

  // Export to global
  global.ST = ST;

  // Export for module systems
  if (typeof module !== 'undefined' && module.exports) {
    module.exports = ST;
  }

  // Dispatch ready event for user code that needs to wait for ST
  // Also set a flag so late listeners can check if already ready
  if (typeof document !== 'undefined') {
    window.__spacetime_ready = true;
    document.dispatchEvent(new CustomEvent('spacetime:ready', { detail: { ST } }));
  }

  // Helper for late listeners: ST.onReady(callback)
  ST.onReady = function(fn) {
    if (window.__spacetime_ready) {
      fn();
    } else {
      document.addEventListener('spacetime:ready', fn, { once: true });
    }
  };

})(typeof window !== 'undefined' ? window : typeof global !== 'undefined' ? global : this);
