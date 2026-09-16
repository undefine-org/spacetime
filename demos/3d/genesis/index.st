@version 2026-07-26;

@import "stdlib"
@import "stdlib/3d"

/* ============================================================================
   Genesis — an object no human hand modeled.

   The artifact: a bronze medallion carved entirely inside a headless Blender
   process an agent drove tool-call by tool-call over MCP (tools/blender-mcp/
   bootstrap.py) — zero GUI, zero manual modeling. Seven calls (extrude a
   96-gon disc → cut a spiral engraving via boolean difference → cut a rim
   groove the same way → fuse a raised boss via boolean union → assign a PBR
   bronze material → export uncompressed glTF) produced models/medallion.glb.
   `@gltf` doesn't know or care where an asset came from — this is the proof
   that the pipeline is real: the file loads exactly like any hand-modeled one.

   The page: a pinned 3D stage with a scroll-driven camera dolly + spin,
   narrated by four acts (hero → pipeline → craft → close) whose headline +
   lede fade/lift in as each act's viewport comes into view (@load, a
   threshold-based on-visible primitive — NOT @reveal/split:"lines", which
   has a pre-existing content-duplication bug independently reproduced on
   demos/reveal-lines-reflow-free; see the NB comment below). The medallion's
   dolly + rotation come from @scroll-3d. No project-local lib/ primitive, no
   authored JavaScript anywhere — the object AND the motion are 100% the
   stdlib/3d @-surface.
   ============================================================================ */

body {
  margin: 0;
  min-height: 100vh;
  background: #0a0906;
  color: #f1ece0;
  font-family: "Inter", system-ui, -apple-system, sans-serif;
  -webkit-font-smoothing: antialiased;
  overflow-x: hidden;
}

/* The pinned 3D world. A warm, near-black studio sweep behind the alpha WebGL
   so the bronze medallion reads as a lit object in space, not a cutout. */
canvas.stage {
  position: fixed;
  inset: 0;
  width: 100%;
  height: 100%;
  z-index: 0;
  background:
    radial-gradient(120% 100% at 50% 38%, #17130c 0%, #0d0b07 55%, #0a0906 100%);

  @stage(camY: 2.0, camZ: 4.4, fov: 42, tone: "aces", exposure: 0.62) {
    /* Studio env for real metal reflections on the bronze — this is what
       sells "solid carved object" over "flat-shaded prop". Kept modest so
       the disc's flat cap doesn't blow the specular under the elevated
       camera + close-range dolly. */
    @env(preset: "studio", intensity: 0.4)

    /* Warm key angled OFF the camera's direct reflection path (offset in X
       + kept lower in Y than the previous pass) + cool rim from
       behind-low for edge separation — tuned so the spiral engraving and
       raised seal read as carved relief, not a single blown highlight. */
    @light(type: "directional", intensity: 0.9, color: #ffdfb0, posX: 5, posY: 2, posZ: 3)
    @light(type: "directional", intensity: 0.55, color: #8fb4ff, posX: -4, posY: 0.5, posZ: -4)
    @light(type: "ambient", intensity: 0.14)

    /* The artifact. Auto-centered + scaled to a 2.6-unit bound; a slow idle
       spin so the engraving/reflections are never fully static even before
       scroll takes over. */
    @gltf(src: "models/medallion.glb", size: 2.6, spinY: 0.12)

    /* Scroll dollies the camera in toward the medallion across the whole
       track — the reveal literally pulls the viewer closer to the carved
       detail as the narrative progresses. */
    @scroll-3d(drive: "dollyZ", from: 4.2, to: 2.2, ease: 0.06)

    /* Drag still works at any scroll position (inertial, gentle idle spin of
       its own so the piece never looks inert while reading copy). */
    @orbit(damping: 0.09, autoRotate: 0.15, enableZoom: false)

    /* A whisper of bloom on the brightest specular highlights — just enough
       to sell "polished metal under studio light", never a glow effect. */
    @post(bloomStrength: 0.06, bloomRadius: 0.2, bloomThreshold: 0.97)
  }
}

/* ============================================================================
   Narrative acts — four real 100vh document-flow sections. The canvas above
   stays position:fixed, so the medallion holds its place in the viewport
   while these scroll past over it (classic pinned-3D + scrolling-copy). Each
   headline/lede kinetic-reveals in via @reveal the moment its own act enters
   view — no scroll-spy/@when signal plumbing needed for this; @reveal already
   IS the proven "animate once visible" primitive (glass-hero, text-engine,
   reveal-lines-reflow-free all use it the same way).
   ============================================================================ */

.acts {
  position: relative;
  z-index: 1;
  pointer-events: none;
}

.act {
  min-height: 100vh;
  display: grid;
  align-content: center;
  gap: 1rem;
  max-width: 34rem;
  padding: 0 clamp(1.5rem, 6vw, 5rem);
}

.act--hero { justify-items: start; text-align: left; }
.act--pipeline { justify-items: end; text-align: right; margin-left: auto; }
.act--craft { justify-items: start; text-align: left; }
.act--close { justify-items: end; text-align: right; margin-left: auto; }

/* NB: @reveal(split:"lines") has a pre-existing content-duplication bug (also
   reproduces on demos/reveal-lines-reflow-free — confirmed independently,
   not introduced here; worth its own BUG ticket). Sidestepping it: @load is
   the OTHER proven on-visible primitive in stdlib (threshold-based
   IntersectionObserver, no text-splitting), animating the whole block as one
   unit via plain keyframes — still zero authored JavaScript, still fully
   declarative, just not the kinetic-per-line variant. */
.eyebrow {
  margin: 0;
  font-size: 0.72rem;
  letter-spacing: 0.28em;
  text-transform: uppercase;
  color: #c9a86a;
  @on &.load(name: in, duration: 500ms, delay: 0ms, threshold: 0.4) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        opacity: 0 -> 1;
        translate-y: 12px -> 0;
      }
}

.headline {
  margin: 0;
  color: #f8f3e6;
  font-size: clamp(2rem, 5.5vw, 3.6rem);
  font-weight: 800;
  line-height: 1.06;
  letter-spacing: -0.02em;
  text-shadow: 0 4px 40px rgba(10, 8, 4, 0.6);
  @on &.load(name: in, duration: 700ms, delay: 80ms, threshold: 0.4) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        opacity: 0 -> 1;
        translate-y: 20px -> 0;
      }
}

.lede {
  margin: 0;
  font-size: clamp(0.95rem, 1.5vw, 1.08rem);
  line-height: 1.6;
  color: #d8cdb6;
  @on &.load(name: in, duration: 600ms, delay: 160ms, threshold: 0.4) {
        easing: ease-out;
        stagger: 0;
        staggerFrom: first;
        opacity: 0 -> 1;
        translate-y: 16px -> 0;
      }
}

.lede code {
  font-family: ui-monospace, "SF Mono", Menlo, monospace;
  font-size: 0.88em;
  color: #f0c987;
  background: rgba(240, 201, 135, 0.1);
  padding: 0.1em 0.4em;
  border-radius: 3px;
}

.hint {
  position: fixed;
  inset: auto auto clamp(1.5rem, 4vw, 2.5rem) 0;
  width: 100%;
  z-index: 1;
  margin: 0;
  text-align: center;
  font-size: 0.78rem;
  letter-spacing: 0.06em;
  color: #7d7461;
  pointer-events: none;
}
