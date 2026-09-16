/* BoardLive's complete Spacetime-owned view (FEAT-162).
   The LiveView host supplies only an ignored structural mount; all behaviour,
   state projection, DnD, optimistic motion, cursors, and transient effects are
   declared here. */
@import "stdlib/macros/data-kind"
@import "stdlib/macros/handle"
@import "stdlib/macros/each"
@import "stdlib/macros/signal-flip"
@import "stdlib/primitives/pointer"
@import "stdlib/dnd"
@import "./$hosts.st"

/* Viewport coords ($x/$y), not element-relative ($localX/$localY): the
   local dot is `position: fixed` and the published payload is rendered
   into the peers' fixed cursor-layer — element-relative values would
   offset every cursor by the board's margin (and misalign peers whose
   boards sit at different viewport offsets). */
%macro local-pointer {
  %form { @local-pointer }
  %binds {
    pointer(&self, events: ["move"]) -> { $x, $y }
  }
}

@type Card {
  id: string;
  title: string;
  column: string;
  position: number;
}

@type Peer {
  id: string;
  name: string;
  color: string;
  x: number;
  y: number;
}

/* Authoritative server assigns. The reconciler writes these through the local
   signal channel; no rendered LiveView template crosses the bridge. */
@data subscribe $items from $board : items ;
@data subscribe $presence from $board : presence ;
@data inline $draft string : "" ;
@data inline $toast string : "" ;
@data inline $moveError string : "" ;

@data query $todo Card[] from $items { where: $.column == "todo"; }
@data query $doing Card[] from $items { where: $.column == "doing"; }
@data query $done Card[] from $items { where: $.column == "done"; }

/* `flash` is a transient typed push, deliberately not an assign: replaying an
   assign reconciliation can never replay a toast. */
@data stream $fx from $board {
  receive to Effect { "flash" => Flash($.payload.message); }
}

@data signal $move_card($card_id string, $column string, $version number) to $board {
  send emit "move_card" { card_id: $card_id, column: $column, version: $version }
  receive to MoveCardResult {
    "ok" => Moved($.reply);
    "err" => Rejected($.reply);
    _ => Rejected($.reply);
  }
  policy latest
}

@data signal $add_card($title string) to $board {
  send emit "add_card" { title: $title }
  receive to AddCardResult {
    "ok" => Added($.reply);
    "err" => AddRejected($.reply);
    _ => AddRejected($.reply);
  }
  policy latest
}

@data signal $cursor($cx number, $cy number) to $board {
  send emit "cursor" ephemeral { x: $cx, y: $cy }
  receive to CursorResult { _ => Sent($.reply); }
}

@data signal $lock_card($card_id string) to $board {
  send emit "lock_card" { card_id: $card_id }
  receive to LockCardResult {
    "ok" => Locked($.reply);
    "err" => LockRejected($.reply);
    _ => LockRejected($.reply);
  }
  policy latest
}

@template &card($card) {
  <article class="card" data-id="`$card.id`" data-column="`$card.column`" data-version="`$card.version`">
    <p class="card__title">`$card.title`</p>
    <button class="card__lock" aria-label="Lock card">Lock</button>
  </article>
}

@template &cursor($peer) {
  <div class="cursor" data-peer="`$peer.id`"
       style="--cursor-x: `$peer.x`; --cursor-y: `$peer.y`; --cursor-color: `$peer.color`;">
    <span class="cursor__label">`$peer.name`</span>
  </div>
}

/* Generic DnD: payload and destination are typed through data-* descriptors;
   no page-local event listener or JavaScript is involved. */
.card {
  @flip $items(duration: 260ms)
  @drag(group: "board-cards", payload: $.dataset) {
    dragging {
      scale: 1.03
      opacity: 0.9
      z-index: 20
    }
    on-drop $zone { $move_card($payload.id, $zone.column, $payload.version); }
  }
}

[data-column="todo"] {
  @drop-zone(group: "board-cards") as $zone {
    over { background: rgba(110, 168, 254, 0.12) }
  }
}
[data-column="doing"] {
  @drop-zone(group: "board-cards") as $zone {
    over { background: rgba(250, 204, 21, 0.12) }
  }
}
[data-column="done"] {
  @drop-zone(group: "board-cards") as $zone {
    over { background: rgba(74, 222, 128, 0.12) }
  }
}

[data-column="todo"] > .column__cards {
  @each($todo as $card, key: $card.id) {
    &card($card);
    :entering { opacity: 0 -> 1 translate-y: 8px -> 0 }
    :exiting { opacity: 1 -> 0 scale: 1 -> 0.97 }
  }
}
[data-column="doing"] > .column__cards {
  @each($doing as $card, key: $card.id) {
    &card($card);
    :entering { opacity: 0 -> 1 translate-y: 8px -> 0 }
    :exiting { opacity: 1 -> 0 scale: 1 -> 0.97 }
  }
}
[data-column="done"] > .column__cards {
  @each($done as $card, key: $card.id) {
    &card($card);
    :entering { opacity: 0 -> 1 translate-y: 8px -> 0 }
    :exiting { opacity: 1 -> 0 scale: 1 -> 0.97 }
  }
}

.cursor-layer {
  @each($presence as $peer, key: $peer.id) {
    &cursor($peer);
    :entering { opacity: 0 -> 1 }
    :exiting { opacity: 1 -> 0 }
  }
}

.board {
  @local-pointer
  --local-cursor-x: $x;
  --local-cursor-y: $y;

  /* Publish my cursor from the @local-pointer yields (viewport-coord signals).
     `$` in an @on body is the ELEMENT, not the DOM event — `$.clientX` reads a
     nonexistent property (undefined). @local-pointer's listener is registered
     first (file order), so $x/$y are fresh when this handler runs. */
  @on pointermove { $cursor($x, $y); }

  @handle $move_card {
    /* Snapshot before the instant local projection. `latest` keeps an old
       rejection from restoring over a newer drag. */
    optimistic {
      $beforeMoveItems <- $items;
      $items <- $items.map(item => item.id == $card_id ? Object.assign({}, item, { column: $column }) : item);
      $moveError <- "";
    }
    receive {
      Moved(_) => { $moveError <- ""; }
      Rejected(message) => { $items <- $beforeMoveItems; $moveError <- message; }
    }
  }
  @handle $add_card {
    receive {
      Added(_) => { $draft <- ""; }
      AddRejected(message) => { $moveError <- message; }
    }
  }
  @handle $lock_card {
    receive {
      Locked(_) => { }
      LockRejected(message) => { $moveError <- message; }
    }
  }
  @handle $fx {
    receive { Flash(message) => { $toast <- message; } }
  }
}

/* BUG-206: every @on stays at top-level selector scope. */
.new-card__input {
  value <- $draft;
  @on input { $draft <- $.value; }
}
.new-card__submit { @on click { $add_card({ title: $draft }); } }
.card__lock { @on click { $lock_card({ card_id: $.dataset.id }); } }

body {
  margin: 0;
  min-height: 100vh;
  background: radial-gradient(120% 100% at 50% 0%, #202841 0%, #090b12 62%);
  color: #eef2ff;
  font-family: Inter, ui-sans-serif, system-ui, sans-serif;
}

.board {
  position: relative;
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 1rem;
  max-width: 1200px;
  margin: 0 auto;
  padding: 2rem;
}
.board::after {
  content: "";
  position: fixed;
  z-index: 51;
  width: 0.6rem;
  height: 0.6rem;
  border: 2px solid #dbeafe;
  border-radius: 50%;
  pointer-events: none;
  transform: translate(calc(var(--local-cursor-x, 0) * 1px), calc(var(--local-cursor-y, 0) * 1px));
}
.column {
  min-height: 22rem;
  padding: 1rem;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 1rem;
  background: rgba(255, 255, 255, 0.035);
  transition: background 160ms ease, border-color 160ms ease;
}
.column__cards { display: grid; gap: 0.7rem; }
.card {
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 0.75rem;
  padding: 0.85rem;
  background: linear-gradient(135deg, #252d45, #171c2d);
  box-shadow: 0 10px 24px rgba(0, 0, 0, 0.2);
}
.card__title { margin: 0; line-height: 1.4; }
.card__lock, .new-card__submit {
  border: 0;
  border-radius: 0.5rem;
  padding: 0.45rem 0.7rem;
  background: #8bb7ff;
  color: #0b1220;
  font: inherit;
  font-weight: 700;
  cursor: pointer;
}
.card__lock { margin-top: 0.75rem; background: transparent; border: 1px solid rgba(255,255,255,.2); color: #cbd5e1; }
.cursor-layer { position: fixed; inset: 0; pointer-events: none; z-index: 50; }
.cursor { position: absolute; transform: translate(calc(var(--cursor-x) * 1px), calc(var(--cursor-y) * 1px)); transition: transform 80ms linear; }
.cursor::before { content: ""; display: block; width: 12px; height: 12px; border-radius: 50% 50% 50% 2px; background: var(--cursor-color, #8bb7ff); }
.cursor__label { margin-left: 14px; white-space: nowrap; color: var(--cursor-color, #8bb7ff); font-size: 0.72rem; font-weight: 700; }
#board-toasts > .toast { text: $toast; }
.toast { position: fixed; right: 1.5rem; bottom: 1.5rem; padding: 0.75rem 1rem; border-radius: 0.75rem; background: #16233b; box-shadow: 0 12px 32px rgba(0,0,0,.35); }
.toast:empty { display: none; }
@media (max-width: 760px) { .board { grid-template-columns: 1fr; padding: 1rem; } }
