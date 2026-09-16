/// Data Channel Module Primitive (FUP-175 R1)
///
/// Owns `__stPublish(name, value)` — the ONE dual-channel publish every
/// data-producing primitive shares:
///   1. `ST.setData(name, value)` — the data registry channel (@each/@computed
///      coordination; handles the 'loaded' event internally).
///   2. The global signal store — write THROUGH the reactive Proxy when it
///      exists (its set-trap dispatches for us), otherwise write the backing
///      store (`_localState`) and dispatch `local:<name>:updated` ourselves.
///
/// LOAD-BEARING (moved verbatim from data-source): the fallback dispatch is
/// deferred to a MICROTASK. Firing it synchronously — inside a load() right
/// after ST.setData() — races the data-registry (afterData) consumers (@each,
/// @computed) that key on the same source name, and breaks @each render.
/// And never replace an existing Proxy with a plain object — that loses the
/// write-trap and makes $state signals unreachable.
///
/// Before this module FOUR copies existed and had already drifted: computed-
/// source and derived-signal omitted the registry channel AND dispatched
/// synchronously — the exact race the canonical version defers against.
%primitive data-channel() {
  %emit prelude-js {
    // Signal-only half: the global signal store write + deferred dispatch,
    // WITHOUT the registry channel — for auxiliary signals (event-source's
    // suffixed channels) that are not registry-backed data names.
    window.__stPublishSignal = window.__stPublishSignal || function (name, value) {
      var __hadProxy = !!(window.SpacetimeLocal && window.SpacetimeLocal.__stProxy);
      if (__hadProxy) {
        window.SpacetimeLocal[name] = value;
      } else {
        window._localState = window._localState || {};
        window._localState[name] = value;
        if (!window.SpacetimeLocal) window.SpacetimeLocal = window._localState;
        // No Proxy yet — dispatch manually, deferred to a microtask (see the
        // module docstring: synchronous dispatch races afterData consumers).
        Promise.resolve().then(function () {
          document.dispatchEvent(new CustomEvent('local:' + name + ':updated', { detail: value }));
        });
      }
    };

    window.__stPublish = window.__stPublish || function (name, value) {
      if (typeof ST !== 'undefined' && ST.setData) ST.setData(name, value);
      window.__stPublishSignal(name, value);
    };
  }
}
