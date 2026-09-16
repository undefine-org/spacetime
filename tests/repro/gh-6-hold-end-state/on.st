.curtain {
  @on &.visible(300ms) as $reveal {
    opacity: 0 -> 1;
  }
}
