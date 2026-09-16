# W6 — randomness and noise, as data

*Working doc for PLAN-150 W6. Builds and runs today. The "randomness and noise
as data" of `../film.st.md` §7.*

```st hidden
@import "stdlib/md"
```

## The problem it solves

Dust motes were placed with `mod(sibling-index() * 83, 100)` — a prime-number
idiom standing in for "random". Camera shake was seven typed keyframe stops
standing in for "noise". Both are data pretending to be arithmetic.

## `random()` — a deterministic per-element seed

```st src
.dust i {
  --seed: random();                       // 0..1, fixed per element, stable across builds
  left: calc(var(--seed) * 100%);
  opacity: calc(0.2 + 0.6 * random(2));   // a second independent stream
}
```

`random()` in a static value is resolved to a per-element stamp, seeded from the
element's document index. It is:

- **per-element** — each `.dust i` gets its own value (not one shared value);
- **independent per stream** — `random()` is stream 0, `random(2)` is stream 2,
  a different seed;
- **stable across builds** — the same DOM stamps the same values, so
  `spacetime render` is bit-identical run to run.

It lowers to a custom property (`random()` → `var(--st-rnd-0)`) plus a hydration
stamp on the selector; the stamp re-fires for nodes added later (`@each` output).

## `--noise-1d` — noise as a motion form

```st src
@form motion --noise($amp = 6px, $hz = 4) {
  translate-x: --noise-1d($amp, $hz);
  translate-y: --noise-1d($amp, $hz, seed: 2);
}
.cam {
  @on &.clip { --noise(amp: 8px); }
}
```

`--noise-1d($amp, $hz)` is a value evaluated against clip **progress**: smooth,
periodic, deterministic. `$amp` keeps its unit (`px`, `deg`, …); `$hz` is cycles
over the window; `seed:` shifts the phase for an independent stream. Shake,
drift and flicker are all `--noise` with different amplitudes on different
properties — and it rides the ordinary keyframe engine (a noise value is driven
on the same clip watch as a keyframe, no new interpolation path).

## What this deletes

The `mod(sibling-index() * prime, 100)` idiom, and the hand-typed shake chains
(`translate: 0 -> 8px -> -6px -> 0`). Randomness is a seed; noise is a form.

## Not yet (later in W6's arc)

- 2-D / N-D noise (`--noise-2d`) for correlated drift.
- `random()` with an explicit range (`random(min, max)`); today it is 0..1 and
  you scale in `calc()`.
