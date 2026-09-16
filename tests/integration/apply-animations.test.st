/// Regression fixture: scoped `@on &.visible` animation heads compile through
/// the test runner's same migration-aware pipeline as site sources.

.hero {
  @on &.visible(2200ms, delay: 100ms) as $h-lines {
    .hero__line--h1 {
      opacity: 0 -> 0.35;
    }
  }
}

// The former timeline assertions were deleted: no test drove these elements,
// so they asserted a driver-progress signal rather than observable behavior.
