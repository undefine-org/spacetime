/// FEAT-149 / PLAN-062 Wave 1 — canvas._stThree.bounds(z) spike test.
///
/// SCOPE NOTE (honest, per reviewer swarm): PLAN-062 Wave 1 originally proposed
/// author-facing dotted-facet-call sugar `&stage.camera.bounds(z)` (reusing
/// FEAT-142's entity/facet mechanism). That requires a tree-sitter grammar
/// change (element_ref has no trailing-call grammar today) beyond a Wave-1
/// spike budget, so THIS wave delivers the underlying computation as a method
/// on the already-proven `canvas._stThree` handle every 3D primitive composes
/// against (`ctx.bounds(z)`) — the correct, tested, reusable VALUE zellijist-2
/// hand-derived per-project — with the author-facing dotted-call sugar
/// deferred as an explicit follow-up (see PLAN-062 Wave 1 NOTE), not silently
/// dropped.
///
/// Proves canvas._stThree.bounds(z) computes the SAME frustum half-width/
/// half-height a project would otherwise hand-derive, using the camera's
/// REAL view direction (`camera.getWorldDirection()`) — not a naive Z-axis
/// delta, which breaks for any elevated/tilted camera (caught by review;
/// see the 3rd test below, modeled on demos/3d/genesis's `camY: 2.0` /
/// everswap-clone's `camY: 18/40` shapes). FIDELITY: needs a real WebGL
/// canvas (perspective camera fov/aspect/position) — CDP paint rung only,
/// per docs/testing/FIDELITY_LADDER.md.
///
///   cargo run --features cdp -- test stdlib/3d/tests/camera-bounds.test.st --cdp

@import "stdlib/testing/test"
@import "stdlib/3d"

@test "bounds(0) matches hand-derived frustum trig at the origin plane" needs layout {
  @mount {
    <canvas class="t-bounds" data-bounds style="width:800px;height:450px;"></canvas>
    [data-bounds] { @stage(fov: 32, camZ: 5.5) {} }
  }
  @let (ctx = document.querySelector('[data-bounds]')._stThree)
  @let (b = ctx.bounds(0))
  @let (vFovRad = (ctx.camera.fov * Math.PI) / 180)
  @let (expectedHalfH = Math.tan(vFovRad / 2) * ctx.camera.position.z)
  @let (expectedHalfW = expectedHalfH * ctx.camera.aspect)
  @assert (Math.abs(b.halfH - expectedHalfH) < 0.0001)
    "bounds(0).halfH must equal the hand-derived tan(fov/2)*camZ"
  @assert (Math.abs(b.halfW - expectedHalfW) < 0.0001)
    "bounds(0).halfW must equal halfH * camera.aspect"
  @assert (b.halfW > 0 && b.halfH > 0)
    "bounds at the origin plane (in front of the camera) must be positive"
}

@test "bounds(z) shrinks toward the camera and grows away from it" needs layout {
  @mount {
    <canvas class="t-bounds2" data-bounds2 style="width:800px;height:450px;"></canvas>
    [data-bounds2] { @stage(fov: 32, camZ: 5.5) {} }
  }
  @let (ctx = document.querySelector('[data-bounds2]')._stThree)
  @let (nearZ = ctx.bounds(4))
  @let (originZ = ctx.bounds(0))
  @let (farZ = ctx.bounds(-4))
  @assert (nearZ.halfW < originZ.halfW)
    "closer to the camera (z=4, depth=1.5) must be narrower than the origin plane"
  @assert (farZ.halfW > originZ.halfW)
    "farther from the camera (z=-4, depth=9.5) must be wider than the origin plane"
}

@test "bounds(0) is correct for a TILTED camera (camY offset, lookAt origin)" needs layout {
  // Reviewer-caught regression case: demos/3d/genesis and everswap-clone both
  // use a non-zero camY with camera.lookAt(0,0,0), so the camera's real view
  // direction is NOT pure -Z. A first draft of bounds(z) used the raw
  // `camera.position.z - z` axis delta as "depth", which is WRONG once the
  // camera is elevated — this test proves the fix (camera.getWorldDirection()
  // -based depth) against that exact shape.
  @mount {
    <canvas class="t-bounds3" data-bounds3 style="width:800px;height:450px;"></canvas>
    [data-bounds3] { @stage(fov: 42, camY: 18, camZ: 30) {} }
  }
  @let (ctx = document.querySelector('[data-bounds3]')._stThree)
  @let (b = ctx.bounds(0))
  @let (forward = ctx.camera.getWorldDirection(new THREE.Vector3()))
  @let (expectedDepth = (0 - ctx.camera.position.z) / forward.z)
  @let (vFovRad = (ctx.camera.fov * Math.PI) / 180)
  @let (expectedHalfH = Math.tan(vFovRad / 2) * expectedDepth)
  @let (expectedHalfW = expectedHalfH * ctx.camera.aspect)
  @assert (Math.abs(b.halfH - expectedHalfH) < 0.0001)
    "tilted camera: bounds(0).halfH must use the REAL view-direction depth, not the raw Z delta"
  @assert (Math.abs(b.halfW - expectedHalfW) < 0.0001)
    "tilted camera: bounds(0).halfW must equal halfH * camera.aspect"
  @assert (b.halfW > 0 && b.halfH > 0)
    "tilted camera still looking at the origin plane must yield positive bounds"
  @assert (Math.abs(expectedDepth - (ctx.camera.position.z - 0)) > 0.5)
    "sanity: the tilted camera's real view-direction depth must differ meaningfully from the naive Z-delta (proves this scene actually exercises the bug the fix addresses)"
}

