# Showcase gallery — the front door for `stdlib/showcases/`

```bash
cargo run -- serve demos/showcase-gallery/
```

## Why this exists

`stdlib/showcases/` curates ten patterns across five design schools. It shipped
with `CATALOG.md`, a metadata schema, a restyle pass (PLAN-139 W2) — and **zero
pages**. `stdlib/showcases/index.st` is a pure import manifest, so the library
whose entire purpose is to be *looked at* could not be looked at.

More pointedly: not one of those patterns had a single call site anywhere in the
repo. They were authored, documented, and never instantiated. This page is their
first call site, which means the tree is now exercised by the same act that
displays it — **if a pattern breaks, this page breaks.**

## What it does

Ten specimens, each the real `@template` invoked with real arguments. No
screenshots, no reimplementation. Beside each: its scenario, school,
description, and the `useWhen` guidance that answers *when to reach for it* —
because that is the question a pattern library exists to answer.

Filtering is by **school**, not by scenario, because the choice an author is
actually making is "which design language am I in", not "which hero".

## The mechanism (no JavaScript)

One signal, one attribute, and the cascade does the rest:

```spacetime
@data inline $school : "all";

.gal { data-school <- $school; }
.chip--school { @on &.click { $school <- $.dataset.school; } }

.gal[data-school="minimalism"] .spec:not([data-school="minimalism"]) { display: none; }
```

A specimen participates by carrying `data-school`. Adding a pattern needs no
filter edit and no list to keep in sync — there is no per-specimen state to
drift. The active chip highlights through the *same* attribute, so the
selection can never disagree with what is actually filtered.

## The trap this page walked into (BUG-344)

A template invocation is a **directive**, so it binds through a selector — never
through nesting:

```spacetime
<div class="frame slot--hero-minimal"></div>          /* the slot */

.slot--hero-minimal { &showcase-hero-minimal("…", "…", "…"); }
```

Written *inside markup* it silently becomes escaped page text
(`&amp;showcase-hero-minimal("…")`) and **the build still reports success**.
The first version of this gallery rendered ten lines of escaped source and
exported cleanly.

That silence defeats string-level tests too: a gate asserting
`html.contains("showcase-hero-min")` passes on the invocation's own source. The
gate here asserts `class="showcase-hero-min"` *and* that no `&amp;showcase-`
survives anywhere on the page.

`@each` refuses this same mistake loudly. The invocation surface should too —
filed as **BUG-344**.

## The list generates itself (PLAN-144 W3)

```spacetime
@import "../../stdlib/showcases/index.st"
@data declarations $patterns from template ;

.gal__specimens {
  @each($patterns as $p) {
    <section class="spec" data-school="`$p.fields.school`"> … </section>
  }
}
```

The page never names the set. Publishing a pattern — a file plus its line in
the tree's manifest — makes a specimen appear here, with its metadata, and
filterable by school, with **no edit to this page**. That property is the
acceptance test (`a_new_pattern_appears_with_no_page_edit`), not the current
count of ten.

`$p.fields` is the pattern's own `/// scenario: / school: / useWhen:` header
read as data (BUG-343 / PLAN-144 W2).

### What stays authored

The COPY. A pattern needs real words to show what it is for; generated lorem
would make the gallery useless. Each slot at the bottom of `index.st` targets
the slot class the loop derives from the pattern's name.

A pattern with no authored copy still gets a specimen and its metadata — its
frame is simply empty. Visible, not silent.

## Seeing a pattern whole

A pattern is a PAGE-sized design (`min-height: 100vh`, 96px padding, a 112px
display). In a fixed 520px frame with `overflow: hidden` you saw its top-left
corner — a headline cropped to three letters.

So a specimen is a scaled preview, the way a device frame works:

```spacetime
@form style --gal-stage {
    width: 1440px;              /* lay out at a real desktop viewport */
    transform: scale(0.779);    /* then scale to the frame's own width */
    transform-origin: top left;
}
```

The pattern still lays out at 1440px and is simply shown smaller — no reflow,
no responsive breakpoint standing in for the real design. Compact scenarios
(`nav`, `footer`, and the `minimal` CTA banner) get shorter frames keyed off
their own `data-scenario` / `data-style`, so a 200px band does not float in a
page-sized box.

## The interactive patterns

Three specimens show CAPABILITY rather than composition — click them:

- **pricing / interactive** — one `$annual` signal drives three prices, three
  period labels and the savings badge. Nothing is stored twice, so nothing can
  disagree.
- **features-grid / tabbed** — `$panel` selects the panel, moves the indicator
  and updates the counter. State without a state machine.
- **cta / magnetic** — `@magnetic(strength: 0.4, radius: 180, lerp: 0.14)` and
  `@reveal(split: "words", stagger: 60)`. Pointer physics and text
  choreography, both declared, neither scripted.

## Known gaps

- A **dynamic** component name does not expand: `` `&$p.name("a")` `` renders
  literally, because the W1 expansion pass resolves a literal `&name(` only.
  That is why the copy is bound by slot class rather than invoked from the row.
  Worth revisiting — it would remove the last hand-maintained list here.
- `hero/media` is omitted — it needs BUG-310 (`__stRefs` has no writer) and
  BUG-311 (dev server ignores `Range`).
- `stdlib/showcases/index.st` and `SCHEMA.md` describe a `cargo run -- showcase`
  CLI and an `@showcases` registry. Neither exists (see BUG-343).

## Related

- `demos/brand-catalog/` — the same browse idea over `@form` declarations,
  where the rows *are* generated.
- `demos/school-gallery/` — one markup tree, five schools, switched live.
