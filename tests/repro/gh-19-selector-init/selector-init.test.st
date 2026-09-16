@import "stdlib/testing/test"

.probe19 {
  @reveal(split: "words", trigger: visible, stagger: 90, duration: 1100, easing: ease-out)
}

@test "selector-scoped reveal splits text after explicit init + flush + wait" {
  @fixture {
    <h1 class="probe19">RAMI OPTIC</h1>
  }
  @eval (ST.initElement(document.querySelector('.probe19')))
  @flush
  @wait 80ms
  @eval (window.__n19 = document.querySelectorAll('.probe19 .st-reveal-word').length)
  @assert (window.__n19 === 2) "expected 2 word units (reveal-engine splitText must have run)"
}
