/// Third test file - no fade-in here
/// This file won't be affected by the migration

.modal {
  @slide-in(300ms) {
    from: "left";
    distance: 100px;
  }
}

.tooltip {
  @pop-in(150ms) {
    scale: 0.8;
  }
}
