/// A mutator call on a MEMBER — `$rows.insert(7);` — is a legal `@on` body.
///
/// That shape was declared (`%capture_type signal_call`,
/// stdlib/capture-types/on-actions.st) and DISPATCHED (st.js `runMutations`,
/// the `$coll.method(args)` branch reaching ST._collections) long before this
/// test. Two other links in the chain were missing:
///
///   1. `on_motion_body` — the body grammar behind the sigil head — had an
///      alternative for a BARE call (`$ping()`) and for a write (`$x <- 1`),
///      but none for a call on a member. The body failed to capture and the
///      whole directive was refused (E0946).
///   2. `drive-body` (src/pipeline/drivers.rs) had no arm for the new capture,
///      so once the grammar accepted it the statement was DROPPED and the body
///      fell through to the ANIMATION path — where the emitted driver reached
///      for `ST._raf` and threw on a handler that had parsed perfectly.
///
/// Failure 2 is why this gate is a RUN. After fixing only the grammar, `check`
/// was green on a handler that crashed the moment it fired.

@import "stdlib/testing/test"

@test "a member mutator call reaches the runtime as a mutation" {
  @fixture {
    <button class="mcb-add">add</button>
  }
  @given .mcb-add {
    @on &.click {
      $mcbLog.push("hit");
    }
  }

  @eval (window.SpacetimeLocal = window.SpacetimeLocal || {})
  @eval (window.__mcbCalls = [])
  @eval (ST._collections = ST._collections || {})
  @eval (ST._collections.mcbLog = { push: function (v) { window.__mcbCalls.push(v); } })

  @when .mcb-add click
  @wait 50ms
  @assert (window.__mcbCalls.length === 1)
    "a member call must dispatch exactly once, through runMutations";
  @assert (window.__mcbCalls[0] === "hit")
    "the argument must survive the capture -> reconstruct -> splitArgs round trip";
}

/// The regression that actually shipped: a member call and a write in ONE body.
/// Before the `drive-body` arm existed the member call was dropped while the
/// write survived, so a half-working handler looked entirely fine.
@test "a member call and a write coexist in one body" {
  @fixture {
    <button class="mcb-both">go</button>
  }
  @given .mcb-both {
    @on &.click {
      $mcbBoth.insert(1);
      $mcbTouched <- true;
    }
  }

  @eval (window.SpacetimeLocal = window.SpacetimeLocal || {})
  @eval (window.SpacetimeLocal.mcbTouched = false)
  @eval (window.__mcbBothCalls = [])
  @eval (ST._collections = ST._collections || {})
  @eval (ST._collections.mcbBoth = { insert: function (v) { window.__mcbBothCalls.push(v); } })

  @when .mcb-both click
  @wait 50ms
  @assert (window.__mcbBothCalls.length === 1)
    "the member call must not be dropped when a write shares the body";
  @assert (window.SpacetimeLocal.mcbTouched === true)
    "the write must still land alongside the member call";
}

/// Two args, one of them named — the arg run is reconstructed from structured
/// `call_arg*` captures and must survive runMutations' splitArgs verbatim.
@test "a member call carries multiple arguments intact" {
  @fixture {
    <button class="mcb-args">go</button>
  }
  @given .mcb-args {
    @on &.click {
      $mcbArgs.set("k", 42);
    }
  }

  @eval (window.SpacetimeLocal = window.SpacetimeLocal || {})
  @eval (window.__mcbArgs = null)
  @eval (ST._collections = ST._collections || {})
  @eval (ST._collections.mcbArgs = { set: function () { window.__mcbArgs = Array.prototype.slice.call(arguments); } })

  @when .mcb-args click
  @wait 50ms
  @assert (window.__mcbArgs !== null)
    "the call must fire";
  @assert (window.__mcbArgs[0] === "k" && window.__mcbArgs[1] === 42)
    "both arguments must arrive in order, with types preserved";
}
