/// Behavior gate: a score is a TRANSPORT (PLAN-155 F1).
///
/// A film studio, a scrub bar, a reviewer pausing on a frame: each needs to
/// hold a playing score, move its playhead and let it go on from there. The
/// time driver had play / pause / reset only, and `play()` after `pause()`
/// restarted the score from 0; nothing could seek it. Now the timeline is a
/// POSITION: a pause holds it, a seek moves it (shown at once, forward or back,
/// paused or playing), play resumes from it. It registers as
/// `ST._transports[name]` and publishes `<name>__playing`.
///
/// NB `--cdp`: wall-clock playback.

@import "stdlib/testing/test"

@form motion --tr-fade { opacity: 0 -> 1; }

.tr-reel {
  @score &.time(6s) as &film { &tr-box at 0s for 6s; }
}
.tr-box { opacity: 0; @on &.clip: --tr-fade; }

@test "a score seeks, holds while paused, and resumes from where it is" needs timing {
  @fixture { <div class="tr-reel"><div class="tr-box">box</div></div> }

  @wait 200ms
  @eval (window.__tr = ST._transports && ST._transports.film)
  @assert (!!window.__tr)
    "a `@score … as &film` registers its transport as ST._transports.film"

  @eval (window.__tr.pause(), window.__tr.seek(3000))
  @wait 80ms
  @eval (window.__o1 = parseFloat(getComputedStyle(document.querySelector('.tr-box')).opacity))
  @assert (Math.abs(window.__o1 - 0.5) < 0.04)
    "seek(3000) on a 6 s score must show its middle frame at once (opacity 0.5)"

  @wait 700ms
  @eval (window.__o2 = parseFloat(getComputedStyle(document.querySelector('.tr-box')).opacity))
  @assert (Math.abs(window.__o2 - window.__o1) < 0.005)
    "a paused score must hold its frame"
  @assert (window.__tr.playing === false)
    "a paused score reports it is not playing"

  @eval (window.__tr.seek(1200))
  @wait 80ms
  @eval (window.__o3 = parseFloat(getComputedStyle(document.querySelector('.tr-box')).opacity))
  @assert (Math.abs(window.__o3 - 0.2) < 0.04)
    "a seek BACK shows the earlier frame (opacity 0.2 at 1.2 s)"

  @eval (window.__tr.play())
  @wait 600ms
  @eval (window.__o4 = parseFloat(getComputedStyle(document.querySelector('.tr-box')).opacity))
  @assert (window.__o4 > 0.24 && window.__o4 < 0.45)
    "play resumes from the playhead (≈1.8 s → 0.3), not from the top (≈0.1)"
}
