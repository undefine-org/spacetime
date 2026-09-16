# studio-cut — the comparison baseline (NOT a Spacetime page)

This directory is the "what a studio would build" twin of `../index.st`: the
same storyboard and sound design, produced with Three.js (vendored at
`stdlib/3d/vendor/three`, resolved through the import map in `index.html`),
custom GLSL, Canvas2D text, and a hand-written CDP frame capturer. It exists
so `../CRITIQUE.md` compares like with like. It deliberately breaks the
"no custom JavaScript" rule because it is the control group, not a Spacetime
site — do not copy patterns from here into a `.st` page.

```
python3 synth.py                       # → scratch/promo-audio/promo.wav (30 s, 48 kHz)
node render.mjs --stills 1.5,9,24      # PNG stills → out/stills/
node render.mjs                        # 1800 frames → out/video.mp4, muxed → out/spacetime-promo-studio.mp4
```

`render.mjs` serves the repo root on a random port, drives Chrome (GPU on,
ANGLE/GL) over raw CDP (`cdp.mjs`, no puppeteer), calls `window.__seek(t)`
per frame and pipes PNGs to ffmpeg. Everything in `promo.js` is a pure
function of `t`, so the output is deterministic.
