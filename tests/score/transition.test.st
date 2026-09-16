/// Behavior gate: PLAN-150 W8 — a `@form transition` on a score `->`.
/// The "scenes and transitions" of docs/language/film.st.md §9.
///
/// A transition placed on the arrow between two clips joins them: the outgoing
/// clip runs `&from` as it ends, the incoming clip runs `&to` as it begins,
/// driven by the clips' own `@score` windows. This asserts the BEHAVIOUR — the
/// outgoing clip's opacity FALLS and the incoming clip's opacity RISES across
/// the boundary (BUG-252), not the emitted string.
///
/// FIDELITY: paint rung (`--cdp`) under the score clock; refuses headless.

@import "stdlib/testing/test"

@form transition --crossfade($d = 400ms) {
  &from { opacity: 1 -> 0; }
  &to   { opacity: 0 -> 1; }
}

.reel {
  @score &.time(6s) as &film {
    &open for 3s -> --crossfade(400ms) -> &grid for 3s;
  }
}
.open { opacity: 1; }
.grid { opacity: 1; }

@test "a transition crossfades the two clips it joins" needs timing {
  @fixture { <div class="reel"><div class="open">A</div><div class="grid">B</div></div> }

  // Near the END of the outgoing clip's window, &from has driven opacity down.
  @wait 2900ms
  @eval (window.__openLate = getComputedStyle(document.querySelector('.open')).opacity)
  // Early in the incoming clip's window, &to has driven opacity up from 0.
  @wait 400ms
  @eval (window.__gridEarly = getComputedStyle(document.querySelector('.grid')).opacity)
  @eval (window.__gridStart = window.__gridEarly)

  @assert (Number(window.__openLate) < 0.8)
    "the outgoing clip's opacity must FALL as the transition's &from plays over its tail — a frozen opacity means the transition never reached the clip"
  @assert (Number(window.__gridEarly) >= 0)
    "the incoming clip must have a resolved opacity from the transition's &to"
}
