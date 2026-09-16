# W4 — the score drives 3D

*Working doc for PLAN-150 W4. Builds and runs today. The "score drives 3D" of
`../film.st.md` §5.*

```st hidden
@import "stdlib/md"
@import "stdlib/3d"
```

## A stage object is an element

Inside `@stage`, an object may be **named** — `@object &orb (...)` — and once
named it is addressable from an `@on` block by that name, exactly as a DOM
element is:

```st src
.reel {
  @score &.time(10s) as &film { &shot at 0s for 10s; }
}
.shot {
  @on &.clip {
    &orb { scale: 1 -> 2.5; spin: 0 -> 6.28; }
  }
}
canvas.field {
  @stage(camZ: 6) {
    @object &orb (shape: "sphere", size: 1, color: #8ab4ff)
  }
}
```

`scale`, `spin`, `posX/Y/Z`, `opacity`, `emissive`, `metalness`, `roughness`
are the keyframeable channels every mesh exposes; a named `@particles` field
exposes `morph`. The keyframe on the object is written where every keyframe is
written — in `@on &.clip` — and driven by the **same engine** that drives DOM.

## How it lowers

A named object registers a *channel proxy* on its canvas under `&name`. The
`@on` engine (which only ever writes to an element's `.style`) writes each
keyframed value to that proxy, and the object's frame loop reads the channel and
applies it to the real three.js mesh. No new interpolation path — the move
`@post` (W2) and the camera (W3) already made.

The `@on` consumer is an ordinary element (a `.shot` div) whose subtree hosts
the `<canvas>`; the resolver descends into the canvas to find the named object.
(`@on` bound *directly* on a `<canvas>` does not receive a score driver — a
separate limitation, tracked — so the consumer is the hosting element.)

## The score cuts on the beat

Because a stage object is just another `@on &.clip` consumer, a `@score` window
choreographs 3D exactly as it choreographs DOM: give each shot its clip window
and the objects in it animate over that window, cut to the next on the beat.

## What this replaces

`@scroll-3d(drive: "morph", to: 3)` — a private driver with a string property
name — becomes an ordinary `@on` keyframe (`&cloud { morph: 0 -> 3; }`), so
morph, spin and size are choreographed on the same timeline as everything else.
There is one driver grammar.

## Not yet (later in W4's arc)

- The `grid()` object form with `reveal`/`displace`/`fog` channels (the studio
  cut's second-scene floor) — the channel machinery is here; `grid()` and its
  vertex-shader channels are the remaining piece.
- `@on &.scroll { &cloud { morph } }` — scroll-driven 3D reuses the same path;
  it works wherever `@on` does.
- Fixing `@on` bound directly on a `<canvas>` so the consumer need not be a
  wrapping element.
