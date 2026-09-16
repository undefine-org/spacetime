// Web-platform API shims for the headless V8 + LinkeDOM `logic` backend (BUG-129).
//
// The bare V8 + LinkeDOM backend lacks several DOM/Web-platform globals that
// logic-rung tests legitimately exercise (HTML sanitization, contenteditable
// selection, viewport observation, WAAPI, focus). These minimal, faithful shims
// are built on the LinkeDOM document already present — keeping such tests on the
// fast backend rather than erroring with ReferenceError/TypeError.
//
// Layout/timing FIDELITY stays gated to CDP by the rung ladder; these shims add
// only logic-level surface (presence + callback contract), never faked geometry.

// ---------------------------------------------------------------------------
// Event init-dict properties. LinkeDOM's MouseEvent/KeyboardEvent constructors
// drop their specialized init fields (`key`, `shiftKey`, `clientX`, `button`,
// ...) - only base Event fields (`bubbles`, `cancelable`) survive. Tests that
// dispatch `new KeyboardEvent('keydown', { key: 'Enter' })` then branch on
// `e.key` therefore never match. Wrap each constructor to copy any init field
// not already present onto the instance.
// ---------------------------------------------------------------------------
(function () {
    const KNOWN_BASE = new Set(['bubbles', 'cancelable', 'composed', 'detail']);
    const wrap = (name) => {
        const Orig = globalThis[name];
        if (typeof Orig !== 'function' || Orig.__stInitWrapped) return;
        const Wrapped = function (type, init) {
            const ev = new Orig(type, init || {});
            if (init && typeof init === 'object') {
                for (const k in init) {
                    if (KNOWN_BASE.has(k)) continue;
                    if (ev[k] === undefined || ev[k] === null) {
                        try { ev[k] = init[k]; } catch (e) { }
                    }
                }
            }
            return ev;
        };
        Wrapped.prototype = Orig.prototype;
        Wrapped.__stInitWrapped = true;
        try { globalThis[name] = Wrapped; } catch (e) { }
        if (typeof globalThis.window !== 'undefined') {
            try { globalThis.window[name] = Wrapped; } catch (e) { }
        }
    };
    ['MouseEvent', 'KeyboardEvent', 'PointerEvent', 'InputEvent', 'WheelEvent', 'TouchEvent'].forEach(wrap);
})();

// ---------------------------------------------------------------------------
// DOMParser. LinkeDOM ships one, but its `text/html` parse mis-nests <body>
// content (body ends up empty, content as siblings). Its `parseHTML` populates
// document.body correctly — wrap that and return the parsed document.
// ---------------------------------------------------------------------------
if (typeof globalThis.DOMParser === 'undefined') {
    globalThis.DOMParser = class DOMParser {
        parseFromString(html, _type) {
            if (typeof linkedom !== 'undefined' && linkedom.parseHTML) {
                return linkedom.parseHTML(
                    '<html><body>' + (html || '') + '</body></html>'
                ).document;
            }
            throw new Error('DOMParser shim: linkedom.parseHTML unavailable');
        }
    };
}

// ---------------------------------------------------------------------------
// window.getSelection. contenteditable edit flows query the selection to place
// the caret / read the edited range. A minimal collapsed-selection stub
// satisfies the read/clear surface those flows use.
// ---------------------------------------------------------------------------
(function () {
    let __range = null;
    const __selection = {
        rangeCount: 0,
        anchorNode: null,
        focusNode: null,
        isCollapsed: true,
        getRangeAt() { return __range; },
        removeAllRanges() { __range = null; this.rangeCount = 0; },
        addRange(r) { __range = r; this.rangeCount = 1; },
        collapse() { this.rangeCount = 0; },
        collapseToEnd() { this.rangeCount = 0; },
        selectAllChildren() { },
        toString() { return ''; }
    };
    if (typeof globalThis.getSelection === 'undefined') {
        globalThis.getSelection = () => __selection;
    }
    if (typeof globalThis.window !== 'undefined' && !globalThis.window.getSelection) {
        globalThis.window.getSelection = () => __selection;
    }
    if (typeof document !== 'undefined' && !document.getSelection) {
        document.getSelection = () => __selection;
    }
    if (typeof document !== 'undefined' && !document.createRange) {
        document.createRange = () => ({
            setStart() { }, setEnd() { },
            selectNodeContents() { }, collapse() { },
            cloneRange() { return this; },
            deleteContents() { },
            insertNode() { },
            commonAncestorContainer: document.body
        });
    }
})();

// ---------------------------------------------------------------------------
// IntersectionObserver. `@on visible` registers one to fire when an element
// enters the viewport. Headless has no layout/scroll, so the faithful
// logic-level behavior is: observe() reports the element as intersecting on a
// microtask (the test asserts the handler fired, not real geometry — geometry
// is a `layout`-rung concern gated to CDP).
// ---------------------------------------------------------------------------
if (typeof globalThis.IntersectionObserver === 'undefined') {
    // Logic-level visibility heuristic. Headless has no real layout, but a test
    // declares an element's position via inline style. Honor that declaration:
    // an element pushed below the fold (`top: 200vh`, a large positive top, or
    // `display:none`) reports NOT intersecting; everything else reports visible.
    // Real pixel-accurate intersection remains a `layout`-rung concern (CDP).
    const __isLikelyVisible = (el) => {
        try {
            const st = el && el.style;
            if (!st) return true;
            if (st.display === 'none' || st.visibility === 'hidden') return false;
            const top = (st.top || '').trim();
            if (top) {
                const vh = top.match(/^(-?\d+(?:\.\d+)?)vh$/);
                if (vh && parseFloat(vh[1]) >= 100) return false;
                const px = top.match(/^(-?\d+(?:\.\d+)?)px$/);
                if (px && parseFloat(px[1]) >= 2000) return false;
            }
            return true;
        } catch (e) { return true; }
    };
    globalThis.IntersectionObserver = class IntersectionObserver {
        constructor(cb, opts) { this._cb = cb; this._opts = opts || {}; this._els = new Set(); }
        observe(el) {
            this._els.add(el);
            const visible = __isLikelyVisible(el);
            const entry = {
                target: el,
                isIntersecting: visible,
                intersectionRatio: visible ? 1 : 0,
                boundingClientRect: el.getBoundingClientRect ? el.getBoundingClientRect() : {},
                intersectionRect: {}, rootBounds: null, time: Date.now()
            };
            // Fire on a microtask so callers finish wiring first (matches the
            // real async-callback contract closely enough for logic-rung tests).
            // An offscreen element reports a non-intersecting entry rather than
            // staying silent, so `$visible` correctly resolves to false.
            Promise.resolve().then(() => { try { this._cb([entry], this); } catch (e) { } });
        }
        unobserve(el) { this._els.delete(el); }
        disconnect() { this._els.clear(); }
        takeRecords() { return []; }
    };
}

// ---------------------------------------------------------------------------
// ResizeObserver. The element-ref primitive (stdlib/syntax/element-ref.st)
// constructs one to keep CSS size vars fresh; LinkeDOM doesn't provide it, so
// a bare `new ResizeObserver(...)` threw and dropped the ref-registration that
// precedes it. A no-op shim keeps the primitive's IIFE running so the named
// ref registers (BUG: selector-form &name refs emitted nothing).
// ---------------------------------------------------------------------------
(function () {
    if (typeof globalThis.ResizeObserver !== 'undefined') return;
    globalThis.ResizeObserver = class ResizeObserver {
        constructor(cb) { this._cb = cb; this._els = new Set(); }
        observe(el) { this._els.add(el); }
        unobserve(el) { this._els.delete(el); }
        disconnect() { this._els.clear(); }
        takeRecords() { return []; }
    };
})();

// ---------------------------------------------------------------------------
// Element.prototype.animate (WAAPI). be_animating asserts an element has an
// active animation. Provide a minimal Animation stub registered on the element
// so getAnimations()/be_animating can detect it.
// ---------------------------------------------------------------------------
(function () {
    const proto = (typeof globalThis.Element !== 'undefined' && globalThis.Element.prototype) ||
        (document && document.body && Object.getPrototypeOf(document.body));
    if (proto && typeof proto.animate !== 'function') {
        proto.animate = function (keyframes, options) {
            const dur = (options && typeof options === 'object') ? (options.duration || 0) : (options || 0);
            const el = this;
            const anim = {
                playState: 'running', currentTime: 0,
                effect: { getTiming: () => ({ duration: dur }) },
                play() { this.playState = 'running'; },
                pause() { this.playState = 'paused'; },
                cancel() {
                    this.playState = 'idle';
                    const list = el.__animations || [];
                    const i = list.indexOf(this);
                    if (i >= 0) list.splice(i, 1);
                },
                finish() { this.playState = 'finished'; },
                finished: Promise.resolve(),
                onfinish: null
            };
            el.__animations = el.__animations || [];
            el.__animations.push(anim);
            return anim;
        };
    }
    if (proto && typeof proto.getAnimations !== 'function') {
        proto.getAnimations = function () { return this.__animations || []; };
    }
})();

// ---------------------------------------------------------------------------
// CSSOM value coercion. A real browser's CSSStyleDeclaration stringifies values
// passed to setProperty (so getPropertyValue('--x') returns '0.5', not 0.5).
// LinkeDOM stores the raw value, so a Number round-trips as a Number and a
// `=== '0.5'` assertion fails. Wrap setProperty to coerce to string, matching
// the real CSSOM contract that the product's `ST.set` (el.style.setProperty)
// and the tests both rely on.
// ---------------------------------------------------------------------------
(function () {
    let cssProto = null;
    try {
        const el = document.createElement('div');
        cssProto = el.style && Object.getPrototypeOf(el.style);
    } catch (e) { }
    if (cssProto && typeof cssProto.setProperty === 'function' && !cssProto.__stCoerced) {
        const __orig = cssProto.setProperty;
        cssProto.setProperty = function (name, value, priority) {
            return __orig.call(this, name, value == null ? value : String(value), priority);
        };
        cssProto.__stCoerced = true;
    }
})();

// ---------------------------------------------------------------------------
// Focus tracking. LinkeDOM has no activeElement/focus model. Track the focused
// element so focus-trap / first-invalid-field tests can assert it.
// ---------------------------------------------------------------------------
// FocusEvent: contenteditable/blur flows construct `new FocusEvent('blur')`.
// LinkeDOM lacks it; alias to CustomEvent (carries type + bubbles, enough for
// logic-rung handlers).
if (typeof globalThis.FocusEvent === 'undefined' && typeof globalThis.CustomEvent !== 'undefined') {
    globalThis.FocusEvent = class FocusEvent extends CustomEvent {
        constructor(type, init) { super(type, init || {}); this.relatedTarget = (init && init.relatedTarget) || null; }
    };
}

(function () {
    const proto = (typeof globalThis.Element !== 'undefined' && globalThis.Element.prototype) ||
        (document && document.body && Object.getPrototypeOf(document.body));
    // Force-install tracking focus/blur even if LinkeDOM already has a no-op
    // `focus` (it does, and it does NOT update activeElement), so be_focused can
    // observe the focused element. Mark with a flag for idempotency.
    if (proto && !proto.__stFocusTracked) {
        proto.focus = function () {
            try { document.__activeElement = this; } catch (e) { }
            if (typeof this.dispatchEvent === 'function') {
                try { this.dispatchEvent(new (globalThis.FocusEvent || CustomEvent)('focus')); } catch (e) { }
            }
        };
        proto.blur = function () {
            try { if (document.__activeElement === this) document.__activeElement = document.body || null; } catch (e) { }
            if (typeof this.dispatchEvent === 'function') {
                try { this.dispatchEvent(new (globalThis.FocusEvent || CustomEvent)('blur')); } catch (e) { }
            }
        };
        proto.__stFocusTracked = true;
    }
    // Redefine activeElement to read our tracked element. LinkeDOM ships an
    // activeElement that never updates (returns a fixed/empty value), so always
    // override it (configurable) to reflect the last focus() call.
    if (typeof document !== 'undefined') {
        if (document.__activeElement === undefined) document.__activeElement = document.body || null;
        try {
            Object.defineProperty(document, 'activeElement', {
                configurable: true,
                get() { return document.__activeElement || document.body || null; }
            });
        } catch (e) { }
    }
})();

// ---------------------------------------------------------------------------
// AbortController / AbortSignal (BUG-196). Every interactive animation driver
// (mouse/hover/focus/click in stdlib/primitives/animation/drivers.st) builds
// one at the top of its selector-init; without it the init throws AFTER the
// element's _st_init flag is set, so the driver silently never attaches (the
// scanner swallows the exception). Minimal faithful contract: aborted flag,
// abort event listeners, throwIfAborted.
// ---------------------------------------------------------------------------
if (typeof globalThis.AbortController === 'undefined') {
    class StAbortSignal {
        constructor() {
            this.aborted = false;
            this.reason = undefined;
            this.onabort = null;
            this.__listeners = new Set();
        }
        addEventListener(type, fn) {
            if (type === 'abort' && typeof fn === 'function') this.__listeners.add(fn);
        }
        removeEventListener(type, fn) {
            if (type === 'abort') this.__listeners.delete(fn);
        }
        throwIfAborted() {
            if (this.aborted) throw this.reason;
        }
        __fire(reason) {
            if (this.aborted) return;
            this.aborted = true;
            this.reason = reason;
            const ev = { type: 'abort', target: this };
            if (typeof this.onabort === 'function') {
                try { this.onabort(ev); } catch (e) { }
            }
            for (const fn of [...this.__listeners]) {
                try { fn.call(this, ev); } catch (e) { }
            }
            this.__listeners.clear();
        }
    }
    globalThis.AbortSignal = StAbortSignal;
    globalThis.AbortController = class AbortController {
        constructor() { this.signal = new StAbortSignal(); }
        abort(reason) {
            this.signal.__fire(reason !== undefined ? reason : new Error('AbortError'));
        }
    };
}

// Signal-aware addEventListener. LinkeDOM accepts the `{ signal }` option but
// ignores it, so driver destroy() paths (controller.abort()) would leak
// listeners across tests in the same world. Honor the contract: pre-aborted
// signals skip registration; abort removes the listener.
(function () {
    const proto = (typeof globalThis.EventTarget !== 'undefined' && globalThis.EventTarget.prototype) || null;
    if (proto && !proto.__stAbortAware) {
        const __origAdd = proto.addEventListener;
        const __origRemove = proto.removeEventListener;
        proto.addEventListener = function (type, listener, options) {
            const signal = options && typeof options === 'object' ? options.signal : null;
            if (signal && typeof signal === 'object' && 'aborted' in signal) {
                if (signal.aborted) return;
                if (typeof signal.addEventListener === 'function') {
                    const target = this;
                    signal.addEventListener('abort', function () {
                        try { __origRemove.call(target, type, listener, options); } catch (e) { }
                    });
                }
            }
            return __origAdd.call(this, type, listener, options);
        };
        proto.__stAbortAware = true;
    }
})();

// ---------------------------------------------------------------------------
// window.matchMedia (BUG-196 related). The reveal engine and scheme detection
// call matchMedia(query).addEventListener('change', ...); without it the
// reveal engine throws on its first line. Logic-level contract: presence +
// listener registry, light-scheme default (matches only explicit light
// queries). Never faked geometry — a `(min-width: N)` query honestly reports
// no match in a viewport-less world.
// ---------------------------------------------------------------------------
(function () {
    if (typeof globalThis.window === 'undefined') return;
    if (typeof globalThis.window.matchMedia === 'function') return;
    const mql = (query) => {
        const q = String(query);
        return {
            matches: /prefers-color-scheme:\s*light/.test(q),
            media: q,
            onchange: null,
            addEventListener() { },
            removeEventListener() { },
            addListener() { },
            removeListener() { },
            dispatchEvent() { return false; }
        };
    };
    globalThis.window.matchMedia = mql;
    if (typeof globalThis.matchMedia !== 'function') {
        globalThis.matchMedia = mql;
    }
})();

// ---------------------------------------------------------------------------
// HTMLInputElement.checked ← the `checked` ATTRIBUTE (BUG-217).
//
// LinkeDOM reflects `disabled` and `value` from their attributes but NOT
// `checked`: `<input type="checkbox" checked>` parses with the attribute present
// and `el.checked === false`. Real browsers reflect it as the element's DEFAULT
// checkedness, so a test asserting on a pre-checked box failed here and passed in
// Chromium — a backend disagreement, not a Spacetime defect.
//
// Two of the test framework's own self-tests (`@then … should be_checked` and
// `be_unchecked`) were red on this, long enough to read as normal.
//
// `HTMLInputElement` is not a global here — LinkeDOM exposes its classes per
// document — so the prototype is reached through an instance the first time one
// is created, and patched once. Once the property is SET, that value wins for the
// element's lifetime, matching the DOM, where interaction detaches checkedness
// from the attribute.
// ---------------------------------------------------------------------------
(function () {
    let patched = false;
    const OWN = Symbol('stCheckedOverride');

    globalThis.__stPatchCheckedReflection = function (el) {
        if (patched || !el || el.tagName !== 'INPUT') return;
        const proto = Object.getPrototypeOf(el);
        if (!proto) return;
        patched = true;

        const existing = Object.getOwnPropertyDescriptor(proto, 'checked');
        // If LinkeDOM already reflects the attribute, leave its accessor alone.
        if (existing && typeof existing.get === 'function') {
            try {
                const probe = Object.create(proto);
                if (probe.checked === true) return;
            } catch (_) {
                /* fall through and install ours */
            }
        }

        Object.defineProperty(proto, 'checked', {
            configurable: true,
            enumerable: false,
            get() {
                if (this[OWN] !== undefined) return this[OWN];
                return !!(this.hasAttribute && this.hasAttribute('checked'));
            },
            set(v) {
                this[OWN] = !!v;
            },
        });
    };
})();

// Apply the patch as soon as any INPUT is observed. Hooking `querySelector` /
// `querySelectorAll` catches every path a test can reach an input by — fixtures,
// mounts, and page markup alike — without depending on how the element was built.
(function () {
    const patch = globalThis.__stPatchCheckedReflection;
    if (!patch || typeof document === 'undefined') return;

    const proto = Object.getPrototypeOf(document) || Document.prototype;
    for (const name of ['querySelector', 'querySelectorAll']) {
        const orig = proto && proto[name];
        if (typeof orig !== 'function') continue;
        Object.defineProperty(proto, name, {
            configurable: true,
            writable: true,
            value: function (...args) {
                const out = orig.apply(this, args);
                try {
                    if (out && out.tagName === 'INPUT') patch(out);
                    if (out && out.nodeType === 1 && globalThis.__stPatchBlur) {
                        globalThis.__stPatchBlur(out);
                    }
                    if (out && out.tagName === 'SELECT' && globalThis.__stPatchSelectValue) {
                        globalThis.__stPatchSelectValue(out);
                    }
                    else if (out && out.length) {
                        for (const el of out) {
                            if (el && el.tagName === 'INPUT') { patch(el); break; }
                        }
                    }
                } catch (_) {
                    /* patching is best-effort; never break a query */
                }
                return out;
            },
        });
    }
})();

// ---------------------------------------------------------------------------
// HTMLElement.blur() → clear document.activeElement (BUG-217).
//
// LinkeDOM implements `focus()` (it sets `activeElement`) but its `blur()` does
// not clear it, so an element stayed "focused" forever. `@when <sel> blur` could
// therefore never be observed, and the framework's own self-test for it was red.
// Another backend disagreement, not a Spacetime defect: the same test passes in
// Chromium.
//
// Patched through an instance, like the `checked` reflection above, because
// LinkeDOM exposes its element classes per document rather than as globals.
// ---------------------------------------------------------------------------
(function () {
    let patched = false;

    // The element most recently blurred, or null. Cleared by any `focus()`.
    let blurred = null;

    globalThis.__stPatchBlur = function (el) {
        if (patched || !el) return;
        let proto = Object.getPrototypeOf(el);
        // Walk up to whichever prototype actually owns `blur`/`focus`.
        while (proto && !Object.getOwnPropertyDescriptor(proto, 'blur')) {
            proto = Object.getPrototypeOf(proto);
        }
        if (!proto) return;
        patched = true;

        const origBlur = proto.blur;
        Object.defineProperty(proto, 'blur', {
            configurable: true,
            writable: true,
            value: function (...args) {
                const doc = this.ownerDocument || (typeof document !== 'undefined' ? document : null);
                let out;
                if (typeof origBlur === 'function') {
                    out = origBlur.apply(this, args);
                }
                // Only clear if THIS element is the focused one — blurring a
                // non-focused element must not steal focus from another.
                //
                // W2 REVIEW: the first version installed a getter returning
                // `doc.body` PERMANENTLY, so `activeElement` froze and no later
                // `focus()` could ever be observed — one blur broke focus tracking
                // for the rest of the runtime. Track the blurred element in a
                // mutable cell instead, and let a subsequent `focus()` win by
                // clearing it.
                try {
                    if (doc && doc.activeElement === this) {
                        blurred = this;
                    }
                } catch (_) {
                    /* best-effort */
                }
                return out;
            },
        });

        // `focus()` cancels a pending blur, so focus tracking keeps working.
        const origFocus = proto.focus;
        if (typeof origFocus === 'function') {
            Object.defineProperty(proto, 'focus', {
                configurable: true,
                writable: true,
                value: function (...args) {
                    blurred = null;
                    return origFocus.apply(this, args);
                },
            });
        }

        // Present `activeElement` as null-when-blurred, without destroying the
        // document's own accessor: read through to it and mask only the element
        // that was blurred and not since re-focused.
        try {
            const doc = el.ownerDocument || (typeof document !== 'undefined' ? document : null);
            if (doc) {
                const docProto = Object.getPrototypeOf(doc);
                const desc =
                    Object.getOwnPropertyDescriptor(doc, 'activeElement') ||
                    (docProto && Object.getOwnPropertyDescriptor(docProto, 'activeElement'));
                if (desc && typeof desc.get === 'function') {
                    const read = desc.get;
                    Object.defineProperty(doc, 'activeElement', {
                        configurable: true,
                        get() {
                            const current = read.call(this);
                            return current && current === blurred ? this.body || null : current;
                        },
                    });
                }
            }
        } catch (_) {
            /* best-effort */
        }
    };
})();

// ---------------------------------------------------------------------------
// HTMLSelectElement.value ← the selected option (BUG-217, W2 review).
//
// LinkeDOM does not implement `select.value`: reading returns undefined and
// assigning does not move the selection. `@when <sel> change "b"` therefore could
// not set a dropdown, and the framework's own self-test for it was red.
//
// Real semantics, kept minimal but honest:
//   get → the `value` of the first option with `selected`, else the FIRST option
//         (a select with no explicit selection shows its first option), else ''.
//   set → select the option whose value matches and deselect the others; an
//         unmatched value selects nothing and reads back as '' (the DOM's own
//         behaviour, rather than silently keeping the old selection).
//
// Patched through an instance like the sibling shims above, because LinkeDOM
// exposes its element classes per document rather than as globals.
// ---------------------------------------------------------------------------
(function () {
    let patched = false;

    globalThis.__stPatchSelectValue = function (el) {
        if (patched || !el || el.tagName !== 'SELECT') return;
        const proto = Object.getPrototypeOf(el);
        if (!proto) return;

        const existing = Object.getOwnPropertyDescriptor(proto, 'value');
        if (existing && typeof existing.get === 'function') {
            try {
                // A working accessor round-trips; leave LinkeDOM's alone if so.
                const probe = el.cloneNode(true);
                const first = probe.querySelector('option');
                if (first) {
                    probe.value = first.getAttribute('value');
                    if (probe.value === first.getAttribute('value')) return;
                }
            } catch (_) {
                /* fall through and install ours */
            }
        }
        patched = true;

        const optionsOf = (sel) => Array.from(sel.querySelectorAll('option'));
        const valueOf = (opt) =>
            opt.hasAttribute('value') ? opt.getAttribute('value') : opt.textContent;

        Object.defineProperty(proto, 'value', {
            configurable: true,
            enumerable: false,
            get() {
                const opts = optionsOf(this);
                const chosen =
                    opts.find((o) => o.hasAttribute('selected')) || opts[0] || null;
                return chosen ? valueOf(chosen) : '';
            },
            set(v) {
                const target = String(v);
                for (const o of optionsOf(this)) {
                    if (valueOf(o) === target) o.setAttribute('selected', '');
                    else o.removeAttribute('selected');
                }
            },
        });
    };
})();
