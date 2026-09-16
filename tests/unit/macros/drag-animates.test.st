/// @drag %animates — card follows the cursor (PLAN-054).
///
/// `@drag`'s `%animates { translate-x: $x; translate-y: $y }` lowers to reactive
/// ST.derive computed signals (fed by the gesture's $deltaX/$deltaY) plus an
/// ST.bindTransform that composes the channels. Dragging the element must update
/// its CSS transform live. Before PLAN-054 the %animates clause was dropped, so
/// the element never moved while dragging.
///
/// `@when ... mouseenter` drives the harness's selector-init flush so the @drag
/// fragments (gesture + the %animates derive watchers) attach to the mounted
/// node before we drive the pointer gesture.

@import "stdlib/testing/test"
@import "stdlib/dnd"

@test "@drag derives x/y reactively from the gesture delta" {
  @fixture {
    <div class="drag-anim" data-id="c1"></div>
  }
  @given .drag-anim {
    @drag(axis: "both") {
      on-drop: $noop()
    }
  }

  @let (el = document.querySelector('.drag-anim'))
  @eval (el.setPointerCapture = function(){})
  @eval (el.releasePointerCapture = function(){})

  // Flush selector-inits onto the node (registers the derive watchers).
  @when .drag-anim mouseenter

  @eval (window.__mk = function(t, x, y){ var e = new Event(t, { bubbles: true, cancelable: true }); e.pointerId = 1; e.clientX = x; e.clientY = y; return e; })
  @eval (el.dispatchEvent(window.__mk('pointerdown', 10, 10)))
  @eval (el.dispatchEvent(window.__mk('pointermove', 90, 140)))
  @wait 60ms

  // deltaX = 90-10 = 80, deltaY = 140-10 = 130; axis "both" so x=deltaX, y=deltaY.
  @eval (ST.set(el, 'probeX', ST.get(el, 'x')))
  @eval (ST.set(el, 'probeY', ST.get(el, 'y')))

  @then $probeX on .drag-anim should equal 80
  @then $probeY on .drag-anim should equal 130
}

@test "@drag updates the CSS transform while dragging" {
  @fixture {
    <div class="drag-tf" data-id="c2"></div>
  }
  @given .drag-tf {
    @drag(axis: "both") {
      on-drop: $noop()
    }
  }

  @let (el = document.querySelector('.drag-tf'))
  @eval (el.setPointerCapture = function(){})
  @eval (el.releasePointerCapture = function(){})

  @when .drag-tf mouseenter

  @eval (window.__mk2 = function(t, x, y){ var e = new Event(t, { bubbles: true, cancelable: true }); e.pointerId = 1; e.clientX = x; e.clientY = y; return e; })
  @eval (el.dispatchEvent(window.__mk2('pointerdown', 0, 0)))
  @eval (el.dispatchEvent(window.__mk2('pointermove', 50, 70)))
  @wait 60ms

  // The composed transform must contain both translate channels at the deltas.
  @eval (ST.set(el, 'tf', el.style.transform))

  @then $tf on .drag-tf should equal "translateX(50px) translateY(70px)"
}
