/// Behavior gate: an event driver listens for ITS OWN event (BUG-253 / PLAN-127)
///
/// Every one of the five event-driver rows — `.hover` `.click` `.focus`
/// `.submit` `.key` — compiled to the SAME logic: `const trig = "hover"`, the
/// primitive's parameter default. `@on &.click:` listened for *mouseenter*.
/// `@on &.submit:` emitted no submit listener at all. All five built clean,
/// with zero diagnostics, and every existing gate stayed green.
///
/// The gates stayed green because they were POSITIVE-ONLY. A test that hovers
/// a `.click` element and observes an animation passes — while proving the
/// exact opposite of what it claims. The NEGATIVE is therefore the load-bearing
/// assertion in this file: `.click` must animate on click AND MUST NOT animate
/// on hover. Nothing weaker distinguishes a working trigger from a broken one.
///
/// NB `--cdp` (real Chromium): "did this element animate" is a paint-adjacent
/// claim, and the fidelity ladder says make it where it is real.
///
/// HOW THE EFFECT IS OBSERVED: the event driver does not create a WAAPI
/// animation — it drives a PROGRESS signal on a rAF loop, and the motion form
/// maps that progress onto the element's style. So the observable is the
/// element's computed opacity moving off its 0 start, after a short wait for
/// rAF to run. (Asserting `getAnimations().length` here would be asserting
/// against the wrong mechanism entirely — it reads 0 even when the driver is
/// working perfectly.)

@import "stdlib/testing/test"

@form motion --edt-rise { opacity: 0 -> 1; }

// One element per row. Separate elements (never a shared node) so that
// "nothing animated" can never be satisfied — or violated — by an animation
// some earlier test started elsewhere.
.edt-click  { opacity: 0; @on &.click:  --edt-rise; }
// A DEDICATED element for the toggle test. Page state persists across tests in
// one file, and a click driver latches its progress — so reusing .edt-click
// would make the toggle test's "first" click actually the SECOND, and it would
// measure a reversal it mistook for a forward run.
.edt-toggle { opacity: 0; @on &.click:  --edt-rise; }
.edt-hover  { opacity: 0; @on &.hover:  --edt-rise; }
.edt-focus  { opacity: 0; @on &.focus:  --edt-rise; }
.edt-submit { opacity: 0; @on &.submit: --edt-rise; }
.edt-key    { opacity: 0; @on &.key:    --edt-rise; }

// =============================================================================
// 1. THE NEGATIVE — the assertion that fails today, and the reason this file
//    exists. A `.click` driver must be INERT under hover.
// =============================================================================

@test "event-driver: .click does NOT animate on hover" needs timing {
  @fixture {
    <div class="edt-click">x</div>
  }

  @when .edt-click mouseenter
  @wait 150ms

  @eval (window.__edt1 = parseFloat(getComputedStyle(document.querySelector('.edt-click')).opacity))
  @assert (window.__edt1 === 0)
    "a .click driver must ignore hover — animating here means the row emitted another row's trigger (BUG-253: every event row emitted trig hover)"
}

// =============================================================================
// 2. …and it must still animate on the event it names.
// =============================================================================

@test "event-driver: .click animates on click" needs timing {
  @fixture {
    <div class="edt-click">x</div>
  }

  @when .edt-click click
  @wait 150ms

  @eval (window.__edt2 = parseFloat(getComputedStyle(document.querySelector('.edt-click')).opacity))
  @assert (window.__edt2 > 0)
    "a .click driver must animate on click"
}

// =============================================================================
// 3. .submit — the row that emitted NO submit listener whatsoever.
// =============================================================================

@test "event-driver: .submit animates on submit" needs timing {
  @fixture {
    <form class="edt-submit"><button type="submit">go</button></form>
  }

  @when .edt-submit submit
  @wait 150ms

  @eval (window.__edt3 = parseFloat(getComputedStyle(document.querySelector('.edt-submit')).opacity))
  @assert (window.__edt3 > 0)
    "a .submit driver must animate on form submit — 0 here means no submit listener was ever registered"
}

// =============================================================================
// 4. .focus — same shape, and inert under hover.
// =============================================================================

@test "event-driver: .focus animates on focus, not on hover" needs timing {
  @fixture {
    <div class="edt-focus" tabindex="0">x</div>
  }

  @when .edt-focus mouseenter
  @wait 150ms

  @eval (window.__edt4a = parseFloat(getComputedStyle(document.querySelector('.edt-focus')).opacity))
  @assert (window.__edt4a === 0)
    "a .focus driver must ignore hover"

  @when .edt-focus focus
  @wait 150ms

  @eval (window.__edt4b = parseFloat(getComputedStyle(document.querySelector('.edt-focus')).opacity))
  @assert (window.__edt4b > 0)
    "a .focus driver must animate on focus"
}

// =============================================================================
// 5. .key — keyboard, and inert under hover.
// =============================================================================

@test "event-driver: .key animates on keydown, not on hover" needs timing {
  @fixture {
    <div class="edt-key" tabindex="0">x</div>
  }

  @when .edt-key mouseenter
  @wait 150ms

  @eval (window.__edt5a = parseFloat(getComputedStyle(document.querySelector('.edt-key')).opacity))
  @assert (window.__edt5a === 0)
    "a .key driver must ignore hover"

  @when .edt-key keydown "a"
  @wait 150ms

  @eval (window.__edt5b = parseFloat(getComputedStyle(document.querySelector('.edt-key')).opacity))
  @assert (window.__edt5b > 0)
    "a .key driver must animate on keydown"
}

// =============================================================================
// 6. NO REGRESSION — .hover was the one row that behaved correctly (by
//    accident: its trigger IS the default). It must still work after the fix.
// =============================================================================

@test "event-driver: .hover still animates on hover" needs timing {
  @fixture {
    <div class="edt-hover">x</div>
  }

  @when .edt-hover mouseenter
  @wait 150ms

  @eval (window.__edt6 = parseFloat(getComputedStyle(document.querySelector('.edt-hover')).opacity))
  @assert (window.__edt6 > 0)
    "the .hover row must keep working — it is the only event row that was correct before the fix"
}

// =============================================================================
// 7. BEHAVIOR THAT MUST SURVIVE THE DISPATCH DELETION.
//
//    The primitive used to branch on the trigger NAME (`if (trig === 'hover')`
//    … `else if (trig === 'click')`). Those branches carry real behavior —
//    hover reverses on mouseleave, click toggles on a second press — which is
//    NOT the same thing as dispatch. These two tests pin that behavior so the
//    branches can be replaced with registry data without silently dropping it.
// =============================================================================

@test "event-driver: .hover reverses on mouseleave" needs timing {
  @fixture {
    <div class="edt-hover">x</div>
  }

  @when .edt-hover mouseenter
  @wait 600ms

  @eval (window.__edt7a = parseFloat(getComputedStyle(document.querySelector('.edt-hover')).opacity))
  @assert (window.__edt7a > 0)
    "hover must animate forward on mouseenter"

  @when .edt-hover mouseleave
  @wait 400ms

  @eval (window.__edt7b = parseFloat(getComputedStyle(document.querySelector('.edt-hover')).opacity))
  @assert (window.__edt7b < window.__edt7a)
    "hover must run BACKWARD on mouseleave — the reverse pairing is behavior, not dispatch"
}

@test "event-driver: .click toggles on a second press" needs timing {
  @fixture {
    <div class="edt-toggle">x</div>
  }

  @when .edt-toggle click
  @wait 1200ms

  @eval (window.__edt8a = parseFloat(getComputedStyle(document.querySelector('.edt-toggle')).opacity))
  @assert (window.__edt8a > 0.5)
    "the first click must drive progress forward (the @on head's duration default is 1000ms, so the wait must clear it)"

  @when .edt-toggle click
  @wait 400ms

  @eval (window.__edt8b = parseFloat(getComputedStyle(document.querySelector('.edt-toggle')).opacity))
  @assert (window.__edt8b < window.__edt8a)
    "a second click must reverse — click's toggle is behavior, not dispatch"
}
