/// gh-18 — custom keyframes body on @reveal. Logic-rung gate.
///
/// The issue: `@reveal(...) { & { filter: blur(18px) -> blur(0); ... } }` — the
/// documented custom-keyframes body — is parsed but SILENTLY DROPPED: the
/// macro's `%binds` never forwards `$body:keyframes` to the reveal-engine, so
/// 'blur' appears 0x in emitted JS (see reveal-keyframes.st + evidence.txt).
///
/// What the headless logic rung CAN observe:
///   - the reveal STILL splits text into word units (reveal-engine runs);
///   - the custom keyframe property is absent from the units' applied inline
///     styles (`unit.style.filter === ''`), because the body was dropped.
///
/// What it CANNOT observe: whether the custom keyframes reached the WAAPI
/// `unit.animate(...)` call. The headless WAAPI shim registers animations on
/// `el.__animations` but exposes NO `effect.getKeyframes()`, so the animation's
/// keyframe set is not introspectable here. That claim — the true behavioral
/// gate — lives in `reveal-keyframes.cdp.test.st` (needs layout, --cdp), which
/// asserts a `filter` keyframe appears in the reveal animation's keyframes.
/// This file documents the rung gap explicitly so the deferral is auditable.
@import "stdlib/testing/test"

.probe18 {
  @reveal(split: "words", trigger: visible, stagger: 90, duration: 800, easing: ease-out) {
    & { filter: blur(18px) -> blur(0); }
  }
}

@test "logic rung: custom keyframes reach the reveal engine" {
  @fixture {
    <h1 class="probe18">RAMI OPTIC</h1>
  }
  @wait 150ms
  // The reveal-engine runs headless (visible-trigger IntersectionObserver shim
  // reports intersecting, and prefers-reduced-motion is false) — it splits into
  // word units and starts a WAAPI animation on each.
  @let (units = document.querySelectorAll('.probe18 .st-reveal-word'))
  @eval (window.__units18 = units.length)
  @assert (window.__units18 === 2)
    "reveal still splits 'RAMI OPTIC' into 2 word units — the reveal-engine runs"
  @eval (window.__animCount = units[0].getAnimations().length)
  @assert (window.__animCount >= 1)
    "each split unit started a WAAPI reveal animation"
  // gh-18 FIXED: `$body:keyframes` is now forwarded through %binds and compiled
  // into the engine's `keyframes` param. The engine exposes the resolved custom
  // keyframes on the element (`el._revealKeyframes`) so a logic-rung gate can
  // assert the author's filter keyframe actually REACHED the engine — the headless
  // proof that the property reaches the emitted engine call. (The WAAPI shim
  // exposes no effect.getKeyframes(), so the on-WAAPI keyframe claim stays in the
  // --cdp gate.)
  @eval (window.__kf = document.querySelector('.probe18')._revealKeyframes)
  @assert (window.__kf && window.__kf.length >= 2)
    "the reveal engine received a custom keyframes array with an end frame"
  @eval (window.__kfFirstProps = window.__kf ? Object.keys(window.__kf[0]) : [])
  @assert (window.__kfFirstProps.indexOf('filter') !== -1)
    "the author's custom filter: blur keyframe property reached the reveal engine"
  @eval (window.__kfFirstFilter = window.__kf ? (window.__kf[0].filter || '') : '')
  @assert (window.__kfFirstFilter === 'blur(18px)')
    "the custom filter keyframe carries the author's starting value blur(18px)"
}
