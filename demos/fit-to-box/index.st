@import "stdlib/text"

/* ============================================================================
   Fit to Box — @fit (pretext shrink-to-fit)

   Each box has a draggable right edge. The headline inside picks the largest
   font-size that fits one line at the box's current width — pure pretext
   arithmetic, no overflow flash, no DOM reflow. Zero custom JS.
   ============================================================================ */

body {
  margin: 0;
  min-height: 100vh;
  display: grid;
  place-items: center;
  padding: 3rem 1.5rem;
  box-sizing: border-box;
  background: radial-gradient(120% 120% at 50% 0%, #11151d 0%, #07080b 60%);
  color: #f2f5fc;
  font-family: "Inter", system-ui, -apple-system, sans-serif;
  -webkit-font-smoothing: antialiased;
}

.stage {
  width: min(720px, 94vw);
  display: grid;
  gap: 1.5rem;
}

.eyebrow {
  margin: 0;
  font-size: 0.7rem;
  letter-spacing: 0.18em;
  text-transform: uppercase;
  color: #7c87a2;
}

.box {
  resize: horizontal;
  overflow: hidden;
  width: 360px;
  min-width: 120px;
  max-width: 100%;
  padding: 1.5rem;
  border-radius: 16px;
  background: linear-gradient(180deg, rgba(255,255,255,0.06), rgba(255,255,255,0.02));
  border: 1px solid rgba(255,255,255,0.08);
  box-shadow: 0 24px 64px rgba(0,0,0,0.4);
}

.box--wide {
  width: 560px;
}

.fit-line {
  margin: 0;
  font-weight: 800;
  letter-spacing: -0.01em;
  white-space: nowrap;
  /* @fit chooses the largest size in [min, max] that fits one line */
  @fit(min: 18, max: 160, maxLines: 1, lineHeight: 1.05)
}

.meta {
  margin: 0;
  font-size: 0.8rem;
  color: #6b7591;
}
