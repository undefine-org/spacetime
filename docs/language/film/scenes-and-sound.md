# Scenes and sound

*Working doc for PLAN-152. Ships today. Companion to `../film.st.md` §4, §5 and §8,
and to `render-engines.md`.*

A film can be a lit 3D world with a character and a soundtrack, written like any
other film: things on a stage, a score that places them in time, `@on &.clip`
bodies that say what changes. Three demos are built this way:
`demos/film-robot-day/`, `demos/film-disco/` and `demos/film-forum/`.

```
spacetime render demos/film-robot-day/ --engine raea -o robot-day.mp4   # GPU, no browser
spacetime render demos/film-robot-day/ -o robot-day.mp4                 # Chromium + three.js
spacetime serve demos/film-robot-day/                                   # live, with sound
```

## The stage is a list of parts

```
canvas.world {
  @stage(fov: 34, camY: 3.2, camZ: 14, shadows: "soft", background: "#1d1a2a", bloom: 0.6) {
    @light &sun (type: "directional", intensity: 2.4, color: #ffb47a, posX: -6, posY: 5, posZ: 3)
    @fog(color: #1d1a2a, near: 12, far: 34)
    @object &ground (shape: "plane", size: 18, rotateX: -90, color: #3a3448)
    @object (shape: "cylinder", size: 0.06, scaleY: 20, posY: 1.2)      // a lamp post
    @gltf &robot (src: "models/robot.glb", clip: "Idle", scale: 0.42)
    @grid &floor (cols: 12, rows: 12, pattern: "ripple")
  }
}
```

- **`@stage`** takes `shadows: "soft" | "hard"` (one switch: every light that can
  casts, every part casts and receives), `background` (the world's clear colour;
  match `@fog`'s) and `bloom` (glow around emissive parts and highlights).
- **Any child can be named** with `&name`. A named part answers to a score.
- **`@object`** shapes: box, sphere, plane, cylinder, cone, torus, coin (and
  icosahedron / torusknot, which raea draws as a sphere / torus). Rest
  `rotateX/Y/Z` (degrees) and `scaleX/Y/Z`.
- **`@gltf`** places a glTF model. Give it `scale:` (both engines agree on it;
  `size:` auto-fit needs a browser) and a starting `clip:`.
- **`@grid`** is a light-up floor: emissive tiles lit by a pattern (`checker`,
  `ripple`, `sweep`, `random`, `chase`, kept as data), with a `pulse` channel.

## A score moves parts

```
.shot {
  @on &.clip {
    &camera { posZ: 14 -> 6.5 at 62%; lookY: 1 -> 0.6; fov: 34 -> 30; }
    &robot  { clip: "Idle" -> "Walking" at 25% --steps(1) -> "Wave" at 46% --steps(1); posZ: -7 -> -1 at 46%; }
    &sun    { intensity: 2.4 -> 0.25 at 80%; }
    &floor  { pulse: 0 -> 1 at 20%; }
  }
}
```

The `@on` consumer must contain the canvas. Channels are a data table in
`stdlib/3d/primitives/three-stage.st`:

| channel | applies to |
|---|---|
| `posX/Y/Z`, `rotateX/Y/Z` (deg), `scale` | any part; `scale` multiplies the rest scale |
| `lookX/Y/Z`, `fov` | `&camera` |
| `intensity`, `color` | lights |
| `opacity`, `emissive`, `metalness`, `roughness`, `color` | meshes (their material) |
| `clip`, `rate` | models: the clip playing, and its speed |
| `near`, `far` | `@fog` |
| `pulse`, `phase` | `@grid` |

`clip` is a word, so it switches rather than blends. `--steps(1)` makes it
switch exactly at the stop's position instead of halfway between stops. Each clip
plays from the moment it began. The pose is a seek into the clip, so a render is
deterministic.

Camera forms (`@on &.clip --crane;`) move the real camera too: their `position` /
`target` / `fov` / `roll` axes are world values on a stage.

## Sound is notes on the same score

```
@form instrument --kick  { wave: sine; pitch: 150hz -> 42hz at 25%; gain: 1 -> 0; for: 420ms; }
@form instrument --pad   { wave: saw; voices: 3; detune: 14; gain: 0 -> 0.45 at 25% -> 0.45 at 80% -> 0; cutoff: 700; reverb: 0.7; }
@form pattern --beat($tempo = 96bpm, $step = 16) {
  --kick:  x . . . . . . . x . x . . . . . ;
  --pluck: C3 . . G3 . . Bb3 . . G3 . . F3 . Eb3 . ;
}
.reel {
  @score &.time(24s) as &film {
    @sound(--pad, notes: "C3 Eb3 G3 Bb3") at 0s for 24s;
    @sound(--beat) as &drums at 5.8s for 9.2s;
    @sound(--thud) at 15.3s;
  }
}
```

- An **instrument** is motion over one note's life, written in the same
  `a -> b at p%` stops as any keyframe. Vocabulary: `wave` (sine, triangle, saw,
  square, noise), `pitch` (Hz; when a note is given, the pitch envelope's shape at
  that note), `gain`, `cutoff`, `highpass`, `detune` (cents), `voices`, `pan`,
  `reverb` (send) and `for` (a note's length; a held chord lasts its window).
- A **pattern** has one row per instrument and one cell per step: `x` plays, `.`
  rests, a note name (`C2`, `Eb3`, `F#4`) plays that pitch. `$step = 16` means
  sixteenth notes. It loops for its window.
- **`@sound`** places either one on the score: `at` (and `for` for a held sound
  or a looped pattern), `notes:` for a chord, `as &name` to address it.

The synthesizer is one file, `stdlib/film/synth/synth.js`. In a browser it
renders the score's sound once and plays it in step with the score. `spacetime
render` runs the same file in V8 and writes the film's soundtrack, which both
engines mux: AAC beside Chromium's frames, the page's `<audio>` under raea's.
`@sound` needs `@import "stdlib/film"`.

## How a stage reaches each engine

Each part is a kind and props in raea's 3D vocabulary (`Mesh` +
`MeshStandardMaterial`, `DirectionalLight`, `Model`, `Fog`, `Group`, `Bloom`,
`PerspectiveCamera`), mirrored as hidden `<i data-3d data-props>` markup beside
the canvas.

- **Browser / Chromium:** a per-kind table builds and updates three.js objects
  from the props. The frame loop ticks through `ST._raf`.
- **raea:** the page runs with no WebGL (`ST._stage3d.renderer = "none"`). Every
  seek reports the page's `data-3d` tree. raea turns the film's frames into its
  scene rows, draws them with Bevy, and lays the page over the result
  (`stdlib/film/vendor/raea/runtime/export/src/html3d.rs`).

## One set of units, both engines

A stage's numbers are three.js's, the units the 3D websites are tuned in. raea
reads its own (`core/scene/src/props.rs` in the vendored engine), so the stage
converts what it hands raea:

| | three.js | raea |
|---|---|---|
| directional light `intensity` | 1 | 1 (8000 lux) |
| point / spot light `intensity` (candela) | I | I / 8.95 (lumens) |
| ambient light `intensity` | I | I × 6.37 (8000/π ÷ 400 cd/m²) |
| `@stage(exposure: E)` | ACES after a 1/0.6 gain | `log2(E) − 2.26` stops, ACES |
| `emissiveIntensity` | exposed | unexposed, so × the exposure gain |
| `@fog` colour, `background` with bloom | divided by the exposure gain | as written |

A film lit for raea's daylight sits near `exposure: 4.8` (raea's 0 stops), with
point lights in the tens of candela. The demo films are written that way and
render alike on both engines. Fonts ship with each film (`@font("Syne") { src:
url("fonts/Syne.woff2") … }`): raea fetches named families, Chromium only loads
what the page declares.

## Limits

- On raea the world lies under the whole page: the stage must be full-frame and
  back-most. Its ancestors' backgrounds are cleared there.
- A clip switches; it does not crossfade.
- Shading still differs at the extremes: a hard spot on a metallic surface
  reads hotter in three.js, and three.js's bloom adds light above a threshold
  where Bevy's blends (calibrated on `demos/film-disco`, not identical).
- A clip's age is exact to the virtual tick; a scrub in a live page restarts it
  from the frame it is seen.
