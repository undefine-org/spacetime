/// Unit Tests: `@on $signal { … }` arms actually FIRE (PLAN-126 W6)
///
/// REGRESSION GUARD for three stacked silent breaks, each of which left the
/// emitted source looking perfect. Every pre-existing arms gate asserted on
/// COMPILED TEXT — `match: "go"` is present, the keyframes are present, the
/// watch call is present — and all of that was true while nothing moved in a
/// browser. Presence-in-source and runs-in-a-browser are different claims.
/// These tests make the second one, against the real WAAPI (`--cdp`).
///
/// The three breaks, all in the `@on $sig { … }` path:
///
///   1. ELEMENT SELF-SHADOW. `signal-arms` opened with `var el = %&el;`, which
///      lowers to `var el = el;` at any splice site whose closure already names
///      the element `el` — the selector-init closure does. Under `var` hoisting
///      that reads `undefined`, so the primitive's own `if (!el) return;` guard
///      killed every instance. (`signal-handler` carried the identical shape;
///      `drag-zones.st` documents the trap.)
///
///   2. BINDING SPLICED AS AN OBJECT. The `signal_path` capture is a Named map
///      `{base, facet?}`. The facet branch re-keyed it to a flat binding; the
///      ordinary branch spliced the map, so the primitive's `String(%signal)`
///      read `"[object Object]"` and watched a name that can never fire.
///
///   3. PRIME-BY-CALL-COUNT. The `primed` flag assumed a subscription always
///      opens with a synchronous current-value call. `ST.watch` makes that call
///      only when THIS element's store already holds the name; for a signal
///      owned by an ancestor or by file scope the first callback IS the first
///      change — and the flag ate it. Priming now reads `ST.resolve` at
///      subscribe time: state-based, identical for both ownership shapes.
///
/// Also pinned: the watch mirrors the resolve (FUP-094 doctrine, stated at
/// stdlib/enum/dispatch.st:192) — `ST.watchScoped`, not `ST.watch`, so a signal
/// owned by an ancestor or by file scope reaches its arms.
///
/// NB `--cdp` (real Chromium). Asserting an animation exists is a paint-adjacent
/// claim; the fidelity ladder says make it where it is real.

@import "stdlib/testing/test"

@form motion --sab-rise { opacity: 0 -> 1; }

@data inline $sabPhase string : "idle";

// NB: each test below writes a DISTINCT matching value. Page state persists
// across tests in one file, so re-writing the same value is a no-op (ST.set
// early-returns on an unchanged value) and the next test would silently
// observe the PREVIOUS test's animation instead of its own.
.sab-target {
  opacity: 0;
  @on $sabPhase {
    "go"    => --sab-rise;
    "go2"   => --sab-rise;
  }
}

// A SEPARATE element+signal for the negative claims, so "nothing animated" can
// never be satisfied-or-violated by an animation a previous test started on a
// shared node.
@data inline $sabQuiet string : "idle";

.sab-quiet {
  opacity: 0;
  @on $sabQuiet {
    "yes" => --sab-rise;
  }
}

// =============================================================================
// 1. A matching arm animates the real element
// =============================================================================

@test "signal-arms: a matching arm plays on the real element" {
  @fixture {
    <div class="sab-target">x</div>
  }

  @when .sab-target mouseenter

  @eval (window.__sab0 = document.querySelector('.sab-target').getAnimations().length)
  @assert (window.__sab0 === 0)
    "before the signal moves, nothing is animating"

  @eval (window.SpacetimeLocal.sabPhase = 'go')
  @eval (window.__sab1 = document.querySelector('.sab-target').getAnimations().length)
  @assert (window.__sab1 > 0)
    "a matching arm must ANIMATE the element — reading 0 here means the primitive lost its element, its signal name, or its first change"
}

// =============================================================================
// 2. Exactly once per change — watchScoped fans out over every scope in the
//    chain, and an arm must not play once per scope.
// =============================================================================

@test "signal-arms: a matching arm plays exactly once per change" {
  @fixture {
    <div class="sab-target">x</div>
  }

  @when .sab-target mouseenter

  @eval (window.SpacetimeLocal.sabPhase = 'go2')
  @eval (window.__sab2 = document.querySelector('.sab-target').getAnimations().length)
  @assert (window.__sab2 === 1)
    "one change plays one animation — a fan-out duplicate would read 2+"
}

// =============================================================================
// 3. Dispatch still discriminates — the fix must not make arms fire on anything
// =============================================================================

@test "signal-arms: a non-matching value plays nothing" {
  @fixture {
    <div class="sab-quiet">x</div>
  }

  @when .sab-quiet mouseenter

  @eval (window.SpacetimeLocal.sabQuiet = 'nope')
  @eval (window.__sab3 = document.querySelector('.sab-quiet').getAnimations().length)
  @assert (window.__sab3 === 0)
    "an unmatched value must not play an arm"
}

// =============================================================================
// 4. Mount is not a change — eager priming must not fire the initial value
// =============================================================================

@test "signal-arms: the initial value does not fire an arm" {
  @fixture {
    <div class="sab-quiet">x</div>
  }

  @when .sab-quiet mouseenter

  @eval (window.__sab4 = document.querySelector('.sab-quiet').getAnimations().length)
  @assert (window.__sab4 === 0)
    "priming reads the current value; it must not COUNT as a change"
}
