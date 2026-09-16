/// Behavior gate: a bare `--story;` splice (no `for` frame) under an own-clock
/// `.loop`/`.time` score scales its relative measures to the PARENT'S total —
/// so `for 50%` is half the score, not half a millisecond (BUG-267).
///
/// This is the website pole of demos/score-launch §3, mounted alone (its own
/// fixture, so the `__clip_*` signal names do not collide with the video
/// pole's — see two-pole.test.st for both poles in one mount).
///
/// THE DISCRIMINATOR: at 1500ms of the 6s loop, `head` (a 3s window — half the
/// score) is exactly mid-progress, so a correct page reads opacity ≈ 0.5. A
/// value near 1.0 means the raw `for 50%` fraction leaked into the window map
/// as span = 0.5 against total = 6000 — a half-millisecond window that is
/// already over, jumping the head to its end at parent progress 0.125.
///
/// NB `--cdp` (real Chromium): timing-dependent by design; `--headless`
/// refuses.

@import "stdlib/testing/test"

@form motion --clip-rise { opacity: 0 -> 1; }

@form score --story {
  &head for 50%;
  &tail for 50%;
}

.by-time {
  @score &.loop(6s) { --story; }
}

.head, .tail { opacity: 0; }
.head { @on &.clip: --clip-rise; }
.tail { @on &.clip: --clip-rise; }

@test "bare-splice: a bare `for 50%` splice samples to half the parent's span" needs timing {
  @fixture {
    <div class="by-time">
      <div class="head">head</div>
      <div class="tail">tail</div>
    </div>
  }

  @wait 1500ms

  @eval (window.__bs1 = parseFloat(getComputedStyle(document.querySelector('.by-time .head')).opacity))
  @eval (window.__bs2 = parseFloat(getComputedStyle(document.querySelector('.by-time .tail')).opacity))

  @assert (window.__bs1 > 0.2 && window.__bs1 < 0.9)
    "BUG-267: at 1500ms the by-time head (a 3s window in the 6s loop) is half-done, so opacity must be mid-window (≈0.5). A value near 1.0 means `for 50%` leaked the raw fraction into a half-millisecond window that is already over — the bare `--story;` splice did not scale relative measures to the parent's total"
  @assert (window.__bs2 === 0)
    "at 1500ms the by-time tail (3000ms..6000ms) has not started — nonzero means the fragment's two clips are not sequenced"
}
