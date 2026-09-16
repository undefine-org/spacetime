/// Three Pose Track Primitive — apply page-scroll-driven pose keyframes to a
/// single 3D object (PLAN-062 Wave 3).
///
/// Interpolates a target `THREE.Object3D`'s `position`/`quaternion` across a
/// sequence of named POSE STOPS as page-scroll progress moves 0..1, using the
/// SAME tested interpolation core `stdlib/3d/primitives/three-pose.st` shipped
/// in Wave 2 (`window.__stPose.lerpVec3`/`flat`/`tilt`/`tumble`/`slerpQuat`) —
/// this primitive is the thin per-frame driver + stop-lookup wrapper around
/// that already-tested math, not a re-implementation of it.
///
/// Each stop is `{ pos: {x,y,z}, face: {x,y,z,w} }` — a plain JS object, NOT
/// the `keyframes` capture-type grammar. Wiring `pos`/`face` as AUTHORED
/// keyframe PROPERTIES (e.g. `@scroll tile-pose { pos: ... -> ...; face: ... }`)
/// is DEFERRED (see FUP-106): it needs `%scope within(<construct>)` DISPATCH —
/// a mechanism `find_macro_by_directive`/`score_macro_form` never actually
/// consume today (`macro_within_matches` in `src/metasystem/registry.rs` is
/// unit-tested in isolation but has ZERO production call sites — confirmed
/// dead code, a real compiler gap bigger than a Wave-3 spike budget). Until
/// that lands, a project's own primitive builds its stops as a plain JS array
/// and passes them to `stops:`, exactly as `three-scroll.st`'s `drive:` params
/// are supplied today — no new grammar, no new dispatch-ambiguity risk.
///
/// @param &el       - the <canvas> hosting the @stage (reads canvas._stThree)
/// @param stops     - an array of { at: number, pos: {x,y,z}, face: quat } —
///                    `at` is the scroll-progress fraction (0..1) this stop
///                    applies at; MUST be sorted ascending by `at`.
/// @param ease      - per-frame smoothing 0..1 (default 0.08, matches
///                    three-scroll's default — same visual feel)
/// BINDING (BUG-263 item 4): each `@pose-track` binds its PRECEDING `@object`
///                    by SOURCE ORDER — the compiler emits sibling primitive
///                    inits in author order, so when this track's init runs,
///                    `canvas._stObject` is exactly the `@object` authored
///                    immediately before it. No `objectIndex` exists: to drive
///                    a specific object, author `@object` then `@pose-track`
///                    as an adjacent PAIR in the `@stage` body.
///
/// @yields $progress - eased scroll progress 0..1 (same shape as three-scroll)

%primitive three-pose-track(
  &el,
  stops: expr,
  ease: number = 0.08
) {
  %emit js {
    const canvas = %&el;
    const ctx = canvas && canvas._stThree;
    if (!ctx) {
      console.warn('[Spacetime] three-pose-track: no @stage on this canvas.');
      return;
    }
    if (!window.__stPose) {
      console.warn('[Spacetime] three-pose-track: three-pose runtime not loaded (missing @pose-runtime / three-pose binding).');
      return;
    }
    // BUG-263 item 4: sibling primitive inits emit in SOURCE ORDER, so when
    // this track's init runs `canvas._stObject` is its PRECEDING `@object` —
    // the interleaved source-order pairing replaces the retired `objectIndex`
    // mechanism (each `@object` + `@pose-track` PAIR drives itself).
    const target = canvas._stObject;
    if (!target) {
      console.warn('[Spacetime] three-pose-track: no @object on this stage to pose (canvas._stObject is empty).');
      return;
    }

    const pose = window.__stPose;
    const stopsList = %stops;
    if (!Array.isArray(stopsList) || stopsList.length === 0) {
      console.warn('[Spacetime] three-pose-track: stops must be a non-empty array.');
      return;
    }

    const clamp01 = (x) => (x < 0 ? 0 : x > 1 ? 1 : x);

    // Find the two stops surrounding `progress` and the local interpolation
    // factor between them — same shape as apply-animations.js's
    // interpolateKeyframes, generalized to a vec3+quat pair instead of a
    // scalar CSS value.
    const sampleAt = (progress) => {
      if (stopsList.length === 1) return { pos: stopsList[0].pos, face: pose.normalizeQuat(stopsList[0].face) };
      let prev = stopsList[0];
      let next = stopsList[stopsList.length - 1];
      for (let i = 0; i < stopsList.length - 1; i++) {
        if (progress >= stopsList[i].at && progress <= stopsList[i + 1].at) {
          prev = stopsList[i];
          next = stopsList[i + 1];
          break;
        }
      }
      // Reviewer-caught bug: the boundary returns (progress at or before the
      // first stop, at or after the last, or the single-stop case above) must
      // NOT hand back a stop's raw `face` verbatim — a stops array a caller
      // supplies is untrusted input (e.g. `{x:0,y:0,z:0,w:0.5}`, a non-unit
      // quaternion), and `target.quaternion.set(...)` copies components
      // WITHOUT normalizing. Route every returned face through
      // `pose.normalizeQuat` so the unit-quaternion invariant holds
      // UNCONDITIONALLY — the same defense-in-depth principle Wave 2's
      // `slerpQuat` fix already established, now applied at every boundary
      // this primitive itself can return, not just the interpolated path.
      if (progress <= prev.at) return { pos: prev.pos, face: pose.normalizeQuat(prev.face) };
      if (progress >= next.at) return { pos: next.pos, face: pose.normalizeQuat(next.face) };
      const span = next.at - prev.at;
      const localT = span > 0 ? (progress - prev.at) / span : 0;
      return {
        pos: pose.lerpVec3(prev.pos, next.pos, localT),
        face: pose.slerpQuat(prev.face, next.face, localT),
      };
    };

    const E = %ease;
    const sampleScroll = () => {
      const max = document.documentElement.scrollHeight - window.innerHeight;
      const y = window.scrollY || window.pageYOffset || 0;
      return max > 0 ? clamp01(y / max) : 0;
    };
    let goal = sampleScroll();
    const onScroll = () => { goal = sampleScroll(); };
    window.addEventListener('scroll', onScroll, { passive: true });

    let cur = goal;
    const unregister = ctx.registerFrame(() => {
      cur += (goal - cur) * E;
      const sample = sampleAt(cur);
      target.position.set(sample.pos.x, sample.pos.y, sample.pos.z);
      target.quaternion.set(sample.face.x, sample.face.y, sample.face.z, sample.face.w);
      %yield cur -> $progress;
    });

    %cleanup {
      window.removeEventListener('scroll', onScroll);
      if (unregister) unregister();
    }
  }

  %exports {
    $progress: number
  }
}
