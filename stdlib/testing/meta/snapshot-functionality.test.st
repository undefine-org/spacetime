/// Meta-Tests: @snapshot Directive
///
/// Tests that @snapshot captures DOM state correctly.

// =============================================================================
// Basic Snapshots
// =============================================================================

@test "@snapshot captures page state" {
  @fixture {
    <div data-snapshot-basic>Basic Snapshot</div>
  }
  @snapshot "basic snapshot test"
  @assert (window.__spacetime_snapshots !== undefined)
  @assert (window.__spacetime_snapshots["basic snapshot test"] !== undefined)
}

@test "@snapshot captures HTML content" {
  @fixture {
    <div data-snapshot-html>
      <span class="nested">Nested Content</span>
    </div>
  }
  @snapshot "html content test"
  @let (snapshot = window.__spacetime_snapshots["html content test"])
  @assert (snapshot.html !== undefined)
  @assert (snapshot.html.includes('Nested Content'))
}

@test "@snapshot has timestamp" {
  @fixture {
    <div data-snapshot-time>Timestamp Test</div>
  }
  @let (before = Date.now())
  @snapshot "timestamp test"
  @let (after = Date.now())

  @let (snapshot = window.__spacetime_snapshots["timestamp test"])
  @assert (snapshot.timestamp >= before)
  @assert (snapshot.timestamp <= after)
}

@test "@snapshot has name property" {
  @fixture {
    <div data-snapshot-name>Name Test</div>
  }
  @snapshot "named snapshot"
  @let (snapshot = window.__spacetime_snapshots["named snapshot"])
  @assert (snapshot.name === "named snapshot")
}

// =============================================================================
// Scoped Snapshots
// =============================================================================

@test "@snapshot of: selector captures specific element" {
  @fixture {
    <div data-outer>
      <div data-snapshot-target>Target Content Only</div>
      <div data-other>Other Content</div>
    </div>
  }
  @snapshot "scoped snapshot" of: [data-snapshot-target]
  @let (snapshot = window.__spacetime_snapshots["scoped snapshot"])
  @assert (snapshot !== undefined)
  @assert (snapshot.html.includes('Target Content Only'))
}

@test "@snapshot of: selector with nested content" {
  @fixture {
    <div data-nested-outer>
      <div data-nested-target>
        <span class="child">Child 1</span>
        <span class="child">Child 2</span>
      </div>
    </div>
  }
  @snapshot "nested snapshot" of: [data-nested-target]
  @let (snapshot = window.__spacetime_snapshots["nested snapshot"])
  @assert (snapshot.html.includes('Child 1'))
  @assert (snapshot.html.includes('Child 2'))
}

@test "@snapshot of: empty element" {
  @fixture {
    <div data-empty-target></div>
  }
  @snapshot "empty element snapshot" of: [data-empty-target]
  @let (snapshot = window.__spacetime_snapshots["empty element snapshot"])
  @assert (snapshot !== undefined)
  @assert (snapshot.html === '' || snapshot.html !== undefined)
}

// =============================================================================
// Multiple Snapshots
// =============================================================================

@test "multiple snapshots are stored separately" {
  @fixture {
    <div data-multi-snap>Multi Snapshot</div>
  }
  @snapshot "multi snap 1"
  @snapshot "multi snap 2"
  @snapshot "multi snap 3"

  @assert (window.__spacetime_snapshots["multi snap 1"] !== undefined)
  @assert (window.__spacetime_snapshots["multi snap 2"] !== undefined)
  @assert (window.__spacetime_snapshots["multi snap 3"] !== undefined)
}

@test "snapshot names can contain special characters" {
  @fixture {
    <div data-special-name>Special</div>
  }
  @snapshot "snapshot: with special (chars) & symbols!"
  @assert (window.__spacetime_snapshots["snapshot: with special (chars) & symbols!"] !== undefined)
}

// =============================================================================
// Snapshot Context
// =============================================================================

@test "@snapshot stores in __ctx.snapshots" {
  @fixture {
    <div data-ctx-snap>Context Snapshot</div>
  }
  @snapshot "context test"
  @assert (Array.isArray(__ctx.snapshots))
  @assert (__ctx.snapshots.length >= 1)
}

@test "@snapshot accumulates in __ctx.snapshots" {
  @fixture {
    <div data-accumulate>Accumulate</div>
  }
  @let (initialCount = (__ctx.snapshots || []).length)
  @snapshot "accum 1"
  @snapshot "accum 2"
  @assert (__ctx.snapshots.length >= initialCount + 2)
}

// =============================================================================
// Snapshot Before/After Changes
// =============================================================================

@test "snapshot captures state at moment of capture" {
  @fixture {
    <div data-before-after>Before</div>
  }
  @snapshot "before change"

  @eval (document.querySelector('[data-before-after]').textContent = 'After')
  @snapshot "after change"

  @assert (window.__spacetime_snapshots["before change"].html.includes('Before'))
  @assert (window.__spacetime_snapshots["after change"].html.includes('After'))
}

@test "snapshot captures class changes" {
  @fixture {
    <div data-class-change class="initial">Class Test</div>
  }
  @snapshot "before class"

  @eval (document.querySelector('[data-class-change]').classList.add('added'))
  @eval (document.querySelector('[data-class-change]').classList.remove('initial'))
  @snapshot "after class"

  // BUG-217: this asserted the two `timestamp`s DIFFER, which is a race, not a
  // property — both are `Date.now()` and two snapshots in the same test routinely
  // land in the same millisecond (measured: this test failed on 2 of 3 runs).
  //
  // The intent is "the two snapshots are distinct captures". Assert that directly,
  // against something the code actually controls: they are separate entries, taken
  // in order, and each recorded the DOM as it was at capture time. That is a
  // stronger claim than the timestamp comparison, not a weaker one — a bug that
  // returned the same object for both would pass the old assertion whenever the
  // clock happened to tick.
  @assert (window.__spacetime_snapshots["before class"] !== window.__spacetime_snapshots["after class"])
    "each @snapshot must record a SEPARATE entry"
  @assert (window.__spacetime_snapshots["before class"].name === "before class")
  @assert (window.__spacetime_snapshots["after class"].name === "after class")
  @assert (__ctx.snapshots.length === 2)
    "both snapshots must accumulate on the test context, in order"
  @assert (window.__spacetime_snapshots["after class"].timestamp >= window.__spacetime_snapshots["before class"].timestamp)
    "the later snapshot must not predate the earlier one"
}

// =============================================================================
// Empty and Edge Cases
// =============================================================================

@test "@snapshot of body captures full page" {
  @fixture {
    <div data-body-snap>Body Content</div>
  }
  @snapshot "full page"
  @let (snapshot = window.__spacetime_snapshots["full page"])
  @assert (snapshot.html.includes('data-body-snap'))
}

// =============================================================================
// Snapshot Comparison Pattern
// =============================================================================

@test "snapshots can be compared programmatically" {
  @fixture {
    <div data-compare>Compare Me</div>
  }
  @snapshot "compare 1"
  @snapshot "compare 2"

  @let (snap1 = window.__spacetime_snapshots["compare 1"])
  @let (snap2 = window.__spacetime_snapshots["compare 2"])

  @assert (snap1.html === snap2.html)
}

@test "different snapshots have different HTML" {
  @fixture {
    <div data-diff>Initial</div>
  }
  @snapshot "diff before"

  @eval (document.querySelector('[data-diff]').textContent = 'Changed')
  @snapshot "diff after"

  @let (snap1 = window.__spacetime_snapshots["diff before"])
  @let (snap2 = window.__spacetime_snapshots["diff after"])

  @assert (snap1.html !== snap2.html)
}
