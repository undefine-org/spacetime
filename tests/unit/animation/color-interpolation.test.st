/// Unit Tests: colour interpolation is dispatched by SPACE, not by a name
///
/// BUG-202 / FUP-160. `animate-color` called `ST.lerpColor(from, to, p, space)`
/// — a function the runtime never defined. Every tick threw a TypeError and the
/// animation silently did nothing; nothing in the repo invoked the primitive, so
/// the break stayed latent until it was read.
///
/// The rule these tests pin down: there is ONE way two colours blend, and it is
/// chosen by colour SPACE from the two entry points the runtime actually
/// provides (`ST.interpolateColor` = OKLCH, `ST.interpolateColorRgb` = sRGB).
/// That is the same dispatch the keyframe path performs
/// (stdlib/primitives/animation/interpolate.st, apply-animations.st) — a
/// per-primitive spelling is exactly how the two drifted apart.
///
/// NB assertions read scalar globals rather than one object literal: a
/// multi-line object literal inside `@eval` is truncated at the first `key:` by
/// the eval-string capture, so the object form silently emits broken JS.

@import "stdlib/testing/test"

// =============================================================================
// The runtime surface exists (the half BUG-202 got wrong)
// =============================================================================

@test "the runtime exposes the colour API the primitives call" {
  @fixture {
    <div class="probe"></div>
  }

  @eval (eval("window.__tParse = typeof ST.parseColor;"))
  @eval (eval("window.__tOklch = typeof ST.interpolateColor;"))
  @eval (eval("window.__tRgb = typeof ST.interpolateColorRgb;"))
  @eval (eval("window.__tLerp = typeof ST.lerpColor;"))

  @assert (window.__tParse === 'function')
    "ST.parseColor must exist — every colour path starts by parsing"

  @assert (window.__tOklch === 'function')
    "ST.interpolateColor (OKLCH) must exist"

  @assert (window.__tRgb === 'function')
    "ST.interpolateColorRgb (sRGB) must exist"

  @assert (window.__tLerp === 'undefined')
    "ST.lerpColor must NOT exist — a primitive naming it calls a ghost (BUG-202)"
}

// =============================================================================
// Blending actually blends
// =============================================================================

@test "interpolation returns distinct values at the extremes" {
  @fixture {
    <div class="probe2"></div>
  }

  @eval (eval("window.__cA = ST.parseColor('#000000');"))
  @eval (eval("window.__cB = ST.parseColor('#ffffff');"))
  @eval (eval("window.__parsedA = window.__cA !== null;"))
  @eval (eval("window.__parsedB = window.__cB !== null;"))
  @eval (eval("window.__start = JSON.stringify(ST.interpolateColor(window.__cA, window.__cB, 0));"))
  @eval (eval("window.__end = JSON.stringify(ST.interpolateColor(window.__cA, window.__cB, 1));"))

  @assert (window.__parsedA === true)
    "a hex colour must parse"

  @assert (window.__parsedB === true)
    "a hex colour must parse"

  @assert (window.__start !== 'null')
    "t=0 must produce a colour, not null"

  @assert (window.__end !== 'null')
    "t=1 must produce a colour, not null"

  @assert (window.__start !== window.__end)
    "black and white must not blend to the same value at both ends"
}

@test "the midpoint differs from both endpoints in both spaces" {
  @fixture {
    <div class="probe3"></div>
  }

  @eval (eval("window.__mA = ST.parseColor('#000000');"))
  @eval (eval("window.__mB = ST.parseColor('#ffffff');"))
  @eval (eval("window.__midOklch = JSON.stringify(ST.interpolateColor(window.__mA, window.__mB, 0.5));"))
  @eval (eval("window.__midRgb = JSON.stringify(ST.interpolateColorRgb(window.__mA, window.__mB, 0.5));"))
  @eval (eval("window.__mStart = JSON.stringify(ST.interpolateColor(window.__mA, window.__mB, 0));"))
  @eval (eval("window.__mEnd = JSON.stringify(ST.interpolateColor(window.__mA, window.__mB, 1));"))

  @assert (window.__midOklch !== 'null')
    "OKLCH midpoint must be a colour"

  @assert (window.__midRgb !== 'null')
    "sRGB midpoint must be a colour"

  @assert (window.__midOklch !== window.__mStart)
    "a midpoint equal to the start means the blend never happened"

  @assert (window.__midOklch !== window.__mEnd)
    "a midpoint equal to the end means the blend never happened"
}

// =============================================================================
// Malformed input degrades to null, it does not throw
// =============================================================================

@test "an unparseable colour yields null rather than throwing" {
  @fixture {
    <div class="probe4"></div>
  }

  @eval (eval("window.__threw = false;"))
  @eval (eval("window.__badResult = 'unset';"))
  @eval (eval("try { window.__badResult = JSON.stringify(ST.interpolateColor(ST.parseColor('not-a-colour'), ST.parseColor('#fff'), 0.5)); } catch (e) { window.__threw = true; }"))

  @assert (window.__threw === false)
    "a bad colour must not throw — that guard is why animate-color can skip a tick"

  @assert (window.__badResult === 'null')
    "interpolating with an unparsed colour must yield null"
}
