/// Signal Watch Module Primitive (FUP-175 follow-on)
///
/// Owns `__stWatchSignals(deps, recompute)` — the ONE dep-watch channel:
/// subscribe a recompute to every compiler-discovered dep (bridging
/// element-scoped primitive exports into the global plane first — the
/// BUG-069 rail — and keying @data fetch sources through the data registry
/// too), then run the initial compute after init settles. Extracted from
/// derived-signal's prelude when derive-match became the second consumer:
/// `%uses` targets must be param-free module primitives, and derived-signal
/// has required params, so the shared half moves here.
%primitive signal-watch() {
  %emit prelude-js {
    window.__stWatchSignals = window.__stWatchSignals || function (deps, recompute) {
      deps.forEach(function (d) {
        if (typeof ST !== 'undefined' && ST._bridgeExport) ST._bridgeExport(d);
        document.addEventListener('local:' + d + ':updated', recompute);
        // @data fetch sources publish via the data registry too.
        if (typeof ST !== 'undefined' && ST.afterData) ST.afterData(d, recompute);
      });
      if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', recompute);
      } else {
        recompute();
      }
    };
  }
}
