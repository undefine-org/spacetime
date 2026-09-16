/// Cycle Macro Tests
///
/// Tests for the @cycle macro that cycles through data source items
/// on an interval, updating element textContent to trigger
/// MutationObserver-based animations like @value-change.

// =============================================================================
// Basic Cycling
// =============================================================================

@test "cycle changes text after interval" {
  @fixture {
    <div class="cycle-text" data-cycle>first</div>
  }
  @eval (eval("ST.setData('cycleTest', ['first', 'second', 'third'])"))
  @given .cycle-text {
    @cycle(source: cycleTest, interval: 100ms)
  }
  @then .cycle-text should have_text "first"
  @wait 150ms
  @then .cycle-text should have_text "second"
}

@test "cycle wraps around to beginning" {
  @fixture {
    <div class="wrap-text" data-wrap>a</div>
  }
  @eval (eval("ST.setData('wrapTest', ['a', 'b'])"))
  @given .wrap-text {
    @cycle(source: wrapTest, interval: 80ms)
  }
  @wait 100ms
  @then .wrap-text should have_text "b"
  @wait 100ms
  @then .wrap-text should have_text "a"
}

// =============================================================================
// Object Arrays with Field Param
// =============================================================================

@test "cycle extracts field from object array" {
  @fixture {
    <div class="field-text" data-field>alpha</div>
  }
  @eval (eval("var _a = Object.create(null); _a.text = 'alpha'; var _b = Object.create(null); _b.text = 'beta'; var _c = Object.create(null); _c.text = 'gamma'; ST.setData('fieldTest', [_a, _b, _c])"))
  @given .field-text {
    @cycle(source: fieldTest, interval: 100ms, field: "text")
  }
  @wait 150ms
  @then .field-text should have_text "beta"
}

// =============================================================================
// Empty / Missing Data
// =============================================================================

@test "cycle does nothing with empty data" {
  @fixture {
    <div class="empty-text" data-empty>unchanged</div>
  }
  @eval (eval("ST.setData('emptyTest', [])"))
  @given .empty-text {
    @cycle(source: emptyTest, interval: 80ms)
  }
  @wait 150ms
  @then .empty-text should have_text "unchanged"
}

// =============================================================================
// MutationObserver Compatibility
// =============================================================================

@test "cycle triggers MutationObserver (value-change compat)" {
  @fixture {
    <div class="compat-text" data-compat>first</div>
  }
  @eval (eval("ST.setData('compatTest', ['first', 'second'])"))
  @eval (window.__mutationFired = false)
  @eval (eval("var __obs_opts = {}; __obs_opts['childList'] = true; __obs_opts['characterData'] = true; __obs_opts['subtree'] = true; new MutationObserver(function() { window.__mutationFired = true; }).observe(document.querySelector('.compat-text'), __obs_opts)"))
  @given .compat-text {
    @cycle(source: compatTest, interval: 100ms)
  }
  @wait 150ms
  @assert (window.__mutationFired === true) "textContent mutation should fire MutationObserver"
}
