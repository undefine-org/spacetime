# W1 — positioned stops, per-step easing, `--steps`

*Working doc for PLAN-150 W1. Everything here builds and runs today. It is the
film-surface keyframe grammar of the golden reference (`../film.st.md` §2), now
landed.*

```st hidden
@import "stdlib/md"
```

## A stop can say WHERE it sits

By default `a -> b -> c` spreads its stops evenly. Add `at <pos>` to pin one:

```st src
.flash {
  opacity: 0;
  @on &.clip { opacity: 0 -> 0.85 at 25% -> 0.18 at 30% -> 0; }
}
```

Read it as: *zero, then 0.85 at 25% of the window, then 0.18 at 30%, then zero
at the end.* `at` takes a percentage of the window or a fraction (`at 0.25`).
Stops with no `at` spread evenly between their positioned neighbours, so you
only annotate the moments that matter — a single spike in a long clip is three
stops, not sixty.

This is the whole reason the promo's six separate hit-flash clips collapse into
one: a flash is a stop that peaks where the sound lands.

## A step can say HOW it moves

A form reference after a stop names the easing of the step that REACHES it:

```st src
.card {
  @on &.clip {
    translate-y: 40px -> 0 --ease-out-expo -> -6px --linear -> 0 --ease-in-out-sine;
  }
}
```

From 40px, expo-out to 0, then linear to −6px, then sine back to 0. The
body-level `easing:` is still the default for any step that does not name its
own. An undeclared curve is a hard error (`--nope` → E0962), never a silent
fall back to linear.

## `--steps(N)` — a stepped curve

`--steps(N)` advances in N discrete jumps (CSS `steps(N, jump-end)`). It is what
makes a counter tick and a typewriter type one glyph at a time:

```st src
.counter {
  @on &.clip { --frame: 0 -> 1800; easing: --steps(1800); }
}
```

## A hold is just two equal positioned stops

There is no `hold` keyword — repeat a value at two positions and the value is
held between them:

```st src
.caret {
  opacity: 1;
  @on &.clip { opacity: 1 -> 1 at 50% -> 0 at 50.1% -> 0; }
}
```

## What this replaces

- The six `&h1 … &h6` hit clips and their 0.12s pre-offset in `demos/promo`.
- The `sign(sin(var(--chars)))` caret-blink trick.
- Every `range:` written only to slide a peak to the right moment.
