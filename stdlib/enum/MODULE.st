/// Spacetime `enum` module — first-class tagged-union/enum types (PLAN-077).
///
/// One semantic concern, one module: a value that is one of N named states,
/// produced by a derive or a primitive, reflected to CSS via one attribute,
/// dispatched to templates or destructured in effects. This module supersedes
/// and DELETES four legacy flat-dir files that were all narrower slices of the
/// same concern (the retired state-machine, state, view, and match-render
/// modules — all deleted across PLAN-077's waves).
///
/// WHY a module (not scattered under macros/* + primitives/*): the same reason
/// `stdlib/dnd` is a module — enum/union types are ONE concern whose capture
/// types, derive machinery, dispatch machinery, and CSS-reflection machinery
/// only make sense together. Co-locating them keeps the surface discoverable
/// and matches the stdlib/AGENTS.md modular layout doctrine.
///
/// WHY `@type` itself is NOT here: `@type` (in stdlib/macros/type-data.st) is a
/// general type-declaration mechanism also used for non-enum PRODUCT types and
/// signal response sums unrelated to state/CSS reflection. This module DEPENDS
/// on `TypeRegistry` (reads declared `@type` sums for exhaustiveness checking)
/// but does not own type declaration.
///
/// WHY `@socket` is NOT here: it is a WebSocket-transport primitive that happens
/// to be a union PRODUCER. It stays put; this module's generalized reflection
/// serves it plus every new producer, without needing to relocate it.
///
/// @example
/// @import "stdlib/enum"
///
/// @type BadgeState { Blue | Orange | Turquoise | Green }
///
/// @data derive $badge BadgeState : @match {
///   ($isOrange)       => Orange;
///   ($isUnauthorized) => Blue;
///   ($isTurquoise)    => Turquoise;
///   _                 => Green;
/// }
///
/// // Arms: guards PARENTHESIZED, every `;` REQUIRED, `_` catch-all last.
/// // The `{type, ...}` value republishes through SpacetimeLocal, so
/// // @state(when: $badge is Blue) and friends consume it unchanged.
///
/// .st-host-widget__dot {
///   @state(when: $badge is Blue)      { /* … */ }
///   @state(when: $badge is Orange)    { /* … */ }
///   @state(when: $badge is Turquoise) { /* … */ }
///   @state(when: $badge is Green)     { /* … */ }
/// }

@import "./index.st"
