/// Behavior gate (W5/P10): a `.playback` score is a MEDIA clock — progress is
/// the media element's currentTime over its duration. (SIP-001 `.playback`:
/// "transport (currentTime)", domain `inherited` — the media element's
/// duration, so placements are times.)
///
/// Like steps-driver, `.playback` shipped as a `planned:` row (E0948); the
/// risk of making it real is a score that compiles but never moves. So these
/// assertions are NEGATIVE-FIRST: before the playhead advances, the clip is
/// STILL — only moving the media element's `currentTime` may move it.
///
/// No real media file is needed: the element's `duration` is stubbed via a
/// getter and the playhead advanced programmatically, exactly what the driver
/// would observe on a real element (it only reads `currentTime`/`duration`).
///
/// NB `--cdp`: media timing is paint-adjacent; `--headless` refuses it.

@import "stdlib/testing/test"

@form motion --play-rise { opacity: 0 -> 1; }

// A 10-second media element. `&bar` owns the first 5 seconds (absolute);
// `&bar2 at 0s for 50%` is the SAME [0, 5s) window written RELATIVELY
// (BUG-270) — half the media duration, which the element does not know until
// metadata loads, so the percentage must scale by the resolved duration.
// `at 0s` keeps bar2 starting at the playhead's origin regardless of where it
// sits in the body (a percentage-only offset would inherit the cursor).
.mv {
  @score &.playback {
    &bar at 0s for 5s;
    &bar2 at 0s for 50%;
  }
}
.bar { opacity: 0; @on &.clip: --play-rise; }
.bar2 { opacity: 0; @on &.clip: --play-rise; }

// Stub the media clock: duration 10s, currentTime backed by window.__ct.
// Written as expression-bodied arrows because `@eval`'s expr capture cannot
// carry a `{ … }` body (an IIFE or descriptor object literal emits JS that
// does not parse — E0952).
@test "playback: a clip is still before the media playhead advances" needs timing {
  @fixture {
    <video class="mv" data-test-media>
      <div class="bar">bar</div>
    </video>
  }

  @wait 250ms

  @eval (window.__ct = 0)
  @eval (window.__mv = document.querySelector('.mv'))
  @eval (window.__mv.__defineGetter__('duration', () => 10))
  @eval (window.__mv.__defineGetter__('currentTime', () => window.__ct))
  @eval (window.__mv.__defineSetter__('currentTime', (v) => window.__ct = v))
  @eval (window.__mv.dispatchEvent(new Event('loadedmetadata')))
  @wait 60ms

  @eval (window.__pb0 = parseFloat(getComputedStyle(document.querySelector('.bar')).opacity))
  @assert (window.__pb0 === 0)
    "at currentTime=0 the clip's window (0..5s) has not opened — a nonzero value means the clip is driven by something other than the media playhead"
}

// =============================================================================
// 2. Advance the playhead to 4s — the clip (0..5s) must move. If the score
//    were wired to a frozen clock or the wrong signal, this fails.
// =============================================================================

@test "playback: a clip moves when the media playhead advances" needs timing {
  @fixture {
    <video class="mv" data-test-media>
      <div class="bar">bar</div>
    </video>
  }

  @wait 250ms

  @eval (window.__ct = 0)
  @eval (window.__mv = document.querySelector('.mv'))
  @eval (window.__mv.__defineGetter__('duration', () => 10))
  @eval (window.__mv.__defineGetter__('currentTime', () => window.__ct))
  @eval (window.__mv.__defineSetter__('currentTime', (v) => window.__ct = v))
  @eval (window.__mv.dispatchEvent(new Event('loadedmetadata')))
  @wait 60ms

  @eval (window.__mv.currentTime = 4)
  @eval (window.__mv.dispatchEvent(new Event('timeupdate')))
  @wait 60ms

  @eval (window.__pb1 = parseFloat(getComputedStyle(document.querySelector('.bar')).opacity))
  @assert (window.__pb1 > 0)
    "at currentTime=4 the clip (0..5s window, 80% through it) must have moved — 0 means the media playhead is not driving the score"
}

// =============================================================================
// 3. THE MAP, NOT A SWITCH — the clip's progress tracks the playhead, not a
//    boolean. At exactly 2s of 10 (20% of the media, 40% of the 0..5s window)
//    the clip should be near 0.4, distinctly between "not started" and "done".
// =============================================================================

@test "playback: the clip tracks the playhead position, not an on/off switch" needs timing {
  @fixture {
    <video class="mv" data-test-media>
      <div class="bar">bar</div>
    </video>
  }

  @wait 250ms

  @eval (window.__ct = 0)
  @eval (window.__mv = document.querySelector('.mv'))
  @eval (window.__mv.__defineGetter__('duration', () => 10))
  @eval (window.__mv.__defineGetter__('currentTime', () => window.__ct))
  @eval (window.__mv.__defineSetter__('currentTime', (v) => window.__ct = v))
  @eval (window.__mv.dispatchEvent(new Event('loadedmetadata')))
  @wait 60ms

  @eval (window.__mv.currentTime = 2)
  @eval (window.__mv.dispatchEvent(new Event('timeupdate')))
  @wait 60ms

  @eval (window.__pb2 = parseFloat(getComputedStyle(document.querySelector('.bar')).opacity))
  @assert (window.__pb2 > 0.2 && window.__pb2 < 0.7)
    "at currentTime=2 of a 10s media, the clip (0..5s window) is 40% through it \
     — a value near 0 means the playhead is not read; a value near 1 means the \
     clip snapped (a switch, not a map)"
}

// =============================================================================
// 4. PERCENTAGE UNDER PLAYBACK (BUG-270). `&bar2 for 50%` is half the MEDIA
//    duration, which a media element does not know until its metadata loads.
//    So the percentage cannot be a compile-time constant span — it must scale
//    by the RESOLVED duration at sample time. At currentTime = duration/4 the
//    clip (0..50% of the media) is exactly half-done.
//
//    The assertion reads the RAW clip-progress signal `__clip_bar2` (via
//    `ST.get`) off the score scope, NOT a mapped opacity — a CSS opacity
//    transition would blur the distinction between "progress 0.5" (fixed) and
//    "progress 1.0" (broken, snapping to completion). The signal is 0..1
//    exactly: under the bug the 0.5ms window makes progress clamp to 1.0 by
//    parentProgress 0.25; fixed, progress is 0.5 at duration/4.
// =============================================================================

@test "playback: a `for 50%` clip is mid-animation at a quarter of the duration" needs timing {
  @fixture {
    <video class="mv" data-test-media>
      <div class="bar">bar</div>
      <div class="bar2">bar2</div>
    </video>
  }

  @wait 250ms

  @eval (window.__ct = 0)
  @eval (window.__mv = document.querySelector('.mv'))
  @eval (window.__mv.__defineGetter__('duration', () => 10))
  @eval (window.__mv.__defineGetter__('currentTime', () => window.__ct))
  @eval (window.__mv.__defineSetter__('currentTime', (v) => window.__ct = v))
  @eval (window.__mv.dispatchEvent(new Event('loadedmetadata')))
  @wait 100ms

  @eval (window.__mv.currentTime = 2.5)
  @eval (window.__mv.dispatchEvent(new Event('timeupdate')))
  @wait 300ms

  @eval (window.__pb3 = ST.get(document.querySelector('.mv'), '__clip_bar2'))
  @assert (window.__pb3 > 0.3 && window.__pb3 < 0.7)
    "at currentTime = duration/4 = 2.5s, the `for 50%` clip (0..50% of the media) is \
     exactly half-done: its progress signal must sit near 0.5, neither still nor \
     completed. A value near 1.0 means the percentage was folded to a constant \
     span (0.5ms) that completed the moment the playhead moved — BUG-270."
}
