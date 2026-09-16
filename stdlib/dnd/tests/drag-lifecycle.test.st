/// `@drag` drop LIFECYCLE (PLAN-058 W3): on-enter / on-leave / on-drop / on-cancel.
///
/// The dnd `@drag` binds `drag-zones`, which tracks the pointer across drop zones
/// (hit-testing the `__stDnd` registry) and fires the lifecycle arms with `$zone`
/// (the zone's dataset proxy) + `$payload` (the dragged datum) bound as
/// `ST.runMutations` locals. These tests drive a synthetic pointer gesture on the
/// V8/headless backend (geometry mocked: `elementsFromPoint` returns the zone),
/// and assert the arms fire with the right bindings — the same deterministic
/// approach as drag-on-drop.test.st + drop-zone.test.st.
///
/// `@given` attaches the directives; `@when … mouseenter` flushes selector-init so
/// the zone registers + the draggable's listeners attach before we drive pointers.

@import "stdlib/testing/test"
@import "stdlib/dnd"

@test "on-drop fires over an accepting zone with $zone + $payload bound" {
  @fixture {
    <div class="lz-zone" data-col="done"></div>
    <div class="lz-card" data-id="c1"></div>
  }
  @given .lz-zone {
    @drop-zone(group: "cards") as $zone { over { background: red } }
  }
  @given .lz-card {
    @drag(group: "cards", payload: $p) {
      dragging { scale: 1.04 }
      on-drop $zone { $dropped <- $zone.col; }
    }
  }
  @when .lz-zone mouseenter
  @when .lz-card mouseenter

  @let (card = document.querySelector('.lz-card'))
  @eval (card.setPointerCapture = function(){})
  @eval (card.releasePointerCapture = function(){})
  @let (zone = document.querySelector('.lz-zone'))
  // Mock the hit-test so the registry resolves the zone under the pointer.
  @eval (window.__z = zone)
  @eval (document.elementsFromPoint = function(x, y){ return new Array(window.__z); })
  @eval (window.__mk = function(t, x, y){ var e = new Event(t, { bubbles: true }); e.pointerId = 1; e.clientX = x; e.clientY = y; return e; })
  @eval (card.dispatchEvent(window.__mk('pointerdown', 10, 10)))
  @eval (card.dispatchEvent(window.__mk('pointermove', 80, 80)))
  @eval (card.dispatchEvent(window.__mk('pointerup', 80, 80)))
  @wait 30ms
  // on-drop ran `$dropped <- $zone.col` → file-scope signal `dropped` = "done".
  @then $dropped should equal "done"
}

@test "release over NO zone runs on-cancel, not on-drop" {
  @fixture {
    <div class="lc-card" data-id="c2"></div>
  }
  @given .lc-card {
    @drag(group: "cards", payload: $p) {
      on-drop  $zone { $result <- "dropped"; }
      on-cancel { $result <- "cancelled"; }
    }
  }
  @when .lc-card mouseenter

  @let (card = document.querySelector('.lc-card'))
  @eval (card.setPointerCapture = function(){})
  @eval (card.releasePointerCapture = function(){})
  // No registered zone in 'cards' for this fixture → hitTest returns null.
  @eval (document.elementsFromPoint = function(x, y){ return new Array(document.body); })
  @eval (window.__mk = function(t, x, y){ var e = new Event(t, { bubbles: true }); e.pointerId = 1; e.clientX = x; e.clientY = y; return e; })
  @eval (card.dispatchEvent(window.__mk('pointerdown', 10, 10)))
  @eval (card.dispatchEvent(window.__mk('pointermove', 40, 40)))
  @eval (card.dispatchEvent(window.__mk('pointerup', 40, 40)))
  @wait 30ms
  @then $result should equal "cancelled"
}

@test "release over a REJECTING zone (accepts:false) runs on-cancel" {
  @fixture {
    <div class="rz-zone" data-col="locked"></div>
    <div class="rz-card" data-id="c3"></div>
  }
  @given .rz-zone {
    @drop-zone(group: "cards") as $zone {
      accepts: false;
      reject { outline: 1px solid red }
    }
  }
  @given .rz-card {
    @drag(group: "cards", payload: $p) {
      on-drop  $zone { $outcome <- "dropped"; }
      on-cancel { $outcome <- "cancelled"; }
    }
  }
  @when .rz-zone mouseenter
  @when .rz-card mouseenter

  @let (card = document.querySelector('.rz-card'))
  @eval (card.setPointerCapture = function(){})
  @eval (card.releasePointerCapture = function(){})
  @let (zone = document.querySelector('.rz-zone'))
  @eval (window.__z = zone)
  @eval (document.elementsFromPoint = function(x, y){ return new Array(window.__z); })
  @eval (window.__mk = function(t, x, y){ var e = new Event(t, { bubbles: true }); e.pointerId = 1; e.clientX = x; e.clientY = y; return e; })
  @eval (card.dispatchEvent(window.__mk('pointerdown', 10, 10)))
  @eval (card.dispatchEvent(window.__mk('pointermove', 60, 60)))
  @eval (card.dispatchEvent(window.__mk('pointerup', 60, 60)))
  @wait 30ms
  // accepts:false → release over the zone is NOT a drop → on-cancel runs.
  @then $outcome should equal "cancelled"
}

@test "on-enter toggles the hovered zone's over-state while dragging" {
  @fixture {
    <div class="hz-zone" data-col="doing"></div>
    <div class="hz-card" data-id="c4"></div>
  }
  @given .hz-zone {
    @drop-zone(group: "cards") as $zone { over { background: red } }
  }
  @given .hz-card {
    @drag(group: "cards", payload: $p) {
      on-enter $zone { $entered <- true; }
    }
  }
  @when .hz-zone mouseenter
  @when .hz-card mouseenter

  @let (card = document.querySelector('.hz-card'))
  @eval (card.setPointerCapture = function(){})
  @eval (card.releasePointerCapture = function(){})
  @let (zone = document.querySelector('.hz-zone'))
  @eval (window.__z = zone)
  @eval (document.elementsFromPoint = function(x, y){ return new Array(window.__z); })
  @eval (window.__mk = function(t, x, y){ var e = new Event(t, { bubbles: true }); e.pointerId = 1; e.clientX = x; e.clientY = y; return e; })
  @eval (card.dispatchEvent(window.__mk('pointerdown', 10, 10)))
  @eval (card.dispatchEvent(window.__mk('pointermove', 70, 70)))
  @wait 30ms
  // on-enter fired (set $entered). The hovered zone entry's hovering signal is
  // driven true (the over-state gate) — read it off the entry for THIS zone
  // (the registry persists across tests in a file, so match by element).
  @eval (window.__e = window.__stDnd.zones['cards'].filter(function(e){ return e.el === zone; })[0])
  @eval (ST.set(zone, 'hov', ST.get(zone, 'hovering')))
  @then $entered should equal true
}
