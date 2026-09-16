/// gh-6: a one-shot `@on visible` keyframe timeline does not hold its end state —
/// after completion the element reverts to its CSS resting state (no fill:forwards).
/// Under headless the IntersectionObserver shim fires once (microtask) with
/// ratio=1; apply-animations interpolates opacity 0->1 at progress=1 and applies
/// the END value inline. The claim under test: that inline end state is RETAINED
/// (not reverted to the resting opacity:0) — advanced well past the 300ms
/// duration via @clock.
///
/// NB TIMING MECHANISM (PLAN-139 W1): this file used `@clock install / advance
/// / run-to-end`. `.visible` now registers `load-driver` — a timeline GATED on
/// a real IntersectionObserver callback — and a virtual clock cannot advance a
/// timeline that a real browser has not started yet, so the clock spelling
/// asserted against an animation that had never begun. Real time, and a real
/// on-screen element, is the only honest way to ask this question; the claim
/// itself is unchanged.
@import "stdlib/testing/test"
@import "stdlib/testing/timing"

.curtain6 {
  @on &.visible(300ms) as $reveal {
    opacity: 0 -> 1;
  }
}

@test "one-shot @on visible RETAINS end keyframe past duration" needs timing {
  @fixture {
    <div class="curtain6" style="opacity: 0"></div>
  }
  @wait 1200ms
  @eval (window.__opac6 = document.querySelector('.curtain6').style.opacity)
  @assert (window.__opac6 === '1')
    "one-shot visible timeline must RETAIN end keyframe opacity:1 after completion"
}
