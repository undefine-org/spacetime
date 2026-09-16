/// Behavior gate: PLAN-150 W9 — `@form shot` carries markup AND motion.
/// The "shots" of docs/language/film.st.md §10.
///
/// A shot is a form whose body has a `:markup` slot (a template) and a `:motion`
/// slot (an `@on &.clip` keyframe body). Invoking it instantiates the markup and
/// runs the motion. This asserts the BEHAVIOUR — the markup is rendered into the
/// DOM and the motion drives it (opacity rises from 0) (BUG-252), not the
/// emitted string.
///
/// FIDELITY: paint rung (`--cdp`); the reveal motion needs a real clock.

@import "stdlib/testing/test"

@form shot --card-reveal($title, $price) {
  :markup {
    <div class="card"><span class="nm">`$title`</span><span class="price">`$price`</span></div>
  }
  :motion {
    opacity: 0 -> 1;
  }
}

.panel { &card-reveal("Ceramic Mug", "$24.99"); }

@test "a shot renders its markup and runs its motion" needs timing {
  @fixture { <div class="panel"></div> }

  // Let the factory instantiate + the reveal motion play.
  @wait 300ms
  @eval (window.__card = document.querySelector('.panel .card'))
  @eval (window.__nm = window.__card ? window.__card.querySelector('.nm').textContent : '')
  @eval (window.__price = window.__card ? window.__card.querySelector('.price').textContent : '')
  @eval (window.__op0 = window.__card ? Number(getComputedStyle(window.__card).opacity) : -1)
  @wait 700ms
  @eval (window.__op1 = window.__card ? Number(getComputedStyle(window.__card).opacity) : -1)

  @assert (!!window.__card)
    "the shot's :markup slot must render a .card into the DOM — a shot that registered no template renders nothing"
  @assert (window.__nm.indexOf("Ceramic Mug") >= 0)
    "the markup must interpolate the shot's params (`$title`)"
  @assert (window.__price.indexOf("$24.99") >= 0)
    "the markup must interpolate a param VALUE that begins with `$` (BUG-384) — a price `$24.99` was dropped when the bare-hole pass re-read the resolved value as a `$24` variable"
  @assert (window.__op1 > window.__op0 + 0.2)
    "the shot's :motion must drive the rendered markup (opacity rises) — motion that never bound leaves it static"
}
