# 11 · The Dynamic Pass

```st hidden
@import "./reader.st"
```

Pass 3 made the page's *shape* carry its argument. This pass asks a narrower and
more dangerous question:

> **Does anything on this page happen *to the reader*, rather than merely sitting
> there being true?**

A static page is a claim delivered all at once. A dynamic page can withhold, then
deliver — and withholding is the only way to make a reader *want* the next thing.

Dangerous, because motion is the cheapest possible way to look like you tried,
and the easiest way to make a page feel like every other page.


## 1 · What "dynamic" actually buys you

Three things, and only three. Everything else is decoration.

**1 · Sequence.** Static markup delivers a section's proposition in one blow;
the reader's eye arrives everywhere at once and the emphasis is whatever is
biggest. Motion lets you say *this, then this* — which is the only way to build
a small argument inside a single section.

**2 · Causality.** When something happens *because the reader did something*,
the page stops being a document and becomes a thing that responds. A response
proves the presence of a mechanism far more cheaply than a sentence claiming one.

**3 · Withholding.** A thing that is not yet visible is a question. A page that
answers questions the reader has started to ask is experienced as *understanding
them* — which is the whole acceptance criterion (`vb-004`).

Note what is NOT on that list: *interest*, *polish*, *modernity*. Those are how
motion is usually justified and they are not reasons, they are moods. If a
proposed animation does not buy sequence, causality, or withholding, it is
decoration — and §6 explains why decoration is not neutral here.


## 2 · The distinction this pass turns on

Pass 3 asked: does the FORM enact the proposition?
This pass asks: does the BEHAVIOUR enact it?

The failure mode is different, and worse. A badly-chosen static form is merely
inert. A badly-chosen behaviour actively *misdescribes the product*, because
behaviour is read as a demonstration.

**A page that animates smoothly is claiming smoothness.** A page where things
snap into place is claiming decisiveness. A page where content appears
progressively as you scroll is claiming there is more here than you can see at
once. These are claims. The reader does not experience them as taste.

∴ the governing rule of the pass:

> **Motion is a claim about the product, made in the product's own medium.**

A workflow tool whose page is fussy and elaborate has said something about the
workflow tool. A tool that claims to end chaos, on a page where six things move
at once, has refuted itself in the medium the reader trusts most — the one they
cannot tell they are reading.


## 3 · What Spacetime actually offers

Verified live in a browser against a production build, not from docs. Where the
repository's documentation disagrees with this list, the documentation is
wrong — several documented features are aspirational (§3.6).

### 3.1 · Event handlers — `@on`

Mutation form. Body may assign signals, emit events, call signal methods:

```
.tgl { @on click { $open <- !$open; $n <- $n + 1; } }
```

Event names are **not a closed set** — the parser accepts any identifier and
registers that literal DOM event type. Commonly available: `click`, `hover`,
`focus`, `blur`, `input`, `change`, `submit`, `keydown`, `keyup`, `scroll`,
`resize`, `mouseenter`, `mouseleave`, `pointerdown`, `pointermove`, `wheel`,
`dblclick`, `visible`, `intersect`.

**Not implemented, despite appearing in docs and examples:** `@on submit.prevent`
(explicitly tracked as never-implemented), `@on keydown:Escape` (no key filtering
exists), and modifiers `once` / `passive` / `debounce` / `throttle` / `stop`.
There is no way to `preventDefault`, navigate, focus, or scroll imperatively from
an `@on` body.

### 3.2 · Animation form of `@on`

A *different* form, selected by the presence of an animation name. Only
`hover`, `click`, `focus`, `visible`, `intersect` route to animation drivers:

```
.lift    { @on hover lift(300ms, easing: ease-out) { translate-y: 0 -> -8px; } }
.ritem   { @on visible reveal(600ms, threshold: 0.2, stagger: 80,
                              staggerFrom: "first") { opacity: 0 -> 1; } }
```

Options: `duration`, `easing`, `threshold`, `delay`, `immediate`, `stagger`,
`staggerFrom` (`first` / `last` / `center` / `random`). **Verified working**:
hover forward+reverse, visible with stagger across siblings.

### 3.3 · Timelines

```
@load  intro(duration: 600ms, delay: 200ms, easing: ease-out-quad) { … }
@scroll parallax(scope: cover, start: 0, end: 1, scrub: true)      { … }
@mouse  tilty(axis: "x", invert: false)                            { … }
@time   pulse(duration: 900ms, iterations: 3, alternate: true)     { … }
@flip   $signal(duration: 240ms)
@after  <name> { … }
```

All verified working live except `@flip` (narrow: server-signal layout delta).

- `@scroll` with `scrub: true` is **continuously** driven by scroll position —
  measured mid-scroll at `opacity 0.205 → 1`, `translate-x 0.39px → 60px`. This
  is the one mechanism that makes a page feel *coupled to the reader* rather
  than merely triggered by them.
- `@mouse` maps pointer position to progress — measured `−4.8° → 0° → +5.4°`
  across the element's width.
- `@time` runs with no input at all: `autoplay`, `iterations`, `alternate`.
- `@after` is the proven sequencing mechanism. `@sequence` and `@chain` exist
  but their source says full step-by-step chaining is incomplete.

Keyframe syntax is `property: a -> b -> c;` — multi-step is supported.

**Presets** (declared `~name`, NOT the `&name` the docs claim): `~fade-in`,
`~fade-out`, `~slide-up`, `~slide-down`, `~slide-left`, `~slide-right`,
`~scale-in`, `~scale-out`, `~bounce-in`, `~spin`, `~pulse`, `~shake`.

### 3.4 · Reactive values

State, and the three ways to consume it:

```
$open bool: false;              // declare

.box  { .active: $open; }       // reactive CLASS  → classList.toggle
.card { --lvl: $n; }            // reactive CSS custom property
<p>n=`$n`</p>                   // text hole
```

All three verified. **The reactive-class form is the one to reach for.** A
ternary inside a literal `class` attribute does NOT evaluate — it interpolates
raw and corrupts the markup. (Found the hard way in Pass 3; the modifier had to
move into the data instead.)

Ambient sources exist as stdlib primitives — `scroll` (x, y, progress, velocity,
direction), `pointer` (x, y, normX, normY, pressure, isOver…), `intersection`
(visible, ratio, rect), `resize`, `media` / `colorScheme` / `reducedMotion`,
`tick` (rAF: t, dt, frame, fps), `interval`, `gesture`, `mutation`, `focus`.
These are primitive-level; the author-facing route to most of them is the
timeline macros above.

### 3.4b · `@scroll-spy` + `@when` — the section-aware surface

The most useful mechanism in the language for this kind of site, and the one
easiest to miss:

```
.bd-monday {
  @scroll-spy
  @when &self.scrollSpy.isActive { $navTheme <- "dark"; }
}
```

`@scroll-spy` tracks whether *this* element is the section currently at the
viewport centre, yielding `$isActive`, `$progress`, `$direction`. `@when` runs
the ordinary mutation grammar while an element-local signal is truthy.

Together they give the page **awareness of where the reader is**, which is the
only mechanism here that changes something OUTSIDE the section being scrolled.
Verified live: nav theme flips `light`→`dark`→`light` as the reader passes
through sections.

NB this corrects the natural reading of §3.5: `&self.<primitive>.<signal>` IS a
real read surface for any primitive bound to the element. What does not exist is
the *generic geometry* facet (`&hero.rect.top`) the docs describe.


### 3.5 · `&elem.rect.*` — the specific thing you asked about

**It is not a general reactive read.** The parser accepts any dotted path
`&name.facet.property`, which makes it *look* available everywhere. Only two
spellings actually lower to anything, and only inside a `%derives` block:

```
&self.rect.<field>        →  ST.rectOf(el).<field>
&$capture.rect.<field>    →  ST.rectOf(<resolved>).<field>
```

Fields: `x`, `y`, `width`, `height`, `top`, `left`, `right`, `bottom`.

Two traps. `&hero.rect.top` — a *named* ref, as written in
`docs/language/symbol-system.md` — does not match the lowering and yields
nothing. And `ST.rectOf` has **no observer**: a derive recomputes only when one
of its declared `$` dependencies publishes, so a rect read can be silently
stale after a layout change. Named element refs (`stdlib/syntax/element-ref.st`)
are a separate mechanism exposing only four CSS vars: width, height, top, left.

∴ **do not build a section whose meaning depends on live geometry.** The
capability is real but narrow, and its failure mode is a stale number that
nothing warns you about.

### 3.6 · Documented but NOT implemented

Trust this list over the docs; each was checked against source:

- `@bind` / `@show` / `@input` — **retired**, compile error by design. All of
  `docs/DATA_BINDING_GUIDE.md`'s examples are stale.
- `@fade-in`, `@fade-in-up`, `@fade-in-stagger` — parse and bind an observer,
  but the animation bodies are literally `TODO`. They do nothing.
- `@transition(from:, to:, on:)` state machine — retired.
- `@parallax`, `@scroll-progress`, `@reduced-motion` — named in docs, not
  implemented.
- `docs/animation-presets.md`'s Rust `PresetRegistry` and `&name` presets —
  no such implementation.
- `$._index` / `$._first` / `$._last` / `$._count` in `@each` — documented,
  absent. Only `data-index` is written.
- Loop metadata, key filters, submit prevention — absent.

**Verify before you build on any of these.** The pattern is consistent: this
repository's docs describe an intended language somewhat ahead of the shipped
one, and a page that uses an aspirational feature fails silently rather than
loudly.

### 3.7 · A verification trap that will lie to you

Under `serve`, a text hole bound to a signal renders correctly in the served HTML
and is then **blanked** by hydration until something writes that signal. Under
`build` it is correct. So a browser check against the dev server can show you an
empty string that ships fine — or hide a real defect (BUG-236).

∴ **verify dynamic behaviour against a production build**, not `serve`:

```
cargo run -q -- build projects/<name>/
(cd projects/<name>/dist && python3 -m http.server 8000)
```

This is not fussiness. Every claim in §3 was checked this way, and two of them
read differently on the two surfaces.


## 4 · The D-gates

Applied per proposed behaviour, before it is built.

**D0 · Provenance still governs.** Pass 3's V0 said a visual form asserts harder
than a sentence. A *behaviour* asserts harder still, because it demonstrates.
Anything that animates like a working product — a status changing itself, a
progress bar completing, a counter rising, a message being answered — is a claim
that this happens. If the record does not hold it as observed fact, it is
CRITICAL FAIL. **A fabricated demo is the strongest lie a website can tell.**

**D1 · Does it buy sequence, causality, or withholding?** (§1) If you cannot
name which, it is decoration. Cut it.

**D2 · Is the behaviour the argument, or a costume on it?** Ask: if this moved
differently, would the page mean something different? If any easing would do,
the motion is not carrying meaning and something else should. The test that
sharpens this: could a competitor apply the identical motion to opposite copy?

**D3 · Does the static state still say everything?** Inherited from V2 and
NON-NEGOTIABLE. No-JS, reduced-motion, a screenshot, a slow connection, the
moment before hydration — all must carry the full proposition. Motion may
*reinforce* a proposition and may never *deliver* one. Reduced-motion handling
in this toolchain is partial, not universal: generated initial-state CSS is
guarded, but `@scroll` / `@load` / `@hover` / `@time` show no universal policy.
∴ **you must handle `prefers-reduced-motion` yourself.**

**D4 · Does it survive the second visit?** A first-visit delight is a
second-visit tax. Anything that withholds content behind a scroll trigger is
paid for on every subsequent read by the person most likely to convert — the one
who came back. Withholding must be cheap to skip.

**D5 · Does the motion match the product's claim?** (§2) Count what moves at
once and ask whether that number describes the product. For a tool claiming to
end chaos, the answer is *one*.

**D6 · Does it work at 393px, on a touch device?** `@mouse` and `@on hover` do
not exist on touch. If a behaviour is the only carrier of something, it is absent
for roughly half of readers. This is a stricter form of D3.


## 5 · Procedure

1. **Inventory behaviour, not sections.** For each section: what currently
   happens over time? Almost always *nothing* — that is the honest baseline.
2. **Find the withholding candidates.** Which sections contain a small argument
   with an order to it? Those are the only real candidates: sequence needs
   something to sequence.
3. **Propose the minimum.** For each candidate name the mechanism (§3), the gate
   it must pass, and the proposition it enacts.
4. **Run D0 first, and cut before building.** Provenance is cheapest to enforce
   before implementation.
5. **Build the static state first, then add behaviour on top.** This makes D3
   structural rather than a thing you check afterwards and fudge.
6. **Verify against a production build** (§3.7), on desktop and at 393px, with
   reduced-motion forced on.
7. **Record refusals with the evidence each would need**, as in Pass 3.


## 6 · The failure this pass is designed to prevent

Pass 3's failure was *confident emptiness* — a beautiful page saying nothing.

This pass has a nastier one: **a page that performs competence it does not have.**

Motion is how software signals that it works. A page that animates like a
functioning product is making the strongest available claim to being one, in
exactly the register a reader cannot consciously audit. When the record behind
it is thin, that is not enthusiasm — it is a demo of something that does not
exist, and the reader who is moved by it has been moved by a fiction.

The second failure is quieter and far more common. Every added behaviour spends
attention. A page with six well-executed animations has told the reader that
everything is equally important, which is the same as saying nothing is. **The
number of things that move IS an emphasis decision**, whether or not it was made
deliberately.

∴ the honest output of this pass, on most pages, is *very little motion and a
long list of refusals*. If a run of this pass produces a lot of animation, the
most likely explanation is not that the page needed it.
