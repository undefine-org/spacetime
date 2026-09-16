/// Behavior gate: `@on &.visible` is an ENTRANCE that completes (BUG-323/BUG-315/BUG-322)
///
/// `@on &.visible(300ms)` means "when enough of me is on screen, play this over
/// 300ms". That is the contract the retired `@on visible $name($duration:time,
/// threshold: …)` macro had, and the one the cutover rewrite still lowers to
/// (`stdlib/migrations/entries/2026-07-26-on-cutover.st` maps `@on visible`
/// onto `@on &.load(duration:, delay:, threshold:, immediate:)`).
///
/// The registry had `.visible` bound to `intersection` — a SENSOR whose only
/// progress export is the intersection ratio. Three silent consequences, all
/// fixed by pointing the entry at `load-driver` (the same threshold gate
/// driving a real rAF timeline):
///
///   1. the author's duration reached nothing — `intersection` has no
///      `duration` param, so `800ms` vanished from the bundle entirely
///      (BUG-322 put it in `threshold` when written positionally);
///   2. progress WAS the ratio, so an element only partly on screen sat
///      permanently mid-animation (measured: stuck at 0.605 while fully on
///      screen), and one taller than the viewport could never finish;
///   3. re-entering the viewport re-published a low ratio, reverting a
///      completed entrance (BUG-315).
///
/// NB `--cdp` (real Chromium): these are visibility + timing claims.
/// `--headless` refuses them by design — and would hide (2) and (3) anyway,
/// since its IntersectionObserver shim reports a binary ratio once
/// (`src/headless_shims.js`).
///
/// HOW THE EFFECT IS OBSERVED: the subject is 200px tall between 1200px
/// spacers, so scrolling controls whether it is on screen at all. Computed
/// opacity is the animation's progress.

@import "stdlib/testing/test"

.spacer { height: 1200px; }

// One subject class PER TEST. `@fixture` markup is not torn down between
// tests (BUG-330), so a shared class would let an earlier test's finished
// element answer a later test's `querySelector` — and the negative below
// would be measuring the wrong element entirely.
.subj-complete, .subj-untouched, .subj-reentry {
  height: 200px;
  background: #333;
  opacity: 0;
}
.subj-complete  { @on &.visible(400ms) { opacity: 0 -> 1; } }
.subj-untouched { @on &.visible(400ms) { opacity: 0 -> 1; } }
.subj-reentry   { @on &.visible(400ms) { opacity: 0 -> 1; } }

// =============================================================================
// 1. THE ENTRANCE COMPLETES. An element scrolled into view must reach its end
//    value. Pre-fix this read 0.605 forever: progress was the intersection
//    ratio, sampled once at a threshold crossing and never updated again.
// =============================================================================

@test "on-visible: an element scrolled into view completes its entrance" needs timing {
  @fixture {
    <div class="spacer"></div>
    <div class="subj-complete">subject</div>
    <div class="spacer"></div>
  }

  // Approach in STEPS, the way a reader scrolls — the gate must fire on a
  // partial crossing and the timeline must then play to its end on its own.
  @eval (window.__vTop = document.querySelector('.subj-complete').offsetTop)
  @eval (window.scrollTo(0, window.__vTop - window.innerHeight + 60))
  @wait 250ms
  @eval (document.querySelector('.subj-complete').scrollIntoView({block: 'center'}))
  @wait 900ms

  @eval (window.__v1 = parseFloat(getComputedStyle(document.querySelector('.subj-complete')).opacity))
  @assert (window.__v1 > 0.97)
    "a 400ms entrance, 900ms after the element came on screen, must be finished — a value stuck below 1 means progress is tracking the intersection RATIO instead of a timeline, so the element sits permanently mid-animation"
}

// =============================================================================
// 2. THE NEGATIVE — without this, a page where the animation simply runs on
//    mount (ignoring visibility entirely) would satisfy test 1. An element that
//    has never been on screen must be untouched.
// =============================================================================

@test "on-visible: an element that was never on screen has not moved" needs timing {
  @fixture {
    <div class="spacer"></div>
    <div class="subj-untouched">subject</div>
    <div class="spacer"></div>
  }

  @eval (window.scrollTo(0, 0))
  @wait 900ms

  @eval (window.__v2 = parseFloat(getComputedStyle(document.querySelector('.subj-untouched')).opacity))
  @assert (window.__v2 < 0.05)
    "at scroll 0 the subject sits below a 1200px spacer, entirely off screen, so its entrance must not have started — a nonzero value means the timeline ignores visibility and plays on mount"
}

// =============================================================================
// 3. IT STAYS DONE (BUG-315). Scrolling away and back must not rewind a
//    completed entrance. This is the assertion a ratio-as-progress
//    implementation cannot satisfy: leaving the viewport republishes ratio 0.
// =============================================================================

@test "on-visible: a completed entrance survives leaving and re-entering the viewport" needs timing {
  @fixture {
    <div class="spacer"></div>
    <div class="subj-reentry">subject</div>
    <div class="spacer"></div>
  }

  @eval (document.querySelector('.subj-reentry').scrollIntoView({block: 'center'}))
  @wait 900ms
  @eval (window.scrollTo(0, 0))
  @wait 400ms
  @eval (document.querySelector('.subj-reentry').scrollIntoView({block: 'center'}))
  @wait 300ms

  @eval (window.__v3 = parseFloat(getComputedStyle(document.querySelector('.subj-reentry')).opacity))
  @assert (window.__v3 > 0.97)
    "the entrance already completed once, so scrolling away and back must leave it at its end value — a value below 1 is BUG-315: leaving the viewport republished a low intersection ratio and rewound a finished animation"
}
