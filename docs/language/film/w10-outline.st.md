# W10 — text as geometry (`@reveal(split: outline)`)

*Working doc for PLAN-150 W10. Builds and runs today. The "text as geometry" of
`../film.st.md` §11.*

```st hidden
@import "stdlib/md"
```

## The word

`@reveal` gains an `outline` split. The text stays text (an sr-only twin keeps
it selectable and accessible); the reveal adds an SVG `<text>` twin rendered as
strokable geometry, and draws it on:

```st src
h1.mark {
  font: 800 80px sans-serif;
  @reveal(split: "outline", trigger: "load", duration: 1200) {
    draw: 0 -> 1; fill: 0 -> 1;
  }
}
```

`draw` is the stroke travelling along the letterforms (0 → 1 draws the outline
on); `fill` fades the real fill in over the outline. With no body an outline
reveal defaults to a full draw + fill.

## How it lowers

The outline split builds an SVG twin: a `<text>` in the element's own font,
`stroke` set to the text colour, `fill-opacity` starting at 0. `draw` animates
`stroke-dashoffset` from the glyph run's dash length down to 0 (the stroke
draws); `fill` animates `fill-opacity` up. The animation is driven by a rAF loop
setting the inline style each frame — WAAPI on an SVG element's
`stroke-dashoffset` does not reliably reflect through `getComputedStyle`, so the
inline-style path is the deterministic one. No font parsing: an SVG `<text>`
element **is** the glyph geometry, natively strokable.

## What it composes with

Because outlines are geometry, `@scatter(from: &mark)` (W5) can sample them —
text that shatters into its own letters and text that draws itself are the same
geometry seen two ways.

## Not yet (later in W10's arc)

- Per-glyph draw with stagger (`by: chars` splitting the outline into per-letter
  paths, each with its own `draw`).
- `easing:` on the draw (today linear).
- True font-to-path (`opentype`-style) for `@scatter` sampling without the SVG
  twin.
