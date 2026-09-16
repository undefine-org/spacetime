# W2 — `@post`, a cinematic lens over any element

*Working doc for PLAN-150 W2. Builds and runs today. The film-surface
post-processing of the golden reference (`../film.st.md` §3), DOM half.*

```st hidden
@import "stdlib/md"
```

## The look

A film has a lens: a glow around bright things (bloom), film grain, an edge
vignette. `@post { … }` stacks those layers over any element:

```st src
.stage {
  width: 100vw; height: 100vh; position: relative;
  @post { bloom: 0.45; grain: 0.035; vignette: 0.55; }
}
```

The keys set the initial look: `bloom` → `--st-post-bloom`, `grain` →
`--st-post-grain`, `vignette` → `--st-post-vignette`.

## Animating it

The layers only ever read their custom property — they never bake a value. So
animating the lens is animating the property, with the ordinary keyframe engine
(positioned stops and all of W1 apply):

```st src
.film {
  @score &.time(30s) as &reel { &stage at 7s for 6s; }
}
.stage {
  position: relative;
  @post { bloom: 0.45; }
  // bloom surges on the hit at 61% of the shot, then settles
  @on &.clip { --st-post-bloom: 0.45 -> 1.3 at 61% -> 0.45; }
}
```

Because the score lives on a parent and the clip consumer on the child it drives
(the standard pairing), the surge lands exactly where the shot wants it.

## The layers

| key | what it does | how, on DOM |
|---|---|---|
| `bloom` | glow around bright pixels | a blurred, screen-blended radial wash whose strength follows the property |
| `vignette` | edge darkening | a radial mask over the element |
| `grain` | film grain | an SVG fractal-noise tile, screen-blended; jitter it with `--st-post-grain-seed` |

**Fidelity, plainly:** DOM bloom is a blurred, screen-blended layer clipped to
the element's own box — it glows, but cannot bleed *outside* the box the way a
real shader pass can. It is the same INTENT as the `@stage` bloom pass (which
IS a real pass), and under `spacetime render` both are captured by the same
screenshot. Same word, two lowerings.

## What this replaces

The promo's hand-built `.grain` / `.vignette` overlay divs and its `.tagr` /
`.tagb` colour-shifted clones — a lens is one directive now.

## Not yet (later in W2's arc)

- `aberration` and `glitch` layers.
- `@post` INSIDE an `@on` body (`@on &.clip { @post { bloom: 0 -> 1 } }`) as
  sugar for the custom-property animation above.
- Unifying the `@stage` pass params under these same key names.
