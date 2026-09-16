/// On Click Boolean Toggle Tests
///
/// Tests for @on click { $var <- !$var } — boolean negation mutation.
/// This is the pattern used by dark mode toggles and similar UI switches.
/// Verifies that $variable <- !$variable correctly toggles boolean state
/// and that reactive class toggle (.cls: $cond) reacts to the state change.

@import "stdlib/testing/test"

// =============================================================================
// Basic boolean toggle: $var <- !$var
// =============================================================================

@test "@on click toggles boolean variable from false to true" {
  @fixture {
    <button class="toggle-btn-1">Toggle</button>
  }
  @given .toggle-btn-1 {
    @on &.click {
      $toggleState <- !$toggleState;
    }
  }

  @eval (window.SpacetimeLocal = window.SpacetimeLocal || {})
  @eval (window.SpacetimeLocal.toggleState = false)

  // Click should flip false -> true
  @when .toggle-btn-1 click
  @wait 50ms
  @assert (window.SpacetimeLocal.toggleState === true)
    "clicking should toggle false to true"
}

@test "@on click toggles boolean variable from true to false" {
  @fixture {
    <button class="toggle-btn-2">Toggle</button>
  }
  @given .toggle-btn-2 {
    @on &.click {
      $toggleBack <- !$toggleBack;
    }
  }

  @eval (window.SpacetimeLocal = window.SpacetimeLocal || {})
  @eval (window.SpacetimeLocal.toggleBack = true)

  // Click should flip true -> false
  @when .toggle-btn-2 click
  @wait 50ms
  @assert (window.SpacetimeLocal.toggleBack === false)
    "clicking should toggle true to false"
}

@test "@on click double-toggle returns to original value" {
  @fixture {
    <button class="toggle-btn-3">Toggle</button>
  }
  @given .toggle-btn-3 {
    @on &.click {
      $doubleToggle <- !$doubleToggle;
    }
  }

  @eval (window.SpacetimeLocal = window.SpacetimeLocal || {})
  @eval (window.SpacetimeLocal.doubleToggle = false)

  // Click twice: false -> true -> false
  @when .toggle-btn-3 click
  @wait 50ms
  @when .toggle-btn-3 click
  @wait 50ms
  @assert (window.SpacetimeLocal.doubleToggle === false)
    "double-clicking should return to original false"
}

// =============================================================================
// reactive class toggle (.cls: $cond) reactive to boolean toggle
// =============================================================================

// Reactive bindings are FILE-SCOPE scopes (the production authoring form: a page
// writes `.sel { .cls: $sig }` at top level, not nested in a test directive). The
// binding listener must exist before the @when click fires; file scope guarantees
// that. (The old `@given .target { .is-active: $sig }` form dropped the binding —
// @given recompiles a directive body, not a CSS scope — see BUG-119 §1.)
.bind-target { .is-active: $bindTest; }

@test "reactive class toggle (.cls: $cond) adds class when boolean toggled to true" {
  @fixture {
    <button class="bind-toggle-btn">Toggle</button>
    <div class="bind-target">Target</div>
  }
  @given .bind-toggle-btn {
    @on &.click {
      $bindTest <- !$bindTest;
    }
  }

  @eval (window.SpacetimeLocal = window.SpacetimeLocal || {})
  @eval (window.SpacetimeLocal.bindTest = false)

  @assert (!document.querySelector('.bind-target').classList.contains('is-active'))
    "class absent before click"
  @when .bind-toggle-btn click
  @wait 100ms
  @assert (document.querySelector('.bind-target').classList.contains('is-active'))
    "reactive class toggle (.cls: $cond) should add class when $bindTest becomes true"
}

.unbind-target { .is-active: $unbindTest; }

@test "reactive class toggle (.cls: $cond) removes class when boolean toggled to false" {
  @fixture {
    <button class="unbind-toggle-btn">Toggle</button>
    <div class="unbind-target">Target</div>
  }
  @given .unbind-toggle-btn {
    @on &.click {
      $unbindTest <- !$unbindTest;
    }
  }

  @eval (window.SpacetimeLocal = window.SpacetimeLocal || {})
  @eval (window.SpacetimeLocal.unbindTest = true)

  // Binding reflects the seeded true (class present), then toggle off.
  @when .unbind-toggle-btn click
  @wait 100ms
  @assert (!document.querySelector('.unbind-target').classList.contains('is-active'))
    "reactive class toggle (.cls: $cond) should remove class when $unbindTest becomes false"
}

// =============================================================================
// body-scoped $darkMode pattern (mirrors ora-ventures)
// =============================================================================

.dark-target { .is-dark: $darkMode; }

@test "body-scoped variable toggle via @on click works" {
  @fixture {
    <button class="dark-toggle">◐</button>
    <div class="dark-target">Content</div>
  }
  @given .dark-toggle {
    @on &.click {
      $darkMode <- !$darkMode;
    }
  }

  @eval (window.SpacetimeLocal = window.SpacetimeLocal || {})
  @eval (window.SpacetimeLocal.darkMode = false)

  @when .dark-toggle click
  @wait 100ms
  @assert (window.SpacetimeLocal.darkMode === true)
    "$darkMode should be true after click"
  @assert (document.querySelector('.dark-target').classList.contains('is-dark'))
    "target should have is-dark class"
}
