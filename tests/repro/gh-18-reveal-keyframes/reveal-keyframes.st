.probe18 {
  @reveal(split: "words", trigger: visible, stagger: 90, duration: 800, easing: ease-out) {
    & { filter: blur(18px) -> blur(0); opacity: 0.25 -> 1; }
  }
}
