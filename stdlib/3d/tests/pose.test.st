/// 3D Module Tests — three-pose (shared pose-interpolation runtime, PLAN-062 Wave 2)
///
/// FEAT-149's diagnosis: zellijist-2's 3 bugs (disconnected ribbon, off-frame
/// scatter, tilted-not-flat) share one root cause — no typed vocabulary for "a
/// pose," so raw Vector3/Quaternion arithmetic in %emit js has no seatbelt.
/// `three-pose` is the shared, tested interpolation core. It has ZERO THREE.js
/// dependency (pure math), so — unlike @stage/@object/@particles — these
/// assertions run on the FAST headless V8 `logic` backend, not CDP: no WebGL,
/// no real canvas needed to prove vec3/quaternion math is correct.
///
///   cargo run --features headless -- test stdlib/3d/tests/pose.test.st --headless

@import "stdlib/testing/test"
@import "stdlib/3d"

@test "rng(seed) is deterministic — same seed produces the same sequence" {
  @fixture {
    <div class="t-pose-a" data-pose-a></div>
    [data-pose-a] { @pose-runtime }
  }
  @let (a = window.__stPose.rng(42))
  @let (b = window.__stPose.rng(42))
  @assert (a() === b())
    "two rng()s built from the SAME seed must produce the SAME first value"
  @assert (a() === b())
    "...and the SAME second value (the sequence, not just one draw, is deterministic)"
}

@test "rng(seed) with different seeds diverges" {
  @fixture {
    <div class="t-pose-b" data-pose-b></div>
    [data-pose-b] { @pose-runtime }
  }
  @let (a = window.__stPose.rng(1))
  @let (b = window.__stPose.rng(2))
  @assert (a() !== b())
    "different seeds must (almost certainly) diverge on the first draw"
}

@test "rand(rngFn, min, max) stays within [min, max)" {
  @fixture {
    <div class="t-pose-c" data-pose-c></div>
    [data-pose-c] { @pose-runtime }
  }
  @let (r = window.__stPose.rng(7))
  @let (samples = Array.from({ length: 200 }, function () { return window.__stPose.rand(r, -3, 5); }))
  @assert (samples.every(function (v) { return v >= -3 && v < 5; }))
    "every sample must fall within the declared [min, max) range"
  @assert (new Set(samples).size > 1)
    "200 draws from a real rng must not all collapse to one value"
}

@test "flat() is the identity quaternion — no slot for stray pitch/roll" {
  @fixture {
    <div class="t-pose-d" data-pose-d></div>
    [data-pose-d] { @pose-runtime }
  }
  @let (q = window.__stPose.flat())
  @assert (q.x === 0 && q.y === 0 && q.z === 0 && q.w === 1)
    "flat() must be exactly the identity quaternion (0,0,0,1) — zero pitch/roll/yaw"
}

@test "tilt(x, y) carries ONLY pitch/roll — zero yaw" {
  @fixture {
    <div class="t-pose-e" data-pose-e></div>
    [data-pose-e] { @pose-runtime }
  }
  @let (q = window.__stPose.tilt(0.3, -0.2))
  @let (yawFree = window.__stPose.eulerToQuat(0.3, -0.2, 0))
  @assert (Math.abs(q.x - yawFree.x) < 0.0001 && Math.abs(q.y - yawFree.y) < 0.0001 && Math.abs(q.z - yawFree.z) < 0.0001 && Math.abs(q.w - yawFree.w) < 0.0001)
    "tilt(x,y) must equal eulerToQuat(x,y,0) — a pure pitch/roll rotation with explicitly zero yaw"
}

@test "tumble(axis, angle) rotates the requested angle around a normalized axis" {
  @fixture {
    <div class="t-pose-f" data-pose-f></div>
    [data-pose-f] { @pose-runtime }
  }
  @let (q = window.__stPose.tumble(0, 0, 1, Math.PI / 2))
  // A 90deg rotation around Z: w = cos(45deg), z = sin(45deg), x=y=0.
  @let (expectedW = Math.cos(Math.PI / 4))
  @let (expectedZ = Math.sin(Math.PI / 4))
  @assert (Math.abs(q.w - expectedW) < 0.0001)
    "tumble around Z by 90deg must match the hand-computed quaternion w component"
  @assert (Math.abs(q.z - expectedZ) < 0.0001)
    "tumble around Z by 90deg must match the hand-computed quaternion z component"
  @assert (Math.abs(q.x) < 0.0001 && Math.abs(q.y) < 0.0001)
    "tumble purely around Z must leave x/y components at zero"
}

@test "tumble() normalizes a non-unit axis vector" {
  @fixture {
    <div class="t-pose-g" data-pose-g></div>
    [data-pose-g] { @pose-runtime }
  }
  @let (unit = window.__stPose.tumble(0, 0, 1, Math.PI / 2))
  @let (scaled = window.__stPose.tumble(0, 0, 5, Math.PI / 2))
  @assert (Math.abs(unit.z - scaled.z) < 0.0001 && Math.abs(unit.w - scaled.w) < 0.0001)
    "a non-unit axis (0,0,5) must produce the SAME rotation as its normalized form (0,0,1)"
}

@test "lerpVec3 interpolates component-wise" {
  @fixture {
    <div class="t-pose-h" data-pose-h></div>
    [data-pose-h] { @pose-runtime }
  }
  @let (a = window.__stPose.vec3(0, 10, -4))
  @let (b = window.__stPose.vec3(4, 0, 4))
  @let (mid = window.__stPose.lerpVec3(a, b, 0.5))
  @assert (mid.x === 2 && mid.y === 5 && mid.z === 0)
    "lerpVec3 at t=0.5 must be the exact midpoint of each component"
  @let (start = window.__stPose.lerpVec3(a, b, 0))
  @assert (start.x === a.x && start.y === a.y && start.z === a.z)
    "lerpVec3 at t=0 must equal the start vector exactly"
  @let (end = window.__stPose.lerpVec3(a, b, 1))
  @assert (end.x === b.x && end.y === b.y && end.z === b.z)
    "lerpVec3 at t=1 must equal the end vector exactly"
}

@test "slerpQuat interpolates between flat and a 90deg tumble" {
  @fixture {
    <div class="t-pose-i" data-pose-i></div>
    [data-pose-i] { @pose-runtime }
  }
  @let (a = window.__stPose.flat())
  @let (b = window.__stPose.tumble(0, 0, 1, Math.PI / 2))
  @let (start = window.__stPose.slerpQuat(a, b, 0))
  @let (end = window.__stPose.slerpQuat(a, b, 1))
  @assert (Math.abs(start.w - a.w) < 0.0001 && Math.abs(start.z - a.z) < 0.0001)
    "slerpQuat at t=0 must equal the start quaternion"
  @assert (Math.abs(end.w - b.w) < 0.0001 && Math.abs(end.z - b.z) < 0.0001)
    "slerpQuat at t=1 must equal the end quaternion"
  @let (mid = window.__stPose.slerpQuat(a, b, 0.5))
  @let (magnitude = Math.sqrt(mid.x * mid.x + mid.y * mid.y + mid.z * mid.z + mid.w * mid.w))
  @assert (Math.abs(magnitude - 1) < 0.0001)
    "slerpQuat must always return a UNIT quaternion (magnitude 1), never an un-normalized lerp"
}

@test "tumble(0,0,0, angle) — degenerate zero-length axis returns flat(), not a broken partial quaternion" {
  // Reviewer-caught bug: a first draft used `len || 1` to dodge divide-by-zero,
  // silently producing a NON-UNIT quaternion (x=y=z=0, w=cos(half)) whenever
  // the axis length was zero. "Rotate around no axis" is meaningless, so the
  // only correct answer is the identity rotation.
  @fixture {
    <div class="t-pose-j" data-pose-j></div>
    [data-pose-j] { @pose-runtime }
  }
  @let (q = window.__stPose.tumble(0, 0, 0, Math.PI / 2))
  @let (flat = window.__stPose.flat())
  @assert (q.x === flat.x && q.y === flat.y && q.z === flat.z && q.w === flat.w)
    "a zero-length tumble axis must degrade to the identity quaternion, never a non-unit partial one"
}

@test "slerpQuat ALWAYS returns a unit quaternion, even from a degenerate/non-unit input" {
  // Reviewer-caught bug: the near-parallel linear-blend fallback path did NOT
  // renormalize its result, so interpolating between flat() and a degenerate
  // tumble(0,0,0,angle) — which used to be non-unit before the tumble fix
  // above — silently carried an invalid rotation into the caller. Every
  // slerpQuat return path must now normalize, unconditionally.
  @fixture {
    <div class="t-pose-k" data-pose-k></div>
    [data-pose-k] { @pose-runtime }
  }
  @let (a = window.__stPose.flat())
  // Construct a deliberately non-unit quaternion directly (bypassing tumble's
  // own fix) to prove slerpQuat is robust to ANY non-unit input, not just the
  // one degenerate case tumble() used to produce.
  @let (nonUnitB = { x: 0, y: 0, z: 0.5, w: 0.5 })
  @let (mid = window.__stPose.slerpQuat(a, nonUnitB, 0.5))
  @let (magnitude = Math.sqrt(mid.x * mid.x + mid.y * mid.y + mid.z * mid.z + mid.w * mid.w))
  @assert (Math.abs(magnitude - 1) < 0.0001)
    "slerpQuat must renormalize even when given a non-unit endpoint quaternion"
}

@test "normalizeQuat divides every component by the true magnitude" {
  @fixture {
    <div class="t-pose-l" data-pose-l></div>
    [data-pose-l] { @pose-runtime }
  }
  @let (q = window.__stPose.normalizeQuat({ x: 0, y: 0, z: 3, w: 4 }))
  // magnitude of (0,0,3,4) is 5 -> normalized (0,0,0.6,0.8)
  @assert (Math.abs(q.z - 0.6) < 0.0001 && Math.abs(q.w - 0.8) < 0.0001)
    "normalizeQuat must scale EVERY component by 1/magnitude, not just check-and-pass"
  @let (mag = Math.sqrt(q.x * q.x + q.y * q.y + q.z * q.z + q.w * q.w))
  @assert (Math.abs(mag - 1) < 0.0001)
    "the normalized result must have unit magnitude"
}

