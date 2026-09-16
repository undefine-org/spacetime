/// Unit Tests: state-reflect — string/boolean form (PLAN-077 W6)
///
/// REGRESSION test for FUP-074's "orphan" bug: `@state(when: "loading")`'s
/// data-st-state carrier had NO reactive writer for GLOBAL signals (the old
/// ST.bindState watched element-locally, so a global write never fired it).
/// state-reflect subscribes via ST.watchScoped (host + ancestors + global
/// channel) and reflects on every write.
///
/// Pinned behaviors:
///   1. orphan fix: a GLOBAL-channel write reaches an element whose scope
///      never owned the signal
///   2. boolean gate (the legacy @state(when: "loading") + $loading
///      semantics): truthy → state name, falsy → cleared — also via a local
///      @on mutation
///   3. instrumentation preserved: __stStateHistory / state.enter / exit

@import "stdlib/testing/test"

@data inline $loading : false;
@data inline $disabled : false;

.srs-gate {
  @state(when: "loading") {
    opacity: 0.4;
  }
}

.srs-multi {
  @state(when: "loading") {
    opacity: 0.4;
  }
  @state(when: "disabled") {
    opacity: 0.2;
  }
}

.srs-btn2 {
  @on &.click { $loading <- true; }
}

// =============================================================================
// 1. ORPHAN REGRESSION: a GLOBAL signal write reaches an element whose scope
//    never owned it (the old ST.bindState watched element-locally and missed
//    the global channel entirely — data-st-state was a dead carrier).
// =============================================================================

@test "state-reflect: global-channel write reflects reactively (orphan fix)" {
  @fixture {
    <div class="srs-gate">x</div>
  }

  @when .srs-gate mouseenter

  @eval (window.__srs0 = document.querySelector('.srs-gate').getAttribute('data-st-state') || '')
  @assert (window.__srs0 === '')
    "initial ($loading=false): data-st-state is empty (gate closed)"

  @eval (window.SpacetimeLocal.loading = true)
  @eval (window.__srs1 = document.querySelector('.srs-gate').getAttribute('data-st-state'))
  @assert (window.__srs1 === 'loading')
    "global-channel write: data-st-state must flip reactively (the orphan fix)"

  @eval (window.SpacetimeLocal.loading = false)
  @eval (window.__srs2 = document.querySelector('.srs-gate').getAttribute('data-st-state') || '')
  @assert (window.__srs2 === '')
    "global-channel clear: data-st-state must clear reactively"
}

// =============================================================================
// 2. Boolean signal: legacy gate semantics (truthy → name, falsy → cleared)
// =============================================================================

@test "state-reflect: boolean signal gates the state name" {
  @fixture {
    <div class="srs-gate">x</div>
    <button class="srs-btn2">go</button>
  }

  @when .srs-gate mouseenter

  @eval (window.__srg0 = document.querySelector('.srs-gate').getAttribute('data-st-state') || '')
  @assert (window.__srg0 === '')
    "initial ($loading=false): data-st-state is empty (gate closed)"

  @eval (document.querySelector('.srs-btn2').dispatchEvent(new Event('click')))
  @eval (window.__srg1 = document.querySelector('.srs-gate').getAttribute('data-st-state'))
  @assert (window.__srg1 === 'loading')
    "after $loading <- true: data-st-state='loading' (legacy gate semantics)"

  @eval (window.SpacetimeLocal.loading = false)
  @eval (window.__srg2 = document.querySelector('.srs-gate').getAttribute('data-st-state') || '')
  @assert (window.__srg2 === '')
    "after loading=false: the gate clears data-st-state"
}

// =============================================================================
// 3. Instrumentation preserved (the retired state() primitive's contract)
// =============================================================================

@test "state-reflect: state history and enter/exit events recorded" {
  @fixture {
    <div class="srs-gate">x</div>
  }

  @when .srs-gate mouseenter

  @eval (eval("window.__sri_events = []; document.querySelector('.srs-gate').addEventListener('state.enter', function(e) { window.__sri_events.push('enter:' + e.detail.to) }); document.querySelector('.srs-gate').addEventListener('state.exit', function(e) { window.__sri_events.push('exit:' + e.detail.from) })"))
  @eval (window.SpacetimeLocal.loading = true)
  @eval (window.SpacetimeLocal.loading = false)
  @eval (window.SpacetimeLocal.loading = true)

  @eval (window.__sri_hist = JSON.stringify(document.querySelector('.srs-gate').__stStateHistory))
  @eval (window.__sri_count = document.querySelector('.srs-gate').__stTransitionCount)
  @assert (window.__sri_hist === '["loading","loading"]')
    "state history must record both entries in order"
  @assert (window.__sri_count === 2)
    "transition count must be 2"
  @eval (window.__sri_ev = window.__sri_events.join(','))
  @assert (window.__sri_ev === 'enter:loading,exit:loading,enter:loading')
    "state.enter/state.exit events fire with from/to detail"

  @eval (window.SpacetimeLocal.loading = false)
}

// =============================================================================
// 4. GATE 6 P1 regression: two gates sharing one element's carrier — one
//    gate's falsy transition must NOT wipe the other gate's active state
//    (legacy ST.bindState cleared only its own state name).
// =============================================================================

@test "state-reflect: one gate's falsy clear preserves another gate's state" {
  @fixture {
    <div class="srs-multi">x</div>
  }

  @when .srs-multi mouseenter

  @eval (window.SpacetimeLocal.loading = true)
  @eval (window.SpacetimeLocal.disabled = true)
  @eval (window.__srm0 = document.querySelector('.srs-multi').getAttribute('data-st-state'))
  @assert (window.__srm0 === 'disabled')
    "both gates truthy: last-writer-wins reflects 'disabled'"

  @eval (window.SpacetimeLocal.loading = false)
  @eval (window.__srm1 = document.querySelector('.srs-multi').getAttribute('data-st-state'))
  @assert (window.__srm1 === 'disabled')
    "loading=false must NOT wipe the still-active 'disabled' state (legacy guard)"

  @eval (window.SpacetimeLocal.disabled = false)
  @eval (window.__srm2 = document.querySelector('.srs-multi').getAttribute('data-st-state') || '')
  @assert (window.__srm2 === '')
    "disabled=false clears its own state (no gate remains)"

  @eval (window.SpacetimeLocal.loading = false)
}
