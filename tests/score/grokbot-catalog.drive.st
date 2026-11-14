/// Live design-language catalog; serve the film on port 4549.
@import "stdlib/testing/test";
@import "stdlib/testing/automation";

@drive "http://127.0.0.1:4549/language" {
  @wait 500ms
  @assert (!!document.querySelector('.language-live .voice')) "the catalog executes the same voice form as the film"
  @eval (document.querySelector('.language-swatch--mint').click())
  @wait 100ms
  @assert (getComputedStyle(document.querySelector('.language-live .bubble--out')).backgroundColor === 'rgb(0, 200, 161)') "one typed color role reskins the real message component"
  @eval (document.querySelector('.language-swatch--orange').click())
  @wait 100ms
  @assert (getComputedStyle(document.querySelector('.language-live .bubble--out')).backgroundColor === 'rgb(255, 121, 0)') "a second edit propagates through the same component anatomy"
}
