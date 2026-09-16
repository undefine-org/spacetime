/// Behavior gate: PLAN-150 W6 — `--noise-1d` as a progress-driven motion value.
/// The "noise as data" of docs/language/film.st.md §7.
///
/// A noise motion form (`@form motion --shake { translate-x: --noise-1d($amp, $hz); }`)
/// is smooth, periodic, deterministic motion evaluated against clip progress —
/// shake/drift/flicker without hand-typed keyframe chains. This asserts the
/// BEHAVIOUR (the transform CHANGES across the window as noise evaluates, seen
/// RED against a static value), not the emitted string (BUG-252).
///
/// FIDELITY: paint rung (`--cdp`); a progress-driven value needs a real clock.

@import "stdlib/testing/test"
@form motion --shake($amp = 10px, $hz = 5) { translate-x: --noise-1d($amp, $hz); }
.reel { @score &.time(6s) as &f { &cam at 0s for 6s; } }
.cam { width: 100px; height: 100px; @on &.clip { --shake; } }
@test "noise-1d animates translate over progress" needs timing {
  @fixture { <div class="reel"><div class="cam">x</div></div> }
  @eval (window.__tr = [], window.__t = setInterval(function(){ window.__tr.push(getComputedStyle(document.querySelector('.cam')).transform); }, 200))
  @wait 3000ms
  @eval (clearInterval(window.__t))
  @eval (window.__distinct = window.__tr.filter(function(v,i,a){ return a.indexOf(v)===i; }).length)
  @eval (window.__nonNone = window.__tr.filter(function(v){ return v !== 'none'; }).length)
  @assert (window.__nonNone > 3) "transform must be applied (not none) during the window"
  @assert (window.__distinct > 4) "transform must CHANGE across the window as noise evaluates against progress"
}
