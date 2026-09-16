/// Spacetime `3d` module — public re-exports.
///
/// `@import "stdlib/3d"` brings in the declarative 3D surface (the `@stage` /
/// `@object` / `@light` / `@orbit` / `@parallax` / `@scroll-3d` / `@post` /
/// `@gltf` / `@particles` / `@env` macros) plus the primitives that wrap the
/// vendored three.js engine. The engine itself is pulled in on demand by the
/// primitives that reference it (see MODULE.st `%vendor`), so a page that imports
/// the module but renders no 3D ships none of three's bytes.

// -----------------------------------------------------------------------------
// VENDORED DEPENDENCY (registers %vendor three so demand-emit can inject it)
// -----------------------------------------------------------------------------

@import "./vendor.st"

// -----------------------------------------------------------------------------
// PRIMITIVES (wrap the vendored three.js engine in %emit js)
// -----------------------------------------------------------------------------

@import "./primitives/three-stage.st"
@import "./primitives/three-object.st"
@import "./primitives/three-light.st"
@import "./primitives/three-env.st"
@import "./primitives/three-orbit.st"
@import "./primitives/three-parallax.st"
@import "./primitives/three-scroll.st"
@import "./primitives/three-post.st"
@import "./primitives/three-gltf.st"
@import "./primitives/three-particles.st"
@import "./primitives/three-pose.st"
@import "./primitives/three-pose-track.st"

// -----------------------------------------------------------------------------
// MACROS (author-facing @-surface)
// -----------------------------------------------------------------------------

@import "./macros/stage.st"
@import "./macros/object.st"
@import "./macros/light.st"
@import "./macros/env.st"
@import "./macros/orbit.st"
@import "./macros/parallax.st"
@import "./macros/scroll-3d.st"
@import "./macros/post.st"
@import "./macros/gltf.st"
@import "./macros/particles.st"
@import "./macros/pose.st"
