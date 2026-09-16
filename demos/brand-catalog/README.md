# Aurora — Brand Form Catalog

The living proof of the brand-forms arc (PLAN-135): a design system where
**the brand is a set of `@form` declarations**, declared once, merged into
every page automatically, and catalogued by the compiler itself.

```
cargo run -- serve demos/brand-catalog/
```

## The three pieces

- **`_prelude.st` — the brand.** Eight `@form` declarations across all six
  kinds (style, motion, easing, score, value, markup). The project overlay
  merges them into every page's compile (W1) — pages splice `--card-surface`,
  ease with `--aurora-glide`, and score `--duet` without importing anything.
  The `///` block above each declaration is its catalog copy.

- **`index.st` — the auto-generated catalog.** `@data forms` (W4) enumerates
  every declaration the build can see — page, imports, the prelude, and the
  stdlib preset library — and `@each` unrolls rows into cards at build time.
  **Add a form to `_prelude.st` and its card appears next build**; the page
  never names a form by hand. Each kind section pairs the cards with a live
  stage consuming the same declarations: style splices expand into CSS (W2),
  the easing curve drives a scrub dot against a linear twin (W3), the score
  form choreographs a pair on scroll.

- **`guidelines.st.md` — the prose.** A literate Spacetime document: prose
  is prose, and the ` ```st ` fences run where they sit against the same
  prelude declarations. The sidebar mixes both surfaces — a docs site that
  is half written, half generated.

## What to try

1. Edit `_prelude.st` — retune `--aurora-glide` or add a form — and watch
   the dev server rebuild: every page, stage, and card updates together.
2. Scroll the easing section: the purple dot rides the brand curve, the
   gray one is linear. The gap between them IS the curve.
3. The score stage: `--duet` gives each subject half the scroll window —
   first rises, then second, from one declarative line.

## Gate

`cargo test --test demo_brand_catalog_test` builds the demo and asserts the
wiring: cards generated from data (all eight brand forms + the stdlib
library), splices expanded in CSS (param substitution included), the brand
curve registered into the runtime, and the literate page's fences executed.
