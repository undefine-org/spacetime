# Design styles taxonomy

The fallback-advisor uses a 5-school taxonomy when recommending direction
to a user who hasn't picked one. Each school is a *family of philosophies*
with a recognizable visual signature; recommending one variant per school
gives the user real choice instead of three slightly-different minimalisms.

## Overview

| School                     | Anchor designers / studios                       | Visual signature                                             |
|----------------------------|--------------------------------------------------|--------------------------------------------------------------|
| information-architecture   | Pentagram, Massimo Vignelli, Bruno Munari, Müller-Brockmann | Grid-driven, data-forward, restrained typographic system     |
| motion-poetics             | Field.io, Active Theory, Resn, Dataveyes        | Dynamic, immersive, scroll-driven, technical-aesthetic       |
| minimalism                 | Kenya Hara, Jasper Morrison, Dieter Rams, Apple Industrial Design | Order, white space, precision, quiet authority               |
| experimental               | Stefan Sagmeister, David Carson, Karel Martens, Irma Boom | Avant-garde, generative, visual disruption, type-as-image    |
| eastern-philosophy         | Kenya Hara (lens), Issey Miyake, wabi-sabi typography, inkstone aesthetic | Warm, poetic, contemplative, asymmetric balance              |

## The 5 schools

### information-architecture

Identity: design as wayfinding. Layout is a grid that earns every cell. Type
ladders from data label → metric → annotation. Color is a secondary signal,
not a hero. The page is a dashboard even when it isn't one.

Use when:
- Annual reports, data-rich landings, pricing pages with comparison tables.
- Brand wants to read as "rigorous", "engineered", "trustworthy".
- The story is in the numbers; ornament would obscure them.

Representative philosophies:
- **Pentagram editorial** — large grid, single hero number, body-set callouts.
- **Vignelli systemic** — Helvetica or geometric sans, two weights, never three.
- **Munari educational** — diagrammatic illustration, neutral background.
- **Müller-Brockmann tabular** — rule lines, axis labels, no decoration.

### motion-poetics

Identity: the page is performed, not displayed. Scroll is the timeline.
Reveal-on-scroll, hero-letter staggers, and palpable physics carry the
narrative. Visual language often borrows from the technical aesthetic
(grids, bounding boxes, code-coloring) and uses motion to humanize it.

Use when:
- Product launches, conference sites, agency portfolios.
- Brand wants "alive", "current", "kinetic".
- Subject benefits from sequencing (a story, a process, a journey).

Representative philosophies:
- **Field.io kinetic** — typographic scrub, generative line work.
- **Active Theory cinematic** — full-bleed video + DOM-overlaid copy.
- **Resn dimensional** — 3D scenes, hover-driven micro-poetry.
- **Dataveyes narrative** — data viz that animates the user through it.

### minimalism

Identity: subtraction is the signal. Whitespace is content. One headline,
one subhead, one CTA. Color palette under five tokens. Type at 1-2 weights.
The brand voice is the absence of bombast.

Use when:
- Premium products, software for serious work, healthcare, finance.
- Brand wants "calm", "considered", "expensive".
- The subject is itself complex; UI must not add to the load.

Representative philosophies:
- **Kenya Hara emptiness** — center-anchored, generous margins, soft greys.
- **Jasper Morrison super-normal** — type that disappears into reading.
- **Dieter Rams "less but better"** — function-led, ornament-suspect.
- **Apple Industrial product-led** — hero shot, type subordinate to subject.

### experimental

Identity: refuse the brief. Type as image. Color collisions. Layouts that
break the grid on purpose. Risk is the point: the page is a proposition.

Use when:
- Cultural institutions, music, fashion, agency landing pages making a claim.
- Brand wants "fearless", "art-school", "irreducible".
- The audience self-selects; pleasing everyone is not the goal.

Representative philosophies:
- **Sagmeister provocation** — body-as-canvas, copy-as-shape.
- **David Carson rule-break** — destroyed grid, deliberate illegibility.
- **Karel Martens systemic-collage** — print-form residue applied to web.
- **Irma Boom book-as-object** — object-thinking applied to a screen.

### eastern-philosophy

Identity: warmth + restraint. Asymmetric balance ("ma" — meaningful
emptiness). Serif or brushwork display fonts. Color palette draws from
ink, washi, indigo, charcoal, persimmon. The page invites contemplation;
it doesn't demand attention.

Use when:
- Wellness, hospitality, craft, slow-food, museum, contemplative tech.
- Brand wants "warm", "quiet", "rooted", "poetic".
- The subject benefits from time; speed-of-comprehension is not the goal.

Representative philosophies:
- **Kenya Hara (whiteness)** — different lens than minimalism: warmth-led.
- **Issey Miyake garment-as-architecture** — fold, shadow, restraint.
- **Wabi-sabi typography** — imperfect alignment as integrity.
- **Inkstone aesthetic** — sparse strokes, generous negative space.

## How fallback-advisor consumes this

When ambiguity exists, the agent recommends 3 directions FROM 3 DIFFERENT
ROWS of the table above. Three minimalism-leaning takes is not a fallback
— it's three slightly-different minimalisms. See
[fallback-advisor.md](fallback-advisor.md) for the 8-phase procedure that
binds this taxonomy to compilable showcase variants under
`stdlib/showcases/<scenario>/<school>.st`.
