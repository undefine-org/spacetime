@import "stdlib/testing/test"

// BUG-109 guard: a `:` in a @test name is opaque string data. If the name-string
// capture/emit ever re-truncates at the first colon again, the emitted JS string
// literal becomes unterminated and the WHOLE file fails with a V8 SyntaxError
// (0 passed / 0 failed) — so the mere fact these named tests RUN is the assertion.
@test "iso A: ST.setData drives render" {
  @assert (1 === 1) "colon-in-name compiled and ran"
}

@test "a: b: c GET /api: 200 OK" {
  @assert (2 + 2 === 4) "multi-colon name compiled and ran"
}
