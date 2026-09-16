/// Behavior gate: a score's clip window must DRIVE something (BUG-258)
///
/// W4 shipped `@score` computing correct windows — and nothing could read one.
/// Every clip signal appeared exactly three times in the emitted bundle: its
/// publish, its priming read, its watch registration. No consumer. The wave was
/// arithmetically right and behaviorally inert, and every gate was green,
/// because every gate asserted the score PRODUCED a window rather than that a
/// window MOVED anything.
///
/// This file is the missing half. It is written NEGATIVE-FIRST for the same
/// reason `event-driver-triggers.test.st` is: a positive-only gate ("the
/// element animated") passes when a score is wired to the wrong signal, wired
/// to a signal nobody publishes, or not wired at all but animated by some
/// other rule on the page. The load-bearing assertions here are the ones that
/// demand an element be STILL.
///
/// NB `--cdp` (real Chromium): "did this element move at time T" is a
/// paint-adjacent, timing-dependent claim. The fidelity ladder says assert it
/// where it is real; `--headless` refuses these by design.
///
/// HOW THE EFFECT IS OBSERVED: a score publishes a 0..1 progress signal per
/// clip (`__clip_<element>`), and `apply-animations` maps that progress onto
/// the element's style — the same rail every driver uses. So the observable is
/// computed opacity, sampled at wall-clock instants chosen to fall INSIDE and
/// OUTSIDE each clip's window.

@import "stdlib/testing/test"

@form motion --clip-rise { opacity: 0 -> 1; }

// A 4s score. `&clip-early` owns the first second, `&clip-late` the last second, and the
// two windows DO NOT OVERLAP — which is what makes the negatives meaningful:
// at t=300ms `late` must not have started, and that is only observable because
// nothing else on the page could have moved it.
.clip-stage {
  @score &.time(4s) {
    &clip-early at 0s for 1s;
    &clip-late  at 3s for 1s;
  }
}

.clip-early { opacity: 0; @on &.clip: --clip-rise; }
.clip-late  { opacity: 0; @on &.clip: --clip-rise; }

// =============================================================================
// 1. THE NEGATIVE — the assertion that fails today, and the reason this file
//    exists. A clip whose window has not opened must be STILL.
// =============================================================================

@test "score-clip: a clip outside its window has not moved" needs timing {
  @fixture {
    <div class="clip-stage">
      <div class="clip-early">early</div>
      <div class="clip-late">late</div>
    </div>
  }

  @wait 300ms

  @eval (window.__cc1 = parseFloat(getComputedStyle(document.querySelector('.clip-late')).opacity))
  @assert (window.__cc1 === 0)
    "at 300ms the `late` clip's window (3s..4s) has not opened, so it must be untouched — a nonzero value here means the clip is being driven by the WRONG signal (e.g. the parent score's raw progress instead of its own window)"
}

// =============================================================================
// 2. …and a clip INSIDE its window must move. Without this the negative above
//    is satisfied by a page where nothing works at all.
// =============================================================================

@test "score-clip: a clip inside its window has moved" needs timing {
  @fixture {
    <div class="clip-stage">
      <div class="clip-early">early</div>
      <div class="clip-late">late</div>
    </div>
  }

  @wait 500ms

  @eval (window.__cc2 = parseFloat(getComputedStyle(document.querySelector('.clip-early')).opacity))
  @assert (window.__cc2 > 0)
    "at 500ms the `early` clip is halfway through its 0s..1s window, so it must have moved — 0 here is BUG-258 exactly: the window is published and nothing consumes it"
}

// =============================================================================
// 3. THE WINDOW IS A MAP, NOT A SWITCH. `early` must be DONE before `late`
//    starts. This is the assertion that distinguishes a real window map from
//    "every clip just plays the whole score".
// =============================================================================

@test "score-clip: an earlier clip completes before a later one begins" needs timing {
  @fixture {
    <div class="clip-stage">
      <div class="clip-early">early</div>
      <div class="clip-late">late</div>
    </div>
  }

  @wait 1500ms

  @eval (window.__cc3a = parseFloat(getComputedStyle(document.querySelector('.clip-early')).opacity))
  @eval (window.__cc3b = parseFloat(getComputedStyle(document.querySelector('.clip-late')).opacity))

  @assert (window.__cc3a > 0.9)
    "at 1500ms `early`'s window (0s..1s) has closed, so it must be at its end value — a value below 1 means the clip is tracking the PARENT's progress (0.375 at this instant) rather than its own window"
  @assert (window.__cc3b === 0)
    "at 1500ms `late`'s window (3s..4s) has not opened — nonzero means windows are not being honored at all"
}

// =============================================================================
// 4. THE JOIN. By 3.5s the LATER clip must have moved. If clip windows were
//    wired by counter rather than by element (the naming defect probed in
//    PLAN-129), this is the test that catches it: `late` would be watching a
//    signal that belongs to `early`, or to nothing.
// =============================================================================

@test "score-clip: the later clip moves when its own window opens" needs timing {
  @fixture {
    <div class="clip-stage">
      <div class="clip-early">early</div>
      <div class="clip-late">late</div>
    </div>
  }

  @wait 3500ms

  @eval (window.__cc4 = parseFloat(getComputedStyle(document.querySelector('.clip-late')).opacity))
  @assert (window.__cc4 > 0)
    "at 3500ms `late` is halfway through its 3s..4s window and must have moved — 0 here means each clip is addressed by something other than its own element name"
}
