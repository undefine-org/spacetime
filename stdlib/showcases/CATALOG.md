# Showcase catalog

Curated, self-contained patterns indexed by scenario × style × school.

The plan calls for 24 showcases (8 scenarios × 3 styles); after FEAT-030 added
3 representative hero anchors for the schools missing from the original
minimal/editorial/maximal triad, 10 of 24 patterns are shipped.

## Brand as forms (PLAN-135 W6)

Every pattern's brand — colors, type, radius, motion — is a SPLICE of the
`@form` declarations in [_schools.st](./_schools.st), one set per design
school (`--sc-min-*`, `--sc-max-*`, `--sc-arch-*`, `--sc-mtn-*`, `--sc-east-*`)
plus shared choreography (`--sc-enter-glide`, `--sc-enter($distance)`,
`--sc-mtn-hero`). A pattern file keeps only its layout. Because the forms are
declarations, they are data: `@data forms` catalogs them, and a project can
splice the same chunks to extend a pattern without forking it. The scroll
sequences are score forms — `hero/motion.st`'s "scroll-driven kinetic" claim
is now literally true (browser-verified against the fix in BUG-312).

A `hero/media.st` playback hero (overlay choreographed by the video's own
playhead) exists but is deliberately NOT imported: the element-ref rail it
needs is unimplemented (BUG-310; BUG-311 blocks seek-testing). The file is the
bug's acceptance fixture.

| Scenario       | Style     | School                   | One-liner                                                                            | Source                                                  |
|----------------|-----------|--------------------------|--------------------------------------------------------------------------------------|---------------------------------------------------------|
| hero           | minimal   | minimalism               | Centered headline, subhead, single primary CTA on a calm background.                 | [hero/minimal.st](./hero/minimal.st)                     |
| hero           | editorial | minimalism               | Magazine-style hero with eyebrow kicker, large serif headline, dual CTAs.            | [hero/editorial.st](./hero/editorial.st)                 |
| hero           | maximal   | experimental             | Bold gradient hero with oversized type, decorative grid, high contrast CTA.          | [hero/maximal.st](./hero/maximal.st)                     |
| hero           | editorial | information-architecture | Twelve-column hero with single headline + three metric callouts; data-forward.       | [hero/architecture.st](./hero/architecture.st)           |
| hero           | editorial | motion-poetics           | Scroll-driven kinetic hero with technical-aesthetic readouts on a dark canvas.       | [hero/motion.st](./hero/motion.st)                       |
| hero           | editorial | eastern-philosophy       | Asymmetric warm hero with serif display, generous "ma" negative space.               | [hero/eastern.st](./hero/eastern.st)                     |
| nav            | minimal   | minimalism               | Single horizontal row with logo, three text links, one CTA pill.                     | [nav/minimal.st](./nav/minimal.st)                       |
| cta            | minimal   | minimalism               | One-row CTA banner with one bold headline and a primary action button.               | [cta/minimal.st](./cta/minimal.st)                       |
| footer         | minimal   | minimalism               | Quiet single-row footer with brand, copyright, three secondary links.                | [footer/minimal.st](./footer/minimal.st)                 |
| features-grid  | minimal   | minimalism               | Three-column responsive grid; title plus paragraph per cell, no icons.               | [features-grid/minimal.st](./features-grid/minimal.st)   |

## By school

| School                   | Anchors shipped                                                                          |
|--------------------------|------------------------------------------------------------------------------------------|
| information-architecture | `hero/architecture.st`                                                                   |
| motion-poetics           | `hero/motion.st`                                                                         |
| minimalism               | `hero/minimal.st`, `hero/editorial.st`, `nav/minimal.st`, `cta/minimal.st`, `footer/minimal.st`, `features-grid/minimal.st` |
| experimental             | `hero/maximal.st`                                                                        |
| eastern-philosophy       | `hero/eastern.st`                                                                        |

## Status

10 of 24 patterns shipped across 5 design schools. The remaining 14
combinations are tracked under
[FUP-013](../../!tasks/follow-ups/FUP-013-feat-018-remaining-17-showcases-8-scenar.org).

Missing combinations (post-FEAT-030):

- **hero**: still missing dedicated school anchors at non-`hero` scenarios.
- **nav**: editorial, maximal
- **features-grid**: editorial, maximal
- **cta**: editorial, maximal
- **footer**: editorial, maximal
- **gallery**: minimal, editorial, maximal
- **form**: minimal, editorial, maximal
- **reveal-on-scroll**: minimal, editorial, maximal

## How to add a new showcase

1. Read `SCHEMA.md` for the 7-field doc-comment contract (now includes
   `/// school: <one of 5>`).
2. Create `stdlib/showcases/<scenario>/<style>.st`.
3. Add `@import "./<scenario>/<style>.st"` to `index.st` (alphabetical within
   each scenario).
4. Add a row to the table above and update the "By school" mapping.
5. Verify: `cargo run -- check stdlib/showcases/index.st`.
6. Look at it: `cargo run -- serve demos/showcase-gallery/`. The specimen list
   is generated from the tree, so the new pattern appears with no page edit —
   add its authored copy at the bottom of that page to fill its frame.
