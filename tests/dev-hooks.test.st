/// Unit Tests: Dev Hooks & Debug Runtime
///
/// Tests that window.__stDebugHook and window.__ST_DEBUG__ are properly
/// defined and functional. Verifies event buffering and replay.

@import "stdlib/testing/test"

// =============================================================================
// __stDebugHook existence
// =============================================================================

@test "__stDebugHook is defined on window" {
  @fixture {
    <div class="hook-test">Hook Test</div>
  }

  @assert (typeof window.__stDebugHook === 'object')
    "__stDebugHook should be an object"
  @assert (typeof window.__stDebugHook.registerTimeline === 'function')
    "registerTimeline should be a function"
  @assert (typeof window.__stDebugHook.state === 'function')
    "state hook should be a function"
  @assert (typeof window.__stDebugHook.signal === 'function')
    "signal hook should be a function"
  @assert (typeof window.__stDebugHook.registerStateMachine === 'function')
    "registerStateMachine should be a function"
}

// =============================================================================
// __ST_DEBUG__ existence
// =============================================================================

@test "__ST_DEBUG__ is defined on window" {
  @fixture {
    <div class="debug-test">Debug Test</div>
  }

  @assert (typeof window.__ST_DEBUG__ === 'object')
    "__ST_DEBUG__ should be an object"
  @assert (typeof window.__ST_DEBUG__.getTimelines === 'function')
    "getTimelines should be a function"
  @assert (typeof window.__ST_DEBUG__.getErrors === 'function')
    "getErrors should be a function"
  @assert (typeof window.__ST_DEBUG__.registerTimeline === 'function')
    "registerTimeline should be a function"
  @assert (typeof window.__ST_DEBUG__.updateProgress === 'function')
    "updateProgress should be a function"
  @assert (typeof window.__ST_DEBUG__.recordError === 'function')
    "recordError should be a function"
  @assert (typeof window.__ST_DEBUG__.subscribe === 'function')
    "subscribe should be a function"
  @assert (typeof window.__ST_DEBUG__.getSnapshot === 'function')
    "getSnapshot should be a function"
  @assert (typeof window.__ST_DEBUG__.clearErrors === 'function')
    "clearErrors should be a function"
  @assert (typeof window.__ST_DEBUG__.getHistory === 'function')
    "getHistory should be a function"
  @assert (typeof window.__ST_DEBUG__.clearHistory === 'function')
    "clearHistory should be a function"
  @assert (typeof window.__ST_DEBUG__.exportJSON === 'function')
    "exportJSON should be a function"
}

// =============================================================================
// registerTimeline via hooks → getTimelines returns it
// =============================================================================

@test "registerTimeline via hook appears in getTimelines" {
  @fixture {
    <div class="reg-test">Register Test</div>
  }

  @eval (window.__stDebugHook.registerTimeline('test-tl-1', { type: 'scroll', scope: null }))

  @eval (eval("window.__tlResult = window.__ST_DEBUG__.getTimelines()"))

  @assert (Array.isArray(window.__tlResult))
    "getTimelines should return an array"
  @assert (window.__tlResult.length > 0)
    "Should have at least one timeline"
  @eval (eval("window.__foundTl = window.__tlResult.find(function(t) { return t.id === 'test-tl-1' })"))
  @assert (window.__foundTl !== undefined)
    "Should find the registered timeline"
  @assert (window.__foundTl.type === 'scroll')
    "Timeline type should match"
}

// =============================================================================
// recordError → getErrors returns it
// =============================================================================

@test "recordError → getErrors returns it" {
  @fixture {
    <div class="err-test">Error Test</div>
  }

  // Clear any previous errors
  @eval (window.__ST_DEBUG__.clearErrors())

  @eval (window.__ST_DEBUG__.recordError('test-source', new Error('test error'), { detail: 'test' }))

  @eval (eval("window.__errResult = window.__ST_DEBUG__.getErrors()"))

  @assert (Array.isArray(window.__errResult))
    "getErrors should return an array"
  @assert (window.__errResult.length === 1)
    "Should have exactly one error"
  @assert (window.__errResult[0].source === 'test-source')
    "Error source should match"
  @assert (window.__errResult[0].message === 'test error')
    "Error message should match"
}

// =============================================================================
// Buffer/replay: hooks buffer before __ST_DEBUG__ ready
// =============================================================================

@test "hook timeline method updates progress in debug runtime" {
  @fixture {
    <div class="progress-test">Progress Test</div>
  }

  // Register a timeline and then update progress via hook
  @eval (window.__stDebugHook.registerTimeline('progress-tl', { type: 'load' }))
  @eval (window.__stDebugHook.timeline('progress-tl', 0.75))

  @eval (eval("window.__pTl = window.__ST_DEBUG__.timelines.get('progress-tl')"))

  @assert (window.__pTl !== undefined)
    "Timeline should exist in map"
  @assert (window.__pTl.progress === 0.75)
    "Progress should be updated to 0.75"
}
