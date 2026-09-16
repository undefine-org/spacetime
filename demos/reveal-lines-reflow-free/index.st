@import "stdlib/text"

/* ============================================================================
   Reveal Lines, Reflow-Free — @reveal split:"lines" fused onto pretext

   The core @reveal primitive splits a headline into visual lines and reveals
   them with a staggered mask animation. With stdlib/text imported, the line
   grouping comes from pretext (arithmetic, reflow-free) instead of reading
   span.offsetTop per word. Progressive enhancement: without the text module the
   same macro still works via the offsetTop fallback. Zero custom JS.
   ============================================================================ */

body {
  margin: 0;
  min-height: 100vh;
  display: grid;
  place-items: center;
  padding: 4rem 1.5rem;
  box-sizing: border-box;
  background: radial-gradient(120% 120% at 50% 0%, #0f1320 0%, #060709 60%);
  color: #f4f6fd;
  font-family: "Inter", system-ui, -apple-system, sans-serif;
  -webkit-font-smoothing: antialiased;
}

.stage {
  width: min(820px, 92vw);
  display: grid;
  gap: 1.75rem;
}

.eyebrow {
  margin: 0;
  font-size: 0.7rem;
  letter-spacing: 0.18em;
  text-transform: uppercase;
  color: #7e88a4;
}

.kinetic {
  margin: 0;
  font-size: clamp(1.8rem, 5vw, 3rem);
  font-weight: 700;
  line-height: 1.22;
  letter-spacing: -0.01em;
  @reveal(split: "lines", trigger: visible, stagger: 90, duration: 800, from: "bottom")
}

.meta {
  margin: 0;
  font-size: 0.85rem;
  line-height: 1.6;
  color: #6e7894;
}

.meta code {
  font-family: "JetBrains Mono", ui-monospace, monospace;
  font-size: 0.8em;
  color: #9aa4c2;
}
