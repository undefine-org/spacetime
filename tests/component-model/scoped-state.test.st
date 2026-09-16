/// Headless Tests: Component Model — Scoped State (CMHT-001)
///
/// Tests that scoped state ($var type: initial;) initializes correctly,
/// stays isolated between elements, mutates via @on click, and supports
/// multiple independent variables on the same element.

@import "stdlib/testing/test"

// =============================================================================
// State Initialization
// =============================================================================

@test "scoped state initializes to declared value" {
  @fixture { <div class="counter-el"></div> }
  @given .counter-el {
    $count number: 0;
  }
  // Scoped-state `$x: v` initializes on the body-local plane (window._localState),
  // resolved via ST.resolve (element-signal ancestors -> SpacetimeLocal). The async
  // selector-init runs on a microtask, so flush before reading.
  @flush
  @assert (ST.resolve(document.querySelector('.counter-el'), 'count') === 0)
    "count should initialize to 0"
}

// =============================================================================
// State Isolation
// -----------------------------------------------------------------------------
// File-scope selector state (`.sel { $x: v }`) lowers to the BODY-LOCAL plane
// (window._localState), keyed by VARIABLE NAME — not per-element. So two
// distinct selectors that declare DIFFERENT names hold independent values; two
// elements sharing one selector+name share one value (the body-local model,
// identical to a real page's compilation). Per-ELEMENT isolation of the SAME
// name is a `@template`-instance feature — pinned in template-refs.test.st
// ("multiple template instances have independent state"), where each instance
// gets its own element-signal store.
// =============================================================================

@test "distinct-named scoped state stays independent" {
  @fixture {
    <div class="el-a"></div>
    <div class="el-b"></div>
  }
  @given .el-a { $countA number: 5; }
  @given .el-b { $countB number: 10; }
  @flush
  @assert (ST.resolve(document.querySelector('.el-a'), 'countA') === 5)
    "el-a countA should be 5"
  @assert (ST.resolve(document.querySelector('.el-b'), 'countB') === 10)
    "el-b countB should be 10"
}

// =============================================================================
// State Mutation via @on click
// =============================================================================

@test "state updates on click event" {
  @fixture { <button class="btn-toggle">Click</button> }
  @given .btn-toggle {
    $active bool: false;
    @on &.click { $active <- !$active; }
  }
  @flush
  @assert (ST.resolve(document.querySelector('.btn-toggle'), 'active') === false)
    "active should start false"
  @when .btn-toggle click
  @wait 50ms
  @assert (ST.resolve(document.querySelector('.btn-toggle'), 'active') === true)
    "active should be true after click"
}

// =============================================================================
// Multiple State Variables
// =============================================================================

@test "element can have multiple independent state variables" {
  @fixture { <div class="multi-state"></div> }
  @given .multi-state {
    $open bool: false;
    $count number: 42;
  }
  @flush
  @assert (ST.resolve(document.querySelector('.multi-state'), 'open') === false)
    "open should initialize to false"
  @assert (ST.resolve(document.querySelector('.multi-state'), 'count') === 42)
    "count should initialize to 42"
}
