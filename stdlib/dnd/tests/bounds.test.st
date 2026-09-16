/// `@drag(bounds: &el)` clamps the drag to an element (PLAN-057, the W4 fix).
///
/// Before W4 the bounds branch was INERT: `$bounds` (an `&name` Element capture)
/// inlined `null` in the detached derive-compute closure, so `$bounds != none`
/// was `null != null` (always false) and the clamp never ran. W4 resolves the
/// bound element through the `__stRefs` registry (`ST.ref("name")`), so the
/// lowered derive reads a REAL rect and the clamp is live.
///
/// The V8/headless backend has no layout, so we MOCK the bound element's rect
/// (via ST.ref + getBoundingClientRect) and drive a synthetic gesture, asserting
/// the derived `$x` is clamped to `[0, track.width - handle.width]`.

@import "stdlib/testing/test"
@import "stdlib/dnd"

@test "bounds clamps the drag translateX to the bound element's width" {
  @fixture {
    <div class="bd-track">
      <div class="bd-handle"></div>
    </div>
  }
  @given .bd-handle {
    @drag(axis: "x", bounds: &track) {
      dragging { scale: 1.0 }
    }
  }
  @when .bd-handle mouseenter

  @let (handle = document.querySelector('.bd-handle'))
  @eval (handle.setPointerCapture = function(){})
  @eval (handle.releasePointerCapture = function(){})
  // Mock the bound element: register it in __stRefs (what the element-ref
  // primitive does in production) with a fixed 200×20 rect; the handle is 40 wide.
  @eval (window.__trackRect = { x: 0, y: 0, width: 200, height: 20, top: 0, left: 0, right: 200, bottom: 20 })
  @eval (window.__track = { getBoundingClientRect: function(){ return window.__trackRect; } })
  @eval (window.__stRefs = window.__stRefs || {})
  @eval (window.__stRefs['track'] = window.__track)
  @eval (window.__handleRect = { x: 0, y: 0, width: 40, height: 20, top: 0, left: 0, right: 40, bottom: 20 })
  @eval (handle.getBoundingClientRect = function(){ return window.__handleRect; })

  @eval (window.__mk = function(t, x, y){ var e = new Event(t, { bubbles: true }); e.pointerId = 1; e.clientX = x; e.clientY = y; return e; })
  // Drag far past the right edge (deltaX = 500); clamp ceiling = 200 - 40 = 160.
  @eval (handle.dispatchEvent(window.__mk('pointerdown', 10, 10)))
  @eval (handle.dispatchEvent(window.__mk('pointermove', 510, 10)))
  @wait 40ms
  @eval (ST.set(handle, 'clampedX', ST.get(handle, 'x')))
  @then $clampedX on .bd-handle should equal 160
}

@test "bounds leaves an in-range drag unclamped" {
  @fixture {
    <div class="br-track">
      <div class="br-handle"></div>
    </div>
  }
  @given .br-handle {
    @drag(axis: "x", bounds: &track2) {
      dragging { scale: 1.0 }
    }
  }
  @when .br-handle mouseenter

  @let (handle = document.querySelector('.br-handle'))
  @eval (handle.setPointerCapture = function(){})
  @eval (handle.releasePointerCapture = function(){})
  @eval (window.__t2Rect = { x: 0, y: 0, width: 200, height: 20, top: 0, left: 0, right: 200, bottom: 20 })
  @eval (window.__t2 = { getBoundingClientRect: function(){ return window.__t2Rect; } })
  @eval (window.__stRefs = window.__stRefs || {})
  @eval (window.__stRefs['track2'] = window.__t2)
  @eval (window.__hr2 = { x: 0, y: 0, width: 40, height: 20, top: 0, left: 0, right: 40, bottom: 20 })
  @eval (handle.getBoundingClientRect = function(){ return window.__hr2; })
  @eval (window.__mk = function(t, x, y){ var e = new Event(t, { bubbles: true }); e.pointerId = 1; e.clientX = x; e.clientY = y; return e; })
  // deltaX = 80, within [0, 160] → no clamp.
  @eval (handle.dispatchEvent(window.__mk('pointerdown', 10, 10)))
  @eval (handle.dispatchEvent(window.__mk('pointermove', 90, 10)))
  @wait 40ms
  @eval (ST.set(handle, 'x80', ST.get(handle, 'x')))
  @then $x80 on .br-handle should equal 80
}
