# LiveView Counter — the self-aware handshake (Wave B)

The smallest possible proof of the Spacetime⇄Phoenix LiveView design's keystone:
**the integration validates itself at compile time.**

```st
@host $counter : live("DemoWeb.CounterLive")

@data signal $inc() to $counter {
  send emit "inc"                       // ← checked against the server contract
  receive to IncResult { "ok" => Bumped($.reply as number); _ => Failed($.reply); }
  policy latest
}
```

Beside the page sits `CounterLive.contract.json` — the server LiveView's declared
events, in the exact shape Elixir's `__spacetime_contract__/0` will emit (Wave C):

```json
{ "module": "DemoWeb.CounterLive",
  "events": [ { "name": "inc" }, { "name": "dec" } ] }
```

## Try it — make the check fail

Change `send emit "inc"` → `send emit "increment"` and run:

```
cargo run -- check demos/liveview-counter/
```

You get a **compile error**, not a runtime surprise:

```
error[E0928]: live host `$counter` (DemoWeb.CounterLive) does not handle event
`increment` — the page fires it via `send emit`, but the server contract
declares: [inc, dec]
```

## Why this matters

This is footgun #2/#3 from the design's catalog (`@research/spacetime-phoenix-liveview-bridge.org`
§11) made **impossible by construction**: a page cannot fire an event the server
doesn't handle, because the build unifies the page's `@host … live(...)` intents
against the server module's introspected contract. One contract, checked both ways.

The check is **opt-in**: a page without a `*.contract.json` sidecar compiles
normally (no false positives). When the sidecar is present, every `send emit "X"`
to a `live(...)` host is verified ⊆ the server's declared events.

Implemented in `src/analysis/liveview_contract.rs` (FEAT-135); the comparator core
is pure data-in/data-out with unit tests, wired into both `check` and `doctor`.

## Known gap

`send emit "X" { }` with an EMPTY body block currently drops the event capture
(BUG-128) — use the no-body form `send emit "X"` or a non-empty body until that
grammar bug is fixed. This demo uses the no-body form.
