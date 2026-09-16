/// Spacetime `text` module — public re-exports.
///
/// `@import "stdlib/text"` brings in the measurement primitives and the
/// author-facing macros. The vendored pretext engine is pulled in on demand by
/// the primitives that reference it (see MODULE.st `%vendor`).

// -----------------------------------------------------------------------------
// VENDORED DEPENDENCY (registers %vendor pretext so demand-emit can inject it)
// -----------------------------------------------------------------------------

@import "./vendor.st"

// -----------------------------------------------------------------------------
// PRIMITIVES (wrap the vendored engine in %emit js)
// -----------------------------------------------------------------------------

@import "./primitives/measure-text.st"
@import "./primitives/balance-text.st"
@import "./primitives/fit-text.st"

// -----------------------------------------------------------------------------
// MACROS (author-facing @-surface)
// -----------------------------------------------------------------------------

@import "./macros/measure.st"
@import "./macros/balance.st"
@import "./macros/fit.st"
