# W11 — tooling: stills, ranges, resolution

*Working doc for PLAN-150 W11. Ships today. The render tooling of
`../film.st.md` §13.*

```st hidden
@import "stdlib/md"
```

## The problem it solves

Iterating a film by rendering it end to end is how a 20-minute job becomes a
day. `spacetime render` is the film's build; it gets the flags a build has.

## The flags

```
spacetime render demos/promo/ --stills 1.5,9,24 --out scratch/stills/    # three PNGs, ~5s
spacetime render demos/promo/ --from 18 --to 23 --out scratch/s5.mp4      # one scene
spacetime render demos/promo/ --dpr 2 --out scratch/film-4k.mp4          # 3840×2160
```

- **`--stills <csv>`** — capture specific timestamps (seconds) as PNGs, no video
  mux. The fast loop: three frames of a 30 s film in seconds instead of
  rendering the whole thing. Writes `still-<ms>.png` beside `--out`. Each still
  is the page at that exact virtual instant (the same deterministic clock the
  video render uses), so what you grab is what the film shows.
- **`--from <s>` / `--to <s>`** — render only a time window. A `[from, to]`
  window fast-forwards the virtual clock to the window head, then renders just
  that span — one scene, not the whole reel.
- **`--dpr <n>`** — device pixel ratio. Frames render at `width·dpr × height·dpr`,
  so `--dpr 2` gives a 4K render of a 1080p film. Every capture path (video and
  stills) honours it.

## How it lowers

All three ride the existing render engine (`src/render.rs`): one Chromium page
under the runtime's virtual clock (`ST._clock`), advanced deterministically.
`--stills` advances to each requested time and screenshots to a PNG (no ffmpeg);
`--from/--to` bound the frame loop's virtual-time span; `--dpr` sets the capture
device-scale. The film renders bit-identically run to run — the clock is
deterministic and the stills path shares it.

## Not yet (later in W11's arc)

- `--sheet 5x6` — a contact sheet (one image, a grid of stills).
- `serve --scrub` — a transport bar on the served page (drag to seek), driving
  the same virtual clock the render uses.
- `--out film.webm | film.gif | frames/` and the long-range `film.lottie` target.
