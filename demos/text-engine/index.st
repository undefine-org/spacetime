@import "stdlib/text"

/* ============================================================================
   Text Engine — the fusion flagship

   Multiple stdlib engines on one page, each shipped ONLY because it is used:
     • pretext (text module): @balance + @measure on the headline, @reveal lines
     • scene (webgl): an ambient glow backdrop
   The emitted bundle therefore contains core + text + scene — and NOT the
   engines this page doesn't touch. Zero hand-written JavaScript.
   ============================================================================ */

body {
  margin: 0;
  min-height: 100vh;
  background: #050510;
  color: #eef1fb;
  font-family: "Inter", system-ui, -apple-system, sans-serif;
  -webkit-font-smoothing: antialiased;
  overflow: hidden;
}

/* Ambient CSS backdrop. The old WebGL @scene engine was retired in PLAN-050;
   live 3D now lives in the stdlib/3d module — see the demos/3d showcases. */
canvas.bg {
  position: fixed;
  inset: 0;
  z-index: 0;
  background:
    radial-gradient(circle at 32% 26%, rgba(74, 0, 128, 0.34), transparent 56%),
    radial-gradient(circle at 68% 72%, rgba(0, 64, 122, 0.26), transparent 56%),
    #050510;
}

.stage {
  position: relative;
  z-index: 1;
  min-height: 100vh;
  box-sizing: border-box;
  width: min(880px, 92vw);
  margin: 0 auto;
  display: grid;
  align-content: center;
  gap: 1.5rem;
  padding: 4rem 1.5rem;
}

.eyebrow {
  margin: 0;
  font-size: 0.7rem;
  letter-spacing: 0.2em;
  text-transform: uppercase;
  color: #8a93ad;
}

/* pretext balances the ragged lines AND reveals them line-by-line,
   AND exposes $lines for the readout below */
.title {
  margin: 0;
  font-size: clamp(2rem, 5.5vw, 3.4rem);
  font-weight: 800;
  line-height: 1.16;
  letter-spacing: -0.015em;
  @balance(lineHeight: 1.16)
  @reveal(split: "lines", trigger: visible, stagger: 90, duration: 800, from: "bottom")
}

.lede {
  margin: 0;
  max-width: 52ch;
  font-size: 1rem;
  line-height: 1.6;
  color: #aeb6cf;
}

.readout {
  margin: 0;
  font-size: 0.85rem;
  color: #6f7891;
}

/* @measure on the lede exposes its $lines for the live readout (distinct
   element from the headline, so no signal collision) */
.lede {
  @measure(lineHeight: 1.6)
}

/* reactive binding: the measured line count repaints this span */
.line-count {
  text: $lines;
  color: #00ffcc;
  font-variant-numeric: tabular-nums;
}
