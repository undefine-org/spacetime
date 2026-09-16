/// `@drop-zone` + the `__stDnd` zone registry (PLAN-058 W1/W2).
///
/// The zone registry + overlay-resilient hit-test ship INSIDE the dnd module via
/// a `%emit prelude-js` block (no edits to public/runtime/st.js). A `@drop-zone`
/// registers an entry on mount and unregisters on cleanup; the drag gesture
/// hit-tests the pointer against `__stDnd.zones[group]` (registry-keyed, NOT a
/// magic selector). These tests drive the registry + the zone's reactive
/// `$hovering`/`$accepting` signals deterministically on the V8/headless backend
/// (which has no real geometry; `elementsFromPoint` is mocked, mirroring
/// tests/unit/macros/drop-target.test.st).
///
/// Each test attaches `@drop-zone` via `@given` then flushes the harness's
/// selector-init with `@when .dz-zone mouseenter` (the same flush idiom
/// drag-animates.test.st uses) so the primitive's `%emit js` runs on the mounted
/// node before we probe the registry.

@import "stdlib/testing/test"
@import "stdlib/dnd"

@test "dnd prelude installs the __stDnd registry and a zone registers on mount" {
  @fixture {
    <div class="dz-zone" data-col="done"></div>
  }
  @given .dz-zone {
    @drop-zone(group: "cards") as $zone {
      accepts: $payload.tag != "locked";
      over   { background: rgba(0,255,0,0.1) }
      reject { outline: 1px solid red }
    }
  }
  @when .dz-zone mouseenter

  @let (zone = document.querySelector('.dz-zone'))
  @eval (window.__d = window.__stDnd || false)
  @eval (window.__api = (window.__d ? (window.__d.register ? (window.__d.hitTest ? true : false) : false) : false))
  @eval (ST.set(zone, 'hasApi', window.__api))
  @eval (ST.set(zone, 'count', (window.__d ? (window.__d.zones['cards'] || []).length : 0)))
  @then $hasApi on .dz-zone should equal true
  @then $count on .dz-zone should equal 1
}

@test "hitTest resolves the registered zone under the pointer" {
  @fixture {
    <div class="dz-zone" data-col="done"></div>
  }
  @given .dz-zone {
    @drop-zone(group: "cards") as $zone {
      over { background: red }
    }
  }
  @when .dz-zone mouseenter

  @let (zone = document.querySelector('.dz-zone'))
  @eval (window.__z = zone)
  @eval (document.elementsFromPoint = function(x, y){ return new Array(window.__z); })
  @eval (window.__hit = window.__stDnd.hitTest(100, 100, 'cards', null))
  @eval (ST.set(zone, 'hitCol', window.__hit ? window.__hit.dataset.col : 'none'))
  @then $hitCol on .dz-zone should equal "done"
}

@test "hitTest sees through an overlay covering the zone" {
  @fixture {
    <div class="dz-zone" data-col="done"></div>
  }
  @given .dz-zone {
    @drop-zone(group: "cards") as $zone {
      over { background: red }
    }
  }
  @when .dz-zone mouseenter

  @let (zone = document.querySelector('.dz-zone'))
  // A fixed overlay sits ON TOP of the zone; elementsFromPoint returns it first.
  // The registry walk must skip it (not a registered zone) and find the zone.
  @eval (window.__ovl = document.createElement('div'))
  @eval (window.__stack = new Array(window.__ovl, zone))
  @eval (document.elementsFromPoint = function(x, y){ return window.__stack; })
  @eval (window.__hit = window.__stDnd.hitTest(100, 100, 'cards', null))
  @eval (ST.set(zone, 'hitCol', window.__hit ? window.__hit.dataset.col : 'none'))
  @then $hitCol on .dz-zone should equal "done"
}

@test "hitTest returns null when the group has no zones" {
  @fixture {
    <div class="dz-zone" data-col="done"></div>
  }
  @given .dz-zone {
    @drop-zone(group: "cards") as $zone {
      over { background: red }
    }
  }
  @when .dz-zone mouseenter

  @let (zone = document.querySelector('.dz-zone'))
  @eval (document.elementsFromPoint = function(x, y){ return new Array(zone); })
  @eval (ST.set(zone, 'isNull', window.__stDnd.hitTest(100, 100, 'no-such-group', null) === null))
  @then $isNull on .dz-zone should equal true
}

@test "the accepts predicate gates by the dragged payload" {
  @fixture {
    <div class="dz-zone" data-col="done"></div>
  }
  @given .dz-zone {
    @drop-zone(group: "cards") as $zone {
      accepts: $payload.tag != "locked";
      over { background: red }
    }
  }
  @when .dz-zone mouseenter

  @let (zone = document.querySelector('.dz-zone'))
  @eval (window.__entry = window.__stDnd.zones['cards'].filter(function(e){ return e.el === zone; })[0])
  @eval (ST.set(zone, 'acceptsOpen',   window.__entry.accepts({ tag: 'open' })))
  @eval (ST.set(zone, 'acceptsLocked', window.__entry.accepts({ tag: 'locked' })))
  @then $acceptsOpen on .dz-zone should equal true
  @then $acceptsLocked on .dz-zone should equal false
}

@test "over/reject states are driven by hovering and accepting" {
  @fixture {
    <div class="dz-zone" data-col="done"></div>
  }
  @given .dz-zone {
    @drop-zone(group: "cards") as $zone {
      over   { background: red }
      reject { outline: 1px solid red }
    }
  }
  @when .dz-zone mouseenter

  @let (zone = document.querySelector('.dz-zone'))
  // Find the entry for THIS element (the registry persists across tests in a
  // file, so index 0 may be a prior test's zone). Match by element identity.
  @eval (window.__entry = window.__stDnd.zones['cards'].filter(function(e){ return e.el === zone; })[0])
  // Simulate a compatible draggable hovering and accepted.
  @eval (window.__entry.setHovering(true))
  @eval (window.__entry.setAccepting(true))
  @eval (ST.set(zone, 'hov', ST.get(zone, 'hovering')))
  @eval (ST.set(zone, 'acc', ST.get(zone, 'accepting')))
  @then $hov on .dz-zone should equal true
  @then $acc on .dz-zone should equal true
}

@test "a zone unregisters from the registry on cleanup" {
  @fixture {
    <div class="dz-zone" data-col="done"></div>
  }
  @given .dz-zone {
    @drop-zone(group: "cards") as $zone {
      over { background: red }
    }
  }
  @when .dz-zone mouseenter

  @let (zone = document.querySelector('.dz-zone'))
  @eval (window.__before = (window.__stDnd.zones['cards'] || []).length)
  @eval (window.__stDnd.unregister(window.__stDnd.zones['cards'][0]))
  @eval (window.__after = (window.__stDnd.zones['cards'] || []).length)
  @eval (ST.set(zone, 'removed', window.__after < window.__before))
  @then $removed on .dz-zone should equal true
}
