/// Behavior gate: an UNNAMED driver arg must reach the param the author meant (BUG-322)
///
/// `@on &.time(300ms)` was lowered as a bare positional and bound BY INDEX into
/// the target primitive's signature. The index is almost never the parameter
/// the author meant:
///
///   .time    -> time-driver(&el, name, duration, …)   `300ms` became the NAME
///   .load    -> load-driver(&el, name, duration, …)   `300ms` became the NAME
///   .visible -> intersection(&el, threshold, …)       `300ms` became threshold: 300
///
/// A driver whose progress publishes under the name `"300"` while the animation
/// watches `__drive_time_2` moves nothing at all, silently. Named args
/// (`duration: 300ms`) were always correct, so the surface LOOKED supported and
/// every string-level gate passed — BUG-252 class.
///
/// This is also the root cause of the suite's largest failure bucket: eight
/// "Timeline <name> not registered" failures, all from `@on &.visible(800ms)`.
///
/// NB `--cdp` (real Chromium): "did this element move, and by when" is a
/// timing claim. `--headless` refuses it by design — and would in any case hide
/// the `.visible` half, since its IntersectionObserver shim reports a binary
/// ratio once (`src/headless_shims.js`).
///
/// HOW THE EFFECT IS OBSERVED: each subject animates opacity 0 -> 1 over a
/// SHORT window under a positional arg. Sampling after that window must find
/// the animation finished. The negative-first companion is test 3: a LONG
/// positional duration must NOT be finished at the same instant — that is what
/// proves the number was read as a duration rather than ignored or defaulted.

@import "stdlib/testing/test"

.t-pos { opacity: 0; @on &.time(300ms) { opacity: 0 -> 1; } }
.l-pos { opacity: 0; @on &.load(300ms) { opacity: 0 -> 1; } }
.t-slow { opacity: 0; @on &.time(4000ms) { opacity: 0 -> 1; } }

// =============================================================================
// 1. `.time` — the plain case. A positional duration must drive the timeline.
// =============================================================================

@test "positional arg: @on &.time(300ms) completes within its window" needs timing {
  @fixture {
    <div class="t-pos">time</div>
  }

  @wait 900ms

  @eval (window.__p1 = parseFloat(getComputedStyle(document.querySelector('.t-pos')).opacity))
  @assert (window.__p1 > 0.95)
    "a 300ms timeline sampled at 900ms must be finished — 0 here is BUG-322: the `300ms` was bound to the primitive's FIRST param (`name`), so the driver published progress under the signal name \"300\" while the animation watched `__drive_time_N`, and the two never met"
}

// =============================================================================
// 2. `.load` — the same defect through a different primitive, so a fix that
//    special-cases one driver cannot satisfy this file.
// =============================================================================

@test "positional arg: @on &.load(300ms) completes within its window" needs timing {
  @fixture {
    <div class="l-pos">load</div>
  }

  @wait 900ms

  @eval (window.__p2 = parseFloat(getComputedStyle(document.querySelector('.l-pos')).opacity))
  @assert (window.__p2 > 0.95)
    "`.load` takes the same (&el, name, duration, …) shape as `.time`, so a positional duration must reach `duration` here too — a per-driver fix that leaves this red has not fixed the rule"
}

// =============================================================================
// 3. THE NEGATIVE — the assertion that makes the two above meaningful. If a
//    positional arg were simply DROPPED, every driver would fall back to its
//    default duration and tests 1 and 2 would pass by accident. A 4000ms
//    timeline must still be running at the instant a 300ms one has finished.
// =============================================================================

@test "positional arg: a long duration is still running when a short one has finished" needs timing {
  @fixture {
    <div class="t-slow">slow</div>
  }

  @wait 900ms

  @eval (window.__p3 = parseFloat(getComputedStyle(document.querySelector('.t-slow')).opacity))
  @assert (window.__p3 < 0.85)
    "a 4000ms timeline sampled at 900ms is roughly a quarter through, so it must NOT be finished — a value at 1 means the positional `4000ms` was discarded and the driver ran on its default duration, which would also make the short-duration tests pass for the wrong reason"
}
