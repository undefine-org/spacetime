/// Unit Tests: inline anonymous union typeref (PLAN-077 W2)
///
/// The derive's type position accepts an ANONYMOUS union literal
/// `(Blue | Orange | Green)` (stdlib/enum/capture-types/inline-union.st) for
/// one-off enums that don't warrant a named `@type`. Also pinned: the new
/// `@data derive … : @match {…}` form does NOT capture plain expr derives —
/// `@data derive $x number : <expr>;` still routes to the plain data-derive
/// form (grammar-only discrimination: `@match` literal + braced body vs a
/// trailing `;`).

@import "stdlib/testing/test"

@data inline $isOrange bool : false;
@data inline $count number : 21;

// Anonymous inline union — no @type declaration anywhere.
@data derive $badge (Blue | Orange | Green) : @match {
  ($isOrange) => Orange;
  _           => Green;
}

// Plain expr derive — must NOT be captured by the match form.
@data derive $doubled number : $count * 2;

// =============================================================================
// 1. Inline union: guard resolution + catch-all + reactivity
// =============================================================================

@test "inline union typeref: derive-match resolves and recomputes" {
  @fixture {
    <div class="iu-1">x</div>
  }

  @eval (window.__iu1_init = window.SpacetimeLocal.badge && window.SpacetimeLocal.badge.type)
  @assert (window.__iu1_init === 'Green')
    "no truthy guard: the inline union's `_ => Green` catch-all must win"

  @eval (window.SpacetimeLocal.isOrange = true)
  @eval (window.__iu1_flip = window.SpacetimeLocal.badge && window.SpacetimeLocal.badge.type)
  @assert (window.__iu1_flip === 'Orange')
    "isOrange=true: $badge must resolve the Orange arm of the inline union"

  @eval (window.SpacetimeLocal.isOrange = false)
  @eval (window.__iu1_back = window.SpacetimeLocal.badge && window.SpacetimeLocal.badge.type)
  @assert (window.__iu1_back === 'Green')
    "isOrange back to false: $badge must recompute to the catch-all"
}

// =============================================================================
// 2. Form discrimination: plain expr derive is untouched by the match form
// =============================================================================

@test "inline union typeref: plain @data derive still computes its expr" {
  @fixture {
    <div class="iu-2">x</div>
  }

  @eval (window.__iu2_init = window.SpacetimeLocal.doubled)
  @assert (window.__iu2_init === 42)
    "@data derive $doubled number : $count * 2 must still route to the plain derive form (21 * 2)"

  @eval (window.SpacetimeLocal.count = 50)
  @eval (window.__iu2_flip = window.SpacetimeLocal.doubled)
  @assert (window.__iu2_flip === 100)
    "plain derive must keep reacting to its deps (50 * 2) — no interference from derive-match"
}
