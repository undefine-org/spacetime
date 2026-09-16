```st hidden
// The film sits ABOVE the title: the renderer captures the viewport from the
// top of the document, so the stage is the first element in it. Its program
// is the §0 fence below; only the skeleton lives here.
<div class="stage">
  <div class="scene sc1">
    <span class="fk">SIP-001 · the timeline release</span>
    <h1 class="fh dsp">One score</h1>
  </div>
  <div class="scene sc2">
    <h1 class="fh dsp">Every clock</h1>
  </div>
  <div class="scene sc3">
    <h1 class="fh dsp fg">No JavaScript</h1>
  </div>
  <div class="scene sc4">
    <h1 class="fh dsp fs">Spacetime</h1>
    <span class="fk">render → film.mp4</span>
  </div>
</div>
```

# One score, every clock — the catalog

This page is a **literate Spacetime document**. The prose is markdown; the
fences are not illustrations, they *are* this page's program. Everything you
see move was compiled from the block printed beside it.

It exists to make one claim checkable:

> A website and a video differ only in **which signal drives the timeline**.

And one claim *visible*: the effects a video catalog sells — cover, dissolve,
iris, glitch, flash, kinetic type, dolly, shimmer — are not a library here.
They are **clip placements + keyframe bodies**, and the same fence that plays
them in your browser renders them to an mp4:

```
cargo run -- serve demos/score-launch/                      # the page
cargo run --features cdp -- render demos/score-launch/ --out scratch/score-launch.mp4   # the film
```

The film is the first thing on the page, because the renderer captures the
viewport from the top. Scroll past it and the page becomes the catalog that
explains it.


```st hidden
// ---------------------------------------------------------------------------
// Setup: brand values and the base sheet. Everything visual below derives
// from these six colors and two type stacks.
// ---------------------------------------------------------------------------

@type Brand {
  ink:    color;
  bg:     color;
  panel:  color;
  accent: color;
  violet: color;
  dim:    color;
  rule:   color;
}

@data inline $brand Brand : {
  "ink":    "#e8eef7",
  "bg":     "#07090d",
  "panel":  "#0e131b",
  "accent": "#5eead4",
  "violet": "#a78bfa",
  "dim":    "#8b96a8",
  "rule":   "#1f2937"
};

body {
  background: $brand.bg;
  color: $brand.ink;
  font-family: system-ui, sans-serif;
  line-height: 1.65;
  margin: 0;
  padding: 0 1.5rem;
}

h1, h2, h3 { line-height: 1.15; }
p, ul, ol, pre, h1, h2, h3, table { max-width: 62rem; margin-left: auto; margin-right: auto; }
code { background: $brand.panel; padding: 0.15em 0.4em; border-radius: 4px; }

// The display stack: condensed and heavy. Body stays system.
.dsp {
  font-family: "Avenir Next Condensed", "Futura", "Century Gothic", "Trebuchet MS", sans-serif;
  font-weight: 800;
  text-transform: uppercase;
  letter-spacing: 0.02em;
}
.mono { font-family: ui-monospace, "SF Mono", Menlo, monospace; }
```


## 0 · The film

Twelve seconds, four scenes, one `@score`. The sequence itself is playing at
the top of this page — it had to be the first thing in the document, because
the renderer captures the viewport from the top. Its skeleton is six lines:

```html
<div class="stage">
  <div class="scene sc1"><span class="fk">…</span><h1 class="fh dsp">One score</h1></div>
  <div class="scene sc2"><h1 class="fh dsp">Every clock</h1></div>
  <div class="scene sc3"><h1 class="fh dsp fg">No JavaScript</h1></div>
  <div class="scene sc4"><h1 class="fh dsp fs">Spacetime</h1><span class="fk">…</span></div>
</div>
```

Everything else — the playhead and all four scenes — is the fence below.
`spacetime render` walks the same playhead under a virtual clock and hands
ffmpeg the frames.

```st src
.stage {
  position: relative;
  height: 100vh;
  min-height: 30rem;
  max-width: none;
  margin: 0 -1.5rem;
  overflow: hidden;
  // NB single line: a line break inside this value trips E0952 (emitter bug).
  // NB background-image, not background: a reactive `$` value lands via
  // setProperty, and the shorthand would reset longhands like clip/size.
  background-image: radial-gradient(60% 50% at 50% 42%, rgba(94, 234, 212, 0.07), transparent 70%);
  background-color: $brand.bg;

  @score &.loop(12s) as &film {
    &sc1 at 0s for 3s;
    &sc2 at 3s for 3s;
    &sc3 at 6s for 3s;
    &sc4 at 9s for 3s;
  }
}

.scene {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 1rem;
  opacity: 0;
}

.fh { font-size: clamp(3rem, 9vw, 7.5rem); margin: 0; }
.fk {
  color: $brand.dim;
  font: 600 0.85rem/1 ui-monospace, Menlo, monospace;
  letter-spacing: 0.32em;
  text-transform: uppercase;
}

/* Scene 1 — focus pull + tracking. Blur and letter-spacing are numbers with
   units, so they keyframe like anything else. */
.sc1 {
  @on &.clip {
    opacity: 0 -> 1 -> 1 -> 0;
    .fh {
      blur: 14px -> 0 -> 0 -> 8px;
      letter-spacing: 0.3em -> 0.04em;
      translate-y: 18px -> 0;
      easing: --ease-out-expo;
    }
    .fk { opacity: 0 -> 1; translate-y: 10px -> 0; }
  }
}

/* Scene 2 — iris wipe. The score animates a CUSTOM PROPERTY; the clip-path
   just reads it. One mechanism, and any CSS shape that takes a var() joins
   the vocabulary. */
.sc2 {
  clip-path: circle(calc(var(--iris, 0) * 1%) at 50% 50%);
  background: $brand.panel;
  @on &.clip {
    --iris: 0 -> 75 -> 75 -> 0;
    opacity: 0 -> 1 -> 1 -> 0;
    .fh { translate-y: 26px -> 0; easing: --ease-out-expo; }
  }
}

/* Scene 3 — glitch. Multi-stop keyframes are a jitter budget: the first
   third shakes, the rest settles. Hue and skew ride along. */
.sc3 {
  @on &.clip {
    opacity: 0 -> 1 -> 1 -> 1 -> 0;
    .fg {
      translate-x: 0 -> 12px -> -9px -> 5px -> 0 -> 0;
      skew-x: 0deg -> 7deg -> -5deg -> 2deg -> 0deg -> 0deg;
      hue-rotate: 0deg -> 110deg -> -60deg -> 25deg -> 0deg -> 0deg;
    }
  }
}

/* Scene 4 — scale punch with an expo ease, then a shimmer sweep across the
   wordmark: a gradient clipped to text, its position driven by --pos. */
.sc4 {
  @on &.clip {
    opacity: 0 -> 1 -> 1 -> 1;
    .fs { scale: 1.7 -> 1; easing: --ease-out-expo; --pos: -120% -> 230%; }
    .fk { opacity: 0 -> 1; range: 0.4 to 1; }
  }
}

.fs {
  background-image: linear-gradient(100deg, $brand.ink 42%, $brand.accent 50%, $brand.ink 58%);
  background-size: 260% 100%;
  background-position: var(--pos, -120%) 0;
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
}
```

Four scenes, and not one of them names a clock. `at 0s for 3s` is placement;
what *drives* the placement is the driver's business — `&.loop` here, a
virtual render clock when ffmpeg is watching.


## 1 · Kinetic type is arrangement

`@on` is choreography: one element animating on its own driver. `@score` is
**arrangement** — several elements sharing one playhead, each owning a window.

The placements are trailing words, not arguments. `at 0s for 3s` is the
*language's* vocabulary for where a clip sits; anything in parens would belong
to the clip's own function. That split is what lets a timeline editor drag a
clip and rewrite `at 2s` to `at 2.4s` in your source: the editor is a second
view of the language, not a layer on top of it.

```st src
@form motion --clip-rise { opacity: 0 -> 1; }

<div class="reel">
  <div class="card title">A score is an arrangement</div>
  <div class="card body">Three clips, one playhead</div>
  <div class="card cta">Nothing here is JavaScript</div>
</div>

.reel {
  position: relative;
  min-height: 13rem;
  padding: 1.2rem;
  border: 1px solid $brand.rule;
  border-radius: 10px;
  background: $brand.panel;

  @score &.loop(9s) as &reel {
    &title at 0s for 3s;
    &body  at 3s for 3s;
    &cta   at 6s for 3s;
  }
}

.card {
  position: absolute;
  left: 1.2rem;
  right: 1.2rem;
  top: 42%;
  padding: 0.9rem 0;
  border-top: 2px solid $brand.accent;
  color: $brand.ink;
  font: 600 1.15rem/1.4 "Avenir Next Condensed", "Futura", sans-serif;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  opacity: 0;
}

/* A clip's window is consumed by name: `&title` in the score must match this
   element's own class, so the `&.clip` driver watches __clip_title here. */
.title { @on &.clip: --clip-rise; }
.body  { @on &.clip: --clip-rise; }
.cta   { @on &.clip: --clip-rise; }
```

Stagger is the same idea at finer grain: one clip, and the body fans out over
the element's children. The words below share a single window — `stagger`
spreads their *local* starts across it.

```st src
<div class="kine">
  <span class="wk">type</span>
  <span class="wk">moves</span>
  <span class="wk">because</span>
  <span class="wk">data</span>
  <span class="wk">does</span>
</div>

.kine {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5em;
  padding: 2rem 1.2rem;
  border: 1px solid $brand.rule;
  border-radius: 10px;
  background: $brand.panel;
  font: 800 clamp(1.6rem, 4.5vw, 3rem)/1.1 "Avenir Next Condensed", "Futura", sans-serif;
  text-transform: uppercase;

  @score &.loop(6s) {
    &kine at 0s for 6s;
  }

  /* The container consumes its own window; the body fans out over its
     children — stagger spreads their LOCAL starts across the one window. */
  @on &.clip {
    .wk {
      opacity: 0 -> 1 -> 1 -> 0;
      translate-y: 0.6em -> 0;
      blur: 6px -> 0;
      easing: --ease-out-expo;
      stagger: 120ms;
      staggerFrom: first;
    }
  }
}

.wk {
  display: inline-block;
  opacity: 0;
}
```


## 2 · Transitions are windows, not widgets

A transition is two clips with adjacent windows and an opinion about the
hand-off. Every family a video catalog ships as a preset is, here, one line of
placement and a keyframe body — cover, dissolve, iris, glitch:

```st src
<div class="txgrid">
  <div class="txs cov">
    <div class="panel pa cova dsp">Cover</div>
    <div class="panel pb covb dsp">Push</div>
  </div>
  <div class="txs dis">
    <div class="panel pa disa dsp">Dissolve</div>
    <div class="panel pb disb dsp">Blur</div>
  </div>
  <div class="txs irs">
    <div class="panel pa irsa dsp">Iris</div>
    <div class="panel pb irsb dsp">Circle</div>
  </div>
  <div class="txs glt">
    <div class="panel pa glta dsp">Glitch</div>
    <div class="panel pb gltb dsp">Cut</div>
  </div>
</div>

.txgrid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1rem;
}

.txs {
  position: relative;
  height: 9rem;
  overflow: hidden;
  border: 1px solid $brand.rule;
  border-radius: 10px;
  background: $brand.panel;
}

.panel {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 1.7rem;
  letter-spacing: 0.08em;
  opacity: 0;
}

.pa { background: $brand.panel; color: $brand.accent; }
.pb { background: #101726; color: $brand.violet; }

/* COVER / PUSH — B slides in over A. Disjoint windows; B's own motion is
   the whole transition. */
.cov { @score &.loop(4s) { &cova at 0s for 2s; &covb at 2s for 2s; } }
.cova { @on &.clip { opacity: 0 -> 1 -> 1 -> 1; } }
.covb { @on &.clip { opacity: 1; translate-x: 100% -> 0; easing: --ease-out-expo; } }

/* DISSOLVE / BLUR — A defocuses out while B defocuses in. Both clips span the
   whole loop with mirrored multi-stop windows, so the cross-dissolve is one
   mechanism, not a scheduler. */
.dis { @score &.loop(4s) { &disa at 0s for 4s; &disb at 0s for 4s; } }
.disa { @on &.clip { opacity: 1 -> 1 -> 0 -> 0 -> 1; blur: 0px -> 0px -> 12px -> 12px -> 0px; } }
.disb { @on &.clip { opacity: 0 -> 0 -> 1 -> 1 -> 0; blur: 12px -> 12px -> 0px -> 0px -> 12px; } }

/* IRIS — the custom-property wipe again, as a transition this time. */
.irs { @score &.loop(4s) { &irsa at 0s for 2s; &irsb at 2s for 2s; } }
.irsa { @on &.clip { opacity: 0 -> 1 -> 1 -> 1; } }
.irsb {
  clip-path: circle(calc(var(--irsb, 0) * 1%) at 50% 50%);
  @on &.clip { opacity: 1; --irsb: 0 -> 78; easing: --ease-in-out-circ; }
}

/* GLITCH CUT — B arrives through a shake-and-settle jitter. */
.glt { @score &.loop(4s) { &glta at 0s for 2s; &gltb at 2s for 2s; } }
.glta { @on &.clip { opacity: 0 -> 1 -> 1 -> 1; } }
.gltb {
  @on &.clip {
    opacity: 1;
    translate-x: -14px -> 10px -> -6px -> 3px -> 0px;
    hue-rotate: 90deg -> -50deg -> 20deg -> 0deg -> 0deg;
  }
}
```

Each loop seams with a hard cut — which is honest: a cut is the zeroth
transition, and it costs no keyframes at all.


## 3 · Color and light are values too

Interpolation is not limited to geometry. Colors interpolate (rgb or oklch),
filters are numbers, and a custom property can steer a gradient — so light
effects are the same two lines as everything else:

```st src
<div class="luxrow">
  <div class="lux tint dsp">Tint</div>
  <div class="lux hues dsp">Hue</div>
  <div class="lux neon dsp">Neon</div>
  <div class="lux shim"><span class="shimt dsp">Shimmer</span></div>
</div>

.luxrow {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 1rem;

  @score &.loop(6s) {
    &tint at 0s for 6s;
    &hues at 0s for 6s;
    &neon at 0s for 6s;
    &shim at 0s for 6s;
  }
}

.lux {
  height: 7rem;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid $brand.rule;
  border-radius: 10px;
  background: $brand.panel;
  font-size: 1.15rem;
  letter-spacing: 0.1em;
}

/* Color keyframes: the palette itself is the animation. */
.tint { @on &.clip { background: #0e131b -> #134e4a -> #5eead4 -> #134e4a -> #0e131b;
                     color: #5eead4 -> #e8eef7 -> #06251f -> #e8eef7 -> #5eead4; } }

/* A full hue orbit over a static gradient. */
.hues {
  background-image: linear-gradient(120deg, $brand.accent, $brand.violet);
  color: #07090d;
  @on &.clip { hue-rotate: 0deg -> 360deg; }
}

/* Neon pulse: brightness past 1 is the glow. */
.neon {
  color: $brand.accent;
  text-shadow: 0 0 18px rgba(94, 234, 212, 0.55);
  @on &.clip { brightness: 1 -> 2.1 -> 1 -> 2.1 -> 1; }
}

/* Shimmer: the film's wordmark trick, isolated. */
.shimt {
  background-image: linear-gradient(100deg, $brand.dim 42%, $brand.ink 50%, $brand.dim 58%);
  background-size: 260% 100%;
  background-position: var(--pos2, -120%) 0;
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
}
.shim { @on &.clip { --pos2: -120% -> 230%; } }
```


## 4 · Depth without a 3D engine

`rotate-x`, `rotate-y` and a parent `perspective` are all the camera this
catalog needs. A flip and a dolly:

```st src
<div class="depthrow">
  <div class="stage3d">
    <div class="flip dsp">Flip</div>
  </div>
  <div class="stage3d">
    <div class="dolly dsp">Dolly</div>
  </div>
</div>

.depthrow {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1rem;

  @score &.loop(5s) {
    &flip  at 0s for 5s;
    &dolly at 0s for 5s;
  }
}

.stage3d {
  height: 9rem;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid $brand.rule;
  border-radius: 10px;
  background: $brand.panel;
  perspective: 700px;
}

.flip, .dolly {
  font-size: 2.2rem;
  letter-spacing: 0.08em;
  color: $brand.ink;
  opacity: 0;
}

.flip {
  transform-origin: left center;
  color: $brand.violet;
  @on &.clip {
    opacity: 0 -> 1 -> 1 -> 0;
    rotate-y: -95deg -> 0deg;
    easing: --ease-out-expo;
  }
}

.dolly {
  @on &.clip {
    opacity: 0 -> 1 -> 1 -> 0;
    scale: 0.5 -> 1;
    blur: 10px -> 0px;
    easing: --ease-out-quart;
  }
}
```


## 5 · `rate` is the derivative

Speed ramp, freeze and reverse look like three features. They are one: `rate`
is the *derivative* of the window map, so all three are shapes of a single
integral.

```st src
@form motion --dot-move { translate-x: 0 -> 180px; }

<div class="rates">
  <div class="lane"><span class="dot normal">rate 1 — normal</span></div>
  <div class="lane"><span class="dot fast">rate 2 — finishes early, holds</span></div>
  <div class="lane"><span class="dot slow">rate 0.5 — never finishes</span></div>
  <div class="lane"><span class="dot back">rate -1 — runs backward</span></div>
</div>

.rates {
  margin-top: 1.4rem;
  padding: 1.2rem;
  border: 1px solid $brand.rule;
  border-radius: 10px;
  background: $brand.panel;

  @score &.loop(4s) {
    &normal at 0s for 4s;
    &fast   at 0s for 4s rate 2;
    &slow   at 0s for 4s rate 0.5;
    &back   at 0s for 4s rate -1;
  }
}

.lane {
  height: 2.2rem;
  margin: 0.35rem 0;
  border-radius: 6px;
  background: #0a0e15;
}

.dot {
  display: inline-block;
  padding: 0.35rem 0.7rem;
  border-radius: 5px;
  background: $brand.accent;
  color: #06251f;
  font: 600 0.85rem/1 ui-monospace, monospace;
  translate-x: 0;
}

.normal { @on &.clip: --dot-move; }
.fast   { @on &.clip: --dot-move; }
.slow   { @on &.clip: --dot-move; }
.back   { @on &.clip: --dot-move; }
```

`rate 2` reaches the end of its own progress halfway through its window and
holds there. `rate 0.5` never arrives. `rate -1` walks backward from 1. None of
these is a special case — they are one expression evaluated with different
constants, which is what "the derivative of the map" buys.


## 6 · The same body, under two clocks

Here is the thesis itself.

A score fragment is declared **once**. It names no driver, because the driver
is supplied at the `@score` head — which is also why a fragment needs no type
parameter to be reusable across projections. It is already
projection-agnostic, by omission.

```st src
@form score --story {
  &head for 50%;
  &tail for 50%;
}
```

Now that fragment runs under two different clocks. On the left it is driven by
**time**. On the right it is a **clip** inside a parent score: `&.clip`
declares `domain: inherited`, so it does not compute a window — it *receives*
the one its parent assigned, and its body never mentions seconds at all.

```st src
<div class="poles">
  <div class="pole by-time">
    <div class="lbl">&amp;.time — the website pole</div>
    <div class="head">head</div>
    <div class="tail">tail</div>
  </div>

  <div class="pole film">
    <div class="lbl">&amp;.clip — the video pole</div>
    <div class="shot">
      <div class="head">head</div>
      <div class="tail">tail</div>
    </div>
  </div>
</div>

.by-time {
  @score &.loop(6s) { --story; }
}

.film {
  @score &.loop(12s) as &film2 {
    &shot at 0s for 6s;
  }
}

.shot {
  @score &.clip { --story; }
}

.poles {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1rem;
  margin-top: 1.4rem;
}

.pole {
  padding: 1rem;
  border: 1px solid $brand.rule;
  border-radius: 10px;
  background: $brand.panel;
  min-height: 9rem;
}

.lbl {
  color: $brand.dim;
  font: 600 0.78rem/1 ui-monospace, monospace;
  margin-bottom: 0.8rem;
}

.head, .tail {
  padding: 0.6rem 0.8rem;
  margin-bottom: 0.4rem;
  border-top: 2px solid $brand.violet;
  background: #0a0e15;
  color: $brand.ink;
  font: 500 0.95rem/1.3 ui-sans-serif, system-ui, sans-serif;
  opacity: 0;
}

/* Single-class consumers so each pole's `&.clip` joins its OWN `__clip_head`
   from the nearest enclosing score (.by-time's clock vs .shot's inherited one). */
.head { @on &.clip: --clip-rise; }
.tail { @on &.clip: --clip-rise; }
```

That is the claim, executable: **the same arrangement, two clocks.** And it is
gated, not asserted — `tests/score/two-pole.test.st` proves in a real Chromium
that both poles march in lockstep, and `tests/score_window_map_test.rs` proves
the same body lowers to the same window map under `&.time` and `&.clip`.


## 7 · What this page proves, and what it does not

Being honest about the boundary is part of the claim.

**Proven, in a real browser, by gates that mount this page's own fences:**

- The film at the top moves scene by scene — `tests/score/score-launch-film.test.st`
  samples a full 12s cycle and asserts sc1 moves and rests, that `--iris` sweeps
  only inside sc2's window, that the cover panel is offstage before its window
  and slides inside it, and that the tint chip's color leaves its first stop.
- The reel (§1), the rate lane (§5) and the two-pole composition (§6) move —
  `tests/score/score-launch.test.st` and `tests/score/two-pole.test.st` (the
  gates that closed BUG-258, BUG-266 and BUG-267).
- The page *renders*: `spacetime render` (W5, PLAN-132) walks this page's
  longest score under a virtual clock and writes an mp4 — the film at the top
  is its twelve seconds, verified by `tests/render/render_test.rs` and by a
  smoke render of this very page.

**Not proven here:**

- The remaining catalog specimens (§2's dissolve/iris-cut/glitch stages, §3's
  hue/neon/shimmer chips, §4's flip and dolly) are compile-checked and built
  only from mechanisms the gates above cover — clip windows, custom-property
  keyframes, color interpolation, nested scopes — but are not individually
  gated. If one stops moving, that is a bug to file, not a behavior this page
  promises.
- `&.steps` and `&.playback` scores are still `planned:` — the deck and the
  music video are the same grammar, waiting on their drivers.
- `hold(until:)` is deliberately not score vocabulary. It stops the playhead,
  which is a property of the transport, not of the arrangement.

Compile-time coverage lives in `tests/score_window_map_test.rs`,
`tests/clip_driver_test.rs` and `tests/score_domain_test.rs`. The behavioral
half is `tests/score/score-launch.test.st` and `tests/score/two-pole.test.st`
on the `--cdp` backend, and the render half is `tests/render/render_test.rs`.


```st hidden
<footer class="foot">
  <p>Spacetime · SIP-001 · one score, every clock</p>
</footer>

.foot {
  margin-top: 4rem;
  padding: 1.5rem 0 3rem;
  border-top: 1px solid $brand.rule;
  color: $brand.dim;
  font-size: 0.9rem;
}
```
