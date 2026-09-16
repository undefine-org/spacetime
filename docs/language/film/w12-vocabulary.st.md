# W12 — a shipped film vocabulary

*Working doc for PLAN-150 W12. Builds and runs today. The "shipped film
vocabulary" of `../film.st.md` §12.*

```st hidden
@import "stdlib/md"
```

## The idea

Everything in a title sequence is a handful of moves. `stdlib/film` ships them
as named forms, so a film's first draft is one line per shot. These are **not
new syntax** — they are the promo's repeated patterns, named, each built only
from the landed film surface (positioned stops, `@post`, camera, noise).

```st src
@import "stdlib/film"

.reel {
  @score &.time(6s) as &film { &title at 0s for 6s; }
}
h1.title {
  @on &.clip { --focus-pull; }
}
.stage {
  @on &.clip { --shake; }
}
```

## What ships

- **Motion:** `--focus-pull` (blur resolves, spacing tightens, a rise + fade —
  "the title arrives"), `--shimmer` (a light-sweep for `background-clip: text`),
  `--typewriter` (a stepped `--chars` reveal), `--glitch` (a horizontal
  stutter), `--drift` (idle motion as 1-D noise, §7).
- **Lens:** `--cinema` (bloom + grain + vignette as one `@post` look, §3).
- **Camera:** `--shake` (handheld stutter that settles, §4), `--push-in` (a slow
  dolly toward the subject).

## How it lowers

Nothing new: each motion/camera form is an ordinary `@form` declaration
(`stdlib/film/index.st`), consumed exactly as any form is
(`@on &.clip { --focus-pull; }`, `@on &.clip { --shake; }`). Importing
`stdlib/film` registers them; the list grows by adding forms, never syntax. W12
also added the `post` form kind (`@form post --cinema { … }`) so a `@post` look
can be named — wiring its consumption (`@post --cinema;`) is the same bare-form
statement gap the camera has, and lands with it.

## Not yet (later in W12's arc)

- Consuming a named `post` look with `@post --cinema;` (the bare-form statement
  gap shared with the camera).
- More moves as the promo demands them (whip-pans, light-leaks, lower-thirds).
- A `--title-card($text)` shot (§10) composing markup + one of these motions.
