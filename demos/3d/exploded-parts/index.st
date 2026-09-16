@import "stdlib/3d"

/* ============================================================================
   Exploded Parts — PLAN-062 Wave 5 second-consumer proof.

   Proves the typed 3D pose surface (FEAT-149, PLAN-062 Waves 1-3) is GENERAL,
   not zellijist-shaped: an independent scene — 5 discrete objects radially
   exploding outward from a shared core, then reassembling on scroll — built
   with ZERO new Rust/stdlib code, consuming ONLY what Waves 1-3 already
   shipped:

     - `canvas._stThree.bounds(z)`     (Wave 1) keeps every exploded part
       provably within the visible frame, at THIS scene's own camera framing
       (fov: 36, camZ: 8 — different from zellijist-2's fov: 32, camZ: 5.5),
       read directly off the SAME canvas @stage already initialized (no
       re-derived or copy-pasted frustum trig).
     - `window.__stPose.flat()`/`.tumble()` (Wave 2) build each part's
       exploded pose — a settling tumble on the way out, flat (face-on) once
       reassembled at either end of the timeline.
     - `@pose-track` (Wave 3) drives each part's position/orientation from
       page scroll — 5 SEPARATE `@object` + `@pose-track` pairs (not
       zellijist's 12-tile single-primitive batch), each binding its own
       `canvas._stObject` (the most-recently-added mesh) independently.

   This is a RADIAL EXPLOSION (5 discrete parts flying outward from one
   center, each on its own axis), structurally different from zellijist's
   VERTICAL ZIGZAG COLUMN choreography (12 co-planar triangle tiles staggered
   along one shared axis) — the invariant under test is that the SAME three
   primitives (bounds / pose-track / pose value-kinds) compose correctly for
   a scene shape neither wave was designed around.
   ============================================================================ */

body {
  margin: 0;
  background: #f4f2ee;
  color: #2a2a26;
  font-family: "Inter", system-ui, -apple-system, sans-serif;
  -webkit-font-smoothing: antialiased;
}

.scroll-track { height: 300vh; }

canvas.parts {
  position: fixed;
  inset: 0;
  width: 100%;
  height: 100%;
  z-index: 0;

  @stage(fov: 36, camZ: 8, tone: "aces", exposure: 1.05) {
    @light(type: "directional", intensity: 1.6)
    @light(type: "ambient", intensity: 0.5)
    @env(preset: "studio", intensity: 0.5)

    /* Part 1 — explodes along +X, tumbles a quarter-turn around Z on the way out. */
    @object(shape: "icosahedron", size: 0.5, color: #c65b3c, roughness: 0.35, metalness: 0.2)
    @pose-track(ease: 0.12, stops: [
      { at: 0, pos: { x: 0, y: 0, z: 0 }, face: window.__stPose.flat() },
      { at: 0.5, pos: { x: document.querySelector('canvas.parts')._stThree.bounds(0).halfW * 0.55, y: 0.3, z: 0.4 }, face: window.__stPose.tumble(0, 0, 1, Math.PI / 2 * 0.6) },
      { at: 1, pos: { x: document.querySelector('canvas.parts')._stThree.bounds(0).halfW * 0.55, y: 0.3, z: 0.4 }, face: window.__stPose.flat() }
    ])

    /* Part 2 — explodes along -X. */
    @object(shape: "box", size: 0.45, color: #3c7ac6, roughness: 0.4, metalness: 0.1)
    @pose-track(ease: 0.12, stops: [
      { at: 0, pos: { x: 0, y: 0, z: 0 }, face: window.__stPose.flat() },
      { at: 0.5, pos: { x: -document.querySelector('canvas.parts')._stThree.bounds(0).halfW * 0.55, y: -0.4, z: 0.6 }, face: window.__stPose.tumble(0, 1, 0, Math.PI / 2 * 0.7) },
      { at: 1, pos: { x: -document.querySelector('canvas.parts')._stThree.bounds(0).halfW * 0.55, y: -0.4, z: 0.6 }, face: window.__stPose.flat() }
    ])

    /* Part 3 — explodes upward. */
    @object(shape: "torus", size: 0.4, color: #c6a63c, roughness: 0.3, metalness: 0.3)
    @pose-track(ease: 0.12, stops: [
      { at: 0, pos: { x: 0, y: 0, z: 0 }, face: window.__stPose.flat() },
      { at: 0.5, pos: { x: 0.2, y: document.querySelector('canvas.parts')._stThree.bounds(0).halfH * 0.5, z: 0.3 }, face: window.__stPose.tumble(1, 0, 0, Math.PI / 2 * 0.5) },
      { at: 1, pos: { x: 0.2, y: document.querySelector('canvas.parts')._stThree.bounds(0).halfH * 0.5, z: 0.3 }, face: window.__stPose.flat() }
    ])

    /* Part 4 — explodes downward. */
    @object(shape: "cone", size: 0.4, color: #6ac63c, roughness: 0.4, metalness: 0.15)
    @pose-track(ease: 0.12, stops: [
      { at: 0, pos: { x: 0, y: 0, z: 0 }, face: window.__stPose.flat() },
      { at: 0.5, pos: { x: -0.3, y: -document.querySelector('canvas.parts')._stThree.bounds(0).halfH * 0.5, z: 0.5 }, face: window.__stPose.tumble(1, 1, 0, Math.PI / 2 * 0.4) },
      { at: 1, pos: { x: -0.3, y: -document.querySelector('canvas.parts')._stThree.bounds(0).halfH * 0.5, z: 0.5 }, face: window.__stPose.flat() }
    ])

    /* Part 5 — the core, stays put but does a slow settling tumble. */
    @object(shape: "sphere", size: 0.55, color: #e8e2d5, roughness: 0.2, metalness: 0.05)
    @pose-track(ease: 0.12, stops: [
      { at: 0, pos: { x: 0, y: 0, z: 0 }, face: window.__stPose.flat() },
      { at: 0.5, pos: { x: 0, y: 0, z: 0 }, face: window.__stPose.tumble(0, 1, 0, Math.PI) },
      { at: 1, pos: { x: 0, y: 0, z: 0 }, face: window.__stPose.flat() }
    ])
  }
}

.captions {
  position: fixed;
  top: 2.5rem;
  left: 2.5rem;
  z-index: 1;
  max-width: 30rem;
  pointer-events: none;
}
.eyebrow { font-size: 0.85rem; letter-spacing: 0.08em; text-transform: uppercase; opacity: 0.6; margin: 0 0 0.5rem; }
.headline { font-size: 2rem; font-weight: 600; margin: 0 0 0.5rem; }
.hint { font-size: 0.9rem; opacity: 0.55; margin: 0; }
