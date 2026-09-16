/// @drag on-drop firing tests (PLAN-053).
///
/// The `@drag { on-drop: <action> }` clause must RUN its action when a drag
/// completes (the gesture's `$active` transitions true -> false). Before
/// PLAN-053 the macro-body `%on $active -> false { ... }` clause was silently
/// dropped by the Evaluate layer, so on-drop never fired — drag-and-drop was
/// dead (demos/liveview-kanban).
///
/// These mount a real `@drag` on a selector, drive a pointer gesture
/// (pointerdown -> pointermove -> pointerup, the events the gesture primitive
/// listens for), and assert the on-drop action ran. The V8 (LinkeDOM) backend
/// has no PointerEvent constructor, so we synthesize a plain bubbling Event and
/// attach the pointer fields the gesture handler reads.

@import "stdlib/testing/test"
@import "stdlib/dnd"

// =============================================================================
// on-drop fires a signal call when the drag completes
// =============================================================================

@test "@drag on-drop fires a signal call on drop" {
  @fixture {
    <div class="drag-sig" data-id="c9">card</div>
  }
  @given .drag-sig {
    @drag(axis: "both") {
      on-drop: $move({ card: $.dataset.id })
    }
  }

  @let (el = document.querySelector('.drag-sig'))
  @eval (el.setPointerCapture = function(){})
  @eval (el.releasePointerCapture = function(){})

  // Register a fake signal so the on-drop signal call has something to fire.
  @eval (ST.set(el, 'fired', null))
  @eval (ST.registerSignalCall('move', function(p){ ST.set(el, 'fired', p); }))

  // Flush pending selector-inits so the @drag fragments (gesture + the on-drop
  // transition watcher) attach to the mounted node before we drive the gesture.
  // A `@when` drives the harness's init flush (the MutationObserver chain).
  @when .drag-sig mouseenter

  // Synthesize pointer events (no PointerEvent ctor under LinkeDOM): a plain
  // bubbling Event carrying the pointerId + client coords the gesture reads.
  @eval (window.__mkPtr = function(type, x, y){ var e = new Event(type, { bubbles: true, cancelable: true }); e.pointerId = 1; e.clientX = x; e.clientY = y; return e; })

  @eval (el.dispatchEvent(window.__mkPtr('pointerdown', 5, 5)))
  @eval (el.dispatchEvent(window.__mkPtr('pointermove', 60, 60)))
  @eval (el.dispatchEvent(window.__mkPtr('pointerup', 60, 60)))
  @wait 50ms

  @assert (ST.get(el, 'fired') && ST.get(el, 'fired').card === "c9")
    "on-drop should fire the $move signal with the card id when the drag ends"
}

@test "@drag on-drop does not fire mid-gesture (only on release)" {
  @fixture {
    <div class="drag-mid" data-id="c2">card</div>
  }
  @given .drag-mid {
    @drag(axis: "both") {
      on-drop: $move({ card: $.dataset.id })
    }
  }

  @let (el = document.querySelector('.drag-mid'))
  @eval (el.setPointerCapture = function(){})
  @eval (el.releasePointerCapture = function(){})

  @eval (ST.set(el, 'fired', null))
  @eval (ST.registerSignalCall('move', function(p){ ST.set(el, 'fired', p); }))
  @when .drag-mid mouseenter
  @eval (window.__mkPtr = function(type, x, y){ var e = new Event(type, { bubbles: true, cancelable: true }); e.pointerId = 1; e.clientX = x; e.clientY = y; return e; })

  // Press + move only — no release yet. on-drop must NOT have fired.
  @eval (el.dispatchEvent(window.__mkPtr('pointerdown', 5, 5)))
  @eval (el.dispatchEvent(window.__mkPtr('pointermove', 60, 60)))
  @wait 50ms

  @assert (ST.get(el, 'fired') === null)
    "on-drop must not fire while the drag is still active (before pointerup)"
}

@test "@drag gesture yields release coords at drop" {
  @fixture {
    <div class="drag-coords" data-id="c5">card</div>
  }
  @given .drag-coords {
    @drag(axis: "both") {
      on-drop: $noop()
    }
  }

  @let (el = document.querySelector('.drag-coords'))
  @eval (el.setPointerCapture = function(){})
  @eval (el.releasePointerCapture = function(){})
  @when .drag-coords mouseenter

  @eval (window.__mkc = function(t, x, y){ var e = new Event(t, { bubbles: true, cancelable: true }); e.pointerId = 1; e.clientX = x; e.clientY = y; return e; })
  @eval (el.dispatchEvent(window.__mkc('pointerdown', 10, 10)))
  @eval (el.dispatchEvent(window.__mkc('pointermove', 60, 80)))
  @eval (el.dispatchEvent(window.__mkc('pointerup', 123, 217)))
  @wait 50ms

  // The release coords must be captured for drop hit-testing.
  @then $dropX on .drag-coords should equal 123
  @then $dropY on .drag-coords should equal 217
}

@test "@drag on-drop does NOT fire on a cancelled gesture (audit #1)" {
  @fixture {
    <div class="drag-cancel" data-id="c6">card</div>
  }
  @given .drag-cancel {
    @drag(axis: "both") {
      on-drop: $move({ card: $.dataset.id })
    }
  }

  @let (el = document.querySelector('.drag-cancel'))
  @eval (el.setPointerCapture = function(){})
  @eval (el.releasePointerCapture = function(){})
  @eval (ST.set(el, 'fired', null))
  @eval (ST.registerSignalCall('move', function(p){ ST.set(el, 'fired', p); }))
  @when .drag-cancel mouseenter

  @eval (window.__mkx = function(t, x, y){ var e = new Event(t, { bubbles: true, cancelable: true }); e.pointerId = 1; e.clientX = x; e.clientY = y; return e; })
  // Press + move, then CANCEL (escape / interrupted pointer) — NOT a release.
  @eval (el.dispatchEvent(window.__mkx('pointerdown', 10, 10)))
  @eval (el.dispatchEvent(window.__mkx('pointermove', 80, 90)))
  @eval (el.dispatchEvent(window.__mkx('pointercancel', 80, 90)))
  @wait 50ms

  // A cancelled drag is not a drop: on-drop must NOT fire (no phantom $move).
  @assert (ST.get(el, 'fired') === null)
    "a cancelled gesture must not fire on-drop"
}

@test "@drag on-drop fires once on a genuine release after a prior cancel" {
  @fixture {
    <div class="drag-recover" data-id="c7">card</div>
  }
  @given .drag-recover {
    @drag(axis: "both") {
      on-drop: $move({ card: $.dataset.id })
    }
  }

  @let (el = document.querySelector('.drag-recover'))
  @eval (el.setPointerCapture = function(){})
  @eval (el.releasePointerCapture = function(){})
  @eval (window.__rcount = 0)
  @eval (ST.registerSignalCall('move', function(p){ window.__rcount = window.__rcount + 1; ST.set(el, 'last', p); }))
  @when .drag-recover mouseenter

  @eval (window.__mkr = function(t, x, y){ var e = new Event(t, { bubbles: true, cancelable: true }); e.pointerId = 1; e.clientX = x; e.clientY = y; return e; })
  // First gesture: cancel (no fire).
  @eval (el.dispatchEvent(window.__mkr('pointerdown', 10, 10)))
  @eval (el.dispatchEvent(window.__mkr('pointermove', 50, 50)))
  @eval (el.dispatchEvent(window.__mkr('pointercancel', 50, 50)))
  @wait 30ms
  // Second gesture: genuine release (fires exactly once).
  @eval (el.dispatchEvent(window.__mkr('pointerdown', 10, 10)))
  @eval (el.dispatchEvent(window.__mkr('pointermove', 70, 70)))
  @eval (el.dispatchEvent(window.__mkr('pointerup', 70, 70)))
  @wait 50ms
  @eval (ST.set(el, 'count', window.__rcount))

  @then $count on .drag-recover should equal 1
  @then $last on .drag-recover should equal (JSON.parse('{"card":"c7"}'))
}
