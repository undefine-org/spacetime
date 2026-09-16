@import "stdlib/testing/test"

// FUP-069 guard: a `@mount`-ed `@on click` handler must fire EXACTLY once per
// `@when click`. The regression bound the handler twice — once correctly via the
// macro's recompiled `%$content.js` (scoped to the mounted node) and once wrongly
// at file scope on `document.body` (an ancestor), so a single click bubbled to
// both and incremented the signal 2×. The fix drops FormMatches that live inside
// an opaque `:block` directive body (they belong to the macro's sub-program only).
@test "one @when click fires @on click exactly once" {
  @mount { <button class="b1">+</button>
    .b1 { $n number: 0; @on &.click { $n <- $n + 1; } } }
  @when .b1 click
  @then .b1 { $n == 1; }
}

@test "three @when clicks increment to exactly three" {
  @mount { <button class="b1">+</button>
    .b1 { $n number: 0; @on &.click { $n <- $n + 1; } } }
  @when .b1 click
  @when .b1 click
  @when .b1 click
  @then .b1 { $n == 3; }
}
