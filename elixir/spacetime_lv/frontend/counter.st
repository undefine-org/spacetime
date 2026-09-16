/* ============================================================================
   counter.st — CounterLive's view, fully Spacetime-owned (FEAT-137).

   The page drives the LiveView wire itself: the buttons fire `@data signal`s
   whose `send emit` travels the Phoenix channel via the `live` transport;
   `handle_event/3` replies `{:reply, %{tag, reply}}` which decodes through the
   receive arms into the IncResult/DecResult sums; `@handle` applies the
   confirmed value to $count. No phx-click, no HEEx controls — the LiveView
   supplies only state + replies (the seed still rides the st-set bridge).

   The DOM skeleton lives in Spacetime.LiveView.render_host/1 (host-supplied);
   this page binds behaviour + style onto it by selector.
   ============================================================================ */
@import "stdlib/macros/data-kind"
@import "stdlib/macros/handle"
@import "stdlib/primitives/pointer"
@import "./$hosts.st"

/* A page-local adapter makes the pointer primitive available to selector code.
   It only yields local signals; no bridge transport is involved. */
%macro local-pointer {
  %form { @local-pointer }
  %binds {
    pointer(&self, events: ["move"]) -> { $localX, $localY }
  }
}

/* The LiveView count assign is the source of truth: mount seed and server-initiated
   diffs both reach this signal through the module-scoped subscribe channel. */
@data subscribe $count from $counter : count ;
@data inline $error string : "" ;
@data inline $flash string : "" ;

/* push_event effects are transient: the bridge routes `flash` into the same
   receive-arm matcher used by signal replies, then this handler updates view state. */
@data stream $fx from $counter {
  receive to Effect { "flash" => Flash($.payload.message); }
}

/* Two intents, both declared in CounterLive.contract.json (E0928-checked). */
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
  // Local-only presence: the pointer primitive writes scoped signals directly
  // into the rendered surface; it never crosses the LiveView bridge.
  @local-pointer
  --st-local-pointer-x: $localX;
  --st-local-pointer-y: $localY;

  > .value { text: $count; }
  > .error { text: $flash || $error; }

  /* The correlated round-trip, consumed: server-confirmed count applies; a
     Failed reply surfaces instead of silently drifting. */
  @handle $inc {
    receive {
      Bumped(n) => { $count <- n; $error <- ""; }
      Failed(m) => { $error <- m; }
    }
  }
  @handle $dec {
    // Snapshot → immediate mutation → tagged rejection restores the snapshot.
    // `policy latest` correlates this handler to its latest fire, so an older
    // rejection never clobbers a later optimistic value.
    optimistic { $beforeDec <- $count; $count <- $count - 1; $error <- ""; }
    receive {
      Bumped(n) => { $count <- n; $error <- ""; }
      Failed(m) => { $count <- $beforeDec; $error <- m; }
    }
  }
  @handle $fx {
    receive {
      Flash(message) => { $flash <- message; }
    }
  }
}

/* The controls. NB: @on must live in a TOP-LEVEL selector block — nested inside
   `.counter { > … }` the event binding hoists to the outer element and one
   click fires EVERY nested handler (BUG: nested-@on hoisting, filed in the
   tracker). The full compound selector binds exactly the one button. */
.counter > .controls > button.inc { "+" @on click { $inc() } }
.counter > .controls > button.dec { "−" @on click { $dec() } }

/* Styling is Spacetime-owned (the LiveView supplies only data). */
body {
  margin: 0;
  min-height: 100vh;
  display: grid;
  place-items: center;
  background: radial-gradient(120% 120% at 50% 0%, #1a1f2e 0%, #0a0b0f 70%);
  font-family: "Inter", system-ui, sans-serif;
  color: #e8eaf2;
}

#st-host {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1.5rem;
}

.counter {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1.5rem;
}

/* The local cursor belongs entirely to this Spacetime view. A future remote
   Presence relay may render peers, but no pointer payload is emitted here. */
.counter::after {
  content: "";
  position: absolute;
  width: 0.65rem;
  height: 0.65rem;
  border: 2px solid #8cc8ff;
  border-radius: 50%;
  pointer-events: none;
  opacity: 0.8;
  transform: translate(
    calc(var(--st-local-pointer-x, 0) * 1px - 50%),
    calc(var(--st-local-pointer-y, 0) * 1px - 50%)
  );
}

.counter .value {
  font-size: 6rem;
  font-weight: 800;
  font-variant-numeric: tabular-nums;
  letter-spacing: -0.04em;
  color: #6ea8fe;
  text-shadow: 0 0 40px rgba(110, 168, 254, 0.35);
  transition: transform 0.18s cubic-bezier(0.34, 1.56, 0.64, 1);
}

.counter .error {
  min-height: 1.25rem;
  font-size: 0.9rem;
  color: #fe6e8e;
}

.controls {
  display: flex;
  gap: 0.75rem;
}

.controls button {
  width: 3.5rem;
  height: 3.5rem;
  font-size: 1.6rem;
  border-radius: 14px;
  border: 1px solid rgba(255, 255, 255, 0.12);
  background: linear-gradient(180deg, #1f2535 0%, #181d2a 100%);
  color: #e8eaf2;
  cursor: pointer;
  transition: transform 0.12s ease, border-color 0.2s ease;
}

.controls button:hover {
  border-color: rgba(110, 168, 254, 0.5);
  transform: translateY(-2px);
}

.controls button:active { transform: translateY(0) scale(0.95); }
