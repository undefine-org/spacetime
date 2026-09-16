# School Gallery

One markup tree. Five design languages. Switched live, with no rebuild and no
hand-written JavaScript.

```sh
cargo run -- serve demos/school-gallery/
```

Click a school in the top bar and the whole page becomes it — ground, ink,
display family, size, weight, tracking, whether the headline shouts in
uppercase, and whether the call to action is a pill, a squared block, a ghost
outline, or an underlined phrase that refuses to be a button at all.

## The idea

A pattern never names a school. It splices a **role**:

```spacetime
.hero__title { --role-title; }
.hero__cta   { --role-cta; }
```

A **school** answers what those roles mean. The gallery ships five, each one a
block of values covering the same questions:

```spacetime
@form style --school-eastern {
  --ground: #f7f3ec;
  --ink: #2d2a26;
  --display-family: "EB Garamond", Garamond, serif;
  --cta-decoration: underline 1px;
  /* … */
}
```

Adding a sixth school is **one value block and one button**. Nothing else on
the page changes — that is the whole claim, and
`tests/demo_school_gallery_test.rs` pins it (the hero title must appear exactly
once in the document; more than one means a per-school copy crept in).

## Why custom properties, not just form splices

A `@form style` splice is resolved by the **compiler**, so a page can carry
many schools but only one binding. Compile-time that is already a win: swapping
school is one edit instead of an edit to every splice in every pattern
(see `stdlib/showcases/_schools.st`).

To switch at **runtime** the choice has to survive into the cascade, and a CSS
custom property is the one mechanism CSS gives you for *"a value my descendants
read, which I can change from above"*. So each school declares its values as
properties, the role forms read them, and a signal-bound attribute repoints all
of them at once:

```spacetime
@data inline $school : "minimalism";

.gallery { data-school <- $school; }
.gallery[data-school="eastern"] { --school-eastern; }
.switch__btn { @on &.click { $school <- $.dataset.school; } }
```

The forms stay the vocabulary; the properties carry the answer.

## The rule the layout section keeps

Everything below the markup is either **structure** (grid, spacing, flow) or a
**role splice**. No colour, no type family, no radius is written at that level
— a value written there cannot be answered by a school, so it silently survives
every switch. That leak is real and this demo hit it: the switcher chips
rendered in the wrong family until the schools were given `--text-family`. A
gate now fails the build if a hex colour appears in the layout section.

## What this demo is built on

| piece | what it needed |
|---|---|
| roles binding schools | BUG-327 — a `@form style` body may splice another form |
| entrances with real durations | BUG-322 — an unnamed driver arg reaches the param the author meant |
| `@on &.visible(720ms)` completing | BUG-323 — `.visible` is an entrance timeline, not a ratio scrub |

Each of those shipped broken and silent. The gallery is the page that could not
have been written before they were fixed.
