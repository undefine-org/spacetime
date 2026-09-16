# Deterministic Animation Testing — `@clock` & `@timeline`

Real-time animation tests are flaky: you `sleep` and hope the frame landed.
Spacetime owns its animation clock, so a test can **seek virtual time** and
assert on the *curve itself* — deterministically, with no waiting.

```spacetime
@import "stdlib/testing/test"
@import "stdlib/testing/timing"
```

## `@clock` — virtual, seekable time

`@clock install` swaps the rAF driver to virtual mode; then you step it:

```spacetime
@test "reveal staggers in over 200ms" needs timing {
  @mount { .items { @reveal stagger(50ms) } }
  @clock install
  @clock advance 200       // jump 200ms of animation instantly
  @clock sample 12         // emit 12 evenly-spaced frames
  @then .items { be_visible; }
}
```

| directive | effect |
|-----------|--------|
| `@clock install` | switch to virtual time |
| `@clock advance <ms>` | advance virtual time by ms (drivers tick) |
| `@clock sample <n>` | emit n frames across the elapsed span |
| `@clock run-to-end` | run all active animations to completion |

`@clock` itself does **not** require the `timing` rung — a seekable virtual
clock is a `logic`-tier mechanism. The `timing` rung is for assertions about
*real* wall-clock behavior.

## `@record-timeline` + `@timeline` — assert on the curve

Capture the driver's per-frame updates, then make claims about the recorded
series:

```spacetime
@test "opacity ramps monotonically to 1" {
  @mount { .fade { @reveal fade(300ms) } }
  @clock install
  @record-timeline fade
  @clock run-to-end
  @timeline fade {
    nonempty;
    monotonic;     // never steps backward
    in-unit;       // every sample in [0, 1]
    ends-at-one;   // settles at the final value
  }
}
```

| timeline claim | asserts |
|----------------|---------|
| `nonempty` | at least one frame was recorded |
| `monotonic` | values never decrease frame-to-frame |
| `in-unit` | every value ∈ [0, 1] |
| `ends-at-one` | final value settles at 1 |

A backward step or a non-settling curve **fails** — caught deterministically,
not "usually."

## Why this is a moat

Because Spacetime owns the rAF coordinator (`public/runtime/raf-coordinator.js`,
`ST._clock`), it can run an animation forward in virtual time and inspect every
intermediate frame. A test framework bolted onto a browser can only sample wall
time and hope. This is one of two structural moats (the other is
[directive coverage](GUIDE.md#coverage--did-you-test-the-directives-you-shipped)).
