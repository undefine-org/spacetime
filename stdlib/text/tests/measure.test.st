/// Text Module Tests: @measure (pretext-backed measurement)
///
/// NOTE: these .test.st assertions do not yet GATE — blocked on BUG-051
/// (:block body capture emits the test body as a dead string literal, so nested
/// directives never execute). Tracked by PLAN-026. Until fixed, the pretext
/// fusion is verified by tests/text_module_test.rs at the emit level. These files
/// gate for real once BUG-051 lands.
///
/// Runs in real Chromium via the playwright_st harness (pretext needs Canvas
/// measureText + Intl.Segmenter, absent in the headless V8 runtime).
///
/// Verifies that importing the text module ships the vendored pretext engine and
/// that @measure yields a plausible line count for a fixed-width element.

@import "stdlib/testing/test"
@import "stdlib/text"

// =============================================================================
// Vendored engine availability (demand-driven emission)
// =============================================================================

@test "pretext global is injected when @measure is used" {
  @fixture {
    <div class="m-avail" style="width: 200px; font: 16px/1.5 sans-serif">
      @measure(lineHeight: 1.5)
    </div>
  }
  @assert (typeof pretext === 'object' && typeof pretext.layout === 'function')
    "vendored pretext engine should be present on the page"
}

// =============================================================================
// Measurement yields line count / height signals
// =============================================================================

@test "@measure yields a line count for wrapped text" {
  @fixture {
    <div class="m-wrap" style="width: 120px; font: 16px/1.5 sans-serif">
      The quick brown fox jumps over the lazy dog
      @measure(lineHeight: 1.5)
    </div>
  }

  @let (el = document.querySelector('.m-wrap'))
  // pretext measures arithmetically — give layout a tick to settle.
  @eval (eval("window.__lines = ST.get(el, 'lines')"))
  @assert (typeof __lines === 'number' && __lines >= 2)
    "wrapped text at 120px should occupy multiple lines"
}

@test "@measure single short line yields one line" {
  @fixture {
    <div class="m-one" style="width: 600px; font: 16px/1.5 sans-serif">
      Short
      @measure(lineHeight: 1.5)
    </div>
  }

  @let (el = document.querySelector('.m-one'))
  @eval (eval("window.__one = ST.get(el, 'lines')"))
  @assert (__one === 1)
    "a short string in a wide box should be exactly one line"
}

@test "@measure exposes a positive height" {
  @fixture {
    <div class="m-height" style="width: 200px; font: 16px/1.5 sans-serif">
      Some measurable copy that wraps a little
      @measure(lineHeight: 1.5)
    </div>
  }

  @let (el = document.querySelector('.m-height'))
  @eval (eval("window.__h = ST.get(el, 'height')"))
  @assert (typeof __h === 'number' && __h > 0)
    "laid-out height should be a positive number"
}
