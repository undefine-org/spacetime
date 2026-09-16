/// gh-6 (follow-up) — ONE-SHOT @on visible MUST NOT REVERT ON VIEWPORT RE-ENTRY.
///
/// gh-6's core claim (end keyframe retained past duration) is FIXED at the logic
/// rung — see `hold-end-state.test.st` / `hold-end-state-transform.test.st`
/// (green). This file covers the NOT-verified caveat: a DRIVER that re-fires
/// from p=0 when the element leaves and re-enters the viewport.
///
/// Reasoning (BUG-315): `@on &.visible(300ms)` compiles to the `intersection`
/// driver with `once: false`, publishing `entry.intersectionRatio -> reveal` on
/// EVERY intersection change. apply-animations' scalar path interpolates at the
/// published progress and applies the START keyframe at progress=0 — so scrolling
/// the element out (ratio→0) REVERTS it to opacity:0, and re-entry re-reveals.
/// A one-shot reveal should hold its end state across a re-entry, not blink.
///
/// THIS IS A REAL-BROWSER GATE: re-entry needs real geometry + a real
/// IntersectionObserver firing a non-intersecting entry, which the logic backend
/// cannot provide (headless has no scroll/viewport model — BUG-305). Run:
///   spacetime test tests/repro/gh-6-hold-end-state/re-entry.cdp.test.st --cdp
///
/// STATUS: RED today (revert on leave); GREEN when one-shot visible drivers stop
/// re-firing from p=0. UNRUNNABLE here (no Chromium) — `needs layout` makes it
/// SKIP on the logic backend, a fidelity deferral, not a pass/fail.
@import "stdlib/testing/test"

.curtainRe {
  position: relative;
  @on &.visible(300ms) as $reval {
    opacity: 0 -> 1;
  }
}

@test "gh-6 one-shot @on visible does not revert when the element re-enters the viewport" needs layout {
  @fixture {
    <div class="curtainRe" style="opacity: 0"></div>
  }
  // Make the element tall enough to leave the viewport on a real scroll.
  @eval (document.querySelector('.curtainRe').style.height = '120vh')

  // Initially in view → the one-shot reveal runs and holds its end state.
  @wait 600ms
  @eval (window.__re0 = document.querySelector('.curtainRe').style.opacity)
  @assert (window.__re0 === '1')
    "the one-shot visible reveal must reach and hold opacity:1 while in view"

  // Leave the viewport: scroll the element out (real layout → the
  // IntersectionObserver fires a NON-intersecting entry, ratio→0).
  @eval (window.scrollTo(0, document.body.scrollHeight))
  @wait 300ms
  @eval (window.__reLeft = document.querySelector('.curtainRe').style.opacity)

  // Re-enter the viewport.
  @eval (window.scrollTo(0, 0))
  @wait 600ms
  @eval (window.__re1 = document.querySelector('.curtainRe').style.opacity)

  @assert (window.__re1 === '1')
    "re-entry must NOT revert the one-shot reveal to opacity:0 — a p=0 re-fire drops the end state (RED, BUG-315)"
}
