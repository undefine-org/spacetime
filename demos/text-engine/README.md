# Text Engine — the Spacetime fusion flagship

The headline demonstration of **lean + fusion-engine-included**: one page drives
*multiple* standard-library engines at once — and the emitted bundle contains
**only** the engines this page actually uses.

## Run

```sh
cargo run -- serve demos/text-engine/
```

## Engines on this page

| Engine | Macro | Role |
|---|---|---|
| pretext (text module) | `@balance` | evens the ragged headline, reflow-free |
| pretext (text module) | `@reveal split:"lines"` | groups visual lines arithmetically, animates them |
| pretext (text module) | `@measure` | exposes `$lines` for the live readout |
| scene (WebGL) | `@scene { @fill glow … }` | ambient glow backdrop |

A reactive `text: $lines` binding repaints the readout whenever `@measure`
recomputes — userland Spacetime signals driving DOM, no custom JavaScript.

## What this page ships (the lean proof)

The compiled `spacetime.js` contains:

```
core ST runtime
+ text module      (pretext.bundle.js — demand-injected: @balance/@reveal/@measure use it)
+ scene engine     (WebGL shader runtime — @scene uses it)
— and NOTHING else: no engine this page does not touch is bundled.
```

Verify it yourself:

```sh
cargo run -- build demos/text-engine/index.st -o /tmp/te
grep -c globalThis.pretext /tmp/te/spacetime.js   # 1  — text engine present
grep -ci webgl              /tmp/te/spacetime.js   # >0 — scene engine present
```

Compare with `demos/balanced-headline` (text but no scene → no WebGL) and any
non-text demo (no pretext bytes at all). Engines ride the **lean rail**
(PLAN-024 W2): a vendored/runtime module is emitted only when a primitive that
references it fires.

## Architecture

- `index.html` — backdrop layer (`<canvas class="bg">`, required by `@scene`) + headline + lede + readout
- `index.st` — styling, `@scene` backdrop, `@balance`/`@reveal`/`@measure` on the
  text, and the reactive `$lines` readout binding. No hand-written JavaScript.
- pretext is vendored as a git submodule and bundled into a single owned artifact
  (see `docs/stdlib/VENDORING.md`).
