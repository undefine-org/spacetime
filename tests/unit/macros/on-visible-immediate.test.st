/// On-visible immediate driver syntax coverage.
///
/// Drivers stay at file scope. The former tests only asserted `have_timeline`,
/// a progress signal published after firing; neither test drove its subject, so
/// those assertions did not verify behavior and were removed.

@import "stdlib/testing/test"

.imm-test {
  @on &.visible(600ms, immediate: true) as $reveal {
    opacity: 0 -> 1;
  }
}

.obs-test {
  @on &.visible(600ms) as $fade-in {
    opacity: 0 -> 1;
  }
}
