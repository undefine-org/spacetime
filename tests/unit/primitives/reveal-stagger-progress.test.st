/// BUG-354: `@reveal` was dead on every page. `stagger` is a `stagger_value`
/// — emitted as an OBJECT `{ delay, from, grid }` (the apply-animations
/// contract) — and reveal.st multiplied it by a number: `i * {…}` -> NaN ->
/// WAAPI throws on the non-finite delay, init unwinds with `revealed` already
/// true, and no retry ever fires. Every reveal on the page stayed hidden
/// forever while the build reported success.
///
/// A source assertion cannot see this: the split DOM and the emitted string
/// were always correct. What was wrong happened at runtime, in time. So this
/// gate needs timing, real WAAPI, and a real finish callback — under
/// `--headless` it SKIPS, by design.
///
/// RED proof: with the old `const staggerMs = %stagger;` both tests fail
/// (progress stays "0", first char stays at opacity 0). Seen 2026-08-08.
@import "stdlib/testing/test"
@import "stdlib/testing/timing"

.bug354-load {
  font-size: 48px;

  @reveal(split: "chars", trigger: load, stagger: 40, duration: 500)
}

.bug354-visible {
  font-size: 48px;

  @reveal(split: "chars", trigger: visible, stagger: 40, duration: 500)
}

@test "reveal with load trigger REACHES progress 1" needs timing {
  @fixture {
    <h1 class="bug354-load">AURA</h1>
  }
  @wait 1600ms
  @eval (window.__b354p = document.querySelector('.bug354-load').style.getPropertyValue('--st-revealProgress'))
  @eval (window.__b354o = getComputedStyle(document.querySelector('.bug354-load .st-reveal-char')).opacity)
  @assert (window.__b354p === '1')
    "load-trigger reveal must COMPLETE: --st-revealProgress reaches 1 (NaN delay froze it at 0)"
  @assert (window.__b354o === '1')
    "the first char must END at opacity 1 (it stayed 0 while the build said success)"
}

@test "reveal with visible trigger REACHES progress 1" needs timing {
  @fixture {
    <h1 class="bug354-visible">FIELD</h1>
  }
  @wait 1600ms
  @eval (window.__b354q = document.querySelector('.bug354-visible').style.getPropertyValue('--st-revealProgress'))
  @assert (window.__b354q === '1')
    "visible-trigger reveal must COMPLETE for an on-screen element (the IntersectionObserver fired but the NaN delay killed the loop)"
}
