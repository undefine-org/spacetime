/// `drop-zone` — register an element as a place draggables can be dropped.
///
/// On mount, registers a zone ENTRY into the module's `__stDnd` registry
/// (installed by the `dnd-runtime` prelude; the `@drop-zone` macro binds
/// `dnd-runtime` alongside this primitive so the prelude is always present — a
/// primitive cannot `%binds`, only macros can). On cleanup, unregisters it. The drag
/// gesture hit-tests the pointer against `__stDnd.zones[group]` (NOT a magic
/// selector) and toggles this zone's `$hovering` / `$accepting` signals, which
/// drive the macro's `over` / `reject` states.
///
/// @param &el     - the zone element
/// @param group   - namespaces zones↔draggables (a draggable in group G hits
///                  only group-G zones); default "" (one implicit global group)
/// @param accepts - OPTIONAL predicate over the dragged payload, as raw text
///                  (`$payload.tag != "locked"`). Compiled to `acceptsFn(p)` and
///                  evaluated BY the gesture at hover with the live payload — it
///                  CANNOT be a zone-resident signal because the dragged datum
///                  does not exist until a drag hovers. `$payload` → the datum;
///                  `$zone` → this zone's dataset. Default: always accept.
///
/// @yields $hovering  - a compatible draggable is over this zone
/// @yields $accepting - hovering AND the accepts predicate passes

%primitive drop-zone(
  &el,
  group: string = "",
  accepts: any? = none
) {
  %emit js {
    (function () {
      var group = %group || "";

      // Compile the optional accepts predicate to acceptsFn(payload). The raw
      // text uses `$payload` (the dragged datum) and `$zone` (this zone's
      // dataset). Mirrors computed-source's `where:` lowering: rewrite the sigil
      // reads, then build a Function. A bad/empty predicate ⇒ always accept.
      var acceptsText = %accepts;
      var acceptsFn = function () { return true; };
      if (acceptsText != null && String(acceptsText).trim() !== "" &&
          String(acceptsText).trim() !== "none") {
        try {
          var src = String(acceptsText)
            .replace(/\$payload\b/g, "payload")
            .replace(/\$zone\b/g, "zone");
          var f = new Function("payload", "zone", "return (" + src + ");");
          acceptsFn = function (payload) { return !!f(payload, %&el.dataset); };
        } catch (e) {
          console.error("[drop-zone] bad accepts predicate:", e);
        }
      }

      // The registry entry. setHovering/setAccepting are the seam the gesture
      // calls to drive this zone's reactive states.
      // Track hovering/accepting locally and publish the COMBINED gate signals
      // `$over` (hovering AND accepting) + `$reject` (hovering AND NOT accepting)
      // that drive the over/reject states. The state machinery (ST.bindState ->
      // ST.watch) keys on a SINGLE signal name, not a compound expression, so the
      // zone computes the booleans here rather than gating on `$hovering &&
      // $accepting` (which ST.watch would treat as a literal signal name and
      // never fire).
      var hovering = false;
      var accepting = false;
      function publish() {
        %yield hovering -> $hovering;
        %yield accepting -> $accepting;
        %yield (hovering && accepting) -> $over;
        %yield (hovering && !accepting) -> $reject;
      }
      var entry = {
        el: %&el,
        group: group,
        dataset: %&el.dataset,
        accepts: acceptsFn,
        setHovering: function (v) { hovering = !!v; publish(); },
        setAccepting: function (v) { accepting = !!v; publish(); },
      };

      // Initial state: not hovered, not accepting.
      publish();

      if (typeof window !== "undefined" && window.__stDnd) {
        window.__stDnd.register(entry);
      }

      %cleanup {
        if (typeof window !== "undefined" && window.__stDnd) {
          window.__stDnd.unregister(entry);
        }
      }
    })();
  }

  %exports {
    $hovering: bool   // a compatible draggable is over this zone
    $accepting: bool  // hovering AND accepts(payload) passes
    $over: bool       // hovering AND accepting (drives the `over` state)
    $reject: bool     // hovering AND NOT accepting (drives the `reject` state)
  }
}
