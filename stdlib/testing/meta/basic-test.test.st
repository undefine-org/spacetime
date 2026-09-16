/// Basic test to verify parsing works

@test "basic assertion works" {
  @assert true "Should pass"
}

@test "expression in parens works" {
  @assert (1 + 1 === 2)
}

