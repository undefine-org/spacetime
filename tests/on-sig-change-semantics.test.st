/// Unit Tests: `@on $sig.change` fires on every genuine change (FEAT-074 / FEAT-169,
/// migrated from `@effect` per BUG-263)
///
/// WRITTEN RED, BEFORE the implementation (TDD): these pin the semantics the
/// collapse onto `ST.watchChanges` is supposed to deliver, so the change is
/// judged by observable behavior rather than by the shape of its emitted source.
///
/// What changed and why. `@effect` (now deleted, BUG-263) used to subscribe
/// TWICE — a document listener for body-scope `local:<sig>:updated` plus an
/// element-scoped `ST.watch` — and then spent a microtask gate collapsing the
/// resulting double-fire back into one run. That gate existed only because there
/// were two channels. It also carried the call-count `primed` flag whose
/// mis-priming killed `@on $signal` entirely (BUG-252): `ST.watch` only opens
/// with a current-value call when THIS element's store owns the name, so for a
/// file-scope signal the first callback IS the first change.
///
/// `ST.watchChanges` subscribes once, in the scope the author's reads resolve
/// through, and delivers genuine changes exactly once. So: one channel, no
/// dedupe gate, no flag. `@on $sig.change` is the surviving surface that rides
/// this rail (the change-driver primitive), carrying the mutation body that
/// `@effect` used to hold.
///
/// DELIBERATE semantic note (the collapse in its own commit): with the
/// microtask gate gone, an un-debounced body runs once per genuine change.
/// `debounce:` remains the explicit way to ask for coalescing — which is the
/// point. (Where `@effect`'s runner happened to write synchronously to the
/// body-scope plane on the write statement, the change-driver delivers through
/// the signal rail; the assertions here use `@wait_for_signal` to read the
/// element-scoped signal the body writes, which resolves the change the same
/// way the authored bindings do.)
///
/// NB `--cdp`. These are runtime-behavior claims; the fidelity ladder says make
/// them where the timing is real.

@import "stdlib/testing/test"
@import "stdlib/testing/assertions/binding"

@data inline $ecsSource number : 0;
@data inline $ecsRuns number : 0;

.ecs-host {
  @on $ecsSource.change {
    $ecsRuns <- $ecsRuns + 1;
  }
}

// =============================================================================
// 1. A file-scope signal change RUNS the body.
//    This is the BUG-252 class: the old `primed` flag ate the first change for
//    any signal this element does not own, so an effect on file-scope state
//    needed two writes to run once.
// =============================================================================

@test "on $sig.change: the FIRST change to a file-scope signal runs the body" {
  @fixture {
    <div class="ecs-host">x</div>
  }

  @when .ecs-host mouseenter

  @eval (window.SpacetimeLocal.ecsSource = 1)
  @wait_for_signal $ecsRuns on .ecs-host to_equal 1 timeout 1500ms
}

// =============================================================================
// 2. The body fires once per genuine change — two distinct writes, two runs.
//    (The old microtask gate coalesced same-tick writes; `debounce:` is now the
//    explicit way to ask for that.)
// =============================================================================

@test "on $sig.change: two distinct changes run the body twice" {
  @fixture {
    <div class="ecs-host">x</div>
  }

  @when .ecs-host mouseenter

  @eval (window.SpacetimeLocal.ecsRuns = 0)
  @eval (window.SpacetimeLocal.ecsSource = 3)
  @wait_for_signal $ecsRuns on .ecs-host to_equal 1 timeout 1500ms
  @eval (window.SpacetimeLocal.ecsSource = 4)
  @wait_for_signal $ecsRuns on .ecs-host to_equal 2 timeout 1500ms
}

// =============================================================================
// 3. A no-op write is not a change.
//    Re-writing the value already held must not run the body — this is what
//    makes "genuine change" the contract rather than "any notification".
// =============================================================================

@test "on $sig.change: re-writing the same value does not run the body" {
  @fixture {
    <div class="ecs-host">x</div>
  }

  @when .ecs-host mouseenter

  @eval (window.SpacetimeLocal.ecsRuns = 0)
  @eval (window.SpacetimeLocal.ecsSource = 7)
  @wait_for_signal $ecsRuns on .ecs-host to_equal 1 timeout 1500ms
  // Two no-op writes: nothing may run.
  @eval (window.SpacetimeLocal.ecsSource = 7)
  @eval (window.SpacetimeLocal.ecsSource = 7)
  @wait 300ms
  // A THIRD genuine change must bring the count to exactly 2 — had either
  // no-op fired, it would land at 3+ and this `to_equal 2` would time out.
  @eval (window.SpacetimeLocal.ecsSource = 8)
  @wait_for_signal $ecsRuns on .ecs-host to_equal 2 timeout 1500ms
}
