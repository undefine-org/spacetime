/// End-to-end drag→drop move (PLAN-058): the liveview-kanban migration pattern.
///
/// A card carries its fields in `data-*`; `@drag(payload: $.dataset)` binds them
/// as `$payload`, and `on-drop $zone { … }` fires the move with `$payload.id`
/// (the card) + `$zone.col` (the destination column, read off the drop-zone's
/// `data-col`). Proves the full path the kanban uses: zero `ST.*`, zero magic
/// selectors, zero `$.dataset` string-fishing in the page.

@import "stdlib/testing/test"
@import "stdlib/dnd"
@test "kanban-style drag: payload from dataset, drop to zone col" {
  @fixture {
    <div class="k-col" data-col="done"></div>
    <div class="k-card" data-id="c7"></div>
  }
  @given .k-col {
    @drop-zone(group: "cards") as $zone { over { background: red } }
  }
  @given .k-card {
    @drag(group: "cards", payload: $.dataset) {
      dragging { scale: 1.04 }
      on-drop $zone { $moved <- $payload.id; $dest <- $zone.col; }
    }
  }
  @when .k-col mouseenter
  @when .k-card mouseenter
  @let (card = document.querySelector('.k-card'))
  @let (zone = document.querySelector('.k-col'))
  @eval (card.setPointerCapture = function(){})
  @eval (card.releasePointerCapture = function(){})
  @eval (window.__z = zone)
  @eval (document.elementsFromPoint = function(x, y){ return new Array(window.__z); })
  @eval (window.__mk = function(t, x, y){ var e = new Event(t, { bubbles: true }); e.pointerId = 1; e.clientX = x; e.clientY = y; return e; })
  @eval (card.dispatchEvent(window.__mk('pointerdown', 10, 10)))
  @eval (card.dispatchEvent(window.__mk('pointermove', 80, 80)))
  @eval (card.dispatchEvent(window.__mk('pointerup', 80, 80)))
  @wait 30ms
  @then $moved should equal "c7"
  @then $dest should equal "done"
}
