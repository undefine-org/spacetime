# `stdlib/dnd` — declarative drag-and-drop

Drag-and-drop with **zero custom JavaScript**: a page declares *what* can be
dragged and *where* it can be dropped, and the module wires the rest. No
`ST.dropTarget`, no `$.closest`, no `$.dataset` string-fishing, no magic
selectors — the page is the program.

```spacetime
@import "stdlib/dnd"

[data-col="doing"] {
  @drop-zone(group: "cards") as $zone {
    over { background: rgba(255,255,255,0.04) }   // highlight while a card hovers
  }
}

.card {
  @drag(group: "cards", payload: $.dataset) {
    dragging { scale: 1.04 }                        // visual state while dragging
    on-drop $zone { $move({ card: $payload.id, to: $zone.col }); }
  }
}
```

## The two directives

### `@drag(group?, payload?, axis?, bounds?, …) { … }`

Makes an element draggable. This is the **sole** `@drag` macro (it lives here
because drag-and-drop is one concern; `@swipe`/`@resize` stay in
`stdlib/macros/drag.st`). A page that drags anything imports `stdlib/dnd`.

- `group` — pairs draggables with `@drop-zone(group:)` (default `""`, one global group).
- `payload` — the dragged datum, bound as `$payload` in the lifecycle clauses.
  `payload: $.dataset` reads the element's `data-*` (so `$payload.id` ← `data-id`);
  any expression / `$signal` also works.
- `axis` — `"x"` / `"y"` / `"both"` motion constraint.
- `bounds: &el` — clamp the drag to a named element ref (PLAN-057).
- States: `dragging` (active) + any custom states.
- Lifecycle clauses (the declarative drop):
  - `on-enter $zone { … }` — pointer entered a compatible zone
  - `on-leave $zone { … }` — pointer left it
  - `on-drop  $zone { … }` — released over an **accepting** zone
  - `on-cancel { … }` — released over nothing / a rejecting zone (declarative spring-back)
  - `on-drop: <expr>` — the zero-zone short form (fires on any genuine release;
    `$` is the dragged element).

Inside a clause, `$zone` is the hovered zone's dataset (`$zone.col` ← `data-col`)
and `$payload` is the dragged datum.

### `@drop-zone(group?) as $zone { accepts?; over; reject }`

Declares an element as a place draggables can be dropped.

- `group` — namespaces zones↔draggables (default `""`).
- `as $zone` — names the zone descriptor (its dataset is read as `$zone.col`).
- `accepts: <expr>` — an optional predicate over `$payload` (the dragged datum),
  evaluated by the gesture at hover. A drop over a non-accepting zone runs the
  draggable's `on-cancel`, not `on-drop`.
- `over` / `reject` — `%states` driven by the live `$over` / `$reject` signals
  (`hovering && accepting` / `hovering && !accepting`).

## How it works (no `st.js` edits)

The runtime — a zone **registry** + an overlay-resilient **hit-test** — ships
*inside* the module via a `%emit prelude-js` block on a yield-less `dnd-runtime`
primitive (`window.__stDnd`, installed once, deduplicated by primitive name).
Both `@drag` and `@drop-zone` bind `dnd-runtime`, so the prelude is always
present; the global runtime (`public/runtime/st.js`) is untouched.

- `@drop-zone` registers `{ el, group, dataset, accepts, setHovering,
  setAccepting }` into `__stDnd.zones[group]` on mount; unregisters on cleanup.
- `@drag` binds the generic `gesture` (visual drag) **and** `drag-zones` (the
  drop lifecycle). `drag-zones` tracks the pointer, hit-tests `__stDnd` (NOT a
  selector), toggles each hovered zone's signals, and runs the matching lifecycle
  arm via `ST.runMutations` with `$zone`/`$payload` bound — the same rail
  `@handle`'s receive arms use.

## Files

```
dnd/
  MODULE.st                  manifest + rationale
  index.st                   public re-exports
  runtime.st                 dnd-runtime: __stDnd registry + hit-test (prelude-js)
  capture-types/lifecycle.st drag_lifecycle_arm + drag_lifecycle (kinds-as-data)
  primitives/drop-zone.st    zone registration + over/reject signals
  primitives/drag-zones.st   pointer tracking + lifecycle dispatch
  macros/drop-zone.st        @drop-zone surface
  macros/drag.st             @drag surface (the sole @drag)
  tests/                     headless: registry, lifecycle, bounds, kanban move
```

## Scope & follow-ups

In scope: cross-zone moves, accept/reject, `over`/`reject` highlight, `bounds:`.

Out of scope (clean follow-ups on the same registry): `@sortable`
(reorder-within-list), touch long-press, multi-select drag. The hover
**highlight** has a known bindState-init nuance (FUP-098); the core move is fully
functional.
