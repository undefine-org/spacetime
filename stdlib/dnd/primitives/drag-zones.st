/// `drag-zones` — the drop-lifecycle runtime for a draggable (PLAN-058 W3).
///
/// Binds alongside the generic `gesture` primitive (the dnd `@drag` macro binds
/// BOTH): `gesture` handles the visual drag (delta/transform/active), while
/// `drag-zones` tracks WHICH drop zone the pointer is over and fires the
/// declarative lifecycle. It hit-tests the pointer against the module's
/// `__stDnd` registry (installed by the `dnd-runtime` prelude) — NOT a magic
/// selector — and toggles the hovered zone's `$hovering`/`$accepting` signals
/// (which drive that zone's `over`/`reject` states), then runs the matching
/// lifecycle arm with `$zone` (the zone's dataset proxy) and `$payload` (the
/// dragged datum) bound as `ST.runMutations` locals — the SAME rail `@handle`'s
/// receive arms use.
///
/// Self-contained pointer tracking (its own pointerdown/move/up listeners on the
/// element) keeps `gesture` generic: dnd logic lives ENTIRELY in the dnd module.
///
/// @param group   - the draggable's group; hit-tests only `__stDnd.zones[group]`
/// @param payload - the dragged datum (object expr text), bound as `$payload`
/// @param arms    - the lifecycle arms [{ clause, zone?, body } | { cancel, cbody }]
///
/// @yields $overZone - true while the pointer is over an accepting zone

%primitive drag-zones(
  &el,
  group: string = "",
  payload: any? = none,
  arms: any? = none
) {
  %emit js {
    // NB: use the outer `el` (the selector-init param) by CLOSURE — do NOT write
    // `var el = %&el`, which lowers to `var el = el` and self-shadows into
    // `undefined` under var-hoisting (the listeners then never attach).
    (function () {
      var group = %group || "";
      var arms = %arms || [];

      // The dragged payload (`payload: <expr>`), bound as `$payload` in the
      // lifecycle arms. A `$.`-form expression (`$.dataset`, `$.dataset.id`) is a
      // DOM read against THIS element — evaluate it live with `$` = el. Any other
      // form (a `$signal` datum, an object literal) is inlined by the macro and
      // used as-is. A read that throws falls back to the element's dataset (the
      // common case: a card carrying its fields in `data-*`).
      var payloadExpr = %payload;
      var payload;
      if (typeof payloadExpr === "string" && payloadExpr.indexOf("$.") === 0) {
        try {
          payload = (new Function("$", "return (" + payloadExpr + ");"))(el);
        } catch (e) {
          payload = el.dataset;
        }
      } else if (payloadExpr === undefined || payloadExpr === null || payloadExpr === "none") {
        payload = el.dataset;
      } else {
        payload = payloadExpr;
      }

      // Index the lifecycle arm bodies by clause name. The capture grammar has
      // two alternation shapes (see capture-types/lifecycle.st):
      //   with a $zone bind  → { clause, zone, zbody }
      //   without (on-cancel)→ { bare, body }
      // The shared body also surfaces a draggable's STATE blocks (e.g.
      // `dragging { … }`) as `{ bare, body }` arms — keep ONLY `on-*` clauses so
      // a state is never run as an arm. Routing is a data lookup by clause name,
      // not a match per clause — adding a clause needs no code here.
      var armBody = {};
      for (var i = 0; i < arms.length; i++) {
        var a = arms[i];
        if (!a) continue;
        var name = a.clause || a.bare;
        if (!name || name.indexOf("on-") !== 0) continue;
        armBody[name] = (a.zbody !== undefined && a.zbody !== null) ? a.zbody : a.body;
      }

      // Run a lifecycle arm body with `$zone` (the hovered zone's dataset) +
      // `$payload` (the dragged datum) bound via runMutations' `params` channel
      // — the $-SIGIL substitution path. (NOT `locals`: that channel binds BARE
      // names like @handle's `Created(todo)` carry and deliberately refuses a
      // `$`-prefixed token, so `$zone`/`$payload` would never substitute there.)
      // Both are plain-object snapshots so runMutations' JSON-encode survives
      // (a live DOMStringMap stringifies to `{}`).
      function snapshot(o) {
        if (o == null) return o;
        if (typeof DOMStringMap !== "undefined" && o instanceof DOMStringMap) {
          var p = {};
          for (var k in o) { p[k] = o[k]; }
          return p;
        }
        return o;
      }
      function runArm(clause, zoneEntry) {
        var text = armBody[clause];
        if (!text || typeof ST === "undefined" || !ST.runMutations) return;
        var stmts = String(text).split(";").map(function (s) { return s.trim(); }).filter(Boolean);
        if (!stmts.length) return;
        var params = {};
        // `$zone.col` reads the zone's data-col; `$payload.id` reads the datum.
        if (zoneEntry) params.zone = snapshot(zoneEntry.dataset);
        if (payload !== undefined && payload !== null && payload !== "none") {
          params.payload = snapshot(payload);
        }
        ST.runMutations(el, { js_statements: stmts }, null, params);
      }

      var dragging = false;
      var pointerId = null;
      var hoverEntry = null; // the zone entry currently under the pointer

      function resolveZone(x, y) {
        if (typeof window === "undefined" || !window.__stDnd) return null;
        return window.__stDnd.hitTest(x, y, group, el);
      }

      // Update the hovered zone as the pointer moves: fire on-leave for the old
      // zone, on-enter for the new, and drive each zone's hovering/accepting
      // signals (the over/reject states).
      function updateHover(x, y) {
        var next = resolveZone(x, y);
        if (next === hoverEntry) {
          // Same zone — refresh accepting in case the predicate is payload-driven.
          if (next && next.setAccepting) next.setAccepting(!!next.accepts(payload));
          return;
        }
        // Leave the old zone.
        if (hoverEntry) {
          if (hoverEntry.setHovering) hoverEntry.setHovering(false);
          if (hoverEntry.setAccepting) hoverEntry.setAccepting(false);
          runArm("on-leave", hoverEntry);
        }
        hoverEntry = next;
        // Enter the new zone.
        if (hoverEntry) {
          if (hoverEntry.setHovering) hoverEntry.setHovering(true);
          if (hoverEntry.setAccepting) hoverEntry.setAccepting(!!hoverEntry.accepts(payload));
          runArm("on-enter", hoverEntry);
        }
      }

      function onPointerDown(e) {
        if (dragging) return;
        dragging = true;
        pointerId = e.pointerId;
        hoverEntry = null;
      }

      function onPointerMove(e) {
        if (!dragging || e.pointerId !== pointerId) return;
        updateHover(e.clientX, e.clientY);
      }

      function onPointerUp(e) {
        if (!dragging || e.pointerId !== pointerId) return;
        dragging = false;
        pointerId = null;
        // Resolve the zone under the release point (hoverEntry may be stale if no
        // move fired since the last hit-test; re-resolve to be safe).
        var entry = resolveZone(e.clientX, e.clientY);
        var accepting = entry ? !!entry.accepts(payload) : false;
        // Clear hover state on whatever zone was highlighted.
        if (hoverEntry) {
          if (hoverEntry.setHovering) hoverEntry.setHovering(false);
          if (hoverEntry.setAccepting) hoverEntry.setAccepting(false);
        }
        hoverEntry = null;
        %yield false -> $overZone;
        // Fire on-drop ONLY over an accepting zone; otherwise on-cancel (so a
        // rejected drop springs back declaratively, no manual spring-back JS).
        if (entry && accepting) {
          runArm("on-drop", entry);
        } else {
          runArm("on-cancel", null);
        }
      }

      function onPointerCancel(e) {
        if (!dragging) return;
        dragging = false;
        pointerId = null;
        if (hoverEntry) {
          if (hoverEntry.setHovering) hoverEntry.setHovering(false);
          if (hoverEntry.setAccepting) hoverEntry.setAccepting(false);
        }
        hoverEntry = null;
        %yield false -> $overZone;
        runArm("on-cancel", null);
      }

      el.addEventListener("pointerdown", onPointerDown);
      el.addEventListener("pointermove", onPointerMove);
      el.addEventListener("pointerup", onPointerUp);
      el.addEventListener("pointercancel", onPointerCancel);

      %yield false -> $overZone;

      %cleanup {
        el.removeEventListener("pointerdown", onPointerDown);
        el.removeEventListener("pointermove", onPointerMove);
        el.removeEventListener("pointerup", onPointerUp);
        el.removeEventListener("pointercancel", onPointerCancel);
      }
    })();
  }

  %exports {
    $overZone: bool   // pointer is over an accepting zone during a drag
  }
}
