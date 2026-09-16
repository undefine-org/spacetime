/// Behavior gate: the SAME `--story` fragment marches under both clocks in one
/// mount — the exact composition of demos/score-launch §3 (BUG-266 + BUG-267).
///
/// One fragment (`&head for 50%; &tail for 50%;`) runs under a `.loop(6s)`
/// time clock (the website pole) and under an inherited `&.clip` window (the
/// video pole, a nested score inside `.film`). Both poles share the DOM here
/// precisely as the demo does, so this gate proves the co-existence works and
/// both heads march in lockstep — the thesis (SIP-001b §5) as an observable.
///
/// The two per-pole bugs are each gated alone too (bare-splice.test.st for
/// BUG-267, nested-clip.test.st for BUG-266); this file is the two together.
/// It is ONE `@test` with one `@fixture` because score signals bridge to a
/// page-global plane and a second mount in the same page could read a stale
/// `__clip_head` — a single mount keeps the claim clean.

@import "stdlib/testing/test"

@form motion --clip-rise { opacity: 0 -> 1; }

@form score --story {
  &head for 50%;
  &tail for 50%;
}

.by-time {
  @score &.loop(6s) { --story; }
}

.film {
  @score &.loop(12s) as &film {
    &shot at 0s for 6s;
  }
}

.shot {
  @score &.clip { --story; }
}

.head, .tail { opacity: 0; }
.head { @on &.clip: --clip-rise; }
.tail { @on &.clip: --clip-rise; }

@test "single mount both poles" needs timing {
  @fixture {
    <div class="by-time"><div class="head">h</div><div class="tail">t</div></div>
    <div class="film"><div class="shot"><div class="head">h</div><div class="tail">t</div></div></div>
  }
  @wait 1500ms
  @eval (window.__bt = parseFloat(getComputedStyle(document.querySelector('.by-time .head')).opacity))
  @eval (window.__fl = parseFloat(getComputedStyle(document.querySelector('.film .shot .head')).opacity))
  @eval (window.__btt = parseFloat(getComputedStyle(document.querySelector('.by-time .tail')).opacity))
  @eval (window.__flt = parseFloat(getComputedStyle(document.querySelector('.film .shot .tail')).opacity))
  @assert (window.__bt > 0.2 && window.__bt < 0.9) "BYTIME-HEAD-MID"
  @assert (window.__fl > 0.2 && window.__fl < 0.9) "FILM-HEAD-MID"
  @assert (window.__btt === 0) "BYTIME-TAIL-STILL"
  @assert (window.__flt === 0) "FILM-TAIL-STILL"
  @assert (Math.abs(window.__bt - window.__fl) < 0.2) "LOCKSTEP"
}
