# Testing Guide — the core grammar

A test is a named block of **directives**. Each directive is a Spacetime macro
(`stdlib/testing/test.st`) that expands to runner JS. You never write JS in a
test — you write directives.

```spacetime
@import "stdlib/testing/test"

@test "<description>" [needs <rung>] {
  <directives…>
}
```

## Mounting a world — `@mount`

`@mount` renders **real Spacetime markup** (and nested primitives) into the test
DOM. This is the component under test — not a string fixture, the actual
reactive thing.

```spacetime
@test "mounts a live component" {
  @mount {
    <button class="inc" data-st-count="0">+</button>
    .inc { @behavior .inc counter(from: 0) }
  }
  @then .inc { exist; }
}
```

> `@mount` replaces the retired `@fixture`. Markup + nested primitives are
> supported; file-references and `@mount @self <sel>` are tracked in FEAT-066.

## Driving it — `@when`

`@when <selector> <action> [value]` fires a **real event** at the mounted DOM
(`click` dispatches a real `MouseEvent`, not a no-op `.click()`), then flushes
pending reactive updates so the next `@then` sees settled state.

```spacetime
@when .inc click
@when .field input "hello"
```

## Asserting — `@then { claims }`

The workhorse. `@then <selector> { <claim>; … }` runs one or more **claims**
against the subject. A claim is `<prop> <op> <value>;` or a bare `<prop>;`
(truthy/exists).

```spacetime
@then .inc {
  exist;
  state            == "3";
  text             ~= "+";
  $count           == 3;
  state_history    ~> ["a", "b"];
  transition_count >= 1;
}
```

### Props

| group | props |
|-------|-------|
| DOM | `exist` · `count` · `text` · `value` · `state` · *any attribute name* |
| signals | `$name` (a reactive signal on the component) |
| machine | `state_history` · `transition_count` · `previous_state` · `has_state_machine` · `animating` |
| layout *(CDP only)* | `be_visible` · `be_hidden` |

### Ops

| op | meaning |
|----|---------|
| `==` `!=` `>=` `<=` `<` `>` | comparison |
| `~=` | contains (substring / membership) |
| `~>` | sequence / history prefix (e.g. state went a → b) |
| `matches` | regex match |

Claims are `;`-terminated. A layout prop (`be_visible`) auto-raises the test to
the `layout` rung — on `--headless` it **refuses**, it does not fake (see
[FIDELITY_LADDER.md](FIDELITY_LADDER.md)).

## Bindings & expressions — `@let`, `@assert`, `@eval`

```spacetime
@let (x = 41)
@assert (x + 1 == 42) "math works"
@eval (someSideEffect())
```

`@assert <cond> [message]` is the low-level escape hatch; prefer `@then{claims}`
for DOM/state. `@assert` GATES — a false condition fails the test (this was the
W0 fix; assertions are no longer vacuous).

## Mocking — `@mock`

```spacetime
@mock fetch returns { "ok": true }
```

Replaces a global for the test's duration; restored on teardown.

## Fidelity — `needs <rung>`

```spacetime
@test "hero wraps to two lines" needs layout {
  @mount { <h1 class="hero">A very long headline…</h1> }
  @then .hero { be_visible; }
}
```

`needs` may only **raise** the inferred floor. The floor is inferred from the
assertion vocabulary; declaring a rung *below* it is a hard error. See
[FIDELITY_LADDER.md](FIDELITY_LADDER.md).

## Coverage — did you test the directives you shipped?

Because every directive is a macro, the runner can report which directives were
*exercised* by a suite:

```sh
cargo run --features headless -- test tests/ --headless --coverage
# → Directive coverage: 18/34 macros exercised (52.9%)
#   unexercised: @mock @snapshot @clock …

cargo run --features headless -- test tests/ --headless --min-directive 50   # gate
```

This is unique to owning the macro layer — it answers "you never tested
`@property`" the way line coverage never could.

## Quick reference

| directive | shape |
|-----------|-------|
| `@test` | `@test "desc" [needs <rung>] { … }` |
| `@mount` | `@mount { <markup + primitives> }` |
| `@when` | `@when <sel> <action> [value]` |
| `@then` | `@then <sel> { <prop> <op> <value>; … }` |
| `@let` | `@let (name = expr)` |
| `@assert` | `@assert (cond) ["message"]` |
| `@eval` | `@eval (expr)` |
| `@mock` | `@mock <global> returns <value>` |
| `@wait` | `@wait <ms>` / `@wait-until (cond)` |

See [GENERATIVE.md](GENERATIVE.md), [ANIMATION.md](ANIMATION.md), and
[AUTOMATION.md](AUTOMATION.md) for `@property`/`@fuzz`, `@clock`/`@timeline`, and
`@drive`.
