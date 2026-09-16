@import "stdlib/3d"

/* ============================================================================
   GLTF Studio — a real PBR model under three-point studio lighting.

   Loads the canonical glTF PBR test asset (the "Damaged Helmet") and presents it
   like a product shot: an environment map for realistic reflections, a warm key
   + cool rim + soft fill, ACES tone-mapping, and drag-to-orbit inspection.

   The model loads through @gltf — the loader rail the old hand-rolled engine
   never delivered reliably. Auto-centered + scaled to frame; zero JavaScript.
   ============================================================================ */

body {
  margin: 0;
  min-height: 100vh;
  background: #0b0b10;
  color: #ececf2;
  font-family: "Inter", system-ui, -apple-system, sans-serif;
  -webkit-font-smoothing: antialiased;
  overflow: hidden;
}

canvas.studio {
  position: fixed;
  inset: 0;
  width: 100%;
  height: 100%;
  z-index: 0;
  background:
    radial-gradient(circle at 50% 45%, #1c1c28 0%, #0b0b10 70%);

  @stage(camZ: 4, tone: "aces", exposure: 1.0) {
    /* Studio env → realistic metal + visor reflections on the helmet. */
    @env(preset: "studio", intensity: 1.0)

    /* Three-point lighting: warm key, cool rim from behind, soft fill. */
    @light(type: "directional", intensity: 2.6, color: #fff0e0, posX: 4, posY: 5, posZ: 4)
    @light(type: "directional", intensity: 1.6, color: #cfe0ff, posX: -4, posY: 2, posZ: -5)
    @light(type: "ambient", intensity: 0.25)

    /* The real model. Auto-centered + fit to a 2.4-unit bound, slow showcase spin. */
    @gltf(src: "models/helmet/DamagedHelmet.gltf", size: 2.4, spinY: 0.25)

    /* Drag to inspect from any angle (inertial). */
    @orbit(damping: 0.08, enableZoom: true, minDistance: 2.5, maxDistance: 8)
  }
}

.overlay {
  position: fixed;
  inset: 0;
  z-index: 1;
  display: grid;
  align-content: start;
  gap: 0.6rem;
  padding: clamp(2rem, 5vw, 4rem);
  pointer-events: none;
}

.eyebrow {
  margin: 0;
  font-size: 0.72rem;
  letter-spacing: 0.28em;
  text-transform: uppercase;
  color: #8b8f9e;
}

.title {
  margin: 0;
  max-width: 14ch;
  color: #f6f6fa;
  font-size: clamp(2rem, 5vw, 3.4rem);
  font-weight: 800;
  line-height: 1.04;
  letter-spacing: -0.02em;
}

.lede {
  margin: 0;
  max-width: 42ch;
  font-size: clamp(0.9rem, 1.5vw, 1.05rem);
  line-height: 1.55;
  color: #b4b7c6;
}

.hint {
  position: fixed;
  inset: auto auto clamp(2rem, 5vw, 4rem) clamp(2rem, 5vw, 4rem);
  z-index: 1;
  margin: 0;
  font-size: 0.8rem;
  color: #6c7080;
  pointer-events: none;
}

code {
  font-family: ui-monospace, "SF Mono", Menlo, monospace;
  font-size: 0.92em;
  color: #c8b8ff;
}
