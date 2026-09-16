# W8 — scenes and transitions

*Working doc for PLAN-150 W8. Builds and runs today. The "scenes and
transitions" of `../film.st.md` §9.*

```st hidden
@import "stdlib/md"
```

## A transition is a form on the arrow

`->` already means *sequence* in a `@score` (one clip follows the previous). A
**transition** is a form placed on that arrow — two `@on`-style keyframe blocks,
one for the outgoing clip (`&from`) and one for the incoming clip (`&to`):

```st src
@form transition --crossfade($d = 400ms) {
  &from { opacity: 1 -> 0; }
  &to   { opacity: 0 -> 1; }
}
@form transition --whip-pan($d = 300ms) {
  &from { translate-x: 0 -> -100vw; }
  &to   { translate-x: 100vw -> 0; }
}

.reel {
  @score &.time(30s) as &film {
    &open for 3.6s -> --crossfade(400ms) -> &grid for 4.6s -> --whip-pan -> &code for 5.4s;
  }
}
```

The outgoing clip runs `&from` as it ends; the incoming clip runs `&to` as it
begins. Both are driven by the clips' own `@score` windows — a transition is
choreography, not a scheduler, so it rides the same clock as everything else. A
bare `->` with no form is a **cut**.

## How it lowers

A transition placed on a `->` becomes a `transition-edge` bind
(`stdlib/primitives/transition-edge.st`): it watches the outgoing clip's
`__clip_<name>` window and plays `&from` over that clip's tail, and the incoming
clip's window to play `&to` over its head. The `&from`/`&to` bodies are the same
keyframe shape `@on &.clip` consumes — no new interpolation path.

## The `->` chain, fixed

Landing this fixed a foundational gap: a score arrow chain `&a -> &b -> &c` only
ever kept `&a` — the repeated capture group `( "->" $next )*` was dropped after
the first step (a repeated glue-group surfaced no name, so its results were
discarded). Now the whole chain is captured, so **sequencing itself** works, and
transitions sit on the arrows between.

## What this deletes

Hand-overlapped windows and the promo's `opacity: 0 -> 1 -> 1 -> 1 -> 0`
envelope on every scene — a transition supplies the ends.

## Not yet (later in W8's arc)

- The transition's `$d` refining the overlap against the joined window lengths
  (today a sensible default overlap).
- Per-transition `easing:` on each region (today linear across the edge).
- A route change reusing a transition (`@on $route "/x" => --whip-pan;`).
- Named score fragments joined by a transition (`--act-one -> --whip-pan -> --act-two`).
