```st hidden
@import "./snippets/karaoke-pill.st.md"
@import "./snippets/weight-shift.st.md"
@import "./snippets/highlight-wipe.st.md"
@import "./snippets/gradient-fill.st.md"
@import "./snippets/kinetic-slam.st.md"
@import "./snippets/glitch-rgb.st.md"
@import "./snippets/clean-bar.st.md"
@import "./snippets/kicker-name.st.md"
@import "./snippets/mask-reveal.st.md"
@import "./snippets/side-rule.st.md"
@import "./snippets/shimmer-sweep.st.md"
@import "./snippets/neon-pulse.st.md"
@import "./snippets/pixelate-wipe.st.md"
@import "./snippets/news-ticker.st.md"
@import "./snippets/morph-text.st.md"
@import "./snippets/layered-vignette.st.md"

/* Gallery chrome: dark reading surface, centered measure, framed stages.
   Snippets style their own internals with literal colors so each file is
   copy-pasteable on its own. */
body { background: #0a0d12; color: #c9d4e2; font-family: system-ui, -apple-system, "Segoe UI", sans-serif; line-height: 1.6; }
p, h1, h2, h3, pre { max-width: 62rem; margin-left: auto; margin-right: auto; }
h1 { font-size: 2.4rem; color: #e9eff7; letter-spacing: -0.02em; margin-top: 3rem; }
h1 .sub { display: block; font-size: 1.05rem; font-weight: 500; color: #6b7a90; letter-spacing: 0.14em; text-transform: uppercase; margin-bottom: 0.6rem; }
h2 { color: #e9eff7; margin-top: 2.6rem; }
code { background: #121926; padding: 0.15em 0.4em; border-radius: 4px; font-size: 0.92em; }
pre { background: #0d1117; border: 1px solid #1c2433; border-radius: 12px; padding: 1.1rem 1.3rem; overflow-x: auto; font-size: 0.85rem; line-height: 1.55; }
pre code { background: none; padding: 0; }

/* Each snippet wraps its live demo in <section class="gx g-name">. */
.gx {
  max-width: 62rem; margin: 1.4rem auto 3rem; padding: 3rem 2rem;
  background: #0d1117; border: 1px solid #1c2433; border-radius: 16px;
  display: grid; place-items: center; min-height: 180px; overflow: hidden;
}
```

# The effect gallery <span class="sub">HyperFrames, redone in Spacetime</span>

Every entry below redoes one block from the
[HyperFrames registry](https://hyperframes.ai) — a catalog of 117 React/Remotion
motion pieces (captions, lower thirds, wipes, texture tricks) — as a
**self-contained literate snippet**: the prose you are reading, the full source,
and the live result all live in one file under `snippets/`. This page only
`@import`s the snippets; edit one and rebuild, and the gallery cannot drift.

Everything you see is one `@score` placement plus one keyframe body. No
JavaScript — not yours, not a library's. For the long-form version (a 12s title
sequence rendered to mp4 with `spacetime render`), see
[demos/score-launch](../score-launch/).
