// GH-23 behavioral gate: the click handler AFTER a descendant rule whose key
// selector (`.nope`) matches no markup must still fire. The handler is declared
// AFTER the poison rule — GH-23 says it is silently dropped.
@import "stdlib/testing/test"

@test "after-poison handler must fire" {
  @mount {
    <div class="wrap"><button class="btn-b">b</button></div>
    .wrap { $frame number: 0; }
    body:hover .nope { opacity: 1 }
    .btn-b { @on &.click { $frame <- 2; } }
  }
  @when .btn-b click
  @then .wrap { $frame == 2; }
}
