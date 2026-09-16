/// `dnd-runtime` — the drag-and-drop runtime, shipped as a prelude.
///
/// A YIELD-LESS primitive whose only job is to install `window.__stDnd` via
/// `%emit prelude-js`. Both `@drop-zone` (the zone primitive) and `@drag` (the
/// gesture) `%binds` this primitive, so the prelude is emitted ONCE per page
/// (deduplicated by primitive name) — the global runtime (`public/runtime/st.js`)
/// is never touched.
///
/// `__stDnd` is the registry + hit-test seam between zones and drags:
///
///   zones[group]            — array of live zone ENTRIES for a group.
///   register(entry)         — a zone adds itself on mount.
///   unregister(entry)       — a zone removes itself on cleanup.
///   hitTest(x,y,group,ignore)
///                           — overlay-resilient: walk `elementsFromPoint`
///                             front-to-back (so a fixed overlay — dev panel,
///                             presence cursor, toast — never shadows a zone),
///                             with `ignore` (the dragged element) made
///                             pointer-transparent, and return the FIRST
///                             registered entry in `group` whose element is at or
///                             above the pointed node. Returns null over nothing.
///                             This is the hardened walk formerly in
///                             `ST.dropTarget`, now keyed off the REGISTRY (not a
///                             magic selector) and owned by the module.
///
/// A zone ENTRY is `{ el, group, dataset, accepts, setHovering, setAccepting }`:
///   el           — the zone's DOM element.
///   dataset      — the element's `dataset` (read as `$zone.col` → `data-col`).
///   accepts(p)   — the compiled `accepts:` predicate over the dragged payload
///                  `p` (default `() => true`). Evaluated BY the drag at hover.
///   setHovering  — drives the zone's `$hovering` signal (the `over` state).
///   setAccepting — drives the zone's `$accepting` signal (the `reject` gate).

%primitive dnd-runtime() {
  %emit prelude-js {
    window.__stDnd = window.__stDnd || (function () {
      var zones = Object.create(null);

      function bucket(group) {
        var g = group || "";
        return (zones[g] = zones[g] || []);
      }

      function register(entry) {
        if (!entry || !entry.el) return;
        bucket(entry.group).push(entry);
      }

      function unregister(entry) {
        if (!entry) return;
        var arr = zones[entry.group || ""];
        if (!arr) return;
        var i = arr.indexOf(entry);
        if (i >= 0) arr.splice(i, 1);
      }

      // Resolve the registered zone ENTRY under (x, y) within `group`.
      // `ignore` (the dragged element) is made pointer-transparent for the
      // hit-test so the zone BENEATH it is found. Walks the whole stack
      // front-to-back and returns the first entry whose element is the node or
      // an ancestor of it — overlays in front that are not zones are skipped.
      function hitTest(x, y, group, ignore) {
        if (typeof document === "undefined" || x == null || y == null) return null;
        var arr = zones[group || ""];
        if (!arr || !arr.length) return null;

        var prevPE = ignore ? ignore.style.pointerEvents : null;
        if (ignore) ignore.style.pointerEvents = "none";
        var stack;
        try {
          stack = (typeof document.elementsFromPoint === "function")
            ? document.elementsFromPoint(x, y)
            : (document.elementFromPoint ? [document.elementFromPoint(x, y)] : []);
        } finally {
          if (ignore) ignore.style.pointerEvents = prevPE;
        }
        if (!stack || !stack.length) return null;

        for (var s = 0; s < stack.length; s++) {
          var node = stack[s];
          if (!node) continue;
          for (var z = 0; z < arr.length; z++) {
            var entry = arr[z];
            if (entry.el === node || (entry.el.contains && entry.el.contains(node))) {
              return entry;
            }
          }
        }
        return null;
      }

      return {
        zones: zones,
        register: register,
        unregister: unregister,
        hitTest: hitTest,
      };
    })();
  }
}
