@import "stdlib/text"

/* ============================================================================
   Balanced Headline — @balance (pretext) vs CSS text-wrap: balance

   Drag the resize handle on each card and watch the line breaks. The pretext
   side stays evenly balanced and resize-stable; the CSS side is the browser's
   best-effort. Zero custom JS — only Spacetime + the vendored pretext engine,
   shipped on demand because @balance fires.
   ============================================================================ */

body {
  margin: 0;
  min-height: 100vh;
  display: grid;
  place-items: center;
  gap: 2rem;
  padding: 3rem 1.5rem;
  box-sizing: border-box;
  background: radial-gradient(120% 120% at 50% 0%, #161a24 0%, #0a0b0f 60%);
  color: #eef1f8;
  font-family: "Inter", system-ui, -apple-system, sans-serif;
  -webkit-font-smoothing: antialiased;
}

.stage {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 2rem;
  width: min(960px, 94vw);
}

.card {
  /* resize handle — drag to test resize stability */
  resize: horizontal;
  overflow: auto;
  min-width: 240px;
  max-width: 100%;
  padding: 2rem;
  border-radius: 18px;
  background: linear-gradient(180deg, rgba(255,255,255,0.06), rgba(255,255,255,0.02));
  border: 1px solid rgba(255,255,255,0.08);
  box-shadow: 0 24px 64px rgba(0,0,0,0.4);
}

.eyebrow {
  margin: 0 0 1rem;
  font-size: 0.7rem;
  letter-spacing: 0.18em;
  text-transform: uppercase;
  color: #8a93ad;
}

.headline {
  margin: 0;
  font-size: clamp(1.6rem, 4vw, 2.4rem);
  font-weight: 700;
  line-height: 1.18;
}

/* pretext-balanced headline */
.headline--pretext {
  @balance(lineHeight: 1.18)
}

/* CSS-only balanced headline for comparison */
.headline--css {
  text-wrap: balance;
}

.meta {
  margin: 1.25rem 0 0;
  font-size: 0.8rem;
  color: #6f7891;
}
