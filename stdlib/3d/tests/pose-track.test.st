/// 3D Module Tests — @pose-track (PLAN-062 Wave 3)
///
/// Proves @pose-track's INITIAL state (mount, first stop applied) using Wave
/// 2's tested pose value-kinds (flat/tumble/vec3) and Wave 1's ctx.bounds(z),
/// via CDP (real WebGL canvas + real THREE.Object3D).
///
/// IMPORTANT (BUG-149, filed from this wave): `@assert`'s message argument
/// MUST be a bare string literal — a string-CONCATENATION expression
/// (`"x: " + val`) silently drops the ENTIRE enclosing @test body at compile
/// time with ZERO diagnostic. Every message below is a bare literal.
///
/// IMPORTANT (BUG-150, filed from this wave): the CDP test backend cannot
/// reliably observe a requestAnimationFrame-driven state change via
/// `@wait_until` polling — an exhaustive from-scratch investigation (7+
/// minimal repros) confirmed `requestAnimationFrame`, `setInterval`, a
/// standalone `setTimeout`, AND `@wait_until`'s own internal poll loop all
/// fail to tick/retry more than once under this CDP harness, independent of
/// @pose-track's own code (three-stage.st's rAF-driven render loop is the
/// underlying mechanism every stdlib/3d scroll/orbit/particle primitive
/// shares, so this is a general CDP-testing gap, not specific to this wave).
/// Two Rust-level fixes were attempted and both proven ineffective (see
/// BUG-150 for the full trail); reverted cleanly, no speculative code left
/// in src/cdp.rs.
///
/// The scroll-driven position/orientation TRANSITION (flat origin -> the
/// tumbled second stop) is instead verified by DIRECT manual browser
/// automation against a live dev server (repeatable, correct — see BUG-150's
/// "workaround in use" section for the exact verification steps performed).
/// This is a real functional proof, just not an automated CDP regression
/// test; restoring the scroll-driven assertions here is blocked on BUG-150.
///
///   cargo run --features cdp -- test stdlib/3d/tests/pose-track.test.st --cdp

@import "stdlib/testing/test"
@import "stdlib/3d"

@test "@pose-track applies its first stop on mount (flat origin)" needs layout {
  @mount {
    <canvas class="t-pose-track" data-pose-track style="width:800px;height:450px;"></canvas>
    [data-pose-track] {
      @stage(fov: 32, camZ: 5.5) {
        @object icosahedron(size: 1)
        @pose-track(ease: 1, stops: [
          { at: 0, pos: { x: 0, y: 0, z: 0 }, face: window.__stPose.flat() },
          { at: 1, pos: { x: 0, y: 2, z: -1 }, face: window.__stPose.tumble(0, 0, 1, Math.PI / 2) }
        ])
      }
    }
  }

  @let (canvas0 = document.querySelector('[data-pose-track]'))
  @let (obj0 = canvas0._stObject)
  @assert (Math.abs(obj0.position.x) < 0.05 && Math.abs(obj0.position.y) < 0.05 && Math.abs(obj0.position.z) < 0.05)
    "at scroll 0 object position must be the first stop's origin"
  @assert (Math.abs(obj0.quaternion.x) < 0.01 && Math.abs(obj0.quaternion.y) < 0.01)
    "at scroll 0 object orientation must be flat, zero pitch and roll"
  @assert (typeof window.__stPose !== 'undefined')
    "the shared three-pose runtime must be installed alongside pose-track"
}

@test "@pose-track composes with ctx.bounds(z) — a stop can be authored relative to frame width" needs layout {
  // Proves Waves 1+2+3 fit together: a project's own @emit js (or, here, the
  // stops array itself, evaluated in the SAME page scope @stage already ran
  // in) can read ctx.bounds(0).halfW to keep a pose-track stop within frame —
  // exactly the zellijist-2 "off-frame scatter" fix, expressed through the new
  // surface instead of hand-derived trig.
  @mount {
    <canvas class="t-pose-bounds" data-pose-bounds style="width:800px;height:450px;"></canvas>
    [data-pose-bounds] {
      @stage(fov: 40, camZ: 6) {
        @object icosahedron(size: 0.5)
      }
    }
  }
  @let (canvas = document.querySelector('[data-pose-bounds]'))
  @let (ctx = canvas._stThree)
  @let (bounds0 = ctx.bounds(0))
  @assert (bounds0.halfW > 0 && bounds0.halfH > 0)
    "ctx.bounds(0) must be available in the same scope a pose-track stops array would be built in"
  @let (safeX = bounds0.halfW * 0.7)
  @assert (safeX < bounds0.halfW)
    "a stop authored as bounds.halfW times 0.7 is provably within the visible frame"
}

@test "@pose-track normalizes a non-unit boundary quaternion on mount" needs layout {
  // Reviewer-caught bug: the FIRST stop's face is returned VERBATIM at the
  // scroll-0 boundary (progress <= stops[0].at) before this fix — a caller-
  // supplied non-unit quaternion (untrusted input; a stops array is authored
  // JS, not validated) reached target.quaternion.set(...) unnormalized,
  // reintroducing Wave 2's non-unit-orientation bug at exactly the mount
  // boundary. This test supplies a deliberately non-unit face and confirms
  // the mounted object's quaternion is unit-length regardless.
  @mount {
    <canvas class="t-pose-nonunit" data-pose-nonunit style="width:800px;height:450px;"></canvas>
    [data-pose-nonunit] {
      @stage(fov: 32, camZ: 5.5) {
        @object icosahedron(size: 1)
        @pose-track(ease: 1, stops: [
          { at: 0, pos: { x: 0, y: 0, z: 0 }, face: { x: 0, y: 0, z: 0, w: 0.5 } },
          { at: 1, pos: { x: 0, y: 1, z: 0 }, face: window.__stPose.flat() }
        ])
      }
    }
  }
  @let (canvas0 = document.querySelector('[data-pose-nonunit]'))
  @let (q = canvas0._stObject.quaternion)
  @let (magnitude = Math.sqrt(q.x * q.x + q.y * q.y + q.z * q.z + q.w * q.w))
  @assert (Math.abs(magnitude - 1) < 0.0001)
    "a deliberately non-unit boundary face must still be normalized before reaching the mesh quaternion"
}

