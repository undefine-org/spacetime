# W0 — the doc gate and honest diagnostics

*Working doc for PLAN-150 W0. Everything here builds today; it is the film
surface as it stands BEFORE any new wave lands. Its job is to prove the
foundation the film is built on already works, and that every not-yet-landed
word now fails loudly instead of compiling to nothing.*

```st hidden
@import "stdlib/md"
```

## What already carries a film

The score, windows, and `@on &.clip` consumers all ship today. This is the
spine every later wave extends.

```st src
.stage {
  @score &.time(8s) as &film {
    &open at 0s for 3.6s;
    &grid at 3s  for 4.6s;
  }
}
.scene { position: absolute; inset: 0; opacity: 0; }
.open  { @on &.clip { opacity: 0 -> 1 -> 1 -> 0; } }
.grid  { @on &.clip { opacity: 0 -> 1; translate-y: 40px -> 0; easing: --ease-out-expo; } }
```

Multi-stop keyframes, nested-selector choreography, `range:`, and stdlib easing
forms are all landed:

```st src
.card {
  @on &.clip {
    opacity: 0 -> 1 -> 1 -> 0;
    .price { color: #8a94a6 -> #ffb454; range: 60% to 100%; }
    easing: --ease-out-expo;
  }
}
```

## What now fails loudly (and did not before W0)

These are the film words whose implementation lands in a later wave. Before W0
each compiled GREEN and emitted nothing — a broken feature hiding in a passing
build. Each is now a hard, self-describing error naming its wave. The snippets
below are shown as inert prose (not `st` fences) precisely because they must not
build; the doc gate keeps them failing.

- ~~Positioned stops, per-step easing, `--steps`~~ — **landed in W1**; see
  `w1-positioned-stops.st.md`.
- ~~`@post`~~ — **landed in W2**; see `w2-post.st.md`.
- ~~Camera forms~~ — **landed in W3**; see `w3-camera.st.md`.
- **`@scatter`** — `@scatter(from: &tag) { :shape { --x; } }` → **E0970** (W5).
- **`@audio` on the score** — `@audio(src: "a.wav") as &bed at 0s;` → **E0970 / E0946** (W7).
- **Transitions** — `@form transition --x { &from { … } &to { … } }` → **E0946** (W8).
- **Shots** — `@form shot --r { :markup { … } :motion { … } }` → **E0946** (W9).

## Known-silent gaps (tracked, not yet loud)

Two unlanded words are value-level, not directives, and still compile green.
They are enforced when their wave lands; the gate records them as expected-silent
so their status is never mistaken for "works":

- `--seed: random();` (a value function) — lands W6.
- `@reveal(split: outline)` (a reveal mode the primitive ignores) — lands W10.
