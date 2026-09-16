@import "stdlib/testing/test"

.probe19b {
  @reveal(split: "words", trigger: visible, stagger: 90, duration: 1100, easing: ease-out)
}

@test "selector-scoped reveal auto-inits from fixture mount (no manual initElement)" {
  @fixture {
    <h1 class="probe19b">RAMI OPTIC</h1>
  }
  @wait 120ms
  @eval (window.__n19b = document.querySelectorAll('.probe19b .st-reveal-word').length)
  @assert (window.__n19b === 2) "expected 2 word units (MutationObserver must have driven init)"
}
