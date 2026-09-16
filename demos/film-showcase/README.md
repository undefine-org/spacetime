# Film Showcase

A 24-second film built on the **landed** Spacetime film surface (PLAN-150), in
one page, with **zero JavaScript**. It is art-directed to stand with the Three.js
"studio cut" (`demos/promo/studio-cut/`) — a warm bloom orb, edge-bleeding
display type, a perspective grid for depth, a gold accent, and a film-grain lens
over everything — while every pixel is declarative Spacetime.

```
cargo run --features cdp -- render demos/film-showcase/ --out film.mp4 --width 1280 --height 720 --fps 30
cargo run --features cdp -- render demos/film-showcase/ --stills 2.5,7.5,12,16.5,21 --out scratch/stills/   # fast preview
cargo run -- serve demos/film-showcase/                                                                     # watch it live
```

## Five beats on one score

The whole film is ONE `@score &.time(24s)`; every scene is a clip window, every
motion an `@on &.clip` body.

1. **SPACETIME** — a point of light. A warm bloom orb blooms behind the wordmark;
   dust motes are placed with `random()` (W6); the title focus-pulls in (blur +
   letter-spacing keyframes).
2. **ONE SOURCE** — the world gets depth. A CSS-perspective grid floor reveals
   with a radial mask; a glowing time-axis sweeps through it.
3. **A component with a timeline** — the product card is a `@form shot` (W9):
   markup + motion in one form, presented in perspective. Its price (`$24.99`)
   exercises the BUG-384 fix (a param value that begins with `$`).
4. **MOTION** — the word shatters into its own letters (`@scatter`, W5): 40
   sampled points fly out on a golden-angle spiral driven by one `--tau`.
5. **NO JAVASCRIPT** — the wordmark shimmers home (a gradient clipped to text,
   `--pos` sweeping the highlight) over a warm dust field.

Over all five: a **lens** — film grain, a vignette, and open/close fades — the
same overlay stack the studio cut approximates with a bloom pass.

The whole thing is `index.st`. Read it top to bottom; it is the film.

## Rendering

The film renders under Chromium's virtual clock (`spacetime render`), so it is
bit-identical run to run. `--stills` grabs specific instants in seconds; `--dpr
2` renders at 4K; `--from`/`--to` render one scene.
