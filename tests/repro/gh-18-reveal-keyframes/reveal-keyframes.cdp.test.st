/// gh-18 - CUSTOM KEYFRAMES REACH THE REVEAL ENGINE (REQUIRES `--cdp`, layout rung).
///
/// The issue: `@reveal(...) { & { filter: blur(18px) -> blur(0); } }` - the
/// documented custom-keyframes body - is parsed but SILENTLY DROPPED: the
/// macro's `%binds` never forwards `$body:keyframes` to the reveal-engine. This
/// is the TRUE behavioral gate: in a real browser, after the reveal runs, the
/// reveal animation's keyframes must include the custom `filter` property. Today
/// the body is dropped so `filter` is absent from every animation's keyframes -
/// RED. The moment the macro forwards `$body:keyframes` and the engine merges it
/// into the per-unit `unit.animate(keyframes, …)`, this goes GREEN.
///
/// Run:  spacetime test tests/repro/gh-18-reveal-keyframes/reveal-keyframes.cdp.test.st --cdp
///
/// STATUS: unrunnable in this environment (no Chromium installed). Compiles
/// clean; `needs layout` makes it SKIP on the headless logic backend (a
/// fidelity deferral, not a pass/fail). The headless counterpart
/// (reveal-keyframes.test.st) documents the rung gap.
@import "stdlib/testing/test"

.probe18c {
  @reveal(split: "words", trigger: visible, stagger: 90, duration: 800, easing: ease-out) {
    & { filter: blur(18px) -> blur(0); opacity: 0.25 -> 1; }
  }
}

@test "cdp gh-18 custom filter keyframe reaches the reveal animation" needs layout {
  @fixture {
    <h1 class="probe18c">RAMI OPTIC</h1>
  }
  // Let the visible-trigger IntersectionObserver fire and reveal() run.
  @wait 300ms
  @let (units = document.querySelectorAll('.probe18c .st-reveal-word'))
  @assert (units.length === 2)
    "reveal splits 'RAMI OPTIC' into 2 word units"
  // Collect every keyframe property name across every unit's WAAPI animations.
  @eval (window.__keyframeProps = Array.from(new Set(
    Array.from(document.querySelectorAll('.probe18c .st-reveal-word'))
      .map(u => u.getAnimations())
      .reduce((acc, anims) => acc.concat(anims), [])
      .map(a => a.effect.getKeyframes())
      .reduce((acc, kfs) => acc.concat(kfs), [])
      .reduce((acc, kf) => acc.concat(Object.keys(kf)), [])
  )))
  @assert (window.__keyframeProps.indexOf('filter') !== -1)
    "the reveal animation must carry the custom filter: blur keyframe - currently the keyframes body is dropped, so 'filter' is absent (RED)"
  @assert (window.__keyframeProps.indexOf('opacity') !== -1)
    "built-in opacity keyframe still present alongside the custom body"
}
