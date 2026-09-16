/// gh-24 — RE-SPLIT IDEMPOTENCE UNDER ResizeObserver (REQUIRES `--cdp`, layout rung).
///
/// The issue: `@reveal(split: "lines")` re-splits its OWN generated DOM. In
/// `stdlib/primitives/reveal.st` `splitText()` re-snapshots `Array.from(el.childNodes)`
/// on every call (line 53-54) and the ResizeObserver re-calls `splitText()` while
/// unrevealed (line 444-452), so after the first split the snapshot includes the
/// sr-only copy + generated line masks, and the next split nests the old
/// generated reveal tree inside a new one. Real-Chromium symptom (from the issue):
/// the element's total `textContent` contains the source several times, and stale
/// `.st-reveal-line` units get nested with computed opacity 0.
///
/// Headless has NO ResizeObserver (see `split-lines.test.st`, which asserts
/// `typeof ResizeObserver === 'undefined'`), so the re-split path cannot fire at
/// the logic rung. THIS file is the real gate. A correct implementation has
/// EXACTLY one accessibility copy and the visible line units carry the source
/// text exactly once (so `el.textContent` == source × 2: sr-only + visible), and
/// the unit count is stable across a resize. The bug multiplies the visible text
/// and nests stale units — RED today; GREEN once splitText is rebuilt from an
/// immutable author snapshot.
///
/// Run:  spacetime test tests/repro/gh-24-reveal-split-lines/split-lines.cdp.test.st --cdp
///
/// STATUS: unrunnable in this environment (no Chromium installed). Compiles
/// clean; `needs layout` makes it SKIP on the headless logic backend (fidelity
/// deferral, not a pass/fail).
@import "stdlib/testing/test"

.audit24c {
  width: 320px;
  @reveal(split: "lines", trigger: visible, stagger: 0, duration: 1)
}

@test "cdp gh-24 lines reveal never multiplies its own generated DOM" needs layout {
  @fixture {
    <h1 class="audit24c">Alpha beta gamma delta</h1>
  }
  // Let the initial split + the pre-reveal ResizeObserver callback + the
  // visible-trigger reveal all settle. This is where the bug's nesting happens
  // (RO fires before IntersectionObserver's visible callback, while unrevealed).
  @wait 400ms
  @let (host = document.querySelector('.audit24c'))

  // Exactly one accessibility copy — the re-split must not multiply the sr-only
  // span or nest it into the visible tree.
  @eval (window.__srOnly = document.querySelectorAll('.audit24c .sr-only').length)
  @assert (window.__srOnly === 1)
    "exactly one accessibility (sr-only) copy — the generated sr-only text must never be re-split (bug nests it, RED)"

  // The visible line units, taken together, carry the authored source text
  // EXACTLY once. The bug re-snapshots the generated tree, so the source text
  // appears in the visible tree multiple times (issue: 'textContent contains it
  // three times').
  @eval (window.__linesText = Array.from(document.querySelectorAll('.audit24c .st-reveal-line')).map(l => l.textContent).join(''))
  @assert (window.__linesText === 'Alpha beta gamma delta')
    "visible .st-reveal-line units must carry the source text exactly once (bug: re-split duplicates it across stale units, RED)"

  // Stable DOM for an unchanged-width resize: a real width change must not spawn
  // extra units (the fix's acceptance 'stable DOM count for unchanged width').
  @eval (window.__count0 = document.querySelectorAll('.audit24c .st-reveal-line').length)
  @eval (host.style.width = '360px')
  @wait 400ms
  @eval (window.__count1 = document.querySelectorAll('.audit24c .st-reveal-line').length)
  @assert (window.__count1 === window.__count0)
    "reveal-unit count must stay unchanged across a resize — RO must NOT re-split generated DOM (RED today)"
}
