/// Spacetime `dnd` module — declarative drag-and-drop, zero custom JS.
///
/// Two cooperating directives, both signal-first, so a page writes NO JS/DOM:
///
///   @drop-zone(group: "cards") as $zone { over { … } reject { … } }
///   @drag(group: "cards", payload: $c) { on-drop $zone { $move({ to: $zone.col }) } }
///
/// `@drop-zone` declares a place things can be dropped; `@drag` gains a
/// declarative lifecycle (`on-enter`/`on-leave`/`on-drop`/`on-cancel $zone`).
/// A drop resolves against a runtime ZONE REGISTRY (not a magic selector), and
/// the dragged datum + hovered zone are BOUND signals (`$payload`, `$zone.col`)
/// — never `$.dataset` string-fishing or `ST.dropTarget` page calls.
///
/// WHY a module (not scattered under macros/* + primitives/*): drag-and-drop is
/// ONE semantic concern — a registry, a hit-test, a zone surface, and a drag
/// lifecycle that only make sense together. Co-locating them (MODULE.st +
/// index.st, like stdlib/md) keeps the surface discoverable and lets the runtime
/// ship INSIDE the module.
///
/// WHY no edits to public/runtime/st.js: the zone registry + overlay-resilient
/// hit-test ride a `%emit prelude-js` block on the yield-less `dnd-runtime`
/// primitive — an idempotent `window.__stDnd = window.__stDnd || {…}` installed
/// ONCE (deduplicated by primitive name), before any per-element IIFE. The
/// global runtime stays untouched; the module is the program. (Same rail as
/// stdlib/primitives/animation/drivers.st's `window.__stScrollHandlers`.)
///
/// @example
/// @import "stdlib/dnd"
///
/// [data-col="doing"] {
///   @drop-zone(group: "cards") as $zone {
///     over { background: rgba(255,255,255,0.04) }
///   }
/// }
/// .card {
///   @drag(group: "cards", payload: $c) {
///     dragging { scale: 1.04 }
///     on-drop $zone { $move({ card: $payload.id, to: $zone.col }); }
///   }
/// }

@import "./index.st"
