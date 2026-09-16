/// Spacetime `md` module — public re-exports.
///
/// `@import "stdlib/md"` brings in the markdown render primitive and the
/// author-facing `@doc` macro. The vendored snarkdown engine is pulled in on
/// demand by the primitive that references it (see MODULE.st `%vendor`).

// -----------------------------------------------------------------------------
// VENDORED DEPENDENCY (registers %vendor snarkdown so demand-emit can inject it)
// -----------------------------------------------------------------------------

@import "./vendor.st"

// -----------------------------------------------------------------------------
// PRIMITIVES (wrap the vendored engine in %emit js)
// -----------------------------------------------------------------------------

@import "./primitives/render-markdown.st"

// -----------------------------------------------------------------------------
// MACROS (author-facing @-surface)
// -----------------------------------------------------------------------------

@import "./macros/doc.st"
