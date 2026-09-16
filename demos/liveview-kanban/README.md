# LiveView Kanban — a board that is alive, shared, and self-aware

A multiplayer Kanban whose **entire client is one `.st` file**, driven in dev by
verse's own dev-ws and (in the integration plan) by a real Phoenix LiveView — the
same `.st` byte-identical across both.

> **The host is config. The page is the program.**

`cargo run -- serve demos/liveview-kanban/` → open two tabs to see multiplayer.

## What it demonstrates — three substrates no other stack combines

### LIVE — the reactive signal graph IS the UI
- `@data collection $cards … via dev-ws` — a live, broadcast-backed array. A card
  move is a reorder op broadcast to every client: **real multiplayer, zero new
  server code.**
- `@data query $todo/$doing/$done … { where: $.col == "…" }` — each column is a
  reactive projection of the one `$cards` signal; metrics re-flow in one frame.
- `@data signal $move(...) to $board { send …; receive to MoveResult { … }; policy
  latest }` + scoped `@handle { optimistic / receive / final }` — the card moves
  INSTANTLY (optimistic), confirms on the server reply, and springs back on reject.
- A `scrub history` slider folds the move stream into time-travel (the board is a
  pure function of a signal, so replay is free).

### SHARED — multiplayer as a first-class material
- `@presence(room: "kanban") as peers: Peer` — every collaborator's cursor, live
  over the `/ws` presence channel, zero custom JS.

### SELF-AWARE — the board renders its OWN protocol
- `@data dispatch $proto from kind "data"` + `@each` — the contract panel reads the
  **compiler's own macro registry** and lists the board's wire vocabulary with full
  signatures. It is read from the registry, so it **cannot drift from the code**.
- The host badge (`ws://verse · dev`) is the ONLY visible difference between the
  verse-served board and the Phoenix-served one — same page, swapped host.

## The host swap (Wave C)

One line in `index.st`:

```st
@host $board : ws("/ws")                       // dev: verse multiplayer
// @host $board : live("MyAppWeb.KanbanLive")  // prod: real Phoenix LiveView
```

The page does not change — only the transport deepens. See the full design +
plan in `@research/spacetime-phoenix-liveview-bridge.org`.

## Bugs found + fixed while building this

This prototype surfaced two real compiler bugs (filed, fixed, tested):
- **BUG-126** — `%binds` yields (`@realtime`/`@presence`/`@data collection`) were
  not registered as referenceable signals (hardcoded primitive list shadowed the
  registry) → E0405. Fixed by reading yields from the MetaRegistry.
- **BUG-127** — `@data query { where: $.col == "todo" }` truncated the closing
  quote (`convert_property_value` over-eager `trim_matches('"')`), making the
  predicate invalid JS so the filter matched everything. Fixed with a
  whole-literal-only unwrap.

## Usage notes (gotchas worth knowing)

- A `@template` body is **pure markup** — directives (`@drag`, `@magnetic-glow`)
  and `/* comments */` inside one render as literal text. Put behaviour in a
  selector scope (`.card { @drag … }`).
- `@each` over **static** data (`@data dispatch`) wants the row markup **inline**
  in the `@each` body (mirroring `demos/spacetime-docs/reference.st`), not a
  `&template()` invocation.
