@import "stdlib/testing/test"
@test "directive in fixture markup should not leak as text" {
  @fixture { <div class="x"> @scroll fade(start: 0, end: 1) { opacity: 0 -> 1; } </div> }
  @then .x { text == ""; }
}
