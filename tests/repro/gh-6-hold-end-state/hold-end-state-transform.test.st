/// gh-6 (transform arm): the issue's root cause named the transform merge
/// (`el._stTransforms`) — a completed one-shot at progress=1 was dropped, so the
/// end-state transform snapped back. This pins that a translate timeline now
/// RETAINS its end transform inline after completion.
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

.curtain6t {
  @on &.visible(300ms) as $lift {
    transform: translateY(0) -> translateY(-100px);
  }
}

@test "one-shot @on visible RETAINS end transform past duration" needs timing {
  @fixture {
    <div class="curtain6t"></div>
  }
  @wait 1200ms
  @eval (window.__tr6 = document.querySelector('.curtain6t').style.transform || '')
  @assert (window.__tr6.indexOf('translateY(-100px)') !== -1)
    "one-shot visible timeline must RETAIN end transform translateY(-100px)"
}
