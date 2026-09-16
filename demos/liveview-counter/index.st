/* ============================================================================
   LiveView Counter — the SELF-AWARE handshake (Wave B, FEAT-135).

   This page binds to a Phoenix LiveView via `@host … live(...)`. At BUILD time,
   the compiler reads `CounterLive.contract.json` (the server module's declared
   events — the shape Elixir's `__spacetime_contract__/0` will emit) and checks
   that every event this page fires actually exists on the server.

   Try it: change `send emit "inc"` to `send emit "increment"` and run
   `cargo run -- check demos/liveview-counter/` — you get a COMPILE error (E0928),
   not a runtime surprise. The integration validates itself.
   ============================================================================ */

@import "stdlib/macros/host"
@import "stdlib/macros/data-kind"
@import "stdlib/macros/handle"

/* The server LiveView. Its contract is read from CounterLive.contract.json. */
@host $counter : live("DemoWeb.CounterLive")

/* Two intents, both declared by the server contract (events: inc, dec). */
@data signal $inc() to $counter {
  send emit "inc"
  receive to IncResult { "ok" => Bumped($.reply as number); _ => Failed($.reply); }
  policy latest
}

@data signal $dec() to $counter {
  send emit "dec"
  receive to DecResult { "ok" => Bumped($.reply as number); _ => Failed($.reply); }
  policy latest
}

.counter {
  > .value { `$count` }

  @handle $inc {
    receive { Bumped(n) => { $count <- n; }  Failed(m) => { $error <- m; } }
  }
  @handle $dec {
    receive { Bumped(n) => { $count <- n; }  Failed(m) => { $error <- m; } }
  }
}
