/// Three Pose Primitive — shared pose-interpolation runtime (PLAN-062 Wave 2).
///
/// FEAT-149's diagnosis: zellijist-2's 3 sequential bugs (disconnected ribbon,
/// off-frame scatter, tilted-not-flat) share ONE root cause — 3D pose
/// choreography authored as raw `THREE.Vector3`/`Quaternion` arithmetic has no
/// typed vocabulary for "a pose," so intent (flat/in-frame/symmetric-axes) is
/// unenforceable prose, not a checked shape. This primitive is the shared,
/// TESTED interpolation core a project's own pose-choreography code composes
/// against — installed ONCE per page via `%emit prelude-js` (mirrors
/// `stdlib/dnd/runtime.st`'s `window.__stDnd` pattern: a yield-less primitive
/// whose only job is installing a shared namespace, deduplicated by primitive
/// name).
///
/// `window.__stPose` provides:
///
///   rng(seed)                    -> a seeded, deterministic PRNG function
///                                   (mulberry32 — the exact algorithm
///                                   zellijist-2 hand-rolled per-project,
///                                   promoted here so every 3D project shares
///                                   ONE tested implementation instead of N
///                                   copies of `rr()`). `rng(seed)()` returns
///                                   0..1; the returned fn is itself stable
///                                   across calls (same seed -> same sequence).
///   rand(rngFn, min, max)        -> one value in [min,max) from an rng() fn.
///   vec3(x, y, z)                -> { x, y, z } (a plain, cheap tuple — NOT a
///                                   THREE.Vector3; callers `.set()`/`.copy()`
///                                   into their own THREE objects, this module
///                                   has NO THREE dependency so it stays
///                                   testable standalone / headless).
///   lerpVec3(a, b, t)            -> component-wise lerp of two vec3-shaped
///                                   objects (works with plain {x,y,z} OR a
///                                   real THREE.Vector3, since both expose
///                                   x/y/z).
///   flat()                       -> { x: 0, y: 0, z: 0, w: 1 } — the IDENTITY
///                                   quaternion's pitch/roll axes are
///                                   categorically zero here; there is no
///                                   slot to leave a stray non-zero value in
///                                   (this is the actual fix for zellijist-2's
///                                   bug #3 — "flat" is a KEYWORD the type
///                                   enforces, not 6 raw numbers a human must
///                                   remember to zero).
///   tilt(x, y)                   -> a quaternion with ONLY pitch (x) / roll
///                                   (y) components (radians), zero yaw.
///   tumble(axisX, axisY, axisZ, angle)
///                                -> a quaternion rotated `angle` radians
///                                   around the given axis (normalized
///                                   internally) — the LIFT-phase tumble;
///                                   the only orientation kind that legitimately
///                                   carries pitch/roll, kept structurally
///                                   distinct from `flat`/`tilt` so a caller
///                                   can never accidentally leave tumble's
///                                   axis/angle non-zero in a `flat` slot.
///   slerpQuat(a, b, t)           -> spherical-linear interpolation between
///                                   two {x,y,z,w} quaternion-shaped objects
///                                   (pure math, no THREE dependency).
///
/// @yields (none — prelude-only primitive, mirrors dnd-runtime's shape)

%primitive three-pose() {
  %emit prelude-js {
    window.__stPose = window.__stPose || (function () {
      // --- seeded PRNG (mulberry32) -------------------------------------------
      // The EXACT algorithm projects/zellijist-2/lib/three-tile-field.st hand-
      // rolled per-project (`rr()`), promoted here so every 3D project shares
      // ONE tested implementation instead of N ad-hoc copies. Deterministic:
      // the SAME seed always produces the SAME sequence (required for CDP-test
      // reproducibility, per docs/testing/ANIMATION.md's determinism bar).
      function rng(seed) {
        let s = (seed >>> 0) || 1;
        return function () {
          s |= 0; s = (s + 0x6D2B79F5) | 0;
          let t = Math.imul(s ^ (s >>> 15), 1 | s);
          t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
          return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
        };
      }

      function rand(rngFn, min, max) {
        return min + (max - min) * rngFn();
      }

      function vec3(x, y, z) {
        return { x: x || 0, y: y || 0, z: z || 0 };
      }

      function lerpVec3(a, b, t) {
        return {
          x: a.x + (b.x - a.x) * t,
          y: a.y + (b.y - a.y) * t,
          z: a.z + (b.z - a.z) * t,
        };
      }

      // --- orientation vocabulary ---------------------------------------------
      // `flat` is the identity quaternion. It has NO axis/angle parameters to
      // accept a stray value — this is the structural fix for zellijist-2's
      // "tilted, not flat" bug: a caller who wants flat literally cannot leave
      // pitch/roll non-zero, because there is no argument slot for it.
      function flat() {
        return { x: 0, y: 0, z: 0, w: 1 };
      }

      // Quaternion from Euler angles (XYZ order, radians) — used by `tilt` to
      // build a pitch/roll-only rotation (yaw always 0) and internally by
      // `tumble` via axis-angle instead. Pure math (no THREE dependency), so
      // this primitive is unit-testable headless with zero WebGL/canvas.
      function eulerToQuat(ex, ey, ez) {
        const cx = Math.cos(ex / 2), sx = Math.sin(ex / 2);
        const cy = Math.cos(ey / 2), sy = Math.sin(ey / 2);
        const cz = Math.cos(ez / 2), sz = Math.sin(ez / 2);
        // XYZ intrinsic order (matches THREE.Euler's default 'XYZ').
        return {
          x: sx * cy * cz + cx * sy * sz,
          y: cx * sy * cz - sx * cy * sz,
          z: cx * cy * sz + sx * sy * cz,
          w: cx * cy * cz - sx * sy * sz,
        };
      }

      function tilt(ex, ey) {
        return eulerToQuat(ex || 0, ey || 0, 0);
      }

      function tumble(axisX, axisY, axisZ, angle) {
        const len = Math.sqrt(axisX * axisX + axisY * axisY + axisZ * axisZ);
        // A degenerate (zero-length) axis cannot define a rotation direction.
        // Reviewer-caught bug: the FIRST draft used `len || 1` to avoid a NaN
        // from dividing by zero, but that silently produced x=y=z=0 while
        // w=cos(half) — a NON-UNIT quaternion whenever half !== 0 (e.g.
        // tumble(0,0,0, PI/2) has magnitude |cos(PI/4)| ≈ 0.707, not 1). Since
        // "rotate around no axis" is meaningless, the only correct answer is
        // the IDENTITY rotation (flat()) — not a broken partial quaternion.
        if (len < 1e-9) {
          return flat();
        }
        const ax = axisX / len, ay = axisY / len, az = axisZ / len;
        const half = angle / 2;
        const s = Math.sin(half);
        return { x: ax * s, y: ay * s, z: az * s, w: Math.cos(half) };
      }

      // Spherical-linear interpolation between two quaternions. Pure math
      // (Ken Shoemake's formula), no THREE dependency.
      // Reviewer-caught bug: EVERY return path below must yield a UNIT
      // quaternion (a rotation's defining invariant) — the linear-blend
      // fallback path (near-parallel `a`/`b`) does NOT preserve unit length by
      // construction (a linear interpolation between two unit vectors is, in
      // general, shorter than 1), and neither does the direct-return path if
      // an already non-unit `a`/`b` reaches this function (e.g. from a
      // degenerate caller). Every path normalizes its result through this one
      // helper so the invariant holds UNCONDITIONALLY, not just for the
      // "well-behaved unit-input" case the original formula silently assumed.
      function normalizeQuat(q) {
        const mag = Math.sqrt(q.x * q.x + q.y * q.y + q.z * q.z + q.w * q.w);
        if (mag < 1e-12) {
          return flat();
        }
        return { x: q.x / mag, y: q.y / mag, z: q.z / mag, w: q.w / mag };
      }

      function slerpQuat(a, b, t) {
        let bx = b.x, by = b.y, bz = b.z, bw = b.w;
        let cosHalfTheta = a.x * bx + a.y * by + a.z * bz + a.w * bw;
        if (cosHalfTheta < 0) {
          bx = -bx; by = -by; bz = -bz; bw = -bw;
          cosHalfTheta = -cosHalfTheta;
        }
        if (cosHalfTheta >= 1.0) {
          return normalizeQuat({ x: a.x, y: a.y, z: a.z, w: a.w });
        }
        const sqrSinHalfTheta = 1.0 - cosHalfTheta * cosHalfTheta;
        if (sqrSinHalfTheta <= 1e-12) {
          const s = 1 - t;
          return normalizeQuat({
            x: a.x * s + bx * t,
            y: a.y * s + by * t,
            z: a.z * s + bz * t,
            w: a.w * s + bw * t,
          });
        }
        const sinHalfTheta = Math.sqrt(sqrSinHalfTheta);
        const halfTheta = Math.atan2(sinHalfTheta, cosHalfTheta);
        const ratioA = Math.sin((1 - t) * halfTheta) / sinHalfTheta;
        const ratioB = Math.sin(t * halfTheta) / sinHalfTheta;
        return normalizeQuat({
          x: a.x * ratioA + bx * ratioB,
          y: a.y * ratioA + by * ratioB,
          z: a.z * ratioA + bz * ratioB,
          w: a.w * ratioA + bw * ratioB,
        });
      }

      return {
        rng, rand, vec3, lerpVec3,
        flat, tilt, tumble, slerpQuat,
        eulerToQuat, normalizeQuat,
      };
    })();
  }
}
