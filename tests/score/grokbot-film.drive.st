/// Run against `spacetime serve demos/film-grokbot/ -p 4549`.
/// The real authored film, not a copied fixture. Shared @drive vocabulary.
@import "stdlib/testing/test";
@import "stdlib/testing/automation";

@drive "http://127.0.0.1:4549/" {
  @wait 600ms
  @assert (!!ST._transports.film) "the reconstructed film exposes its named Studio transport"
  @assert (document.querySelectorAll('.film__canvas > audio').length === 1) "the reference soundtrack is one native score entity"
  @wait_until (document.querySelector('.film__canvas > audio').readyState > 0 || document.querySelector('.film__canvas > audio').error) timeout: 10s
  @assert (!document.querySelector('.film__canvas > audio').error) "the real soundtrack decodes in Chromium"
  @assert (Math.abs(document.querySelector('.film__canvas > audio').duration - 79.1) < 0.02) "the real soundtrack asset loads at the reference duration"
  @eval (ST._transports.film.pause(), ST._transports.film.seek(6000))
  @wait 100ms
  @assert (Math.abs(document.querySelector('.film__canvas > audio')._stSeekTarget - 6) < 0.001 && document.querySelector('.film__canvas > audio').paused) "the real recording seeks with the paused picture"
  @assert (parseFloat(getComputedStyle(document.querySelector('.introduction')).opacity) > 0.99) "the introduction owns its reference window"
  @assert (document.querySelectorAll('.scene').length === 24) "all reference beats are assembled as individually editable scenes"
  @assert (document.querySelectorAll('[class^="cast-"] .bot').length === 5) "the introduction uses the shared character anatomy"
  @eval (ST._transports.film.seek(15000))
  @wait 100ms
  @eval (window.__photoA = document.querySelector('.meeting .filmstrip img').getBoundingClientRect().top)
  @eval (ST._transports.film.seek(17500))
  @wait 100ms
  @assert (Math.abs(document.querySelector('.meeting .filmstrip img').getBoundingClientRect().top - window.__photoA) > 200) "photographic meeting frames advance on the film transport"
  @assert (Math.abs((document.querySelector(".meeting .filmstrip img").getBoundingClientRect().width / 10) / (document.querySelector(".meeting .filmstrip img").getBoundingClientRect().height / 12) - 766/398) < 0.001) "portrait and landscape meeting tiles preserve the photographic aspect ratio"
  @eval (ST._transports.film.seek(25000))
  @wait 100ms
  @eval (window.__chartA = document.querySelector('.feedback__marker').getBoundingClientRect().left)
  @eval (ST._transports.film.seek(26500))
  @wait 100ms
  @assert (Math.abs(document.querySelector('.feedback__marker').getBoundingClientRect().left - window.__chartA) > 5) "the chart pointer moves through the authored curve"
  @assert (parseFloat(getComputedStyle(document.querySelector('.meeting')).opacity) === 0) "a finished meeting cannot double-expose the chart"
  @eval (ST._transports.film.seek(43500))
  @wait 100ms
  @assert (parseFloat(getComputedStyle(document.querySelector('.design-canvas')).opacity) > 0.99) "the design editor owns its window"
  @assert (document.querySelectorAll('.design-canvas__selection i').length === 4) "the editable selection is real layout geometry"
  @eval (ST._transports.film.seek(77000))
  @wait 100ms
  @assert (parseFloat(getComputedStyle(document.querySelector('.wordmark')).opacity) > 0.99) "the final wordmark is visible at 77 seconds"
  @eval (ST._transports.film.seek(6000))
  @wait 100ms
  @assert (parseFloat(getComputedStyle(document.querySelector('.introduction')).opacity) > 0.99) "reverse seeking restores the introduction"
  @assert (parseFloat(getComputedStyle(document.querySelector('.wordmark')).opacity) === 0) "reverse seeking hides the later wordmark"
}
