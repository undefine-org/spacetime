# The Fidelity Ladder (PLAN-027 W1)

A Spacetime test declares — or has inferred — the **fidelity rung** it needs. A
backend that cannot provide that rung **refuses to run the test** rather than
executing it against faked browser primitives and reporting a meaningless green.

This permanently closes the silent-false-green class of bugs (BUG-051 / BUG-053
family): a layout assertion can no longer pass on a layout-less engine.

## Refusal is a DEFERRAL — and a skipped gate looks green

A refusal is reported as **SKIP**, not FAIL, and the run still exits 0:

```
$ cargo run --features headless -- test tests/bugs/BUG-323-….test.st --headless
0 passed, 0 failed, 3 skipped        # exit 0
```

That is deliberate — a `logic`-rung CI run should not go red over tests it
structurally cannot judge; the same file runs for real on `--cdp`
(`src/main.rs`, `e.__stRungSkip`). But it has a sharp consequence:

> **A `--headless` green tells you NOTHING about a `needs layout` / `needs
> timing` file.** An entire behavior suite can be skipped and the summary is
> indistinguishable from success.

So a behavior gate counts as proven only once you have seen it **run under
`--cdp`** and seen it **FAIL against the unfixed code**. Run new `needs layout`
files with `--cdp` explicitly:

```sh
cargo run --features cdp -- test tests/bugs/<file>.test.st --cdp
```

And beware the inverse trap: a headless SHIM can hide the defect under test.
The V8 `IntersectionObserver` shim reports `intersectionRatio: visible ? 1 : 0`
once, on a microtask (`src/headless_shims.js`) — a binary snapshot that cannot
express a partially-visible element at all. BUG-323 (`@on &.visible` frozen
mid-animation at ratio 0.605) was invisible to it by construction and shipped
into a demo page.

## The ladder

A total order, lowest to highest fidelity:

| rung     | world                              | what it can test                              | backend            |
|----------|------------------------------------|-----------------------------------------------|--------------------|
| `pure`   | no DOM, no time                    | stagger math, `take`, `fold`, pure values     | V8                 |
| `logic`  | LinkeDOM + fake rAF                | reactivity, signals, event wiring, state graph| V8 + LinkeDOM      |
| `layout` | real CSSOM / `getComputedStyle`    | `@balance`, pretext reflow, visibility, size  | CDP (W2)           |
| `timing` | real / seekable clock              | reveal stagger, easing, transition timing     | CDP (W2/W5)        |
| `paint`  | pixels                             | visual regression, screenshot diff            | CDP + screenshot   |

The canonical model lives in `src/rung.rs` (`Rung`, ordered by declaration).
The runtime mirror is `ST._rung` (`public/runtime/st.js`).

## `%emit build-js` is a rung-caller

`%emit build-js` (i18n / locale, `stdlib/primitives/i18n.st`) runs at **build**
time in the same V8 embed the test runner uses — it is the `pure`/`logic` rung
invoked by a *different caller* (`src/pipeline/expand.rs` →
`execute_build_scripts`). It needs no DOM and no real timing, so it sits at the
bottom of the ladder and is unaffected by the test-side refuse-to-fake gate.
There is one V8 engine; the rung is a property of *what a caller needs*, not of
the engine.

## Declaring fidelity: `needs`

```spacetime
@test "hero wraps to two lines" needs layout {
  @mount @self .hero
  @then .hero should have_height 96
}
```

- `needs <rung>` is **optional**. When omitted, the rung is the **inferred
  floor** (below).
- `needs` may only **raise** the floor above what is inferred. Declaring a rung
  *below* the inferred floor is a hard error (you cannot claim a layout test
  only needs logic).
- Effective rung = `max(inferred_floor, declared)`.

## Inference: the assertion vocabulary sets the floor

An assertion's vocabulary determines the minimum rung it needs — one data table
(`src/rung.rs::assertion_min_rung`), not conditionals scattered through emit:

| assertion / directive                              | min rung |
|----------------------------------------------------|----------|
| `be_visible`, `be_hidden`, `have_style`, `have_width`, `have_height` | `layout` |
| `@clock …`, `@record-timeline …`                   | `timing` |
| `@capture screenshot …`                            | `paint`  |
| everything else (`exist`, `have_text`, `have_class`, `have_state`, `have_length`, …) | `logic` |

A test using `@then .x should be_visible` is therefore auto-routed to `layout`
even without an explicit `needs` — and on the `logic` backend it is refused, not
faked.

## The refuse-to-fake gate

Each backend advertises its ceiling via `ST._rung.setBackend(<rung>)`:

- **V8 + LinkeDOM** (the `--headless` runner): `logic`.
- **CDP / real browser** (W2): `paint`.

A layout/timing/paint operation calls `ST._rung.require('<rung>', '<what>')`
*before* touching a faked primitive. If the backend cannot satisfy it, the call
throws — the test FAILS with:

```
@then .hero should be_visible needs 'layout' fidelity but the current backend
tops out at 'logic' — run on a higher-fidelity backend (e.g. CDP) instead of
faking 'layout'
```

### `--rung` flag

```
cargo run --features headless -- test tests/ --headless --rung logic
```

`--rung` advertises the backend's **ceiling** (default `logic` for the V8
runner). It does **not** fake fidelity: setting `--rung layout` on the V8 runner
would not grant real layout — the V8 backend caps at `logic`, so layout tests
still refuse. The flag exists so a future CDP backend can advertise `paint`
through the same path.

## Why this is the keystone

Before W1, `getComputedStyle = el.style || {}` and a synchronous `rAF` meant a
`be_visible` / `have_width` assertion ran against a lie and reported green. The
ladder makes the lie impossible: the operation that *needs* the real primitive
asks for it first, and a backend that can't provide it must say so. Fidelity a
backend lacks can no longer produce a pass.

## Deferred

- A `pure` fast sub-path that skips LinkeDOM entirely for DOM-free suites is an
  optimization, not a correctness concern: `pure ≤ logic`, so pure tests already
  run correctly on the logic backend. Wire the skip when profiling warrants.
- Real `layout`/`timing`/`paint` backends arrive in W2 (CDP) and W5 (virtual
  clock). Until then those tests correctly refuse on the V8 runner.
