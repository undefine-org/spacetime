/// Pose Runtime Macro — installs the shared pose-interpolation module.
///
/// `@pose-runtime` is a thin, params-free binding point for `three-pose`
/// (PLAN-062 Wave 2) — mirrors how `stdlib/dnd/macros/{drag,drop-zone}.st` each
/// `%binds dnd-runtime()` to guarantee `window.__stDnd` exists before their own
/// logic runs (a `%primitive` cannot `%binds`; only a `%macro` can). Consuming
/// pose-driven macros (the Wave 3 `@scroll`-into-3D dispatch, a project's own
/// pose-choreography primitive) bind `three-pose()` the SAME way, directly,
/// without needing this macro — `@pose-runtime` exists purely so `window.__stPose`
/// can be exercised standalone (e.g. by `stdlib/3d/tests/pose.test.st`, which has
/// no need for a full `@stage`/`@object` scene since the module has zero THREE.js
/// dependency).
///
/// @example install the pose runtime with no scene at all
/// .pose-test-host {
///   @pose-runtime
/// }

%macro pose-runtime {
  %scope selector

  %form {
    @pose-runtime
  }

  %binds {
    three-pose() -> {}
  }
}


/// Pose Track Macro — scroll-drives the SAME @stage's `@object`'s
/// position/orientation through a sequence of pose stops.
///
/// `stops:` is a JS array literal (or an expression evaluating to one) of
/// `{ at, pos, face }` records, where `pos`/`face` are built from
/// `window.__stPose`'s tested value-kinds (`vec3`, `flat`, `tilt`, `tumble`) —
/// not raw numbers. This is the actual fix for zellijist-2's 3 bugs: `flat()`
/// cannot carry stray pitch/roll, `tumble(axis, angle)` names its own intent,
/// and `&stage.camera.bounds` (Wave 1's `ctx.bounds(z)`) is available inside
/// the SAME `%emit js` scope a project's stop-builder runs in.
///
/// @example a two-stop pose track (flat -> a 60deg tumble around Z)
/// canvas.hero {
///   @stage(fov: 32, camZ: 5.5) {
///     @object icosahedron(size: 1)
///     @pose-track(stops: [
///       { at: 0,   pos: {x:0,y:0,z:0}, face: window.__stPose.flat() },
///       { at: 1,   pos: {x:0,y:0,z:2}, face: window.__stPose.tumble(0,0,1, Math.PI/3) }
///     ])
///   }
/// }
///
/// @param stops - array of { at: number, pos: {x,y,z}, face: quat } stops
/// @param ease  - per-frame smoothing 0..1 (default 0.08)
///
/// BINDING (BUG-263 item 4): a track drives the `@object` authored immediately
/// before it (source-order interleaving — sibling primitive inits emit in
/// author order, so `canvas._stObject` at track-init is its preceding object).
/// The old `objectIndex:` parameter is RETIRED and dropped (the compiler has
/// no general unknown-parameter diagnostic, so it is a dead no-op rather than
/// an error). To drive N objects, author N adjacent `@object`+`@pose-track`
/// pairs in the `@stage` body.

%macro pose-track {
  %scope selector

  %form {
    @pose-track(
      stops: $stops:expr,
      ease: $ease:number = 0.08
    )
  }

  %binds {
    // ORDERING (BUG-263 item 4): three-pose-track MUST be the FIRST bind so it
    // sorts at the SAME order slot as the sibling `@object` and interleaves with
    // it in SOURCE ORDER — each track then resolves `canvas._stObject` = its OWN
    // preceding object, so no objectIndex param exists. three-pose is a
    // `%emit prelude-js` primitive (emitted ONCE, deduplicated, before every
    // init IIFE), so its SECOND bind slot (order 500001, after all same-bucket
    // primitives) is harmless: the shared `window.__stPose` runtime is installed
    // regardless of where its bind sorts.
    three-pose-track(&self,
      stops: $stops,
      ease: $ease
    ) -> {
      $progress
    }
    three-pose(&self) -> {}
  }
}

