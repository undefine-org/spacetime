# Showcases — pattern lookup

`stdlib/showcases/<scenario>/<style>.st`. Use these as starting points; do not
invent a new layout when an existing one fits.

## "I need X" → showcase

| Need                                                           | Showcase                                                                                  | Macro                                  |
|----------------------------------------------------------------|-------------------------------------------------------------------------------------------|----------------------------------------|
| A simple hero (calm)                                           | [`hero/minimal.st`](../../../../stdlib/showcases/hero/minimal.st)                          | `&showcase-hero-minimal`               |
| A premium / serif hero with two CTAs                           | [`hero/editorial.st`](../../../../stdlib/showcases/hero/editorial.st)                      | `&showcase-hero-editorial`             |
| A loud / launch hero with gradients                            | [`hero/maximal.st`](../../../../stdlib/showcases/hero/maximal.st)                          | `&showcase-hero-maximal`               |
| A data-forward hero (annual report, metrics)                   | [`hero/architecture.st`](../../../../stdlib/showcases/hero/architecture.st)                | `&showcase-hero-architecture`          |
| A kinetic / scroll-driven hero                                 | [`hero/motion.st`](../../../../stdlib/showcases/hero/motion.st)                            | `&showcase-hero-motion`                |
| A warm, contemplative serif hero                               | [`hero/eastern.st`](../../../../stdlib/showcases/hero/eastern.st)                          | `&showcase-hero-eastern`               |
| Top-of-page nav                                                | [`nav/minimal.st`](../../../../stdlib/showcases/nav/minimal.st)                            | `&showcase-nav-minimal`                |
| End-of-page CTA banner                                         | [`cta/minimal.st`](../../../../stdlib/showcases/cta/minimal.st)                            | `&showcase-cta-minimal`                |
| Quiet utility footer                                           | [`footer/minimal.st`](../../../../stdlib/showcases/footer/minimal.st)                      | `&showcase-footer-minimal`             |
| 3-column features grid                                         | [`features-grid/minimal.st`](../../../../stdlib/showcases/features-grid/minimal.st)        | `&showcase-features-minimal`           |

Full catalog with status (shipped + planned + deferred): [`stdlib/showcases/CATALOG.md`](../../../../stdlib/showcases/CATALOG.md).

## By school

The 5-school taxonomy (see [design-styles.md](design-styles.md)) maps to
showcase anchors:

| School                   | Anchor showcases                                                                          | Use when                                                          |
|--------------------------|--------------------------------------------------------------------------------------------|-------------------------------------------------------------------|
| information-architecture | `hero/architecture.st`                                                                     | Annual reports, data-rich landings, Pentagram-style brands         |
| motion-poetics           | `hero/motion.st`                                                                           | Product launches, conference sites, agency portfolios              |
| minimalism               | `hero/minimal.st`, `hero/editorial.st`, `nav/minimal.st`, `cta/minimal.st`, `footer/minimal.st`, `features-grid/minimal.st` | Premium products, calm authority, software for serious work        |
| experimental             | `hero/maximal.st`                                                                          | Cultural institutions, music, fashion, bold proposition pages      |
| eastern-philosophy       | `hero/eastern.st`                                                                          | Wellness, hospitality, craft, slow-food, contemplative tech        |

## How to use a showcase

1. Pick the scenario × style row above (or filter by school).
2. `@import "stdlib/showcases/<scenario>/<style>.st"` into the project's `index.st`.
3. Invoke the template inside a host selector:

```st
.page {
  &showcase-hero-minimal("Build less, ship more.", "Subhead.", "Get started");
}
```

4. Override token vars on the parent selector if you need to retheme. The
   showcase declares vars on its root selector; CSS cascade lets you override
   from outside without touching the showcase file.

## When uncertain — fallback advisor

If you cannot pick between schools for the user, recommend three from
different rows of the taxonomy and show all three. See
[fallback-advisor.md](fallback-advisor.md).
