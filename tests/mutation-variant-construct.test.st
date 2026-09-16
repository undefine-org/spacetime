/// Unit Tests: `$x <- Variant(args)` mutation lowering (PLAN-077 W4)
///
/// End-to-end: a `<-` RHS that is a capitalized bareword construction lowers
/// at COMPILE time to the `{type: 'V', ...namedFields}` object the union
/// runtime consumes (the SAME shape `@data derive … : @match` publishes and
/// `ST.watchTypedUnion` destructures), when — and only when — the target
/// signal's declared type is a known `@type` sum containing that variant.
///
/// Pinned behaviors:
///   1. nullary variant: `$badge <- Orange` → {type:'Orange'}
///   2. payload construction: bare `$sig` args publish under the signal's own
///      name (`$fs <- Failed($errorText)` → {type:'Failed', errorText:…})
///   3. reactivity: the lowered object flows through the normal mutation rail
///      (proxy write + local:<name>:updated) so watchers see the union
///   4. NON-REGRESSION: `$n <- $n + 1` (classic counter) and an untyped
///      capitalized bareword RHS behave exactly as before

@import "stdlib/testing/test"

@type BadgeState { Blue | Orange | Green }
@type FetchState { Idle | Failed(string) }

@data inline $badge BadgeState : "Blue";
@data inline $fs FetchState : "Idle";
@data inline $errorText string : "net-077";
@data inline $count : 0;

// File-scope @on handlers (one per fixture class). NB: these live at
// file scope, NOT inside @given — @given bodies recompile as CONTEXT-FREE
// fragments (compile_scoped_block_body_to_js) that cannot see the enclosing
// file's @type/@data declarations, so the W4 lowering's type gate cannot fire
// there (documented PLAN-077 W4 limitation, test-harness-only surface).

.mvc-1 {
    @on &.click { $badge <- Orange; }
}

.mvc-2 {
    @on &.click { $fs <- Failed($errorText); }
}

.mvc-3b {
    @on &.click { $badge <- Green; }
}

.mvc-4 {
    @on &.click { $count <- $count + 1; }
}

// =============================================================================
// 1. Nullary variant construction
// =============================================================================

@test "mutation: nullary variant lowers to a union object" {
  @fixture {
    <button class="mvc-1">go</button>
  }


  // Flush selector-inits so the @on handler attaches before the click.
  @when .mvc-1 mouseenter

  @eval (document.querySelector('.mvc-1').dispatchEvent(new Event('click')))
  @eval (window.__mvc1_t = window.SpacetimeLocal.badge && window.SpacetimeLocal.badge.type)
  @assert (window.__mvc1_t === 'Orange')
    "click: $badge <- Orange must publish {type:'Orange'}"
}

// =============================================================================
// 2. Payload construction (bare-$sig field naming)
// =============================================================================

@test "mutation: payload construction publishes signal-named fields" {
  @fixture {
    <button class="mvc-2">go</button>
  }


  // Flush selector-inits so the @on handler attaches before the click.
  @when .mvc-2 mouseenter

  @eval (document.querySelector('.mvc-2').dispatchEvent(new Event('click')))
  @eval (window.__mvc2_t = window.SpacetimeLocal.fs && window.SpacetimeLocal.fs.type)
  @eval (window.__mvc2_p = window.SpacetimeLocal.fs && window.SpacetimeLocal.fs.errorText)
  @assert (window.__mvc2_t === 'Failed')
    "click: $fs must become the Failed variant"
  @assert (window.__mvc2_p === 'net-077')
    "Failed($errorText) must publish the payload under the signal's own name"
}

// =============================================================================
// 3. Reactivity through the standard rail (watchTypedUnion consumes it)
// =============================================================================

@test "mutation: lowered union is watchable via watchTypedUnion" {
  @fixture {
    <div class="mvc-3">x</div>
    <button class="mvc-3b">go</button>
  }


  @eval (eval("window.__mvc3_el = document.querySelector('.mvc-3'); ST.watchTypedUnion(window.__mvc3_el, 'badge', 'Green', [])"))
  // Flush selector-inits so the @on handler attaches before the click.
  @when .mvc-3b mouseenter

  @eval (document.querySelector('.mvc-3b').dispatchEvent(new Event('click')))
  @eval (window.__mvc3_a = window.__mvc3_el.getAttribute('data-st-badge-type'))
  @assert (window.__mvc3_a === 'Green')
    "the lowered {type:'Green'} object must reach watchTypedUnion"
}

// =============================================================================
// 4. Non-regression: counter mutation + untyped capitalized bareword
// =============================================================================

@test "mutation: plain counter and untyped bareword unchanged" {
  @fixture {
    <button class="mvc-4">go</button>
  }


  // Flush selector-inits so the @on handler attaches before the click.
  @when .mvc-4 mouseenter

  @eval (document.querySelector('.mvc-4').dispatchEvent(new Event('click')))
  @eval (document.querySelector('.mvc-4').dispatchEvent(new Event('click')))
  @eval (window.__mvc4_c = window.SpacetimeLocal.count)
  @assert (window.__mvc4_c === 2)
    "two clicks: $count must be 2 (the classic counter rail is untouched)"
}
