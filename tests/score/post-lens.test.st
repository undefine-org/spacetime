/// Behavior gate: PLAN-150 W2 — `@post` puts a real, animatable lens over a DOM
/// element. The film-surface post-processing of docs/language/film.st.md §3.
///
/// Two claims, both observable in the browser (BUG-252: assert the pixels, not
/// the emitted JS):
///   1. `@post { … }` injects the three lens layers as children of the host,
///      each reading its `--st-post-*` custom property.
///   2. Animating `--st-post-bloom` in `@on &.clip` DRIVES the bloom layer's
///      opacity — the whole point of routing the look through a custom property.
///
/// NB `--cdp` under the virtual clock: a computed-opacity-at-time claim is
/// timing-dependent; `--headless` refuses by design.

@import "stdlib/testing/test"

// The score lives on a parent; the clip consumer on the child it drives — the
// standard shape (a `@score`/`@on &.clip` pair on the SAME element does not
// join). The `@post` lens sits on the same child the clip animates.
.film {
  @score &.time(10s) as &reel { &stage at 0s for 10s; }
}
.stage {
  width: 400px; height: 300px; position: relative;
  @post { bloom: 0.2; grain: 0.03; vignette: 0.5; }
  // ramp bloom up across the window; the layer's opacity is bloom * 0.6.
  @on &.clip { --st-post-bloom: 0.2 -> 1.5; }
}

@test "post: the lens layers mount and read their custom properties" needs timing {
  @fixture { <div class="film"><div class="stage">content</div></div> }

  @wait 100ms

  @eval (window.__pl = {
    bloom: !!document.querySelector('.stage > .st-post-bloom'),
    grain: !!document.querySelector('.stage > .st-post-grain'),
    vignette: !!document.querySelector('.stage > .st-post-vignette'),
    vigOpacity: parseFloat(getComputedStyle(document.querySelector('.stage > .st-post-vignette')).opacity),
  })

  @assert (window.__pl.bloom && window.__pl.grain && window.__pl.vignette)
    "all three lens layers (bloom, grain, vignette) must be injected as children of the host"
  @assert (window.__pl.vigOpacity > 0.4 && window.__pl.vigOpacity < 0.6)
    "the vignette layer reads --st-post-vignette (seeded 0.5) as its opacity — a value proves the layer is wired to the property, not a baked constant"
}

@test "post: animating --st-post-bloom drives the bloom layer's opacity" needs timing {
  @fixture { <div class="film"><div class="stage">content</div></div> }

  @eval (window.__pb = [], window.__pbt = setInterval(function () {
    var l = document.querySelector('.stage > .st-post-bloom');
    window.__pb.push(parseFloat(getComputedStyle(l).opacity));
  }, 250))

  @wait 5200ms
  @eval (clearInterval(window.__pbt))
  @eval (window.__pbFirst = window.__pb[0])
  @eval (window.__pbLast = window.__pb[window.__pb.length - 1])
  @eval (window.__pbMax = Math.max.apply(null, window.__pb))

  @assert (window.__pbLast > window.__pbFirst + 0.1)
    "the bloom layer's opacity must RISE as --st-post-bloom ramps 0.2 -> 1.5 across the window — a flat opacity means the custom property never reached the layer"
  @assert (window.__pbMax > 0.3)
    "at the top of the ramp the bloom opacity (bloom * 0.6) clears 0.3 — proof the animated property, not the seed, drives the layer"
}
