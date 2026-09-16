// BUG-297 (behavior gate, per AGENTS "A source assertion is not a behavior
// assertion"): a custom `@form easing` curve must reach the LIVE runtime —
// ST.getEasing must resolve the DECLARED curve, never the silent linear
// fallback. This executes the compiled page's real bundle: the compiler's
// registration IIFE must have run (compile -> emit -> execute -> registry).
//
// END-TO-END PROOF of the full chain (declaration -> registration -> engine
// -> element) was done in a real browser (2026-08-05): a scroll-scrub page
// read opacity 0.776 on the custom element vs 0.500 on the linear sibling
// at scroll midpoint — the exact curve value. The in-harness driver gaps
// that keep the LAST hop out of reach here are filed as follow-ups:
// one-shot `@on visible` timelines don't tick mid-run under `@clock`
// (@record-timeline captures 0 samples), the CDP backend never fires
// IntersectionObserver (gh-6 fails there), and headless has no scroll model.
@import "stdlib/testing/test"

@form easing --gate-curve { cubic-bezier(0.4, 0, 0.2, 1) }

.easeCustom {
  @on &.scroll(name: gate, start: 0, end: 1, scrub: true) {
    opacity: 0 -> 1;
    easing: --gate-curve;
  }
}

@test "a declared easing form resolves its curve in the live runtime" {
  @fixture { <div class="easeCustom">A</div> }

  @assert (typeof ST.getEasing === 'function')
    "the runtime easing registry must be present"
  @assert (Math.abs(ST.getEasing('gate-curve')(0.5) - 0.776) < 0.01)
    "the registered curve must BE cubic-bezier(0.4,0,0.2,1): 0.776 at t=0.5 — 0.5 means the registration never ran (BUG-297)"
  @assert (Math.abs(ST.getEasing('gate-curve')(0) - 0) < 0.001)
    "curve anchors: f(0) = 0"
  @assert (Math.abs(ST.getEasing('gate-curve')(1) - 1) < 0.001)
    "curve anchors: f(1) = 1"
  @assert (ST.getEasing('never-declared-anywhere')(0.5) === 0.5)
    "control: an undeclared name still falls back to linear (the fallback itself is not broken)"
  @assert (ST.getEasing('linear')(0.5) === 0.5)
    "control: the linear keyword is untouched"
}
