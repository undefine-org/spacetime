/// Spacetime `dnd` module — public re-exports.
///
/// `@import "stdlib/dnd"` brings in declarative drag-and-drop: the `@drop-zone`
/// and `@drag` author surfaces, backed by an in-module runtime registry. See
/// MODULE.st for the design rationale.

// -----------------------------------------------------------------------------
// RUNTIME (the prelude-js zone registry + overlay hit-test; no st.js edits)
// -----------------------------------------------------------------------------

@import "./runtime.st"

// -----------------------------------------------------------------------------
// CAPTURE TYPES (the @drag lifecycle arm grammar — kinds-as-data)
// -----------------------------------------------------------------------------

@import "./capture-types/lifecycle.st"

// -----------------------------------------------------------------------------
// PRIMITIVES (wrap the registry in %emit js per element)
// -----------------------------------------------------------------------------

@import "./primitives/drop-zone.st"
@import "./primitives/drag-zones.st"

// -----------------------------------------------------------------------------
// MACROS (author-facing @-surface)
// -----------------------------------------------------------------------------

@import "./macros/drop-zone.st"
@import "./macros/drag.st"
