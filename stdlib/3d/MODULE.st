/// Spacetime `3d` module — declarative WebGL 3D, powered by vendored three.js.
///
/// Vendors mrdoob/three.js (r0.184.0, MIT) so Spacetime sites can build the
/// ambitious 3D experiences seen on award-winning websites — studio-lit PBR
/// objects, refractive glass, GPU particle fields, scroll-driven camera work,
/// GLTF models, and real post-processing (bloom, chromatic aberration) — WITHOUT
/// hand-writing WebGL or a Rust 3D engine (AGENTS.md: don't hand-roll engines for
/// stdlib constructs; vendor and own them behind a primitive). Site code only
/// ever sees the `@`-surface (`@stage` / `@object` / `@light` / …), never three's
/// API. three.js is also the low-level escape hatch (RawShaderMaterial,
/// EffectComposer), so there is no second "raw WebGL" module — one engine.
///
/// See docs/stdlib/VENDORING.md for the module + vendoring discipline.
///
/// @example a refractive glass hero
/// @import "stdlib/3d"
///
/// canvas.hero {
///   @stage(tone: aces, exposure: 1.1) {
///     @env(preset: studio)
///     @light directional(intensity: 2, position: [3, 5, 2])
///     @object icosahedron(detail: 6, size: 1.4) {
///       material: glass(roughness: 0.04, transmission: 1, ior: 1.5, thickness: 1.2)
///       spin: [0, 0.3, 0]
///       @orbit(damping: 0.08, autoRotate: 0.4)
///       @parallax(strength: 0.15)
///     }
///     @post { bloom(strength: 0.7, radius: 0.4, threshold: 0.85) }
///   }
/// }

// =============================================================================
// VENDORED DEPENDENCY
// =============================================================================

// three.js — pinned via git submodule at vendor/three. The %vendor declaration
// lives in `vendor.st` (imported by index.st) so it is registered whenever the
// module is imported; demand-driven emission then injects the bundled engine
// only when a primitive that references the `THREE` global actually fires (see
// VENDORING.md "Lean law"). A page that imports stdlib/3d but renders no 3D
// ships none of three's ~0.83 MB. Restated here as the manifest:
//
//   %vendor three {
//     source:  submodule "vendor/three"
//     entry:   "vendor/three-entry.ts"
//     bundle:  bun
//     exports: { THREE }
//     out:     "vendor/three.bundle.js"
//     license: MIT
//   }

// =============================================================================
// PUBLIC SURFACE
// =============================================================================

@import "./index.st"
