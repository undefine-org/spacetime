# Review rubric — interpreting `Spacetime.review()`

`Spacetime.review()` returns a 7-dimension audit. This document explains what
each dimension means, how to read scores, and when it is OK to defer a finding.
Two of the seven dims (`hierarchy` and `detail`) ship as runtime walkers via
FEAT-032; two more (`philosophy-coherence` and `innovation`) are recorded
here as **doc-only dims** for human review and have no runtime walker.

## Output shape

```js
{
  score: 0..10,                         // top-level = min(dim scores)
  dims: {
    idiomatic: 0..10,
    animation: 0..10,
    brand:     0..10,
    a11y:      0..10,
    compile:   0..10,
    hierarchy: 0..10,
    detail:    0..10
  },
  findings: [
    { dim, severity: 'error'|'warn'|'info', message, selector?, value? }
  ],
  generatedAt: number
}
```

## Per-dim score formula

```
score = clamp(10 - errors*2 - warns, 0, 10)
```

Errors weigh 2x warnings; infos do not affect score.

## Dimension semantics

### `idiomatic`

Detects: inline event handlers (`onclick=` ...), `<script>` blocks outside
`/__spacetime/`, and inline scripts not marked Spacetime-emitted.

Pass criterion: zero of the above. Each one is a hard `error`.

### `animation`

Walks `window.__ST_DEBUG__.getTimelines()`.

- `error`: `elementCount === 0` (timeline matched no DOM).
- `error`: `state.destroyed === true && state.completed === false`.
- `warn`: `state.started === false` after `document.readyState === 'complete'`.

### `brand`

Counts hex / `rgb()` / `hsl()` literals in inline `style=` attributes and
`<style>` blocks NOT wrapped in `var(--*)`. Custom-property declarations
(lines containing `--`) are ignored — those are tokens, the good case.

Each unique unbound literal produces one `warn` row sorted by frequency.

### `a11y`

- `<img>` missing `alt`: `error`
- `<a>` with no accessible name (text/aria-label/title): `error`
- `<button>` with no label: `error`
- Heading skipped (h1 → h3 jump): also surfaced under `hierarchy`.

### `compile`

Drains `window.__ST_DEBUG__.getErrors()` (or `__ST_DEBUG__.errors`). Each
captured error becomes one `error` finding.

### `hierarchy`

Heading order + font-size monotonicity (FEAT-032).

- `warn`: heading level skip in document order (h1 → h3, h2 → h5).
- `warn`: a deeper level renders at a font-size ≥ a shallower level's max
  computed font-size (e.g. h2 ≥ 32px when h1 max was 32px). Font-size
  read from `window.getComputedStyle(el).fontSize`.

Single hN documents are silent (the walker only fires on relations between
multiple headings).

### `detail`

Three execution-quality checks (FEAT-032). All findings except text-wrap
are warns; text-wrap is `info` (cosmetic).

- `info`: display-class heading (h1/h2 with text > 12 chars and
  computed font-size > 24px) missing `text-wrap: pretty` or
  `text-wrap: balance`. Avoids dangling first lines.
- `warn`: `--brand-*` custom property on `:root` declared as
  `#hex` / `rgb(...)` / `hsl(...)` without an `oklch(...)` value. Prefer
  oklch for perceptually uniform brand tokens.
- `warn`: heading element (h1/h2/h3) carrying an inline `style="..."` with
  `font-family` set to a literal string instead of `var(--brand-display)`.

## Doc-only dims

These are NOT runtime-walkable; they exist for human review only and do
not appear in `Spacetime.review().dims`. They show up as guidance in the
fallback-advisor flow and the junior-designer review pass.

### `philosophy-coherence`

Does the page commit to *one* of the 5 design schools (see
[design-styles.md](design-styles.md))? A page that mixes a
motion-poetics hero with an information-architecture features grid feels
incoherent even when each section reviews cleanly in isolation.

Human evaluation only.

### `innovation`

Does the page propose anything the corpus mean wouldn't have produced?
Anti-targets: every visual rule from
[anti-patterns.md](anti-patterns.md) §"Visual anti-patterns" (purple
gradient, emoji icons, Inter display, GitHub dark, etc).

Human evaluation only.

## Score thresholds

| Score | Meaning                                                                        |
|-------|--------------------------------------------------------------------------------|
| 10    | Clean. No findings.                                                            |
| 8-9   | Acceptable. Cosmetic warns only.                                               |
| 6-7   | Doctor pass threshold. One or two warns or a minor error to address.           |
| 0-5   | Block delivery until findings are resolved or explicitly deferred to a FUP.    |

`cargo run -- doctor` defaults to fail-on-`score < 6` across any dim on any
route (extends from 5 dims to 7 with FEAT-032; same threshold semantics).

## When to defer a finding

A finding may be deferred (left in place, no fix) if and only if all three
hold:

1. The user explicitly accepts the trade-off and has been informed in plain
   language (not "warning silenced").
2. A FUP item exists (`org create -c followups`) referencing the finding.
3. The skill emits a doctor report row pointing at the FUP.

"It's just a warning" is not a deferral reason. Either the warning is real
(fix it) or it is a false positive (file a FUP to refine the walker).

## Cache behavior

The walker caches its report for 250ms to keep the dev panel poll cheap. Rapid
repeated calls return the same object. Pass through `__stDevReview()` directly
if you need to bypass the cache (you almost never do).
