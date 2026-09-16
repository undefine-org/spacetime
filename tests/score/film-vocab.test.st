/// Behavior gate: PLAN-150 W12 — the shipped film vocabulary.
/// The "shipped film vocabulary" of docs/language/film.st.md §12.
///
/// `stdlib/film` ships named forms (focus-pull, shimmer, cinema, shake, …) built
/// only from the landed film surface, so a film's first draft is one line per
/// shot. This asserts the BEHAVIOUR — a vocabulary form imported from stdlib and
/// consumed by `@on &.clip` actually drives its element (opacity rises as
/// `--focus-pull` plays) (BUG-252), not the emitted string. It also proves the
/// forms COMPOSE (`--focus-pull` on the title over the whole reel).
///
/// FIDELITY: paint rung (`--cdp`) under the score clock; refuses headless.

@import "stdlib/testing/test"
@import "stdlib/film"

.reel {
  @score &.time(6s) as &film { &title at 0s for 6s; }
}
.title {
  opacity: 1;
  @on &.clip { --focus-pull; }
}

@test "a stdlib film-vocabulary form drives its element" needs timing {
  @fixture { <div class="reel"><h1 class="title">HELLO</h1></div> }

  @eval (window.__title = document.querySelector('.title'))
  @eval (window.__op0 = Number(getComputedStyle(window.__title).opacity))
  @wait 3000ms
  @eval (window.__op1 = Number(getComputedStyle(window.__title).opacity))

  @assert (window.__op0 < 0.5)
    "at the clip head `--focus-pull` starts the title near opacity 0 — a form that never bound would leave it at 1"
  @assert (window.__op1 > window.__op0 + 0.2)
    "the imported `--focus-pull` must drive the title's opacity up across the window — a vocabulary form that only compiled but never ran leaves it frozen"
}
