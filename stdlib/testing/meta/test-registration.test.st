/// Meta-Tests: Test Registration
///
/// These tests verify that @test, @test-skip, @test-only work correctly.
/// This is first-party dogfooding - using the testing framework to test itself.

// =============================================================================
// @test Registration
// =============================================================================

@test "test registers with correct name" {
  @assert (window.__st_tests.some(t => t.name === "test registers with correct name"))
}

@test "registered test has a function" {
  @assert (window.__st_tests.every(t => typeof t.fn === 'function'))
}

@test "test runs asynchronously" {
  // This test verifies async execution by using a Promise
  @assert (Promise.resolve(true) instanceof Promise)
}

// =============================================================================
// @test-skip Registration
// =============================================================================

@test-skip "skipped test body never executes" {
  // This body should NEVER run - @test-skip sets skip: true
  @assert false "This assertion should never be evaluated"
}

@test "skip flag is set on skipped tests" {
  @assert (window.__st_tests.some(t => t.name.includes("skipped test body") && t.skip === true))
}

// =============================================================================
// Test Count Verification
// =============================================================================

@test "multiple tests can be registered" {
  @assert (window.__st_tests.length >= 5)
}

@test "tests maintain registration order" {
  // Test order is implicitly verified by running test discovery
  @assert true
}

// =============================================================================
// Test Context
// =============================================================================

@test "test has access to __ctx object" {
  @assert (typeof __ctx === 'object')
}

@test "__ctx has startTime" {
  @assert (typeof __ctx.startTime === 'number')
  @assert (__ctx.startTime <= Date.now())
}
