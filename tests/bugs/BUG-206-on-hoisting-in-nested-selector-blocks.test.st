/// BUG-206 — @on hoisting in nested selector blocks (correctness-breaking, silent)
///
/// Symptom: with natural CSS-style nesting
///
///   .counter { > .controls { > button.inc { @on click { … } } } }
///
/// the event binding registered on the OUTER element (`.counter`) instead of the
/// innermost (`button.inc`). One click on `+` bubbled to `.counter` and fired
/// EVERY handler bound under it — so inc AND dec both ran → net zero → the button
/// appeared to do nothing. Against a real server this is double-dispatch: one
/// user gesture, two handle_event calls, two state mutations (a duplicate-
/// submission / double-charge class of bug).
///
/// Root cause: directive FormMatches sitting in a nested PLAIN (non-template)
/// selector scope got the OUTERMOST scope's selector assigned
/// (`fm.selector = scope_ast.selector`) instead of the compound path down to the
/// deepest nested scope. Text/style bindings were unaffected because they are
/// CSS declarations (the CSS pipeline composes `.counter > .controls > button.inc`
/// via `compose_selector`); only the directive-binding lowering read the hoisted
/// `form_match.selector`. The tell is exactly that text/style nesting worked.
///
/// Discriminator: two nested buttons each mutate a SEPARATE counter. Clicking
/// `+` must move ONLY `$inc_n` — `$dec_n` must stay 0. Pre-fix BOTH handlers
/// bound to `.counter`, so a single click on `+` fired dec too → `$dec_n === 1`
/// (RED). Post-fix only the inc handler is bound to `button.inc` (GREEN).
///
/// See also: tests/integration/counter.test.st (single-level, does NOT exercise
/// the nested hoist). This file is the nested-scope regression.

@import "stdlib/testing/test"

// =============================================================================
// FILE-SCOPE nested selector scopes — the production authoring form that triggers
// the hoist. (The `@given .sel { @on }` test-directive form is single-level and
// does NOT reproduce the bug; the nested `@on` MUST sit in a real nested
// `.sel { .sel { @on } }` selector tree at file scope.)
// =============================================================================

.counter-206 {
  > .controls-206 {
    > button.inc-206 {
      @on &.click {
        $inc_n <- $inc_n + 1;
      }
    }
    > button.dec-206 {
      @on &.click {
        $dec_n <- $dec_n - 1;
      }
    }
  }
}

.counter-206b {
  > .controls-206b {
    > button.inc-206b {
      @on &.click {
        $inc_nb <- $inc_nb + 1;
      }
    }
    > button.dec-206b {
      @on &.click {
        $dec_nb <- $dec_nb - 1;
      }
    }
  }
}

@test "BUG-206: nested @on binds the inner element, not the outer scope" {
  @fixture {
    <div class="counter-206">
      <div class="controls-206">
        <button class="inc-206">+</button>
        <button class="dec-206">-</button>
      </div>
    </div>
  }

  // Seed the two independent counters as global signals.
  @eval (window.SpacetimeLocal = window.SpacetimeLocal || {})
  @eval (window.SpacetimeLocal.inc_n = 0)
  @eval (window.SpacetimeLocal.dec_n = 0)

  // One click on `+`. Pre-fix this bubbles to `.counter-206` and fires dec too.
  @when .inc-206 click
  @wait 80ms

  @assert (window.SpacetimeLocal.inc_n === 1)
    "one click on + must increment exactly once (was 0 when inc+dec cancelled)"
  @assert (window.SpacetimeLocal.dec_n === 0)
    "clicking + must NOT fire the dec handler (BUG-206: was -1, dec hoisted to .counter)"
}

@test "BUG-206: nested @on on the dec button is independent of inc" {
  @fixture {
    <div class="counter-206b">
      <div class="controls-206b">
        <button class="inc-206b">+</button>
        <button class="dec-206b">-</button>
      </div>
    </div>
  }

  @eval (window.SpacetimeLocal = window.SpacetimeLocal || {})
  @eval (window.SpacetimeLocal.inc_nb = 0)
  @eval (window.SpacetimeLocal.dec_nb = 0)

  // One click on `-`. Pre-fix this also fires inc → inc_nb becomes 1 (RED).
  @when .dec-206b click
  @wait 80ms

  @assert (window.SpacetimeLocal.dec_nb === -1)
    "one click on - must decrement exactly once"
  @assert (window.SpacetimeLocal.inc_nb === 0)
    "clicking - must NOT fire the inc handler (BUG-206: was 1, inc hoisted to .counter)"
}
