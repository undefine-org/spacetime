# W3 — the camera

*Working doc for PLAN-150 W3. Builds and runs today. The camera of the golden
reference (`../film.st.md` §4).*

```st hidden
@import "stdlib/md"
```

## A camera move is motion

So it is written where motion is written. A `@form camera` names a move over a
reserved set of axes — `position`, `target`, `fov`, `roll` — and an `@on`
consumes it exactly like a motion form:

```st src
@form camera --crane { position: 0 9.5 4.5 -> 0 2.1 9.5; }

.reel {
  @score &.time(10s) as &film { &stage at 0s for 10s; }
}
.stage {
  width: 100vw; height: 100vh;
  @on &.clip { --crane; }
}
```

`position` is an `x y z` vector; the crane above pulls the camera from high and
close (`0 9.5 4.5`) down to eye level and back (`0 2.1 9.5`). Because the camera
is just another `@on` consumer, it composes with everything else in the same
block:

```st src
.stage2 {
  @on &.clip {
    --crane;
    opacity: 0 -> 1;
  }
}
```

## How it lowers

The axes become `--cam-*` custom-property tracks (`position` → `--cam-x/y/z`,
`target` → `--cam-tx/ty/tz`, plus `--cam-fov` and `--cam-roll`), animated by the
ordinary keyframe engine. A static rig transform on the host reads them, so
moving the camera moves the host's contents — a dolly on `--cam-z`, a pan on
`--cam-x/y`, a roll on `--cam-roll`. CSS re-evaluates the transform whenever a
track updates, the same mechanism `@post` uses.

**Fidelity, plainly:** on a `@stage` canvas these axes drive the REAL camera;
on ordinary DOM they drive one inverse transform on the host — a convincing
dolly/pan/roll, but not a true perspective camera. Same word, two lowerings.

## The vocabulary is closed

`position`, `target`, `fov`, `roll` — an unknown axis is a hard error, never a
silent no-op.

## What this replaces

The promo's `perspective-origin` keyframe hack and its seven-stop `translate`
camera-shake chains: a crane is one form now, and shake is a camera form too
(`position: 0 0 0 -> 8px 0 0 -> -6px 0 0 -> 0 0 0`).

## Not yet (later in W3's arc)

- A true `lookAt` from `target` (today `target` tracks exist but the DOM rig
  uses position/roll; the stage rig will use the full matrix).
- `@on &.scroll { --dolly; }` scroll-driven cameras and `$route` whip-pans reuse
  the same form — they work wherever `@on` does.
- ~~The bare `@on &.clip --crane;` statement form~~ — **landed in W3-arc**; the
  colon-free spelling now lowers like the brace form.
