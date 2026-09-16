/// Behavior gate: PLAN-150 W1 — a positioned keyframe stop (`0.85 at 25%`)
/// peaks at its DECLARED position, not at the even-spaced one, and a per-step
/// easing (`0 -> 1 --ease-out-expo`) curves the segment it names.
///
/// This is the film-surface keyframe syntax of docs/language/film.st.md §2.
/// The discriminator is the whole point: with the old even-spacing a 4-stop
/// chain put the second stop at 33%; W1 puts it where the author said (25%).
/// A gate that only asserted "opacity moves" would pass on BOTH, so it asserts
/// the VALUE at a time only the positioned interpretation can produce.
///
/// NB `--cdp` (real Chromium) under the virtual clock: a value-at-time claim is
/// timing-dependent; `--headless` refuses by design.

@import "stdlib/testing/test"

// A 10s transport so 1s = 10% — easy exact sampling. `.flash` peaks at 20%.
.stage {
  @score &.time(10s) as &film {
    &flash at 0s for 10s;
  }
}

.flash {
  opacity: 0;
  // 0 -> 1 (peak) at 20% -> 0. Even spacing would put the peak at 50%.
  @on &.clip { opacity: 0 -> 1 at 20% -> 0; }
}

@test "positioned stop: the peak is at 20%, not the even-spaced 50%" needs timing {
  @fixture { <div class="stage"><div class="flash">x</div></div> }

  // sample the whole first 4s at 200ms; the max opacity must occur near 2s.
  @eval (window.__ps = [], window.__pt = setInterval(function () {
    window.__ps.push({ t: (window.__ps.length) * 0.2, o: parseFloat(getComputedStyle(document.querySelector('.flash')).opacity) });
  }, 200))

  @wait 4200ms
  @eval (clearInterval(window.__pt))
  @eval (window.__peak = window.__ps.reduce(function (a, b) { return b.o > a.o ? b : a; }, { t: 0, o: -1 }))
  @eval (window.__at50 = window.__ps.reduce(function (a, b) { return Math.abs(b.t - 5) < Math.abs(a.t - 5) ? b : a; }, window.__ps[0]))

  @assert (window.__peak.o > 0.9)
    "the stop reaches opacity 1 somewhere — the animation must run at all"
  @assert (window.__peak.t > 1.4 && window.__peak.t < 2.6)
    "the peak must fall near 20% of a 10s clip (≈2s). Even-spacing would peak at 5s — a value only positioned stops can produce"
}

// Per-step easing: an ease-out-expo segment is far past halfway at its own
// midpoint; a linear segment sits exactly halfway. Sampling the SAME element
// under each easing at the segment midpoint separates them.
.ramp-expo { width: 0px; @on &.clip { width: 0px -> 1000px --ease-out-expo; } }
.ramp-lin  { width: 0px; @on &.clip { width: 0px -> 1000px --linear; } }

.stage2 {
  @score &.time(10s) as &film2 {
    &ramp-expo at 0s for 10s;
    &ramp-lin  at 0s for 10s;
  }
}

@test "per-step easing: an expo segment leads a linear one at the midpoint" needs timing {
  @fixture {
    <div class="stage2">
      <div class="ramp-expo">e</div>
      <div class="ramp-lin">l</div>
    </div>
  }

  @wait 5000ms

  @eval (window.__we = parseFloat(getComputedStyle(document.querySelector('.ramp-expo')).width))
  @eval (window.__wl = parseFloat(getComputedStyle(document.querySelector('.ramp-lin')).width))

  @assert (window.__wl > 400 && window.__wl < 600)
    "the linear ramp is ~halfway (≈500px) at the midpoint of its window"
  @assert (window.__we > window.__wl + 150)
    "the ease-out-expo ramp must LEAD the linear one at the midpoint — a per-step `--ease-out-expo` that did nothing would sit at the same 500px"
}
