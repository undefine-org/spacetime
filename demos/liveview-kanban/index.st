/* ============================================================================
   LiveView Kanban — a board that is ALIVE, SHARED, and SELF-AWARE.

   The ENTIRE client is this one .st file. In dev it is driven by verse's own
   multiplayer dev-ws; flip the single @host line to `live("DemoWeb.KanbanLive")`
   and the same page runs over a real Phoenix LiveView socket — byte-identical.

   "The host is config. The page is the program."
   ============================================================================ */

@import "stdlib/macros/host"
@import "stdlib/macros/data-kind"
@import "stdlib/macros/handle"
@import "stdlib/macros/each"
@import "stdlib/dnd"
@import "stdlib/macros/magnetic-glow"
@import "stdlib/macros/timeline"
@import "stdlib/macros/websocket"
@import "stdlib/macros/bindings"

/* ---- The host. ONE line is the only difference between verse and Phoenix. --- */
@host $board : ws("/ws")
// Wave C: @host $board : live("DemoWeb.KanbanLive")

/* ---- Types ---------------------------------------------------------------- */
@type Card {
  id: string;
  title: string;
  col: string;
  tag: string;
}

/* ---- Live, broadcast-backed state. A move = a reorder op = multiplayer. ---- */
@data collection $cards Card[] from "data/board.json" via dev-ws {
  initial: [];
}

/* ---- Derived metrics ARE the board's own signal (LIVE substrate). ---------
   Columns project from $viewedCards (the time-travel head), NOT $cards directly:
   at HEAD $viewedCards === $cards (live); while scrubbing it is a past frame, so
   the whole board replays for free (a column is a pure fn of the viewed board). */
@data query $todo  Card[] from $viewedCards { where: $.col == "todo"; }
@data query $doing Card[] from $viewedCards { where: $.col == "doing"; }
@data query $done  Card[] from $viewedCards { where: $.col == "done"; }

@data fold $todoCount  number from $todo  : acc + 1 ;
@data fold $doingCount number from $doing : acc + 1 ;
@data fold $doneCount  number from $done  : acc + 1 ;

/* ---- The move signal: optimistic out, typed reply in (co-located). -------- */
@data signal $move(card string, to string) to $board {
  send emit "move" { card: $card, to: $to }
  receive to MoveResult {
    "ok"      => Moved($.payload as Card);
    "blocked" => Blocked($.payload as Card);
    _         => Failed($.payload);
  }
  policy latest
}

/* ---- Per-card template ---------------------------------------------------- */
/* The card template is PURE MARKUP — a @template body hosts no directives or
   comments (those become text). Behaviour (drag, glow) lives in the .card
   selector scope below. */
@template &card($c) {
  <article class="card" data-id="`$c.id`" data-tag="`$c.tag`">
    <span class="card__tag">`$c.tag`</span>
    <p class="card__title">`$c.title`</p>
  </article>
}

/* Card behaviour, attached by selector (applies to every rendered .card):
   the cursor melts into the card (glow); drag fires the optimistic $move,
   whose @handle on .board reconciles against the server reply and springs
   a rejected card back (the dragging-state release).

   The drop is fully DECLARATIVE (stdlib/dnd, PLAN-058): `@drag(group, payload)`
   pairs with `@drop-zone(group:)` on each column; `on-drop $zone` fires the move
   with `$payload.id` (the card, from its data-id) and `$zone.col` (the
   destination column, from the zone's data-col). Zero `ST.*`, zero `$.dataset`
   string-fishing, zero magic selectors — the page is the program. */
.card {
  @magnetic-glow(radius: 160, color: "#6ea8fe", intensity: 0.5)
  @drag(group: "cards", payload: $.dataset, axis: "both", momentum: true) {
    dragging {
      scale: 1.04
      box-shadow: 0 18px 48px rgba(0,0,0,0.45)
      z-index: 50
    }
    on-drop $zone { $move({ card: $payload.id, to: $zone.col }); }
  }
}

/* ---- Columns: each binds @each to its HTML container (.col__cards under the
   matching [data-col]). Keyed reconciliation + enter/exit live HERE, in the
   selector scope — NOT inside a @template (a template body can't host @each). - */
/* ---- Drop zones: each column is a first-class place cards can be dropped
   (stdlib/dnd). `group: "cards"` pairs with the card's `@drag(group: "cards")`;
   `data-col` is read by the drop as `$zone.col`. `over` highlights the column
   while a card hovers it — a live signal, no JS. */
[data-col="todo"]  { @drop-zone(group: "cards") as $zone { over { background: rgba(255,255,255,0.045) } } }
[data-col="doing"] { @drop-zone(group: "cards") as $zone { over { background: rgba(255,255,255,0.045) } } }
[data-col="done"]  { @drop-zone(group: "cards") as $zone { over { background: rgba(255,255,255,0.045) } } }

[data-col="todo"] > .col__cards {
  @each($todo as $c, key: $c.id) {
    &card($c);
    :entering { opacity: 0 -> 1  translate-y: 8px -> 0 }
    :exiting  { opacity: 1 -> 0  scale: 1 -> 0.96 }
  }
}
[data-col="doing"] > .col__cards {
  @each($doing as $c, key: $c.id) {
    &card($c);
    :entering { opacity: 0 -> 1  translate-y: 8px -> 0 }
    :exiting  { opacity: 1 -> 0  scale: 1 -> 0.96 }
  }
}
[data-col="done"] > .col__cards {
  @each($done as $c, key: $c.id) {
    &card($c);
    :entering { opacity: 0 -> 1  translate-y: 8px -> 0 }
    :exiting  { opacity: 1 -> 0  scale: 1 -> 0.96 }
  }
}

/* ---- The board: consume the move signal (optimistic, confirm, spring-back). - */
.board {
  @handle $move {
    /* Optimistic: snapshot the board into history (for time-travel scrub),
       then move the card immediately — before the server confirms. */
    optimistic {
      $history <- $history.concat([$cards]);
      $scrub <- $history.length;
      $cards.update($card, { col: $to });
    }
    /* Reply: reconcile the optimistic card against the server's authority.
       Moved/Blocked both replace the card; Failed surfaces the error. */
    receive {
      Moved(c)   => { $cards <- $cards.map(x => x.id == c.id ? c : x); }
      Blocked(c) => { $cards <- $cards.map(x => x.id == c.id ? c : x); }
      Failed(m)  => { $error <- m; }
    }
    /* Terminal: release the drag-state regardless of outcome. */
    final { $dragging <- null; }
  }
}

/* ============================================================================
   SHARED — multiplayer presence. Every collaborator's cursor, live.
   ============================================================================ */
@type Peer {
  x: number;
  y: number;
  name: string;
  color: string;
}

@template &peer($p) {
  <div class="peer" data-user="`$p.id`"
       style="--px: `$p.x`; --py: `$p.y`; --pc: `$p.color`;">
    <span class="peer__label">`$p.name`</span>
  </div>
}

/* First-class multiplayer presence (BUG-126 fixed: %binds yields now resolve).
   @presence yields $users (live peer list) + $set (broadcast my state) + $myId.
   Real cursors over the /ws presence channel, zero custom JS. */
.stage {
  @presence(room: "kanban", throttle: 30ms) as peers: Peer;

  @each($users as $p, key: $p.id) {
    &peer($p);
    :entering { opacity: 0 -> 1 }
    :exiting  { opacity: 1 -> 0 }
  }

  @on &.mousemove {
    $set({ x: $.clientX, y: $.clientY });
  }
}

/* ============================================================================
   LIVE — time-travel. Fold every move into a history; scrub to re-project.
   The board is a pure function of a signal, so replay is free.
   ============================================================================ */
@data inline $history Card[][] : [] ;
@data inline $scrub number : 0 ;

/* The viewed board: live HEAD, or a past frame when scrubbing back. */
@data derive $viewedCards Card[] :
  $scrub >= $history.length ? $cards : $history[$scrub] ;

/* The scrubber input lives in index.html; wire its reactive surface here:
   - `max <- $history.length` grows the track as moves accrue (was hardcoded 0);
   - `value <- $scrub` keeps the thumb in sync when a new move snaps to HEAD;
   - `@on &.input` scrubs the viewed frame as you drag the thumb back. */
.tt__range {
  max <- $history.length;
  value <- $scrub;
  @on &.input {
    $scrub <- $.valueAsNumber;
  }
}

/* ============================================================================
   SELF-AWARE — the board renders its OWN protocol, read from the compiler
   registry (cannot drift from the code). The unique flex.
   ============================================================================ */
@data dispatch $proto from kind "data" ;

/* Mirror reference.st's WORKING pattern: @each in a selector scope with the row
   markup INLINE in the @each body (not a &template invocation). The compiler
   SSG-unrolls the static @data dispatch rows into the .proto container. */
.proto {
  @each($proto as $m, key: $m.name) {
    <li class="proto__row">
      <code class="proto__name">`$m.directive` `$m.name`</code>
      <span class="proto__sig">`$m.signature`</span>
    </li>
  }
}


/* ============================================================================
   Styling — dark, calm, so the LIVE motion reads clearly. Plain CSS.
   ============================================================================ */
body {
  margin: 0;
  min-height: 100vh;
  background: radial-gradient(130% 130% at 50% 0%, #161a24 0%, #0a0b0f 60%);
  color: #e8eaf2;
  font-family: "Inter", system-ui, -apple-system, sans-serif;
  -webkit-font-smoothing: antialiased;
}

.stage {
  position: relative;
  max-width: 1100px;
  margin: 0 auto;
  padding: 2rem 1.5rem 4rem;
}

.hostbadge {
  display: inline-block;
  font-family: "JetBrains Mono", ui-monospace, monospace;
  font-size: 0.78rem;
  color: #8fb4ff;
  background: rgba(110,168,254,0.10);
  border: 1px solid rgba(110,168,254,0.30);
  border-radius: 999px;
  padding: 0.3rem 0.8rem;
  margin-bottom: 1.5rem;
}

/* ---- Board grid ---- */
.board {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 1rem;
  align-items: start;
}

.col {
  background: rgba(255,255,255,0.025);
  border: 1px solid rgba(255,255,255,0.06);
  border-radius: 16px;
  padding: 1rem;
  min-height: 260px;
  transition: border-color 0.2s ease, background 0.2s ease;
}
.col__head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  margin-bottom: 0.9rem;
}
.col__head h2 {
  margin: 0;
  font-size: 0.82rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: #aeb4c8;
}
.col__cards { display: flex; flex-direction: column; gap: 0.6rem; }

/* ---- Cards ---- */
.card {
  position: relative;
  background: linear-gradient(180deg, #1c2130 0%, #161a26 100%);
  border: 1px solid rgba(255,255,255,0.08);
  border-radius: 12px;
  padding: 0.85rem 0.9rem;
  cursor: grab;
  user-select: none;
  box-shadow: 0 1px 0 rgba(255,255,255,0.04) inset;
  transition: transform 0.18s cubic-bezier(0.34,1.56,0.64,1), box-shadow 0.18s ease;
}
.card:active { cursor: grabbing; }
.card__tag {
  display: inline-block;
  font-size: 0.66rem;
  font-weight: 700;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: #7f88b8;
  margin-bottom: 0.4rem;
}
.card[data-tag="runtime"] .card__tag { color: #6ee7b7; }
.card[data-tag="ux"]      .card__tag { color: #f7a072; }
.card[data-tag="meta"]    .card__tag { color: #c4a0ff; }
.card[data-tag="spec"]    .card__tag { color: #6ea8fe; }
.card__title { margin: 0; font-size: 0.92rem; line-height: 1.35; color: #eef0f8; }

/* ---- Peer cursors (SHARED) ---- */
.peer {
  position: absolute;
  left: 0; top: 0;
  transform: translate(calc(var(--px) * 1px), calc(var(--py) * 1px));
  pointer-events: none;
  z-index: 80;
  transition: transform 0.12s linear;
}
.peer::before {
  content: "";
  display: block;
  width: 14px; height: 14px;
  background: var(--pc);
  border-radius: 50% 50% 50% 2px;
  box-shadow: 0 2px 8px rgba(0,0,0,0.4);
}
.peer__label {
  position: absolute;
  left: 16px; top: 2px;
  font-size: 0.7rem;
  font-weight: 600;
  color: #0a0b0f;
  background: var(--pc);
  padding: 0.1rem 0.4rem;
  border-radius: 6px;
  white-space: nowrap;
}

/* ---- Time-travel scrubber (LIVE) ---- */
.timetravel {
  margin-top: 1.5rem;
  display: flex;
  align-items: center;
  gap: 0.8rem;
}
.tt__label {
  font-size: 0.72rem;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: #7f88b8;
}
.tt__range { flex: 1; accent-color: #6ea8fe; }

/* ---- Self-aware contract panel (SELF-AWARE) ---- */
.contract {
  margin-top: 2rem;
  background: rgba(196,160,255,0.04);
  border: 1px solid rgba(196,160,255,0.18);
  border-radius: 14px;
  padding: 1rem 1.2rem;
}
.contract__head {
  font-size: 0.72rem;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  color: #c4a0ff;
  margin-bottom: 0.6rem;
}
.proto { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 0.35rem; }
.proto__row { display: flex; gap: 0.8rem; align-items: baseline; font-size: 0.8rem; }
.proto__name { font-family: "JetBrains Mono", ui-monospace, monospace; color: #9fb8ff; white-space: nowrap; }
.proto__sig { color: #6b7290; font-size: 0.74rem; overflow: hidden; text-overflow: ellipsis; }

