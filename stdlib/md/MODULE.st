/// Spacetime `md` module — Markdown → HTML rendering, self-documenting docs.
///
/// Vendors developit/snarkdown (a ~1KB zero-dependency CommonMark-ish parser) so
/// Spacetime sites can render Markdown files as HTML WITHOUT a hand-written JS
/// parser or a Rust markdown implementation (AGENTS.md: don't hand-roll parsers
/// for stdlib constructs; vendor and own them behind a primitive). Site code only
/// ever sees `@doc`, never snarkdown's API.
///
/// See docs/stdlib/VENDORING.md for the module + vendoring discipline.
///
/// @example
/// @import "stdlib/md"
///
/// .guide {
///   @doc(src: "docs/testing/GUIDE.md")
/// }

// =============================================================================
// VENDORED DEPENDENCY
// =============================================================================

// snarkdown — pinned via git submodule at vendor/snarkdown. The %vendor
// declaration lives in `vendor.st` (imported by index.st) so it is registered
// whenever the module is imported; demand-driven emission then injects the
// bundled engine only when a primitive that references the `snarkdown` global
// actually fires (see VENDORING.md "Lean law"). Restated here as the manifest.
//
//   %vendor snarkdown {
//     source:  submodule "vendor/snarkdown"
//     entry:   "vendor/snarkdown-entry.ts"
//     bundle:  bun
//     exports: { snarkdown }
//     out:     "vendor/snarkdown.bundle.js"
//     license: MIT
//   }

// =============================================================================
// PUBLIC SURFACE
// =============================================================================

@import "./index.st"
