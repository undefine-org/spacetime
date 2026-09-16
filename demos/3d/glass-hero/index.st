@import "stdlib/3d"

/* ============================================================================
   Iridescent Hero — a thin-film iridescent gem on a deep gradient.

   The soap-bubble / oil-slick sheen that wins awards: a physically-based
   thin-film iridescent clearcoat whose colour shifts with light + view angle,
   on a flat-faceted gem so the highlights leap between faces as it tumbles.
   Slowly auto-spinning, yielding to your drag, following your mouse.

   (We reach for iridescence rather than transmissive "glass" deliberately:
   three.js transmission is screen-space refraction that needs rich content
   BEHIND the object to read — a dark minimalist hero starves it. Iridescence
   carries its colour on the surface itself, so it's gorgeous at every angle.)

   Every line is declarative Spacetime. The 3D engine is vendored three.js,
   reached only through the @stage / @object / @light / @env / @orbit surface —
   no hand-written JavaScript, no three.js API in sight.
   ============================================================================ */

body {
  margin: 0;
  min-height: 100vh;
  background: #05060d;
  color: #eaeaf2;
  font-family: "Inter", system-ui, -apple-system, sans-serif;
  -webkit-font-smoothing: antialiased;
  overflow: hidden;
}

/* The stage canvas. A deep radial wash sits BEHIND the transparent WebGL (the
   @stage renders with alpha) so the iridescent gem reads against dark space. */
canvas.hero {
  position: fixed;
  inset: 0;
  width: 100%;
  height: 100%;
  z-index: 0;
  background:
    radial-gradient(circle at 50% 38%, #241a4d 0%, #0c0a1f 46%, #05060d 100%);

  @stage(camZ: 5, tone: "aces", exposure: 0.55) {
    /* A vivid studio environment drives the iridescent sheen — iridescence is a
       reflection effect, so a rich env is what makes the colours dance. */
    /* Modest env so reflections don't wash the thin-film colours to white —
       iridescence pops hardest over a DARK base, lit by coloured light. */
    @env(preset: "studio", intensity: 0.35)

    /* Saturated key + rim lights. Iridescence shifts hue with light + view
       angle, so two strongly-tinted lights from opposite sides make the surface
       sweep through a spectrum as it turns. */
    @light(type: "directional", intensity: 2.6, color: #ff55dd, posX: 4, posY: 5, posZ: 4)
    @light(type: "directional", intensity: 2.2, color: #4db8ff, posX: -5, posY: -2, posZ: -3)
    @light(type: "ambient", intensity: 0.1)

    /* The artifact: an iridescent faceted gem. Per the three.js transmission
       reality (screen-space refraction needs rich content behind it, which a dark
       minimalist hero can't provide), this uses a thin-film IRIDESCENT clearcoat
       instead — a soap-bubble / oil-slick sheen that is gorgeous at EVERY angle
       because its colour comes from the surface itself, not from whatever happens
       to be behind it. Flat-shaded facets make the highlights leap between faces
       as it tumbles. */
    @object(
      shape: "icosahedron",
      detail: 1,
      size: 1.7,
      material: "physical",
      color: #0a0c18,
      transmission: 0,
      roughness: 0.08,
      metalness: 0.0,
      clearcoat: 1,
      iridescence: 1,
      iridescenceIOR: 1.35,
      flatShading: true,
      spinY: 0.15,
      spinX: 0.08
    )

    /* Drag to rotate (inertial) with a slow idle auto-spin + mouse parallax. */
    @orbit(damping: 0.1, autoRotate: 0.3)
    @parallax(strength: 0.12)
  }
}

/* Text overlay — pointer-events:none so it never steals the drag. */
.overlay {
  position: fixed;
  inset: 0;
  z-index: 1;
  display: grid;
  align-content: end;
  gap: 0.75rem;
  padding: clamp(2rem, 6vw, 5rem);
  pointer-events: none;
}

.eyebrow {
  margin: 0;
  font-size: 0.72rem;
  letter-spacing: 0.28em;
  text-transform: uppercase;
  color: #8a8fb5;
}

.title {
  margin: 0;
  max-width: 16ch;
  color: #f4f4fb;
  font-size: clamp(2.4rem, 7vw, 5rem);
  font-weight: 800;
  line-height: 1.02;
  letter-spacing: -0.02em;
  text-shadow: 0 4px 40px rgba(10, 8, 30, 0.85);
}

.lede {
  margin: 0;
  max-width: 46ch;
  font-size: clamp(0.95rem, 1.6vw, 1.1rem);
  line-height: 1.55;
  color: #b6bbdd;
}

.hint {
  margin: 0.5rem 0 0;
  font-size: 0.8rem;
  color: #6b6f93;
}
