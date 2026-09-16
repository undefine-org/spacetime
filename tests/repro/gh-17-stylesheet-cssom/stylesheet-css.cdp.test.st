/// gh-17 — THE POSITIVE HALF (REQUIRES `--cdp`, layout rung).
///
/// The sibling `stylesheet-css.test.st` asserts the REFUSAL: headless, a
/// stylesheet-derived `getComputedStyle` read must decline by naming the rung it
/// needs (`layout`) and how to get it (`--cdp`), rather than dying with a generic
/// TypeError or — worse — returning a plausible fake.
///
/// A refusal is only half a contract. If the read merely never works, refusing is
/// trivially "correct" and proves nothing. This file asserts the other half: in a
/// real browser the same read returns the REAL value from the stylesheet.
///
/// Together they state the fidelity-ladder rule completely: the backend that can
/// answer, answers; the backend that cannot, says so by name.
///
/// Run:  spacetime test tests/repro/gh-17-stylesheet-cssom/stylesheet-css.cdp.test.st --cdp
@import "stdlib/testing/test"

.g17box {
  width: 240px;
}

@test "a stylesheet-derived getComputedStyle read returns the real value under --cdp" needs layout {
  @fixture {
    <div class="g17box">x</div>
  }
  // The width lives in the STYLESHEET, not in an inline style attribute — which
  // is exactly the read the headless stub could not serve.
  @assert (getComputedStyle(document.querySelector('.g17box')).width === '240px')
    "the stylesheet-declared width must be readable through CSSOM in a real browser"
}
