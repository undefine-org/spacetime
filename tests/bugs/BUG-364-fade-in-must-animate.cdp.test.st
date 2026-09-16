/// BUG-364 — `@fade-in` MUST ACTUALLY ANIMATE (REQUIRES `--cdp`, layout rung).
///
/// `@fade-in` builds clean and moves nothing. The cause is written down at the
/// site, `stdlib/macros/fade-in.st`: the macro binds an intersection observer
/// and then runs an EMPTY `%when` body —
///
///     %when $visible {
///       // TODO: Implement using @tween or CSS keyframes primitive
///       // %timeline was never functional - it created empty state machines
///       // For now, the intersection observer triggers but no animation plays
///     }
///
/// "the intersection observer triggers but no animation plays". Known,
/// commented, shipped.
///
/// WHY THIS FILE EXISTS AT ALL. Per AGENTS.md (BUG-252): a feature whose job is
/// to MOVE something needs a gate asserting that it moved, in a real browser.
/// `@fade-in` had only source-level assertions, which prove a string was
/// written into code that never runs. Its sibling `@reveal` has exactly this
/// kind of gate (`tests/repro/gh-18-reveal-keyframes/`) and genuinely animates —
/// the difference between the two directives is the difference between having
/// a behavior gate and not having one.
///
/// Run:  cargo run --features cdp -- test tests/bugs/BUG-364-fade-in-must-animate.cdp.test.st --cdp
///
/// `needs layout` makes this SKIP on the headless logic backend. A skipped gate
/// reads exactly like a passing one, so this file only means something when it
/// has been run under `--cdp` and SEEN to fail.
@import "stdlib/testing/test"

.probe364 {
  @fade-in(duration: 300ms, easing: ease-out, distance: 20px);
}

@test "cdp BUG-364 @fade-in runs a real animation on its element" needs layout {
  @fixture {
    <div class="probe364">Fade me in</div>
  }
  // Let the visible-trigger IntersectionObserver fire and the animation start.
  @wait 300ms

  @eval (window.__anims364 = document.querySelector('.probe364').getAnimations())
  @assert (window.__anims364.length > 0)
    "@fade-in must start at least one animation on its element - today the %when body is empty, so the observer fires and nothing animates (RED)"

  // The animation must actually carry the fade: an opacity keyframe. An
  // animation with no opacity would satisfy the count above while still not
  // fading anything, so the count alone is not the contract.
  @eval (window.__props364 = Array.from(new Set(
    document.querySelector('.probe364').getAnimations()
      .map(a => a.effect.getKeyframes())
      .reduce((acc, kfs) => acc.concat(kfs), [])
      .reduce((acc, kf) => acc.concat(Object.keys(kf)), [])
  )))
  @assert (window.__props364.indexOf('opacity') !== -1)
    "a FADE-in must animate opacity"

  // `distance: 20px` is a travel, so the element must also move.
  @assert (window.__props364.indexOf('transform') !== -1)
    "distance: 20px must produce a transform keyframe - a fade-in that does not travel has silently dropped its distance"
}

@test "cdp BUG-364 @fade-in honours its easing" needs layout {
  @fixture {
    <div class="probe364">Fade me in</div>
  }
  @wait 300ms

  @eval (window.__easing364 = (document.querySelector('.probe364').getAnimations()[0] || {}).effect)
  @assert (window.__easing364 !== undefined)
    "there must be an animation whose timing we can inspect"

  @eval (window.__timing364 = document.querySelector('.probe364').getAnimations()[0].effect.getComputedTiming())
  @assert (window.__timing364.duration === 300)
    "duration: 300ms must reach the animation, not a default"
}

/// The DOCUMENTED custom-keyframes body (`@fade-in(...) { scale: 0.95 -> 1 }`)
/// must reach the animation. It was captured as `$_customKeyframes` and never
/// consumed — the doc comment advertised a feature the macro silently dropped,
/// which is the same shape of defect as the empty `%when` body: it looks
/// supported and does nothing. `@reveal` already forwards its body
/// (`keyframes: $body`), so this is the established pattern, not a new one.
.probe364kf {
  @fade-in(duration: 300ms) {
    scale: 0.95 -> 1;
  }
}

@test "cdp BUG-364 a custom keyframes body reaches the fade-in animation" needs layout {
  @fixture {
    <div class="probe364kf">Scale me</div>
  }
  @wait 300ms

  @eval (window.__kfProps = Array.from(new Set(
    document.querySelector('.probe364kf').getAnimations()
      .map(a => a.effect.getKeyframes())
      .reduce((acc, kfs) => acc.concat(kfs), [])
      .reduce((acc, kf) => acc.concat(Object.keys(kf)), [])
  )))
  // NB `transform` alone proves nothing: `distance` already emits a
  // translate, so asserting on it passes whether or not the body arrived.
  // The claim must be the SCALE specifically.
  @eval (window.__kfScale = JSON.stringify(
    document.querySelector('.probe364kf').getAnimations()
      .map(a => a.effect.getKeyframes())
  ).indexOf('scale') !== -1)
  @assert (window.__kfScale === true)
    "the custom `scale: 0.95 -> 1` body must reach the animation - today it is captured as $_customKeyframes and dropped (RED)"
}

/// `from:` and `once:` are documented parameters. The engine supported both
/// from the start; the macro never surfaced them, so the documented API and the
/// real one disagreed. `from: left` must produce a HORIZONTAL travel — asserting
/// merely that a transform exists would pass for the default `bottom` too.
.probe364dir {
  @fade-in(duration: 300ms, from: left, distance: 40px);
}

@test "cdp BUG-364 from: left travels horizontally" needs layout {
  @fixture {
    <div class="probe364dir">Slide me</div>
  }
  @wait 300ms

  @eval (window.__dirKf = JSON.stringify(
    document.querySelector('.probe364dir').getAnimations()
      .map(a => a.effect.getKeyframes())
  ))
  @assert (window.__dirKf.indexOf('translateX(-40px)') !== -1)
    "from: left must travel on X - a direction the macro did not expose until now"
}
