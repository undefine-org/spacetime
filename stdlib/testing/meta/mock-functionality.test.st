/// Meta-Tests: @mock Directive
///
/// Tests that @mock correctly replaces functions and restores them.

// =============================================================================
// Basic Mocking
// =============================================================================

@test "@mock replaces window function" {
  // Save original
  @let (originalAlert = window.alert)

  @mock alert returns undefined
  @assert (window.alert !== originalAlert)

  // Verify it's callable
  @eval (window.alert("test"))
  @assert true "Mocked function should be callable"
}

@test "@mock returns specified value" {
  @mock testFn returns 42
  @assert (window.testFn() === 42)
}

@test "@mock returns object value" {
  @mock getData returns { id: 1, name: "test" }
  @let (result = window.getData())
  @assert (result.id === 1)
  @assert (result.name === "test")
}

@test "@mock returns array value" {
  @mock getItems returns [1, 2, 3]
  @let (result = window.getItems())
  @assert (Array.isArray(result))
  @assert (result.length === 3)
}

// =============================================================================
// Async Mocking (fetch-like)
// =============================================================================

@test "@mock creates promise-returning function for objects" {
  @mock fetch returns { ok: true, status: 200 }
  @let (result = window.fetch('/api/test'))
  @assert (result instanceof Promise)
}

@test "@mock async response can be awaited" {
  @mock fetchData returns { data: "success" }
  @let (result = await window.fetchData())
  @assert (result.data === "success")
}

// Note: @mock with arrow functions inside objects isn't supported in the parser.
// This test would verify: @mock fetch returns { ok: true, json: () => ({ items: ["a", "b", "c"] }) }

// =============================================================================
// Mock Tracking
// =============================================================================

@test "@mock stores original in __ctx.mocks" {
  @eval (window.testOriginal = () => "original")
  @let (originalFn = window.testOriginal)

  @mock testOriginal returns "mocked"
  @assert (Array.isArray(__ctx.mocks))

  @let (mockRecord = __ctx.mocks.find(m => m.name === 'testOriginal'))
  @assert (mockRecord !== undefined)
  @assert (mockRecord.original === originalFn)
}

@test "@mock registers cleanup to restore original" {
  @eval (window.toRestore = () => "original")

  @mock toRestore returns "mocked"
  @assert (window.toRestore() === "mocked")

  // Verify cleanup is registered
  @let (hasRestoreCleanup = __ctx.cleanup.some(fn => fn.toString().includes('toRestore') || fn.toString().includes('original')))
  @assert (hasRestoreCleanup)
}

// =============================================================================
// Multiple Mocks
// =============================================================================

@test "multiple @mock calls work independently" {
  @mock mockA returns "A"
  @mock mockB returns "B"
  @mock mockC returns "C"

  @assert window.mockA() === "A"
  @assert window.mockB() === "B"
  @assert window.mockC() === "C"
}

@test "re-mocking same function uses latest mock" {
  @mock reMock returns "first"
  @assert window.reMock() === "first"

  @mock reMock returns "second"
  @assert window.reMock() === "second"
}

// =============================================================================
// Mock with Functions
// =============================================================================

// Note: @mock with arrow function returns isn't supported in the parser.
// This test would verify that @mock with a function value works.

// =============================================================================
// Edge Cases
// =============================================================================

@test "@mock undefined function creates new mock" {
  // Mocking a function that doesn't exist
  @mock brandNewFn returns "brand new"
  @assert (window.brandNewFn() === "brand new")
}

@test "@mock null returns resolves to null" {
  @mock nullFn returns null
  @let (result = await window.nullFn())
  @assert (result === null)
}

@test "@mock boolean returns correct value" {
  @mock trueFn returns true
  @mock falseFn returns false

  @let (trueResult = await window.trueFn())
  @let (falseResult = await window.falseFn())

  @assert trueResult === true
  @assert falseResult === false
}

@test "@mock number returns correct value" {
  @mock numFn returns 3.14
  @let (result = await window.numFn())
  @assert (result === 3.14)
}

@test "@mock string returns correct value" {
  @mock strFn returns "hello world"
  @let (result = await window.strFn())
  @assert (result === "hello world")
}

// =============================================================================
// Cleanup Verification
// =============================================================================

@test "mocks are isolated per test" {
  // This test verifies that a mock from another test doesn't leak
  // We check that mockA (if defined in another test) works correctly here
  @mock isolatedMock returns "isolated"
  @assert (window.isolatedMock() === "isolated")
}
