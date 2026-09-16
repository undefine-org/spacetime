/// Unit Tests: state-reflect — union form (PLAN-077 W6)
///
/// NEW CAPABILITY: a plain `@data derive`-produced union (NOT @socket) drives
/// `@state(when: $x is V)`. The union flows through the SAME SpacetimeLocal +
/// local:<name>:updated channel every derive uses; state-reflect reflects its
/// `.type` into data-st-state + data-st-<sig>-type and destructures payload
/// fields into element scope.
///
/// Pinned behaviors:
///   1. derive-produced union reflects to data-st-state (CSS gate applies)
///   2. namespaced compat attr data-st-<sig>-type also written
///   3. payload destructure: `@state(when: $fs is Failed { $errorText })` makes
///      the payload resolvable in element scope
///   4. variant transitions reflect reactively (guard flip → attr flip)

@import "stdlib/testing/test"

@type FetchState { Idle | Failed(string) }

@data inline $errorText string : "";

@data derive $fs FetchState : @match {
  ($errorText != "") => Failed($errorText);
  _                  => Idle;
}

.sru-card {
  @state(when: $fs is Failed { $errorText }) {
    color: red;
  }
}

.sru-btn {
  @on &.click { $errorText <- "boom-077"; }
}

// =============================================================================
// 1. Union reflection + namespaced attr + reactivity
// =============================================================================

@test "state-reflect: derive union reflects variant to both attributes" {
  @fixture {
    <div class="sru-card">x</div>
    <button class="sru-btn">go</button>
  }

  @when .sru-card mouseenter

  @eval (window.__sru0 = document.querySelector('.sru-card').getAttribute('data-st-state'))
  @assert (window.__sru0 === 'Idle')
    "initial: the derive's catch-all Idle reflects to data-st-state"

  @eval (window.__sru0n = document.querySelector('.sru-card').getAttribute('data-st-fs-type'))
  @assert (window.__sru0n === 'Idle')
    "initial: the namespaced compat attr data-st-fs-type also reflects"

  @eval (document.querySelector('.sru-btn').dispatchEvent(new Event('click')))
  @eval (window.__sru1 = document.querySelector('.sru-card').getAttribute('data-st-state'))
  @eval (window.__sru1n = document.querySelector('.sru-card').getAttribute('data-st-fs-type'))
  @assert (window.__sru1 === 'Failed')
    "after errorText set: data-st-state must flip to Failed reactively"
  @assert (window.__sru1n === 'Failed')
    "after errorText set: data-st-fs-type must flip to Failed reactively"

  @eval (window.SpacetimeLocal.errorText = '')
  @eval (window.__sru2 = document.querySelector('.sru-card').getAttribute('data-st-state'))
  @assert (window.__sru2 === 'Idle')
    "after errorText cleared: data-st-state reflects the recompute back to Idle"
}

// =============================================================================
// 2. Payload destructure into element scope
// =============================================================================

@test "state-reflect: union payload destructures into element scope" {
  @fixture {
    <div class="sru-card">x</div>
    <button class="sru-btn">go</button>
  }

  @when .sru-card mouseenter

  @eval (document.querySelector('.sru-btn').dispatchEvent(new Event('click')))
  @eval (window.__sru_p = ST.get(document.querySelector('.sru-card'), 'errorText'))
  @assert (window.__sru_p === 'boom-077')
    "Failed($errorText) payload must destructure into the element's scope (ST.get)"

  @eval (window.SpacetimeLocal.errorText = '')
}
