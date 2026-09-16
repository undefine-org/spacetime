# Film in Spacetime

*This document is the golden reference for Spacetime's film surface. It is written
affirmatively, as the language will be when the work it describes is done. Every
fenced `st` block is a program; the ones marked `src` show their source. Sections
marked `— status —` say what ships today and what is still planned; see
`!tasks/plans/PLAN-150-film-surface.org` for the build order.*

```st hidden
@import "stdlib/md"
```

## 0. Why this exists

A 30-second promo for Spacetime was made twice from one storyboard: once with a
studio pipeline (Three.js, custom GLSL, a hand-written frame capturer — about 560
lines of JavaScript on a 636 KB engine), and once in Spacetime (`demos/promo/`,
513 lines, no JavaScript). The comparison is in `demos/promo/CRITIQUE.md`.

The Spacetime cut was close. Where it lost, it lost for a small number of
reasons, and every one of them is a missing *word*, not a missing *engine*. This
document adds the words. The bet: a film is a web page with a clock, and the
same vocabulary that places a card on a page should place a shot on a timeline.

Three rules shape every proposal here (from `AGENTS.md`):

1. **One sigil, one meaning.** `->` is a step between values. `at` places
   something at a moment. `--name` refers to a form. `&name` refers to an
   element. `@word` is a directive. Nothing below gives any of these a second job.
2. **Variants are data.** New behaviour arrives as a `@form <kind>`, never as a
   new special case in the compiler.
3. **Every new word deletes something.** Each section names the hack it retires.

## 1. The picture in one paragraph

There is **one clock**. A `@score` puts things on it with `at … for …`. Each
thing — a DOM element, a 3D object, a camera, a sound, a whole scene — receives a
*window* of that clock, and consumes it with `@on &.clip { … }`. Inside that
block you write what changes: `opacity: 0 -> 1`. That block is the *only* place
motion is written, and it works the same whether the driver is the clock, the
scroll position, a click, or a video's playhead. Everything in this document is
either a new thing you can put on the clock, or a new thing you can write inside
that block.

```st src
.stage {
  @score &.time(30s) as &film {
    &open  at 0s  for 3.6s;
    &grid  at 3.2s for 4.6s;
    &code  at 7.4s for 5.4s;
  }
}
.open { @on &.clip { opacity: 0 -> 1 -> 1 -> 0; } }
```

That already works today. Everything below extends it.

---

## 2. Keyframes: positioned stops and per-step easing

### The problem

`a -> b -> c` places its stops evenly. A flash that peaks at 2.2 s inside a 30 s
clip needed a sixty-stop chain, or six extra clips placed 0.12 s early so an
evenly-spaced peak would land on the sound. Blinks, ticks and hard cuts had to be
faked with `sign(sin(…))`.

### The words

A stop may say **where** it sits with `at`, and a step may say **how** it moves
with a form reference. Both words already exist: `at` places things in a
`@score`; `--name` names an easing form.

```st src
.flash {
  @on &.clip {
    opacity: 0 -> 0.85 at 7.3% -> 0.18 at 8% -> 0;
  }
}
```

Read it as: *zero, then 0.85 at 7.3 % of the window, then 0.18 at 8 %, then zero
at the end.* Stops without `at` spread evenly between their positioned
neighbours, exactly as they do today when nothing is positioned. `at` accepts a
percentage of the window or an absolute time inside it (`at 2.2s`).

A step's easing sits on the step:

```st src
.card {
  @on &.clip {
    translate-y: 40px -> 0 --ease-out-expo -> -6px --linear -> 0 --ease-in-out-sine;
  }
}
```

Read it as: *from 40px, expo-out to 0, then linear to −6px, then sine to 0.* A
form reference after a stop names the curve of the step that *reaches* that
stop. The existing body-level `easing: --name;` is still the default for every
step that does not name its own.

A **hold** is a stop that repeats its value:

```st src
.caret { @on &.clip { opacity: 1 -> 1 at 50% -> 0 at 50.1% -> 0; } }
```

No `hold` keyword; two positioned stops with the same value is a hold. A
**step** curve is just another easing form, shipped in stdlib:

```st src
.counter { @on &.clip { --fr: 0 -> 1800; easing: --steps(1800); } }
```

### What this deletes

The six `&h1 … &h6` hit clips in `demos/promo/index.st`; the `sign(sin(var(--chars)))`
caret; every `range:` written only to move a peak.

### — status —

**Shipped (PLAN-150 W1).** `at` and per-step easing are peeled from each stop
by the shared keyframe reify (`metasystem::expand::parse_keyframes_value`), which
emits `{ at, value, easing? }` with author positions; unpositioned stops
distribute evenly between positioned neighbours, all at compile time. A per-step
easing lands on the earlier keyframe (the runtime reads a segment's curve from
its start). `--steps(N)` is a built-in the runtime's `getEasing` parses. Working
doc + runnable examples: `film/w1-positioned-stops.st.md`; behaviour gate:
`tests/score/positioned-stops.test.st` (peak-at-declared-position, seen RED
against even-spacing).

---

## 3. Scene post-processing for any element

### The problem

A film has a *lens*. Bloom around bright things, a touch of chromatic
aberration on impacts, film grain, a vignette. In the Spacetime cut these were a
grain `div`, a vignette `div`, two colour-shifted clones of the `<script>` text
for RGB split, and no bloom at all. Meanwhile `stdlib/3d` already has
`@post(bloomStrength: …, rgbShift: …)` — but only inside `@stage`, and only as
fixed parameters.

### The word

`@post` applies to **any element**, its properties are **keyframeable**, and it
lives in the same block as every other property.

```st src
.stage {
  @post { bloom: 0.45; grain: 0.035; vignette: 0.55; aberration: 0.002; }

  @on &.clip {
    @post { bloom: 0.45 -> 1.3 at 7.3% -> 0.45; aberration: 0.002 -> 0.014 at 7.3% -> 0.002; }
  }
}
```

The static block sets the look; the block inside `@on &.clip` animates it with
the ordinary arrow syntax. The properties are a small, named set:

| property | meaning | DOM lowering | `@stage` lowering |
|---|---|---|---|
| `bloom` | glow around bright pixels | a blurred `screen`-blend clone of the element behind a luminance mask | UnrealBloomPass strength |
| `aberration` | colour fringing | two channel-tinted clones offset ± the amount | RGB-shift pass |
| `grain` | film grain, per frame | an SVG `feTurbulence` layer whose seed follows the frame | noise pass |
| `vignette` | edge darkening | a radial mask | vignette pass |
| `flash` | whole-frame light | a white overlay | additive pass |
| `glitch` | horizontal slice displacement | `clip-path` slices with per-slice offsets | slice-shader pass |

A look is data, so a look is a form:

```st src
@form post --cinema { bloom: 0.45; grain: 0.035; vignette: 0.55; }
.stage { @post --cinema; }
```

**Fidelity note, stated plainly.** Inside `@stage` these are real render passes.
On DOM they are compositions of `filter`, `backdrop-filter`, `mix-blend-mode`
and masks; DOM bloom cannot bleed *outside* the element's box the way a shader
can. The words are the same because the *intent* is the same; the doc for each
property states its DOM approximation. Under `spacetime render` both are
captured by the same screenshot, so a film gets whichever the page uses.

### What this deletes

`.grain`, `.vignette`, `.hit` overlays and `.tagr/.tagb` clones in the promo;
the `@post(bloomStrength: …)` parameter bag in `stdlib/3d` (now the same
block).

### — status —

**Shipped (PLAN-150 W2), DOM half.** `@post { bloom; grain; vignette }` on any
element injects compositor layers (`stdlib/primitives/post.st`) that read
`--st-post-*` custom properties; the block seeds their initial values and
animating the look is animating those properties with the W1 keyframe engine
(`@on &.clip { --st-post-bloom: 0.45 -> 1.3 at 61% -> 0.45 }`). Working doc:
`film/w2-post.st.md`; behaviour gate: `tests/score/post-lens.test.st` (the bloom
layer's opacity rises with the animated property, seen RED against a constant).
Still ahead in W2's arc: `aberration`/`glitch` layers, the `@post`-inside-`@on`
sugar, and unifying the `@stage` pass params under these key names.

---

## 4. Camera

### The problem

A crane shot — overhead to eye-level as the grid materialises — is the single
most convincing thing in the studio cut. On DOM, `perspective-origin` keyframes
approximate it badly; in `@stage`, the camera is a `camZ:` parameter.

### The word

A camera move is **motion**, so it is written where motion is written. `camera`
is a form kind whose vocabulary is `position`, `target`, `fov`, `roll`, and a
camera form is consumed by `@on` exactly like a motion form:

```st src
@form camera --crane {
  position: 0 9.5 4.5 -> 0 2.1 9.5 -> 0 1.4 8.6;
  target:   0 0 0    -> 0 0.9 0;
  fov: 38;
  easing: --ease-in-out-cubic;
}

.world { @on &.clip --crane; }
```

That is the whole surface. Because it is an `@on` consumer:

- it runs on any driver: `@on &.scroll --dolly;` is a scroll-driven camera;
  `@on $route "/pricing" => --whip-pan;` is a view transition; `@on &.clip`
  is a shot;
- it composes with everything else in the block — `@on &.clip { --crane;
  .title { opacity: 0 -> 1; } }`;
- it takes params like any form: `--crane(height: 9.5)`.

The **rig** is the element the `@on` sits on. Inside `@stage`, it is the real
camera. On a DOM element with `transform-style: preserve-3d`, the compiler
lowers the camera to one inverse matrix on a generated rig child, so the same
form moves a CSS 3D layout. Camera shake is a camera form too:

```st src
@form camera --shake($amount = 8px) {
  position: 0 0 0 -> 0 0 0 at 60% -> $amount 0 0 -> calc(-1 * $amount) 0 0 -> 0 0 0;
  roll: 0deg -> 0deg at 60% -> 0.4deg -> -0.3deg -> 0deg;
}
```

### What this deletes

`@stage(camZ: …)`, `@scroll-3d(camZ: …)` camera params; `perspective-origin`
keyframe hacks; the seven-stop `translate` shake chains on `.cam` in the promo.

### — status —

**Shipped (PLAN-150 W3), DOM half.** `camera` is a form kind (`stdlib/macros/form.st`);
a camera form's axes rewrite to `--cam-*` custom-property tracks
(`src/pipeline/drivers.rs::rewrite_camera_axes`) animated by the W1 engine, and
a static rig transform on the host reads them (dolly on `--cam-z`, pan on
`--cam-x/y`, roll on `--cam-roll`). Consumed via the bare statement form
`@on &.clip --crane;` (landed in W3-arc) or a body splice `@on &.clip { --crane; }`;
a true `lookAt` from `target` remains in the arc. Working doc: `film/w3-camera.st.md`;
behaviour gate: `tests/score/camera.test.st` (the rig transform changes across
the window as the dolly pulls in, seen RED against a constant transform).

---

## 5. The score drives 3D

### The problem

`stdlib/3d` has objects, lights, particles, post and a scroll binding — and none
of it listens to `@score`. `@scroll-3d(drive: "morph", to: 3)` is a private
driver with a string property name. A film cannot cut a 3D shot on the beat.

### The word

Inside `@stage`, every object property is keyframeable in `@on &.clip`,
exactly as a DOM property is. `@scroll-3d` becomes sugar for `@on &.scroll`.

```st src
canvas.field {
  @stage {
    @particles &cloud (count: 30000, size: 0.018, color: #8ab4ff)
    @object &floor grid(size: 64, divisions: 128, color: #7dd3fc)
    @post --cinema;
  }

  @on &.clip {
    &cloud { morph: 0 -> 3; }
    &floor { reveal: 0 -> 1 --ease-out-quart; displace: 0 -> 1 at 70% -> 0; }
    @post  { bloom: 0.45 -> 1.2 at 70% -> 0.45; }
  }
}
```

An object declared with `&name` is addressable in any consumer block by that
name — the same `&` that names elements, because inside a stage an object *is*
the element. Its keyframeable properties are declared by the object's form
(`grid()` exposes `reveal`, `displace`, `fog`; `particles` exposes `morph`,
`spin`, `size`); an unknown property is a compile error naming the form.

The grid in the studio cut's second scene — fog falloff, a reveal ring, a ripple
that displaces vertices on the hit — is exactly `&floor { reveal; displace; fog }`
under one `@on &.clip`.

### What this deletes

`@scroll-3d(drive:)`; the `pose-track objectIndex` shape SIP-001 already
retired; any future "3D driver" — there is one driver grammar.

### — status —

**Shipped (PLAN-150 W4), object channels.** A `@stage` object may be named
(`@object &orb (...)`) and is then addressable from `@on &.clip { &orb { … } }`.
A named object registers a channel proxy on its canvas; the `@on` engine writes
keyframed values to it (`stdlib/primitives/animation/apply-animations.st` resolves
a `&name` scope to the stage object, descending into a hosted canvas), and the
object's frame loop applies each channel to the real mesh
(`stdlib/3d/primitives/three-object.st`) — no new interpolation path.
Keyframeable channels: `scale`, `spin`, `posX/Y/Z`, `opacity`, `emissive`,
`metalness`, `roughness`, plus `morph` for a particle field. Working doc:
`film/w4-score-3d.st.md`; behaviour gate: `stdlib/3d/tests/stage-clip.test.st`
(the mesh's real `scale.x` grows across the clip window, seen RED against an
ignored channel). Remaining in W4's arc: the `grid()` object form with
`reveal`/`displace`/`fog`, and `@scroll-3d` as sugar for `@on &.scroll`.

Infra: 3D behaviour gates need WebGL in headless Chromium — the `--cdp` backend
now launches with SwiftShader (`src/cdp.rs`) instead of `--disable-gpu`.

---

## 6. Scatter: positions from a shape

### The problem

`<script>` should shatter *into its own letters*. CSS trigonometry over
`sibling-index()` gives a beautiful generic burst; it cannot know where the
glyphs are.

### The word

`@scatter` stamps positions onto children from a source. It has **slots**, like
`@each`'s `:entering`/`:exiting`, because a particle system has more than one
arrangement:

```st src
.burst {
  @scatter(from: &tag, count: 400) {
    :shape { --x: sample-x; --y: sample-y; }   // sampled from the rendered glyphs of &tag
    :rest  { --rx: sample-x; --ry: sample-y; } // where each particle starts (default: its :shape)
  }

  @on &.clip {
    --tau: 0 -> 1;
  }
}
.burst i {
  translate: calc(var(--x) + cos(sibling-index() * 137.5deg) * 400px * var(--tau))
             calc(var(--y) + 620px * var(--tau) * var(--tau));
}
```

`@scatter` runs at build time when the source is static text (positions are
sampled from a rasterised copy via the vendored text engine) and at hydration
otherwise. It stamps custom properties only; **motion stays in `@on`**, and the
per-particle geometry stays CSS. Under `@stage`, `@particles(shape: &tag)` is
the same idea on the GPU.

Other slots are forms: `:shape --ring(radius: 200px)`, `:shape --grid(12, 8)`.

### What this deletes

Nothing in the promo — this is new reach. It replaces the studio cut's
`getImageData` sampler, which is the one thing in that pipeline that had no
Spacetime spelling at all.

### — status —

**Shipped (PLAN-150 W5), DOM sampler.** `@scatter(from: &src, count: N) { :shape
{ … } }` (`stdlib/macros/scatter.st` + `stdlib/primitives/scatter.st`) samples the
source's rendered glyph coverage — the text is drawn to an offscreen canvas and
`count` points are chosen by rejection sampling over the alpha channel, so the
scatter follows the actual ink. Deterministic (PRNG seeded from the source text),
so a film renders bit-identically. Each slot MAPS a property to a sample axis
(`--x: sample-x`) rather than a bare `--x;` — a bare dashed-ident is the
form-splice spelling, and `:` introduces a value everywhere, so the mapping stays
inside the one grammar (one sigil, one meaning). The optional `:rest` slot binds
via a new rule: an absent optional pseudo-selector still binds its inner captures
(`src/syntax/events/form_compiler.rs`), so `@each`-style optional slots compose in
`%binds`. Working doc: `film/w5-scatter.st.md`; behaviour gate:
`tests/score/scatter.test.st` (children receive distinct sampled positions within
the source box, seen RED against a collapsed sampler). Remaining in W5's arc:
shape forms as slots (`:shape --ring(…)`), the GPU path
(`@particles(shape: &tag)`), and resample-on-resize.

---

## 7. Randomness and noise as data

### The problem

Dust motes were placed with `mod(sibling-index() * 83, 100)` — a prime-number
idiom standing in for "random". Camera shake was seven typed stops standing in
for "noise".

### The words

A **seed** is stamped at compile time, deterministic per element:

```st src
.dust i {
  --seed: random();                         // 0..1, fixed per element, stable across builds
  left: calc(var(--seed) * 100%);
  opacity: calc(0.2 + 0.6 * random(2));     // a second independent stream
}
```

`random()` in a static declaration is resolved by the compiler (seeded from the
element's document position, so a rebuild gives the same film). Because it is
data, `spacetime render` is still bit-identical run to run.

**Noise** is a motion form, driven like any other:

```st src
@form motion --noise($amp = 6px, $hz = 4) { translate-x: --noise-1d($amp, $hz); translate-y: --noise-1d($amp, $hz, seed: 2); }
.cam { @on &.clip --noise(amp: 8px); }
```

`--noise-1d` is a stdlib *value* form evaluated against clip progress: smooth,
periodic, deterministic. Shake, drift, flicker are all `--noise` with different
amplitudes on different properties.

### What this deletes

The `mod(sibling-index() * prime, 100)` idiom; hand-typed shake chains.

### — status —

**Shipped (PLAN-150 W6).** `random()` / `random(N)` in a static value lowers to a
per-element custom-property stamp (`src/compiler.rs`: `rewrite_random_calls` +
`emit_random_stamp_js`), seeded from the element's document index — per-element,
independent per stream, and stable across builds (bit-identical render). The
stamp rides the `registerSelectorInit` rail, so it also covers nodes added later
(`@each` output). `--noise-1d($amp, $hz[, seed:])` is a progress-driven motion
value: `stdlib/primitives/animation/apply-animations.st` detects it in a static
property and drives it on the clip watch (deterministic layered sines) instead of
applying it once — the same engine as keyframes, no new interpolation path.
Working doc: `film/w6-random-noise.st.md`; behaviour gates:
`tests/score/random.test.st` (distinct deterministic per-element seeds, two
independent streams) and `tests/score/noise.test.st` (the transform changes
across the window as noise evaluates) — both seen RED against the pre-fix code.
Remaining in W6's arc: N-D noise, `random(min, max)` ranges.

---

## 8. Audio is on the score

### The problem

The promo's six hits are locked to a wav by hand-placed clip times, and the
finished film needs a manual `ffmpeg` step. The clock knows nothing about sound.

### The words

Sound is a score entity with `at` and `for`; marks on the score are audio
markers; `render` muxes.

```st src
.stage {
  @score &.time(30s) as &film {
    @audio(src: "assets/promo.wav") as &bed at 0s;
    mark &hit1 at 2.2s;
    mark &hit2 at 6.4s;

    &open at 0s for 3.6s;
    &flash1 at &hit1 for 0.5s;             // a mark is a position
  }
}
&bed { @on &.clip { gain: 0 -> 1 at 3% -> 1 -> 0 at 96%; } }
```

`gain`, `rate`, `pan` are keyframeable on an audio entity. **One model**: the
score is truth. In the browser, `&bed` is an `<audio>` element on the same
transport (seek follows the virtual clock; play follows `.time`). Under
`render`, the track is muxed at its `at` offset with its gain curve applied by
ffmpeg — and a conformance test asserts the two backends agree on every mark to
within one frame.

### What this deletes

The manual mux; `&h1 at 2.08s` arithmetic (a mark plus a positioned stop says
"peak on the hit").

### — status —

**Shipped (PLAN-150 W7), browser-play half.** `@audio(src:) as &name at <t>;` is
a SCORE ENTRY (`stdlib/macros/score.st` grammar; `src/pipeline/score.rs::audio_entries`
emits a `score-audio` bind). The `score-audio` primitive
(`stdlib/primitives/score-audio.st`) creates a hidden `<audio>` on the score's
ONE transport: it seeks to `progress·total−at` as the score advances and plays
while its window is live — the score, not the audio, owns the clock. `gain`/`rate`
are keyframed on the named entity via `@on &.clip` (the W4 stage-object rail
reused for media). `@audio` outside a `@score` body is a hard error steering to
the score. This did NOT need the `.playback` driver (BUG-306/310/311 are the
inverse — video DRIVES score); a `.time` score plays audio directly. Working doc:
`film/w7-audio.st.md`; behaviour gate: `tests/score/audio.test.st` (the audio
element exists with the right src and its transport-derived seek target advances
with the clock, seen RED with the seek frozen). Remaining in W7's arc: the
`render` MUX (ffmpeg audio track at the `at` offset with the gain curve), `pan`,
and a two-backend conformance test.

---

## 9. Scenes and transitions

### The problem

Overlapping windows are a crossfade by accident. A cut, a wipe, a whip-pan have
no spelling.

### The words

`->` already means *sequence* in a `@score` (SIP-001 §4). A **transition** is a
form placed on the arrow:

```st src
@form transition --crossfade($d = 400ms) {
  &from { opacity: 1 -> 0; easing: --ease-in-out-quad; }
  &to   { opacity: 0 -> 1; easing: --ease-in-out-quad; }
}
@form transition --whip-pan($d = 300ms) {
  &from { translate-x: 0 -> -100vw; blur: 0 -> 12px; easing: --ease-in-expo; }
  &to   { translate-x: 100vw -> 0;  blur: 12px -> 0; easing: --ease-out-expo; }
}

.stage {
  @score &.time(30s) {
    &open for 3.6s -> --crossfade(400ms) -> &grid for 4.6s -> --whip-pan -> &code for 5.4s -> &drivers;
  }
}
```

A transition form has two region params, `&from` and `&to`; its body is two
`@on`-style blocks. Its duration overlaps the two windows it joins (the outgoing
clip's tail and the incoming clip's head both get the transition's `$d`). A bare
`->` with no form is a **cut**. Because a transition is a form with element
params, a route change can use the same one: `@on $route "/pricing" => --whip-pan;`.

Named groups of shots are score forms, which already exist:

```st src
@form score --act-one { &open for 3.6s -> --crossfade -> &grid for 4.6s; }
@form score --act-two($items) { &code for 5.4s -> &drivers(items: $items) for 6s; }
.stage { @score &.time(30s) { --act-one; -> --whip-pan -> --act-two($items); } }
```

### What this deletes

Hand-overlapped windows; the promo's `opacity: 0 -> 1 -> 1 -> 1 -> 1 -> 0`
envelope on every scene (a transition supplies the ends).

### — status —

**Shipped (PLAN-150 W8).** `transition` is a form kind
(`stdlib/macros/form.st` + `stdlib/capture-types/transition-body.st`): its body
is two region blocks `&from { … } &to { … }`. A transition placed on a score
`->` lowers to a `transition-edge` bind (`stdlib/primitives/transition-edge.st`,
emitted from `src/pipeline/score.rs::transition_edges`) that plays `&from` over
the outgoing clip's window tail and `&to` over the incoming clip's window head
— driven by the clips' own `__clip_<name>` signals, no new interpolation path.

Landing this fixed a FOUNDATIONAL gap: a score arrow chain `&a -> &b -> &c` kept
only `&a` because the repeated capture group `( "->" $next )*` surfaced no name
and its results were discarded (`src/syntax/events/extractors/custom.rs`). So
**`->` sequencing itself** now works, with transitions on the arrows. Working
doc: `film/w8-transitions.st.md`; behaviour gate: `tests/score/transition.test.st`
(the outgoing clip's opacity falls across the boundary, seen RED with the edge
emission disabled). Remaining in W8's arc: the transition `$d` refining the
overlap, per-region `easing:`, route-change transitions, and fragment-joined
transitions.

---

## 10. Shots: forms that carry markup *and* motion

### The problem

The same card reveal was pasted six times. A "component with a timeline" had
no single spelling.

### The word

Forms already come in kinds. A **shot** is a form whose body has a markup slot
and a motion slot, using the `:slot` shape `@each` uses:

```st src
@form shot --card-reveal($title, $price, $rise = 40px) {
  :markup {
    <div class="card">
      <div class="img"><i class="mug"></i></div>
      <div class="row"><span class="nm">`$title`</span><span class="price">`$price`</span></div>
    </div>
  }
  :motion {
    opacity: 0 -> 1; translate-y: $rise -> 0; easing: --ease-out-expo;
    .price { color: #8a94a6 -> #ffb454; range: 60% to 100%; }
  }
}

.panels {
  @score &.clip { &card-reveal("Ceramic Mug", "$24.99") for 100%; }
}
```

Placing a shot on a score instantiates its markup at that point and binds its
motion to that window. Outside a score, `&card-reveal(…)` is the ordinary
`@template` call; the markup slot IS a template. `:motion` is the same body an
`@on &.clip` takes, so anything in §2–§7 works inside it — including `@post`
and camera forms.

### What this deletes

Six pasted card blocks in the promo; the split between "the thing" and "how it
enters".

### — status —

**Shipped (PLAN-150 W9), invocation.** `shot` is a form kind
(`stdlib/macros/form.st` + `stdlib/capture-types/shot-body.st`): its body has a
`:markup` slot and a `:motion` slot. Invoking `&card-reveal(…)` instantiates the
markup (a template, params interpolated) and runs the motion as a mount reveal —
the shot binds `register-template` with the markup as body and the motion as
`animations:`; the `--form` / `&template` names are normalised to one key. Landing
this also fixed a DEAD reference: `@template`'s `animations:` called
`Spacetime.applyAnimations`, which was never defined, so template-attached motion
had silently never run — it is defined now (`public/runtime/templates.js`). Working
doc: `film/w9-shots.st.md`; behaviour gate: `tests/score/shot.test.st` (the
markup renders with interpolated params and the motion drives its opacity up,
seen RED with the reveal disabled). Remaining in W9's arc: placing a shot on a
`@score` window (motion binds the clip rather than a mount reveal), and per-region
nested motion.

---

## 11. Text as geometry

### The problem

`@reveal` splits live type into characters and words. A film also wants type
that *draws itself* — a stroke that travels along the letterform — and type that
particles can be sampled from (§6).

### The word

`@reveal` gains an `outline` mode. The text stays text (selectable, accessible);
the compiler adds an SVG twin whose paths come from the font, and the ordinary
keyframe engine drives it:

```st src
h1.mark {
  @reveal(split: outline, by: chars) {
    @on &.clip {
      draw: 0 -> 1; stagger: 40ms; easing: --ease-in-out-cubic;
      fill: 0 -> 1 at 70% -> 1;
    }
  }
}
```

`draw` is dash-offset along each glyph's path; `fill` fades the real text in over
the outline. Because outlines are paths, `@scatter(from: &mark)` can sample
them without rasterising.

### — status —

**Shipped (PLAN-150 W10), SVG twin.** `@reveal(split: "outline")`
(`stdlib/primitives/reveal.st`) keeps the text (an sr-only twin stays accessible)
and adds an SVG `<text>` twin in the element's own font, rendered as strokable
geometry. `draw` animates `stroke-dashoffset` (the stroke travels along the
letterforms); `fill` animates `fill-opacity` (the real fill fades in over the
outline). Driven by a rAF loop setting the inline style each frame — WAAPI on an
SVG `stroke-dashoffset` does not reliably reflect through `getComputedStyle`, so
the inline path is the deterministic one. No font parsing: an SVG `<text>` IS the
glyph geometry. Working doc: `film/w10-outline.st.md`; behaviour gate:
`tests/score/outline.test.st` (the twin builds, the text stays accessible, and
the stroke draws on — seen RED with the draw frozen). Remaining in W10's arc:
per-glyph draw with stagger, draw easing, and true font-to-path for `@scatter`
sampling without the SVG twin.

---

## 12. A shipped film vocabulary

Everything in a title sequence is five moves. They ship as stdlib forms so a
film's first draft is one line per shot:

```st src
@form motion --focus-pull($from = 24px) { blur: $from -> 0; letter-spacing: 0.3em -> 0.01em; translate-y: 16px -> 0; opacity: 0 -> 1; easing: --ease-out-expo; }
@form motion --shimmer($width = 16%) { --pos: -120% -> 230%; easing: --ease-in-out-sine; }
@form motion --typewriter($chars) { --chars: 0 -> $chars; easing: --steps($chars); }
@form motion --glitch($amount = 14px) { translate-x: 0 -> $amount -> calc(-0.7 * $amount) -> 0; skew-x: 0deg -> 3deg -> -2deg -> 0deg; hue-rotate: 0deg -> 40deg -> -30deg -> 0deg; }
@form post   --cinema { bloom: 0.45; grain: 0.035; vignette: 0.55; aberration: 0.002; }
@form camera --shake($amount = 8px) { position: 0 0 0 -> $amount 0 0 -> calc(-0.75 * $amount) 0 0 -> 0 0 0; }

h1.title { @on &.clip --focus-pull; }
.stage   { @post --cinema; @on &.clip --shake; }
```

These are not new syntax. They are the promo's repeated patterns, named. The
list lives in `stdlib/macros/film.st` and grows by adding forms.

### — status —

**Shipped (PLAN-150 W12).** The vocabulary lives in `stdlib/film`
(`@import "stdlib/film"`): `--focus-pull`, `--shimmer`, `--typewriter`,
`--glitch`, `--drift` (motion), `--cinema` (post), `--shake`, `--push-in`
(camera) — each an ordinary `@form` built only from the landed surface,
consumed as any form is (`@on &.clip { --focus-pull; }`). W12 also added the
`post` form kind so a `@post` look can be named. Working doc:
`film/w12-vocabulary.st.md`; behaviour gate: `tests/score/film-vocab.test.st`
(an imported `--focus-pull` drives the title's opacity up, seen RED with its
opacity stop removed). Remaining in W12's arc: consuming a named `post` look via
`@post --cinema;` (the bare-form statement gap shared with the camera), and more
moves as the promo demands them.

---

## 13. Tooling: stills, ranges, scrub, targets

### The problem

A two-frame-per-second preview of a 30 s page took 66 s. Iterating a film by
rendering it end to end is how a 20-minute job becomes a day.

### The words

`spacetime render` is the film's build; it gets the flags a build has.

```
spacetime render demos/promo/ --stills 1.5,9,24 --out scratch/stills/     # three PNGs, ~5 s
spacetime render demos/promo/ --from 18s --to 23s --out s5.mp4             # one scene
spacetime render demos/promo/ --sheet 5x6 --out sheet.png                  # a contact sheet
spacetime render demos/promo/ --dpr 2 --out film-4k.mp4                    # 3840×2160
spacetime render demos/promo/ --out film.webm | film.gif | frames/          # other targets
spacetime render demos/promo/ --settle 16ms                                # per-frame paint settle (default 40)
spacetime serve demos/promo/ --scrub                                       # transport bar, drag to seek
```

`--scrub` puts a transport on the served page: play, pause, a draggable
playhead, frame step. It drives the same virtual clock `render` uses, so what
you scrub is what you render. Every page with a `&.time` score gets it for free.

`--out film.lottie` is the long-range target: a score whose consumers are all
DOM keyframes is expressible as Lottie JSON, which makes a Spacetime film
portable to places that cannot run a browser.

### — status —

**Shipped (PLAN-150 W11), stills + ranges + dpr.** `spacetime render` gains
`--stills <csv>` (capture specific timestamps as PNGs, no video mux — the fast
iteration loop; `src/render.rs::render_stills`), `--from`/`--to` (render one time
window), and `--dpr <n>` (render at `width·dpr × height·dpr`, e.g. a 4K render of
a 1080p film). All ride the existing virtual-clock render engine, so a still is
the page at that exact deterministic instant and the film renders bit-identically
run to run. Working doc: `film/w11-render-tooling.st.md`; behaviour gates:
`tests/render/render_test.rs` (`w11_stills_capture_requested_timestamps_that_differ`,
`w11_dpr_scales_the_captured_pixels`, `w11_from_to_window_trims_the_render`).
Remaining in W11's arc: `--sheet` (contact sheet), `serve --scrub` (transport
bar), and the `webm`/`gif`/`lottie` targets.

---

## 14. What stays exactly as it is

Worth saying, because the promo proved these already carry a film:

- `sibling-index()` with CSS trigonometry is a particle system. It stays; §6 and
  §7 feed it better numbers.
- A CSS counter is a frame counter (`counter-reset: fr calc(round(var(--fr)))`).
- `clip-path: inset(… var(--chars) …)` is a typewriter.
- `background-clip: text` with an animated `--pos` is a shimmer.
- `range: a to b` inside a nested selector is how one clip choreographs many
  children at different times.
- Determinism costs nothing: the virtual clock makes any page a film.

## 15. The promo, rewritten

With every section above landed, the first scene of `demos/promo/index.st`
reads:

```st src
@form camera --push-in { position: 0 0.6 10.5 -> 0 0.6 9.4; easing: --ease-in-out-cubic; }

.s1 {
  @on &.clip {
    --push-in;
    .orb   { scale: 0.15 -> 1.9 at 60% --ease-out-cubic -> 1; opacity: 0 -> 1 at 20% -> 1 -> 0; }
    .dust  { --t: 0 -> 1; }
    .dust i { --seed: random(); translate: --noise(90px, 0.5); }
    .t1    { --focus-pull; }
    @post  { bloom: 0.45 -> 1.3 at 61% -> 0.45; flash: 0 -> 0.85 at 61% -> 0 at 63%; }
  }
}
```

Nine lines, one hit, one camera, one lens. The whole film is about a hundred.
