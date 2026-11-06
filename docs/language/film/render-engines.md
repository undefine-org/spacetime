# Render engines: Chromium and raea

*Working doc for PLAN-151. Ships today. Companion to `w11-render-tooling.st.md`
and `../film.st.md` §13.*

`spacetime render` turns a page with a `@score` into a video by driving the
runtime's virtual clock (`ST._clock`) instead of the wall clock. The page never
knows it is being rendered. Two engines can draw the frames:

```
spacetime render demos/promo/ --out film.mp4                     # Chromium (default)
spacetime render demos/promo/ --engine raea --out film.mp4       # raea: no browser
spacetime render demos/promo/ --engine raea --stills 2,9,21 --out stills/x
spacetime render demos/promo/ --engine raea --from 18 --to 23 --out s5.mp4
```

| | `chromium` | `raea` |
|---|---|---|
| draws with | headless Chromium over CDP, one screenshot per frame | Blitz/Stylo layout, Vello on the GPU, in-process encode |
| needs | the `cdp` feature, a Chrome, `ffmpeg` | the vendored `raea` binary and a Vulkan GPU |
| `--stills`, `--from/--to` | yes | yes |
| `--dpr` | yes | refused (render at the size you want) |
| fonts | what the machine and the page's links provide | raea fetches the families the page names |

## How the raea engine lowers

raea renders an *HTML composition*: a page whose root declares its size, frame
rate and length as data, and whose timelines are registered on
`window.__timelines` with `totalDuration()` and `totalTime(t)`. raea seeks each
frame's time, then lays out and paints the page.

`src/render/raea.rs` makes the compiled page one of those, without touching it:

1. The `<body>` becomes the root composition (`data-composition-id`,
   `data-width`, `data-height`, `data-fps`, `data-duration`).
2. A bridge script after the runtime installs `ST._clock` exactly as the
   Chromium backend does and registers it as the composition's one timeline:
   `totalTime(t)` advances the clock to `t`. A `--from` window offsets it.
3. A browser's white canvas is declared before the page's own CSS (raea
   composites over black), so a page without a background looks the same on
   both engines.
4. The document is staged in a temp dir beside symlinks to the site's files, so
   root-relative assets resolve as they do under `serve`.

Both engines tick the same virtual instants, so a frame at `t` shows the same
picture: the promo's frame counter reads `698 / 1800` at 20 s on both.

## Measured (2026-11-06, Work: Intel UHD 630 via Mesa, 1280×720 @ 30 fps)

Wall clock for the whole command, same harness for both engines:

| film | Chromium | raea | |
|---|---|---|---|
| `demos/film-showcase/` (24 s, 720 frames) | 180.7 s | 41.9 s | 4.3× |
| `demos/promo/` (30 s, 900 frames) | 245.7 s | 76.5 s | 3.2× |

The showcase renders faster than real time on an integrated GPU. raea's time per
frame is mostly the page's own JavaScript (the runtime ticking drivers), run in
QuickJS on Linux; painting the promo costs about 9 ms a frame.

## What making it first party took

Rendering the two films found seven engine gaps, each fixed inside the vendored
engine with a gate that was seen failing first (`tests/render/raea_render_test.rs`,
and a Blitz-level test in raea's `systems/page/tests/vars.rs`):
markup in `innerHTML`, `<template>` content, `background-clip: text`, inline
custom properties reaching Stylo, CSS counters, HTML stills to a folder, and a
TypeScript cascade that re-ran on every inline-style write (702 → 44 ms per
promo frame). The list, with the why of each, is in
`stdlib/film/vendor/PROVENANCE.md`.

## Running the gates

The raea gates need the binary and a GPU, so they are ignored by default:

```
cargo test --test raea_render_test -- --ignored
```

Read the count: an ignored gate proves nothing.

## Not yet

- Mesa's NVK driver (GTX 1660 Ti here) fails device creation with
  FEATURE_NOT_PRESENT when Bevy requests every advertised feature, so raea runs
  on the Intel iGPU on Work.
- A 3D-perspective layer is drawn at its full content size every frame (the
  showcase's floor: 2816×1872 for a 1280×720 film).
- Drivers seed their start from the first virtual tick, so every frame sits one
  tick (16 ms) behind `t` on both engines.
- raea's audio, 3D scenes and games are not reachable from a Spacetime page yet;
  only the HTML composition path is used.
