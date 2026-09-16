/// G4 (browser half) — WHAT ONLY A REAL BROWSER CAN ANSWER.
///
/// The sibling `g4-shim-honesty.test.st` pins the logic rung's honest limit: its
/// `intersectionRatio` is BINARY, because a viewport-less world has no fraction
/// to report. That limit is only defensible if the fraction is genuinely
/// available one rung up — otherwise "refuses to fake it" would just be a
/// permanent hole wearing a principle's clothes.
///
/// So this file proves the other half in real Chromium: a partially-scrolled
/// element reports a ratio strictly between 0 and 1. Together the two files say
/// the whole thing — headless decides IF an element is visible, the browser
/// measures HOW MUCH, and no layer pretends to the other's job.
///
/// This is the pair BUG-323 needed and did not have.
@import "stdlib/testing/test"

@test "a real browser reports a FRACTIONAL intersection ratio" needs layout {
  @fixture {
    <div style="height: 300vh">
      <div class="g4x" style="height: 50vh; margin-top: 90vh">partial</div>
    </div>
  }
  @eval (window.__g4x = [])
  @eval (new IntersectionObserver((es) => { window.__g4x.push(es[0].intersectionRatio); },
         { threshold: [0, 0.25, 0.5, 0.75, 1] }).observe(document.querySelector('.g4x')))
  @eval (window.scrollTo(0, window.innerHeight * 0.5))
  @wait 300ms
  @assert (window.__g4x.some(r => r > 0 && r < 1))
    "a partially-visible element must report a ratio strictly between 0 and 1 — the measurement the logic rung correctly refuses to invent"
}
