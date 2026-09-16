# Browser Automation — `@drive`

**Test ≡ automation.** A `@drive` block is a `@test` whose *world* is a live URL
instead of a mounted fixture. The exact same probes — `@when`, `@then`,
`@observe`, `@capture` — run against the live page. Nothing new to learn; you
swap the world.

```spacetime
@import "stdlib/testing/test"
@import "stdlib/testing/automation"

@drive "https://example.com" {
  @then h1 { exist; text ~= "Example"; }
  @capture screenshot "example.png";
}
```

Run it on the real-browser backend:

```sh
cargo run --features cdp -- test path/to/script.test.st --cdp
```

The CDP runner reads the `drive` URL, navigates the page to it, re-injects the
runtime, and runs the body against the **real DOM**. An `@assert` reading live
DOM gates exactly like a unit test: a wrong expectation fails.

## Directives

| directive | shape | effect |
|-----------|-------|--------|
| `@drive` | `@drive "<url>" { … }` | navigate to a live URL, run the body against it |
| `@observe` | `@observe <sel> as $handle` | bind a stable handle to DOM you didn't author |
| `@capture` | `@capture screenshot "<path>"` | screenshot the live page (Rust-side, full page → file) |
| `@wait-for` | `@wait-for <sel>` | poll until an element appears (5s budget) |

## Remote / kernel backend

`@drive` is endpoint-agnostic. Point the runner at a **remote** CDP websocket —
the kernel.sh `:9222` `connectOverCDP` model — and the same script drives a
remote browser with zero change to the `.st`:

```sh
SPACETIME_CDP_WS="ws://kernel-host:9222/devtools/browser/…" \
  cargo run --features cdp -- test script.test.st --cdp
```

Local launch vs remote connect is selected at the runner level
(`Browser::connect`), not in the macro — the automation surface is identical.

## Fidelity

`@drive` and `@capture screenshot` need a real browser: they auto-require the
`layout` / `paint` rungs and **refuse** on the V8 `--headless` backend rather
than pretending. See [FIDELITY_LADDER.md](FIDELITY_LADDER.md).

## Known gap — BUG-058

Today, **bare-block probes** (`@observe`, `@then{claims}`, `@capture`,
`@wait-for`) do not execute *inside* a `@drive` body — only `@assert` / `@eval`
run against the live page. The post-navigation re-injection swallows
block-scope directives; the richer probes need the BUG-058 re-injection fix.
Live-DOM `@assert` gating (the test ≡ automation proof) works today.
