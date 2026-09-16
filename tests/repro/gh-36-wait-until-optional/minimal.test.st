// GH-36 minimal variants — isolate whether @wait_until full form matches at all.
@import "stdlib/testing/test"

// (1) trivial full form, always-true condition, explicit timeout literal
@test "trivial full" {
  @wait_until true timeout: 500ms
  @assert (true)
}

// (2) bare form
@test "trivial bare" {
  @wait_until true
  @assert (true)
}

// (3) condition with parens (maybe expr needs wrapping)
@test "paren full" {
  @wait_until (true) timeout: 500ms
  @assert (true)
}
