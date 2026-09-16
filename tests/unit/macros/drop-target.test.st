/// ST.dropTarget — resolve a drag's drop zone from the release point (PLAN-055).
///
/// `@drag` moves a card with a CSS transform (no DOM reparent), so at drop time
/// `card.closest('.col')` returns the SOURCE column, not where it was released.
/// `ST.dropTarget(el, sel)` hit-tests the release point (the gesture's
/// `$dropX`/`$dropY`) and returns the matching drop zone.
///
/// The V8/LinkeDOM backend has no `document.elementFromPoint`, so these tests
/// MOCK it to drive `ST.dropTarget`'s resolution logic deterministically. Real
/// geometric hit-testing is covered by the live demo (and a CDP test once the
/// CDP harness is available — see PLAN-056).

@import "stdlib/testing/test"
@import "stdlib/dnd"

@test "ST.dropTarget returns the zone under the release point" {
  @fixture {
    <div class="col" data-col="done">
      <div class="dt-card" data-id="c1"></div>
    </div>
  }

  @let (el = document.querySelector('.dt-card'))
  @let (zone = document.querySelector('.col'))

  // Mock the hit-test to return the done column's child cell. dropTarget
  // prefers elementsFromPoint (stack walk); mock both for backend parity.
  @eval (document.elementFromPoint = function(x, y){ return zone; })
  @eval (window.__z = zone)
  @eval (document.elementsFromPoint = function(x, y){ return new Array(window.__z); })
  @eval (ST.set(el, 'dropX', 250))
  @eval (ST.set(el, 'dropY', 50))
  @eval (ST.set(el, 'col', (ST.dropTarget(el, '.col') || {dataset:{}}).dataset.col || 'none'))

  @then $col on .dt-card should equal "done"
}

@test "ST.dropTarget sees through an overlay covering the drop zone" {
  @fixture {
    <div class="col" data-col="done">
      <div class="dt-ovl-card" data-id="c1"></div>
    </div>
  }

  @let (el = document.querySelector('.dt-ovl-card'))
  @let (zone = document.querySelector('.col'))

  // A fixed overlay (the dev panel) sits ON TOP of the drop zone at the release
  // point. elementsFromPoint returns it FIRST, the real zone behind it. The
  // dropTarget walk must skip the overlay (matches no .col) and return the zone.
  @eval (window.__ovl = document.createElement('div'))
  @eval (window.__stack = new Array(window.__ovl, zone))
  @eval (document.elementsFromPoint = function(x, y){ return window.__stack; })
  @eval (ST.set(el, 'dropX', 250))
  @eval (ST.set(el, 'dropY', 50))
  @eval (window.__hit = ST.dropTarget(el, '.col'))
  @eval (ST.set(el, 'col', window.__hit ? window.__hit.dataset.col : 'none'))

  @then $col on .dt-ovl-card should equal "done"
}

@test "ST.dropTarget returns null when no coords are set" {
  @fixture {
    <div class="dt-card2" data-id="c2"></div>
  }

  @let (el = document.querySelector('.dt-card2'))
  @eval (document.elementFromPoint = function(){ return document.body; })
  @eval (document.elementsFromPoint = function(){ return new Array(document.body); })
  // No dropX/dropY set -> null (no release has happened).
  @eval (ST.set(el, 'isNull', ST.dropTarget(el, '.col') === null))

  @then $isNull on .dt-card2 should equal true
}

@test "ST.dropTarget returns null when release is over no matching zone" {
  @fixture {
    <div class="dt-card3" data-id="c3"></div>
  }

  @let (el = document.querySelector('.dt-card3'))
  // hit-test returns body (no .col ancestor) -> closest('.col') is null.
  @eval (document.elementFromPoint = function(){ return document.body; })
  @eval (document.elementsFromPoint = function(){ return new Array(document.body); })
  @eval (ST.set(el, 'dropX', 5))
  @eval (ST.set(el, 'dropY', 5))
  @eval (ST.set(el, 'isNull', ST.dropTarget(el, '.col') === null))

  @then $isNull on .dt-card3 should equal true
}

@test "ST.dropTarget hides the dragged element during hit-test" {
  @fixture {
    <div class="dt-card4" data-id="c4"></div>
  }

  @let (el = document.querySelector('.dt-card4'))
  // The hit-test must make the dragged element pointer-transparent so the
  // element BELOW is found, then restore pointerEvents afterward. Mock BOTH
  // hit-test APIs: dropTarget prefers elementsFromPoint (stack walk) and falls
  // back to elementFromPoint, so whichever the backend exposes records the PE.
  @eval (window.__peDuring = "unset")
  @eval (document.elementFromPoint = function(){ window.__peDuring = el.style.pointerEvents; return null; })
  @eval (document.elementsFromPoint = function(){ window.__peDuring = el.style.pointerEvents; return new Array(); })
  @eval (el.style.pointerEvents = "auto")
  @eval (ST.set(el, 'dropX', 10))
  @eval (ST.set(el, 'dropY', 10))
  @eval (ST.dropTarget(el, '.col'))
  @eval (ST.set(el, 'duringPE', window.__peDuring))
  @eval (ST.set(el, 'afterPE', el.style.pointerEvents))

  @then $duringPE on .dt-card4 should equal "none"
  @then $afterPE on .dt-card4 should equal "auto"
}
