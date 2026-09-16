/// 3D Module Tests — @pose-track multi-object source-order pairing (BUG-263 item 4)
///
/// Proves the SOURCE-ORDER interleaving that replaces the retired `objectIndex`
/// mechanism (PLAN-062 Wave 5): a @stage with MULTIPLE `@object` + `@pose-track`
/// PAIRS must drive EACH object independently. The compiler now emits sibling
/// primitive inits in author order, so each `@pose-track` binds its PRECEDING
/// `@object` (via `canvas._stObject`) — no explicit index is needed or accepted.
///
/// The BINDING itself (track i -> object i) is proven deterministically by the
/// Rust emit-order gate in `tests/gate_pt_interleave.rs`: it asserts the emitted
/// page JS interleaves `three-object` and `three-pose-track` init blocks so each
/// track resolves the object authored immediately before it. That gate needs no
/// CDP/rAF timing (BUG-150 makes position assertions unreliable under this
/// harness), so it is the authoritative behavioral proof.
///
/// This CDP file proves the RUNTIME scene-structure half: N author-order PAIRs
/// compile to N DISTINCT objects in the SAME source order the pairs are written
/// in (the precondition source-order binding relies on). The single-object case
/// regression is pinned here too.
///
///   cargo run --features cdp -- test stdlib/3d/tests/pose-track-multi.test.st --cdp

@import "stdlib/testing/test"
@import "stdlib/3d"

@test "N @object+@pose-track pairs create N distinct objects in SOURCE ORDER" needs layout {
  @mount {
    <canvas class="t-multi" data-multi style="width:800px;height:450px;"></canvas>
    [data-multi] {
      @stage(fov: 36, camZ: 8) {
        @object(shape: "box", size: 0.5)
        @pose-track(ease: 1, stops: [
          { at: 0, pos: { x: 0, y: 0, z: 0 }, face: window.__stPose.flat() },
          { at: 1, pos: { x: 2, y: 0, z: 0 }, face: window.__stPose.flat() }
        ])
        @object(shape: "sphere", size: 0.5)
        @pose-track(ease: 1, stops: [
          { at: 0, pos: { x: 0, y: 0, z: 0 }, face: window.__stPose.flat() },
          { at: 1, pos: { x: -2, y: 0, z: 0 }, face: window.__stPose.flat() }
        ])
        @object(shape: "icosahedron", size: 0.5)
        @pose-track(ease: 1, stops: [
          { at: 0, pos: { x: 0, y: 0, z: 0 }, face: window.__stPose.flat() },
          { at: 1, pos: { x: 0, y: 2, z: 0 }, face: window.__stPose.flat() }
        ])
      }
    }
  }
  @let (canvas = document.querySelector('[data-multi]'))
  @let (objs = canvas._stObjects)
  @assert (objs.length === 3)
    "three @object calls must produce three distinct THREE objects"
  @assert (objs[0].uuid !== objs[1].uuid && objs[0].uuid !== objs[2].uuid && objs[1].uuid !== objs[2].uuid)
    "the three objects must be genuinely distinct instances, not the same mesh reused"
  @assert (objs[0].geometry.type === "BoxGeometry")
    "the FIRST pair's object is the box (source-order creation preserved)"
  @assert (objs[1].geometry.type === "SphereGeometry")
    "the SECOND pair's object is the sphere (source-order creation preserved)"
  @assert (objs[2].geometry.type === "IcosahedronGeometry")
    "the THIRD pair's object is the icosahedron (source-order creation preserved)"
}

@test "single @object + @pose-track pair still applies its first stop on mount" needs layout {
  @mount {
    <canvas class="t-single" data-single style="width:800px;height:450px;"></canvas>
    [data-single] {
      @stage(fov: 32, camZ: 5.5) {
        @object(shape: "icosahedron", size: 1)
        @pose-track(ease: 1, stops: [
          { at: 0, pos: { x: 0, y: 0, z: 0 }, face: window.__stPose.flat() },
          { at: 1, pos: { x: 0, y: 2, z: -1 }, face: window.__stPose.tumble(0, 0, 1, Math.PI / 2) }
        ])
      }
    }
  }
  @let (canvas0 = document.querySelector('[data-single]'))
  @let (obj0 = canvas0._stObject)
  @assert (Math.abs(obj0.position.x) < 0.05 && Math.abs(obj0.position.y) < 0.05 && Math.abs(obj0.position.z) < 0.05)
    "the single-object case must still apply the first stop correctly on mount"
  @assert (Math.abs(obj0.quaternion.x) < 0.01 && Math.abs(obj0.quaternion.y) < 0.01)
    "the single-object case must still be flat (zero pitch/roll) on mount, matching Wave 3's own behavior"
}
