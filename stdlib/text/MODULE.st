/// Spacetime `text` module — reflow-free text measurement & layout.
///
/// Vendors chenglou/pretext (a 15KB zero-dependency text layout engine) to give
/// Spacetime sites accurate text measurement WITHOUT triggering DOM reflow.
/// The engine is owned, pinned, and wrapped behind our primitives — site code
/// only ever sees `@measure` / `@balance`, never pretext's API.
///
/// See docs/stdlib/VENDORING.md for the module + vendoring discipline.
///
/// @example
/// @import "stdlib/text"
///
/// .headline {
///   @measure -> { $lines, $height };
/// }

// =============================================================================
// VENDORED DEPENDENCY
// =============================================================================

// pretext — pinned via git submodule at vendor/pretext. The %vendor declaration
// lives in `vendor.st` (imported by index.st) so it is registered whenever the
// module is imported; demand-driven emission then injects the bundled engine
// only when a primitive that references the `pretext` global actually fires
// (see VENDORING.md "Lean law"). It is restated here as the human manifest.
//
//   %vendor pretext {
//     source:  submodule "vendor/pretext"
//     entry:   "vendor/pretext-entry.ts"
//     bundle:  bun
//     exports: { prepare, prepareWithSegments, layout, layoutWithLines, walkLineRanges, measureNaturalWidth }
//     out:     "vendor/pretext.bundle.js"
//     license: MIT
//   }

// =============================================================================
// PUBLIC SURFACE
// =============================================================================

@import "./index.st"
