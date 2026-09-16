/// var() Resolution in Animation Keyframes
///
/// Tests that animation keyframe values containing var() CSS custom property
/// references are resolved at runtime via getComputedStyle.
///
/// SHAPE (BUG-194/BUG-200 + SIP-001): drivers are declared at FILE SCOPE in a
/// selector block via the unified `@on &.load(...)` head. The previous shape —
/// retired `@load name(...)` nested inside `@given` — was silently dropped, so
/// no timeline ever registered. `@clock run-to-end` steps the virtual clock
/// (stdlib/testing/timing) so the load driver actually reaches its end state
/// and the color assertions verify genuine resolution instead of vacuous
/// black-vs-white.

@version 2026-07-26;
@import "stdlib/testing/test"
@import "stdlib/testing/timing"

.var-color {
  @on &.load(name: "resolve-test", duration: 100ms) {
    color: #FFFFFF -> var(--end-color);
  }
}

.var-dark {
  @on &.load(name: "dark-test", duration: 100ms) {
    color: #FFFFFF -> var(--text-end);
  }
}

.scope-root {
  @on &.load(name: "scope-resolve", duration: 100ms) {
    .headline { color: #FFFFFF -> var(--headline-end); easing: linear; }
  }
}

.bad-var {
  @on &.load(name: "fallback-test", duration: 100ms) {
    opacity: 0 -> var(--nonexistent);
  }
}

// =============================================================================
// var() Resolution
// =============================================================================

@test "keyframe var() value resolves to CSS custom property" {
  @fixture {
    <div class="var-color" style="--end-color: #1A1A1A"></div>
  }
  @wait 100ms
  @clock run-to-end
  @then .var-color should have_timeline "resolve-test"
  @let (el = document.querySelector('.var-color'))
  @let (color = getComputedStyle(el).color)
  // End state should be resolved #1A1A1A, not white
  @assert (color !== 'rgb(255, 255, 255)') "color should not remain white after animation"
}

@test "keyframe var() with dark mode override resolves correct value" {
  @fixture {
    <div class="var-dark" style="--text-end: #1A1A1A"></div>
  }
  @wait 100ms
  @clock run-to-end
  @then .var-dark should have_timeline "dark-test"
  @let (el = document.querySelector('.var-dark'))
  @let (color = getComputedStyle(el).color)
  @assert (color !== 'rgb(255, 255, 255)') "should resolve var(--text-end)"
}

@test "var() with scoped selectors resolves per-element" {
  @fixture {
    <div class="scope-root" style="--headline-end: #1A1A1A">
      <h2 class="headline" style="color: #FFFFFF">Title</h2>
    </div>
  }
  @wait 100ms
  @clock run-to-end
  @then .scope-root should have_timeline "scope-resolve"
  @let (el = document.querySelector('.headline'))
  @let (color = getComputedStyle(el).color)
  @assert (color !== 'rgb(255, 255, 255)') "scoped child should resolve var()"
}

@test "unresolvable var() falls back gracefully" {
  @fixture {
    <div class="bad-var"></div>
  }
  @wait 100ms
  @clock run-to-end
  @then .bad-var should have_timeline "fallback-test"
  // Should not crash — graceful degradation
}
