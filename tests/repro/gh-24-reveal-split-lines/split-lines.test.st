/// gh-24: @reveal split:"lines" re-splits generated DOM. Root cause: splitText()
/// re-snapshots `Array.from(el.childNodes)` on every call; the ResizeObserver
/// re-calls splitText() while unrevealed. Headless has no ResizeObserver shim
/// (src/headless_shims.js), so the re-split path cannot fire under --headless.
/// This pin records what headless CAN observe: RO availability + whether the
/// initial lines split happens. The real behavioral gate (resize → count stable,
/// textContent not multiplied) lives in `split-lines.cdp.test.st` (needs layout).
@import "stdlib/testing/test"

.audit24 {
  width: 320px;
  @reveal(split: "lines", trigger: visible, stagger: 0, duration: 1)
}

@test "lines-mode reveal: RO availability + initial split under headless" {
  @fixture {
    <h1 class="audit24">Alpha beta gamma delta</h1>
  }
  @wait 150ms
  @eval (window.__ro = typeof ResizeObserver !== 'undefined' ? 'defined' : 'undefined')
  @eval (window.__units24 = document.querySelectorAll('.audit24 .st-reveal-line').length)
  @eval (window.__masks24 = document.querySelectorAll('.audit24 .st-reveal-mask').length)
  @assert (window.__ro === 'undefined')
    "headless rung has no ResizeObserver — the gh-24 re-split path (RO-driven) is NOT exercisable under --headless"
  @assert (window.__units24 >= 1)
    "lines-mode reveal must split into line units at init"
}

@test "lines-mode split is idempotent (a 2nd split does not multiply content)" {
  @fixture {
    <h1 class="audit24">Alpha beta gamma delta</h1>
  }
  @wait 150ms
  // Baseline after the initial split.
  @eval (window.__baseUnits = document.querySelectorAll('.audit24 .st-reveal-line').length)
  @eval (window.__baseText = Array.from(document.querySelectorAll('.audit24 .st-reveal-line')).map(l => l.textContent).join(''))
  // Drive a SECOND split directly through the primitive's re-split entry. There
  // is no ResizeObserver at the logic rung, so this is the only way the RO
  // re-split path (gh-24's trigger) can fire headless. Before the fix, splitText
  // re-snapshotted el.childNodes and nested the generated tree — a 2nd split
  // MULTIPLIED the units and text.
  @eval (window.__el24 = document.querySelector('.audit24'))
  @eval (window.__el24._revealSplit())
  @eval (window.__afterUnits = document.querySelectorAll('.audit24 .st-reveal-line').length)
  @eval (window.__afterText = Array.from(document.querySelectorAll('.audit24 .st-reveal-line')).map(l => l.textContent).join(''))
  @assert (window.__afterUnits === window.__baseUnits)
    "a 2nd split must not add reveal units (old code nested the generated tree, RED)"
  @assert (window.__afterText === window.__baseText)
    "a 2nd split must not multiply visible line text (old code duplicated it, RED)"
  @assert (window.__baseText === 'Alpha beta gamma delta')
    "the visible line units must carry the authored source text exactly once"
}

