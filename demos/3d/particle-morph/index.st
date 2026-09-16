@import "stdlib/3d"

/* ============================================================================
   Particle Morph — 30,000 GPU particles that morph between shapes on scroll.

   A glowing point cloud transitions sphere → torus-knot → galaxy → wave grid as
   you scroll, additively blended and lifted by bloom for that signature
   particle-hero glow. The scroll progress drives the morph entirely through the
   declarative @scroll-3d binding — no scroll listeners in site code.
   ============================================================================ */

body {
  margin: 0;
  background: #04040a;
  color: #e6e9f5;
  font-family: "Inter", system-ui, -apple-system, sans-serif;
  -webkit-font-smoothing: antialiased;
}

/* Tall scroll track gives the morph room to breathe. */
.scroll-track { height: 420vh; }

/* The canvas is pinned; scrolling the track drives the particle morph. */
canvas.field {
  position: fixed;
  inset: 0;
  width: 100%;
  height: 100%;
  z-index: 0;
  background: radial-gradient(circle at 50% 45%, #0a0a22 0%, #04040a 70%);

  @stage(camZ: 6.5, tone: "aces", exposure: 0.85) {
    /* 30k additively-blended particles. The field morphs across its four
       built-in shapes (0=sphere, 1=torus-knot, 2=galaxy, 3=wave grid). */
    @particles(count: 30000, size: 0.018, radius: 2.2, color: #8ab4ff, spin: 0.05)

    /* Scroll progress (0→1 over the track) drives the morph 0→3. */
    @scroll-3d(drive: "morph", to: 3, ease: 0.06)

    /* Gentle bloom on the additive build-up — enough to glow, not to flood the
       dense centre to a white disc. */
    @post(bloomStrength: 0.55, bloomRadius: 0.45, bloomThreshold: 0.35)
  }
}

/* Fixed overlay captions, one per morph stage, fading with scroll position
   via @scroll-spy-style reveals. Kept simple + declarative. */
.captions {
  position: fixed;
  inset: auto auto 8vh 8vw;
  z-index: 1;
  pointer-events: none;
}

.eyebrow {
  margin: 0 0 0.6rem;
  font-size: 0.72rem;
  letter-spacing: 0.28em;
  text-transform: uppercase;
  color: #7e84ad;
}

.headline {
  margin: 0;
  max-width: 18ch;
  color: #f2f4fc;
  font-size: clamp(2rem, 5.5vw, 3.6rem);
  font-weight: 800;
  line-height: 1.05;
  letter-spacing: -0.02em;
}

.hint {
  margin: 1rem 0 0;
  font-size: 0.8rem;
  color: #646a90;
}
