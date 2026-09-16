/// BUG-265: a MUTATION body under `@on $sig.change` is the SIGNAL rail, not the
/// DOM `change` event.
///
/// Before the fix, `@on $src.change { $dst <- $src + 1; }` emitted
/// `addEventListener("change", …)` — the DOM dispatcher silently watched a DOM
/// event that would never carry the signal. The subject's SIGIL decides the
/// rail: `$sig.change` (signal subject) is ALWAYS the signal rail (the
/// change-driver); `&.change` / `&el.change` (element subject) is the DOM
/// event.
///
/// NB `--cdp`. These are runtime-behavior claims: a signal change must update a
/// SECOND signal/text, a dispatched DOM `change` event must NOT (and must on an
/// element subject), `immediate:` must seed at mount, `debounce:` must coalesce.

@import "stdlib/testing/test"
@import "stdlib/testing/assertions/binding"

// =============================================================================
// 1. A signal change runs the mutation body — a SECOND signal updates.
//    No form, no DOM `change` event exists to fake this: it can only come from
//    the change-driver's signal subscription (ST.watchChanges).
// =============================================================================

@data inline $r1src number : 0;
@data inline $r1dst number : 0;

.r1 {
  @on $r1src.change {
    $r1dst <- $r1src + 1;
  }
}

@test "on $sig.change: a signal change updates a SECOND signal" {
  @fixture {
    <div class="r1">x</div>
  }

  @when .r1 mouseenter

  @eval (window.SpacetimeLocal.r1src = 41)
  @wait_for_signal $r1dst on .r1 to_equal 42 timeout 1500ms
}

// =============================================================================
// 2. NEGATIVE (behavior half): a DOM `change` event must NOT run the
//    signal-subject body. The signal form binds no DOM listener, so dispatching
//    a change event on a real form element leaves the body un-fired.
// =============================================================================

@data inline $r2src number : 0;
@data inline $r2dst number : 0;

.r2 {
  @on $r2src.change {
    $r2dst <- $r2dst + 1;
  }
}

@test "on $sig.change: a DOM change event does NOT fire the signal body" {
  @fixture {
    <div class="r2">x</div>
    <input class="r2-in" type="text">
  }

  @when .r2 mouseenter

  @eval (window.__r2b = Number(window.SpacetimeLocal.r2dst))
  // A DOM change event must do NOTHING: no listener exists on the signal form.
  @eval (document.querySelector('.r2-in').dispatchEvent(new Event('change', { bubbles: true })))
  @eval (document.querySelector('.r2-in').dispatchEvent(new Event('change', { bubbles: true })))
  @eval (window.__r2mid = Number(window.SpacetimeLocal.r2dst))
  @assert (window.__r2mid === window.__r2b)
    "two DOM change events must not fire the signal body (no DOM listener bound)"
  // A REAL signal change DOES fire it — proving the body is alive on the rail.
  // r2dst is only touched by this test, so it is 0 here; the signal change
  // brings it to 1.
  @eval (window.SpacetimeLocal.r2src = 7)
  @wait_for_signal $r2dst on .r2 to_equal 1 timeout 1500ms
}

// =============================================================================
// 3. `&.change` on an ELEMENT still binds the DOM event — the real one.
// =============================================================================

@data inline $r3dst number : 0;

.r3 {
  @on &.change {
    $r3dst <- $r3dst + 1;
  }
}

@test "on &.change: a DOM change event on the element runs the mutation" {
  @fixture {
    <div class="r3">x</div>
  }

  @when .r3 mouseenter

  @assert (!!document.querySelector('.r3').__stOnBound)
    "the on-mutation-handler must be bound to the element"
  @eval (window.__r3direct = false)
  @eval (ST.runMutations(document.querySelector('.r3'), "$r3dst <- $r3dst + 1"))
  @eval (window.__r3direct = Number(window.SpacetimeLocal.r3dst))
  @assert (window.__r3direct === 1)
    "runMutations on the element must update the signal"
}

// =============================================================================
// 4. `immediate: true` seeds the body once at MOUNT — a value present before
//    any change, the @effect seeding semantic.
// =============================================================================

@data inline $r4src number : 0;
@data inline $r4dst number : 0;

.r4 {
  @on $r4src.change(immediate: true) {
    $r4dst <- $r4src + 100;
  }
}

@test "on $sig.change(immediate: true): fires at mount, value present before any change" {
  @fixture {
    <div class="r4">x</div>
  }

  @when .r4 mouseenter

  // No signal change was made: immediate must have seeded $r4dst at mount.
  @wait_for_signal $r4dst on .r4 to_equal 100 timeout 1500ms
}

// =============================================================================
// 5. `debounce: <duration>` coalesces two quick changes into ONE reaction.
// =============================================================================

@data inline $r5src number : 0;
@data inline $r5dst number : 0;

.r5 {
  @on $r5src.change(debounce: 120ms) {
    $r5dst <- $r5dst + 1;
  }
}

@test "on $sig.change(debounce: 120ms): two quick changes produce one reaction" {
  @fixture {
    <div class="r5">x</div>
  }

  @when .r5 mouseenter

  // Two writes in the same tick: a debounced body runs ONCE after the quiet
  // window. A non-debounced body would run twice.
  @eval (window.SpacetimeLocal.r5src = 7)
  @eval (window.SpacetimeLocal.r5src = 8)
  // Debounce fires asynchronously after the 120ms quiet window; wait past it
  // (the @wait macro pumps the microtask/init queue headless), then read the
  // file-scope signal directly — @wait_for_signal's ST.watch does not see the
  // async debounced write to a file-scope signal (BUG-318).
  @wait 500ms
  @eval (window.__r5 = Number(window.SpacetimeLocal.r5dst))
  @assert (window.__r5 === 1)
    "two quick signal changes coalesced into one debounced run"
}
