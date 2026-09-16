@import "stdlib/3d"

/* ============================================================================
   Scroll Tunnel — a scroll-driven flythrough of a neon tube.

   Scrolling dollies the camera down a glowing procedural tunnel (a torus seen
   from inside, ringed with emissive geometry) while a chromatic-aberration +
   bloom post stack gives it the cinematic neon-flythrough look. The scroll
   progress drives the camera Z entirely through @scroll-3d — no scroll code.
   ============================================================================ */

body {
  margin: 0;
  background: #02020a;
  color: #dfe4ff;
  font-family: "Inter", system-ui, -apple-system, sans-serif;
  -webkit-font-smoothing: antialiased;
}

/* Tall track for the flythrough. */
.scroll-track { height: 520vh; }

canvas.tunnel {
  position: fixed;
  inset: 0;
  width: 100%;
  height: 100%;
  z-index: 0;
  background: radial-gradient(circle at 50% 50%, #0a0a2a 0%, #02020a 75%);

  /* The camera flies THROUGH a big glowing torus-knot: it starts in front of the
     knot (camZ positive) and scroll pulls it forward, threading the camera into
     and through the knot's tube — the loops sweep past like a neon corridor. */
  @stage(camZ: 12, fov: 82, near: 0.01, far: 80, tone: "aces", exposure: 1.0) {
    /* The corridor: a large, many-segment torus-knot of glowing material. Up
       close from inside, its tube reads as a rushing neon corridor. A steady
       roll makes the walls spiral past. */
    @object(
      shape: "torusknot",
      detail: 6,
      size: 6,
      material: "standard",
      color: #3a1a6a,
      emissive: #7b2cff,
      emissiveIntensity: 1.6,
      roughness: 0.4,
      metalness: 0.2,
      spinZ: 0.08
    )

    /* Drifting point lights give the corridor moving hotspots as you pass. */
    @light(type: "point", color: #ff3aa6, intensity: 60, distance: 22, posX: 3, posY: 2, posZ: 2)
    @light(type: "point", color: #2ec5ff, intensity: 60, distance: 22, posX: -3, posY: -2, posZ: -8)
    @light(type: "ambient", intensity: 0.35)

    /* Scroll flies the camera forward through the knot (z: 12 → -22). */
    @scroll-3d(drive: "dollyZ", from: 12, to: -22, ease: 0.07)

    /* Bloom + chromatic aberration for the neon-flythrough cinematics. */
    @post(bloomStrength: 0.85, bloomRadius: 0.6, bloomThreshold: 0.3, rgbShift: 0.0022)
  }
}

.captions {
  position: fixed;
  inset: auto auto 8vh 7vw;
  z-index: 1;
  pointer-events: none;
}

.eyebrow {
  margin: 0 0 0.6rem;
  font-size: 0.72rem;
  letter-spacing: 0.28em;
  text-transform: uppercase;
  color: #8089c0;
}

.headline {
  margin: 0;
  max-width: 16ch;
  color: #eef0ff;
  font-size: clamp(2rem, 5.5vw, 3.6rem);
  font-weight: 800;
  line-height: 1.05;
  letter-spacing: -0.02em;
  text-shadow: 0 4px 40px rgba(40, 20, 90, 0.7);
}

.hint {
  margin: 1rem 0 0;
  font-size: 0.8rem;
  color: #5f6798;
}
