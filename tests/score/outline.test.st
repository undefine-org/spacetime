/// Behavior gate: PLAN-150 W10 — `@reveal(split: outline)`, text as geometry.
/// The "text as geometry" of docs/language/film.st.md §11.
///
/// The outline reveal keeps the text (an sr-only twin stays accessible) and adds
/// an SVG `<text>` twin rendered as strokable geometry: `draw` animates the
/// stroke dash progress (a stroke travelling along the letterforms), `fill`
/// fades the real fill in. This asserts the BEHAVIOUR — the SVG twin exists and
/// its `--st-reveal-draw` custom property advances across the window (BUG-252),
/// not the emitted string.
///
/// FIDELITY: paint rung (`--cdp`); the draw needs a real clock + SVG layout.

@import "stdlib/testing/test"

h1.mark {
  font: 800 80px sans-serif;
  @reveal(split: "outline", trigger: "load", duration: 1200) {
    @on &.clip { draw: 0 -> 1; fill: 0 -> 1; }
  }
}

@test "outline reveal draws an SVG twin of the text" needs timing {
  @fixture { <h1 class="mark">DRAW</h1> }

  // Let the reveal split + begin animating.
  @wait 150ms
  @eval (window.__svgText = document.querySelector('h1.mark svg text'))
  @eval (window.__hasTwin = !!window.__svgText)
  @eval (window.__srOnly = document.querySelector('h1.mark .sr-only'))
  @eval (window.__accessible = window.__srOnly ? window.__srOnly.textContent : '')
  // `draw` drives stroke-dashoffset high -> 0 (the stroke draws on). Read the
  // inline style the reveal's rAF sets each frame.
  @eval (window.__draw0 = window.__svgText ? (parseFloat(window.__svgText.style.strokeDashoffset) || 0) : -1)
  @wait 700ms
  @eval (window.__draw1 = window.__svgText ? (parseFloat(window.__svgText.style.strokeDashoffset) || 0) : -1)

  @assert (window.__hasTwin === true)
    "the outline reveal must add an SVG <text> twin — no twin means the outline mode did not build the geometry"
  @assert (window.__accessible.indexOf("DRAW") >= 0)
    "the real text must stay accessible (an sr-only twin) — outline geometry must not replace the readable text"
  @assert (window.__draw0 - window.__draw1 > 5)
    "the twin's stroke-dashoffset must FALL across the window as `draw: 0 -> 1` plays (the stroke draws on) — a frozen offset means the keyframe never reached the stroke"
}
