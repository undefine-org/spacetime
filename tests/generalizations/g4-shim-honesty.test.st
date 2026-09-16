/// G4 — A HEADLESS SHIM MUST NOT ANSWER A QUESTION IT CANNOT ANSWER.
///
/// GH-17 established the rule for ONE api: a backend that cannot provide a rung
/// REFUSES by name (`getComputedStyle` on a stylesheet-derived read now says
/// "needs the layout rung — run with --cdp"). The rule was never applied to the
/// rest of the shim set, and that gap has already cost a shipped bug.
///
/// BUG-323 (`@on &.visible` frozen mid-animation) was invisible to the entire
/// headless suite BY CONSTRUCTION: `src/headless_shims.js:142` reports
/// `intersectionRatio: visible ? 1 : 0` — a BINARY answer to a CONTINUOUS
/// question. Any timeline driven by a fractional ratio is therefore untestable
/// headless, and reads as passing.
///
/// This file pins what the logic rung HONESTLY provides, so the boundary is
/// written down and a future shim cannot quietly widen it:
///   - a binary visible/not-visible decision IS available (and is genuinely
///     useful — it is what `@on &.visible` needs to fire at all);
///   - a FRACTIONAL ratio is NOT, and must not be invented.
///
/// The paired behavioral proof lives in `g4-shim-honesty.cdp.test.st`, where a
/// real browser reports real intermediate ratios. Read them together: this one
/// says what headless may claim, that one says what only a browser can.
@import "stdlib/testing/test"

@test "the logic rung answers the BINARY visibility question" {
  @fixture {
    <div class="g4a">visible</div>
  }
  @eval (window.__g4a = null)
  @eval (new IntersectionObserver((entries) => { window.__g4a = entries[0].isIntersecting; }).observe(document.querySelector('.g4a')))
  @wait 50ms
  @assert (window.__g4a === true)
    "an element with no off-screen declaration must report intersecting — this is the decision @on &.visible needs to fire"
}

@test "an element declared below the fold honestly reports NOT intersecting" {
  // The shim reads the authored inline style rather than inventing geometry:
  // a declared `top: 200vh` is a statement about intent that the logic rung CAN
  // honor without a layout engine. That is the difference between a heuristic
  // grounded in the source and a fabricated pixel measurement.
  @fixture {
    <div class="g4b" style="top: 200vh">below</div>
  }
  @eval (window.__g4b = null)
  @eval (new IntersectionObserver((entries) => { window.__g4b = entries[0].isIntersecting; }).observe(document.querySelector('.g4b')))
  @wait 50ms
  @assert (window.__g4b === false)
    "a declared below-the-fold position must report not-intersecting"
}

@test "the ratio is binary at the logic rung — it must never look continuous" {
  // THE BUG-323 GATE. The shim reports 1 or 0 and nothing between. That is the
  // honest answer for a viewport-less world, but it means a fractional-ratio
  // feature cannot be proven here. Pinning it makes the limit explicit: if
  // someone later "improves" the shim to emit 0.5, this test fails and forces
  // the question — is that a real measurement, or a fabricated one?
  //
  // A shim that invents a plausible intermediate value would make every
  // scroll-linked and ratio-driven timeline appear tested while never having
  // run against a real one. That is exactly how BUG-323 reached a demo page.
  @fixture {
    <div class="g4c">x</div>
  }
  @eval (window.__g4c = null)
  @eval (new IntersectionObserver((entries) => { window.__g4c = entries[0].intersectionRatio; }).observe(document.querySelector('.g4c')))
  @wait 50ms
  @assert (window.__g4c === 1 || window.__g4c === 0)
    "the logic-rung ratio must stay binary; a fractional value here would be fabricated, and a fabricated ratio is what hid BUG-323"
}

@test "matchMedia refuses to fake geometry" {
  // A `(min-width: N)` query is a question about a viewport that does not exist
  // at this rung. The shim answers `false` rather than guessing a plausible
  // width — the same discipline as the ratio above.
  @fixture {
    <div class="g4d">x</div>
  }
  @assert (window.matchMedia('(min-width: 800px)').matches === false)
    "a width query must not be answered with an invented viewport"
}
