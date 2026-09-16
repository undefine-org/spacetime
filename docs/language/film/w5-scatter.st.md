# W5 — scatter: positions from a shape

*Working doc for PLAN-150 W5. Builds and runs today. The "positions from a
shape" of `../film.st.md` §6.*

```st hidden
@import "stdlib/md"
```

## The problem it solves

`<script>` should shatter into its *own letters*. CSS trigonometry over
`sibling-index()` gives a beautiful generic burst — but it cannot know where the
glyphs of a word actually are. `@scatter` samples a source's rendered ink and
stamps those positions onto its children.

## The word

```st src
.burst {
  position: relative;
  @scatter(from: &tag, count: 400) {
    :shape { --x: sample-x; --y: sample-y; }   // sampled from the glyphs of &tag
    :rest  { --rx: sample-x; --ry: sample-y; } // a start arrangement (optional)
  }
  @on &.clip { --tau: 0 -> 1; }
}
.tag { font: 800 120px sans-serif; }
.burst i {
  position: absolute;
  translate: calc(var(--x) + cos(sibling-index() * 137.5deg) * 400px * var(--tau))
             calc(var(--y) + 620px * var(--tau) * var(--tau));
}
```

`@scatter` stamps custom properties **only** — motion stays in `@on`, and the
per-particle geometry stays CSS. Each slot MAPS a property you name to a sample
axis: `--x: sample-x` means "stamp the sampled point's x into `--x`". `:shape` is
where the particles live; the optional `:rest` is a second arrangement (a start
pose your `--tau` animates away from).

## How it lowers

At hydration the source's text is drawn to an offscreen canvas and `count`
points are chosen by rejection sampling over the alpha channel — denser strokes
get more points, so the scatter follows the actual ink of the letters, not a
bounding box. The sampler is deterministic (a PRNG seeded from the source text),
so a rebuild samples the same points and a film renders bit-identically.

## One sigil, one meaning

The slot writes `--x: sample-x`, not a bare `--x;`. That is deliberate: a bare
`--x;` is the form-splice spelling (`--role;`), and `:` is what introduces a
VALUE everywhere in the language. `--x: sample-x` reads exactly as it means —
"this property gets this sample" — and stays inside the one grammar you already
know.

## What this replaces

The studio cut's `getImageData` glyph sampler — the one step in that pipeline
that had no Spacetime spelling at all. Under `@stage`,
`@particles(shape: &tag)` will be the same idea on the GPU (later in the arc).

## Not yet (later in W5's arc)

- Shape forms as slots: `:shape --ring(radius: 200px)`, `:shape --grid(12, 8)`.
- The GPU path (`@particles(shape: &tag)` inside `@stage`).
- Re-sampling on resize (today it samples once at hydration).
