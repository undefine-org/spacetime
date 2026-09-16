# One timeline, every clock

**Spacetime 0.9 — the timeline release.** Scroll-driven stories, slide decks,
hover micro-interactions and 60-second rendered films are now the same
construct. Nine animation directives became one. Motion became a thing you can
name, parameterize, and reuse. And a page can now be edited into a video
without leaving the file.

This page is a literate Spacetime document — every fence below is this page's
program. The animations you see are the code you read.

```st hidden
@import "stdlib/md"

// A brand is one typed VALUE, not five loose ones. Fields travel together,
// so they belong to a type -- and $brand.accent is checked, not hoped for.
@type Brand {
  ink:    color;
  bg:     color;
  panel:  color;
  accent: color;
  dim:    color;
  rule:   color;
}

@data inline $brand Brand : {
  "ink":    "#e8eef7",
  "bg":     "#0b0e14",
  "panel":  "#121722",
  "accent": "#5eead4",
  "dim":    "#8b96a8",
  "rule":   "#222b3a"
};

body { background: $brand.bg; color: $brand.ink; font-family: system-ui, sans-serif;
       line-height: 1.65; max-width: 46rem; margin: 0 auto; padding: 3rem 1.5rem; }
h1, h2, h3 { line-height: 1.2; }
code { background: $brand.panel; padding: 0.15em 0.4em; border-radius: 4px; }
```

---

## The thing that changed

Before this release, Spacetime had nine ways to start an animation — `@on
visible`, `@scroll`, `@loop`, `@time`, `@load`, `@hover`, `@click`, `@mouse`,
`@pointer` — plus `@after` for chaining and `@effect` for signal reactions.
Eleven surfaces, one idea.

They are now one:

```st src
@form motion --rise($distance = 24px) {
  opacity: 0 -> 1;
  translate-y: $distance -> 0;
  easing: --ease-out-expo;
}

<p class="lede">A driver, a form, and a scope. That is the whole model.</p>

.lede {
  font-size: 1.25rem;
  color: $brand.accent;
  @on &.visible: --rise;
}
```

Read it as a sentence: *on this element becoming visible, rise.* The word
after `@form` is the kind of chunk being named — `motion` here — one word,
always required. More kinds below.

`&.visible` is a **driver** — a typed projection of a reference. `--rise` is a
**form** — a pure, parameterized chunk of language. Neither is a keyword; both
are registry entries you can add to.

---

## Drivers: what makes a timeline move

A driver is a member of a reference, and the reference's kind decides which
members exist. Every one of these is legal, and each replaces a directive that
used to be its own macro:

```st hidden
<div class="driver-table">
  <div class="drow"><code>&.visible(threshold: 0.5)</code><span>enters the viewport</span></div>
  <div class="drow"><code>&.scroll(scope: cover)</code><span>scroll progress</span></div>
  <div class="drow"><code>&.hover</code><span>pointer over it (reversible)</span></div>
  <div class="drow"><code>&voice.playback</code><span>an audio element's playhead</span></div>
  <div class="drow"><code>&intro.done</code><span>another timeline finishing</span></div>
  <div class="drow"><code>&.clip</code><span>this element's span in a score</span></div>
  <div class="drow"><code>$cart.open</code><span>a signal changing</span></div>
</div>

.driver-table { display: grid; gap: 0.4rem; margin: 1.5rem 0; }
.drow {
  display: grid; grid-template-columns: 20rem 1fr; gap: 1rem;
  padding: 0.6rem 0.8rem; background: $brand.panel; border-radius: 6px;
  border-left: 2px solid transparent;
  @on &.visible {
    --rise(distance: 12px);
    stagger: 60ms;
    staggerFrom: first;
  }
  @on &.hover { border-left-color: $brand.accent; translate-x: 0 -> 4px; }
}
.drow span { color: $brand.dim; }
```

The last row is the one that ends a long-standing split. `$cart.open` is a
signal, not an element — and a signal changing is now a driver like any other.
That is what retired `@effect`:

```st src
<button class="toggle">Toggle the drawer</button>
<div class="drawer">The drawer body.</div>

.toggle {
  $open bool: false;
  @on &.click { $open <- !$open; }
}

.drawer {
  @on $open {
    true  => --slide-open(280ms);
    false => --slide-shut(200ms);
  }
}

@form motion --slide-open($duration = 280ms) { height: 0 -> 4.5rem; opacity: 0 -> 1; easing: --ease-out-quart; }
@form motion --slide-shut($duration = 200ms) { height: 4.5rem -> 0; opacity: 1 -> 0; easing: --ease-in-quad; }

.toggle { background: $brand.panel; color: $brand.ink; border: 1px solid $brand.rule;
          border-radius: 6px; padding: 0.6rem 1rem; cursor: pointer; }
.drawer { overflow: hidden; background: $brand.panel; border-radius: 6px;
          margin-top: 0.5rem; padding: 0 1rem; }
```

Those are **arms** — the same `pattern => consequence` shape `@match`,
`receive`, and `handle` already use. A bare pattern means *"arrived here from
anywhere."* When the source matters, name it:

```
@on $take {
  a -> b => --cut;          // arrived at b, specifically from a
  b      => --dissolve(6f); // arrived at b from anywhere else
  _      => --cut;
}
```

The combinatorial matrix is opt-in. Six states never force thirty-six arms.

Arms can also *filter*, not just match. A pattern with bindings gives the
transition's endpoints names, and `where` turns them into a guard:

```st src
@form motion --flash-up { translate-y: 4px -> 0; opacity: 0.4 -> 1; }
@form motion --flash-down { translate-y: -4px -> 0; opacity: 0.4 -> 1; }

.stock {
  $price number: 100;
  @on $price {
    $old -> $new where $new > $old => --flash-up;
    $old -> $new where $new < $old => --flash-down;
    _ => --flash-up;
  }
}
```

`$old -> $new` matches *any* change and binds the two values; the guard
decides at match time. And when you want the previous value outside a
pattern, it is a property — `.prev` reads it anywhere expressions live:

```st src
.stock { color: $price > $price.prev ? #16a34a : #dc2626; }
```

One more statement shape, for the smallest consequences: a body-less `@on`
takes `:` and then a form *or a mutation* — `@on &.click: $open <- !$open;`
is the drawer's toggle in one line. The colon introduces the value; braces
are choreography. The same rule, everywhere.

---

## Bodies: lines, splices, or nothing at all

A driver takes its own parameters in the head, and a body can be written
inline — keyframe lines and settings, no form needed:

```st src
.card {
  @on &.visible(threshold: 0.5) {
    opacity: 0 -> 1;
    translate-y: 16px -> 0;
    easing: --ease-out-quad;
    stagger: 80ms;
  }
}
```

A body can even be empty. Then the driver only publishes its progress as a
binding, and anything reactive can consume it:

```st src
.scroll-fill {
  @on &.scroll(name: page) { }
  --fill: $page;
  transform: scaleX(var(--fill));
}
```

## References: elements can depend on each other

`&` is identity, so a driver projects ANY element reference — `&.` (this
element) is just the common case. Name another element and this one moves
with it:

```st src
<section class="hero">…</section>
<div class="card">…</div>

&hero .hero;

.card {
  @on &hero.visible: --rise;
}
```

The card rises when the HERO enters the viewport: the driver observes the
hero, the animation plays on the card. Observed and animated are different
questions, and `@on` answers them separately — the subject after `&` is
observed, the scope that owns the rule is animated.

---

## Names: a timeline you can point at

A driver can *name* what it publishes. Then it is not just playing an
animation — it is producing a value:

```st src
.hero {
  @on &.scroll(start: 0.2) as $reveal {
    opacity: 0 -> 1;
    translate-y: 24px -> 0;
  }
}
.progress-bar {
  width: $reveal;
  height: 3px; background: $brand.accent; transform-origin: left;
}
```

`as $reveal` binds the driver's progress into scope as a signal. The
progress bar does not observe anything — it just reads. A timeline *is* a
signal; naming it makes that literal.

And once a timeline is a signal, its lifecycle is data. `done` is a
property of progress bindings — true when progress reaches 1 — and arms
give it edges:

```st src
.subtitle {
  @on $reveal.done { true => --rise(distance: 12px); }
}
```

That is what retired `@after`: sequencing is no longer a registry of names
under the hood, it is one signal reading another. The arm shape chooses the
semantics. `true =>` is a *gate* — scrub back up and the subtitle reverses
with you. `false -> true =>` is a *latch* — it fires on the transition and
stays fired. Reversibility is in the pattern, not in a declaration.

For the one-liner there is a sugar — `@on $reveal.done: --rise;` — which
desugars at compile time to exactly that true arm. One mechanism, two
spellings.

---

## Forms: motion you can name

Spacetime has three sigils, and each answers exactly one question:

| Sigil | Category | Asks |
|-------|----------|------|
| `&` | identity — a thing in the world | *which one?* |
| `$` | data — values that flow | *what value?* |
| `--` | form — a chunk of language | *what shape?* |

The third is new, and it is not an invention. CSS names reusable chunks with
dashed-idents — `@function --double(--x)`, `@mixin --card` — so Spacetime uses
the same namespace and the same position rule: **statement position splices,
value position calls.** The kind-word sits between the directive and the name
— `@form style --card-surface` — in the position `@data inline` already owns:
the language's word before the thing's name. It is a word the author writes,
never a guess the compiler makes. A CSS author already knows how to read this:

```st src
@form style --card-surface {
  background: $brand.panel;
  border: 1px solid $brand.rule;
  border-radius: 10px;
  padding: 1.1rem 1.25rem;
}

<div class="card">
  <strong>A form is pure.</strong>
  <p>No identity, no state — it is spliced where you name it, and that is all
  it does.</p>
</div>

.card {
  --card-surface;
  @on &.visible: --rise(distance: 16px);
}
.card p { color: $brand.dim; margin: 0.4rem 0 0; }
```

Because forms take parameters, the 40 near-identical animation blocks in a
typical motion library collapse to one form and a call site. Easings became
forms too — `--ease-out-expo` instead of `&ease-out-expo`, which had been
quietly wrong: an easing curve has no identity, so it never belonged to the
identity sigil.

---

## Signals: when values move things

Elements are not the only subjects. `$` is the signal rail, and a signal
can drive animation directly — `.change` is its projection:

```st src
.badge {
  $count number: 0;
  @on $count.change(duration: 500ms) {
    font-size: 14px -> 24px;
  }
}
```

Every time the value changes, the badge replays — duration rides the head
because a replay *is* a driver run. And text gets the element-level
version: `text-change` watches an element's content and choreographs the
characters as they leave and arrive:

```st src
.price {
  @on &.text-change(duration: 300ms, stagger: 30ms, from: "center") {
    :entering { translate-y: -50% -> 0; opacity: 0 -> 1; }
    :exiting  { translate-y: 0 -> 50%;  opacity: 1 -> 0; }
  }
}
```

Two slots, because a change has two sides. The driver watches the text; the
body says what an entering character does and what an exiting one does.

---

## Scores: when elements share one playhead

`@on` handles elements that animate independently. When they must share a
timeline — a sequence, a deck, a film — that is a **score**. Same driver
grammar in the head, so the only thing that changes between a scrollytelling
section and a rendered film is the clock:

```
.story { @score &.scroll(cover) {
  &fact-income for 30% -> &fact-mobility for 40% -> &fact-map;
  &chart-race during &fact-income to &fact-mobility;
} }

.deck  { @score &.steps(advance: &.click | &.key(" ")) { &title -> &problem -> &close; } }

.film  { @score &.time(60s) as &film {
  &intro as &opening -> --whip-pan(300ms) -> &features(items: $items) -> &outro;
  &lower-third(name: "Ada Lovelace", role: "Analyst") at 2s for 3s;
  &broll(src: "city.mp4") in 4s out 9s during &opening;
  &bgm(gain: 0.6, duck: &voice) during &opening to &outro;
  mark &chapter1;
} }

.mv    { @score &voice.playback { &cover for beat(16) -> &verse for beat(32); } }
```

Note the punctuation rule, because it carries more weight than it looks like:
**parentheses hold what the function owns; trailing words hold what the
language owns.** `&broll(src: "city.mp4")` is the clip's own parameter. `in 4s
out 9s during &opening` is placement vocabulary — the language's, not the
template's.

That distinction is what makes the timeline editor possible. Dragging a clip's
edge rewrites `in 4s` to `in 5.2s` in your file. The editor is not a layer over
the language; it is a second view of it.

---

## Speed is a property now

Retiming, reversing and freezing used to be three separate concepts. They are
one property, and because it is a property it keyframes like any other:

```
rate: 1 -> 2;   // speed ramp, easable
rate: 0;        // freeze frame — this clip holds, the timeline keeps running
rate: -1;       // reverse
```

`hold(until: $form.valid)` remains distinct, and the difference is worth
stating: `rate: 0` stops **one clip**; `hold` stops **the playhead**.

---

## Rendering

A score whose driver is seekable can be rendered:

```
spacetime render projects/launch/ --out film.mp4
spacetime render projects/launch/ --data reps.json --out renders/   # one cut, N films
spacetime render projects/launch/ --aspect 9:16 --out vertical.mp4
```

The renderer seeks the transport and captures on the paint rung — the same
fidelity rung the browser-automation tests use. Which motion is renderable
follows from the driver's value type, so the compiler can tell you in advance:
`Transport` and coordinate-`Progress` drivers seek exactly; `Event` drivers
(hover, click) render at their rest state and emit a warning naming the clip.
No silent difference between what you previewed and what you shipped.

---

## What this deleted

Nine macro heads and their dispatchers · `@after` · `@effect` · `trigger: "…"`
string references · ident-based timeline naming and the string registry behind
it · `&ease-*` as entities · `~` presets · `@pose-track(objectIndex:)` · the
preview-mode special case · freeze, ramp and reverse as separate concepts ·
multicam's state-machine workaround.

Added: one dashed-ident capture type, one directive (`@form`) with its kind
word, one generalized head (`@on`), one directive (`@score`).

Existing projects keep compiling. The migration capsule carries the retired
definitions verbatim, rewrites the mechanical cases in memory, and `spacetime
migrate` writes them to disk when you are ready. Two shapes need a human —
`trigger:` strings becoming typed references, and pose tracks becoming named
entities — and both produce a hint that says so.

---

## The bet

A website and a video differ only in which signal drives the timeline.

Everything above follows from taking that seriously: if the driver is the only
difference, then one composition grammar serves a scroll-driven story, a slide
deck, a hover state, and a rendered film. The parts that make video work —
sequencing, transitions, source trims, retiming, markers — turn out to be what
pages wanted anyway. Route transitions are a score edge. A sticky section is a
shared playhead. A design system's motion library is a set of forms.

If some corner of that turns out to need a second mechanism, the design is
wrong and we would rather reopen it than paper over it.

```st hidden
<footer class="foot">
  <p>Spacetime 0.9 · SIP-001, SIP-001b, SIP-001c</p>
</footer>

.foot {
  margin-top: 4rem; padding-top: 1.5rem; border-top: 1px solid $brand.rule;
  color: $brand.dim; font-size: 0.9rem;
  @on &.visible(threshold: 0.2): --rise(distance: 8px);
}
```
