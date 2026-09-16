/// Behavior gate: PLAN-150 W3-arc — the bare `@on <driver> --form;` statement.
/// The film surface's natural spelling (docs/language/film.st.md), landed.
///
/// `@on &.clip --rise;` (no colon, no braces) now LOWERS identically to
/// `@on &.clip: --rise;` and `@on &.clip { --rise; }` — the golden uses the
/// colon-free spelling throughout. This asserts the BEHAVIOUR: the bare form
/// actually drives its element (opacity rises across the clip window), not the
/// emitted string (BUG-252).
///
/// FIDELITY: timing rung (`--cdp`) under the score clock.

@import "stdlib/testing/test"

@form motion --rise { opacity: 0 -> 1; }

.reel { @score &.time(6s) as &film { &box at 0s for 6s; } }
.box { opacity: 1; @on &.clip --rise; }

@test "the bare @on driver --form; statement drives its element" needs timing {
  @fixture { <div class="reel"><div class="box">shot</div></div> }

  @eval (window.__box = document.querySelector('.box'))
  @eval (window.__op0 = Number(getComputedStyle(window.__box).opacity))
  @wait 3000ms
  @eval (window.__op1 = Number(getComputedStyle(window.__box).opacity))

  @assert (window.__op0 < 0.5)
    "the colon-free `@on &.clip --rise;` must start the box near opacity 0 — a form that errored or dropped would leave it at 1"
  @assert (window.__op1 > window.__op0 + 0.2)
    "the bare statement must DRIVE the box's opacity up across the window, exactly as the colon/brace forms do"
}
