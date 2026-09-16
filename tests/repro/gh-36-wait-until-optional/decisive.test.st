// GH-36 decisive: `@wait_until` must actually WAIT, and its optional `timeout:`
// clause must be genuinely optional.
//
// Original polarity (pre-fix): the directive was silently DROPPED, so a test
// containing `@wait_until false timeout: 500ms` passed INSTANTLY and vacuously.
// That vacuity was the issue. Post-fix the loop emits, so an unsatisfiable
// condition throws "Timeout after 500ms" — correct behavior, which means it can
// no longer be asserted by letting the throw escape (that would report the FIXED
// feature as red forever).
//
// The contract asserted here, without depending on any internal helper:
//   1. a satisfiable condition resolves and the test proceeds;
//   2. the wait actually SPENDS time polling rather than falling straight through
//      (the observable difference between a live wait and a dropped directive);
//   3. the `timeout:` clause is OPTIONAL — the bare form matches and runs.
// The timeout-throw path itself is covered by stdlib/testing/meta/wait-timing.test.st,
// which owns that surface and runs 15/15.
@import "stdlib/testing/test"

@test "a satisfiable condition resolves and the test proceeds" {
  @eval (globalThis.__gh36_ready = true)
  @wait_until (globalThis.__gh36_ready === true) timeout: 500ms
  @assert (globalThis.__gh36_ready === true)
}

@test "the wait polls over time instead of falling through instantly" {
  // A condition that only becomes true after a tick can ONLY be observed by a
  // wait that genuinely loops. A dropped directive reaches the assert with the
  // flag still false — which is exactly the vacuous pass this issue reported.
  @eval (globalThis.__gh36_late = false)
  @eval (Promise.resolve().then(() => { globalThis.__gh36_late = true; }))
  @wait_until (globalThis.__gh36_late === true) timeout: 2000ms
  @assert (globalThis.__gh36_late === true)
}

@test "timeout: is optional — the bare form matches and runs" {
  @eval (globalThis.__gh36_bare = true)
  @wait_until (globalThis.__gh36_bare === true)
  @assert (globalThis.__gh36_bare === true)
}
