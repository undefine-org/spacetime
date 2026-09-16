# promo — a 30-second film, in Spacetime

`index.st` is a seven-scene promo for Spacetime, authored entirely in Spacetime:
one `@score &.time(30s)` and seventeen `@on &.clip` bodies. No author
JavaScript; the served page carries only the compiler's runtime.

```
cargo run -- serve demos/promo/                                              # play it in a browser
cargo run --features cdp -- render demos/promo/ --out scratch/promo-st/video.mp4   # 1800 frames, 30.000 s
ffmpeg -i scratch/promo-st/video.mp4 -i scratch/promo-audio/promo.wav -c:v copy -c:a aac -shortest film.mp4
```

The film is a fair-comparison twin of a Three.js/WebGL "studio" cut built from
the same storyboard and sound design. **[CRITIQUE.md](CRITIQUE.md)** compares
the two frame by frame and lists the language gaps the exercise surfaced.

## What each scene demonstrates

| Scene | Mechanism |
|---|---|
| S1 point of light | multi-stop keyframes on `scale`/`opacity`/`blur`/`letter-spacing`; stars and dust are `<i>`s placed by `mod(sibling-index() · k, 100)%` and moved by one `--t` |
| S2 grid + time axis | `perspective` + `rotate-x` floor; reveal is a `mask-image` reading `--reveal`; the axis is a screen-space glow on `translate-x`; camera shake is a 7-stop `translate` chain under `range:` |
| S3 code types itself | per-line `clip-path: inset(… (var(--chars) − var(--o)) · 1ch …)`; caret blink is `sign(sin(var(--chars)))`; the card's reveal (`opacity 0 -> 1; translate-y 40px -> 0; --ease-out-expo`) IS the snippet on screen |
| S4 every clock | four panels reveal the same card on different `range:` windows; convergence is four `translate-x` keyframes; the waveform is `sibling-index()`-sized bars |
| S5 render it | 16-frame filmstrip on a `rotate-y` strip; the frame counter is a CSS counter reset from the animated `--fr` |
| S6 no JavaScript | 140 particles whose trajectory is CSS trig over `sibling-index()` and one `--tau`; RGB split is two blend-mode clones |
| S7 wordmark | `background-clip: text` shimmer swept by `--pos` |
| hits | six 0.5 s clips (`&h1 at 2.08s for 0.5s …`) sharing one `--flash` motion form, placed 0.12 s early so the peak stop lands on the sound |

## Gates

- `tests/score/promo-film.test.st` (`--cdp`): frame 0 puts each child's
  initial value on the child (BUG-382), scenes own their windows, and the
  CSS-trig particle system fans out on one property.
- `src/pipeline/expand.rs::tests::test_initial_state_css_keyframes_capture_keeps_nested_scope`
  (unit) — the compiler half of BUG-382.
- `tests/render/render_test.rs` — the render verb's own determinism gates
  cover this page like any other.
