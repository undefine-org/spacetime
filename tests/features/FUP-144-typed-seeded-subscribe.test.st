/// FUP-144 — `@data subscribe` parity: optional `$type:typeref?` slot + an
/// explicit pre-first-diff seed (mirrors sibling @data kinds). The seed is the
/// value the subscribed signal holds BEFORE the first server assign diff; the
/// diff then overwrites it through the SAME registered setter, so the seed is
/// never authoritative server data.
///
/// Guards the silent-degrade regression found in PLAN-118: a token between
/// `$name` and `from` used to fall through to a plain local (no subscription
/// emitted). These tests assert the TYPED forms still register a real
/// `__stLiveSubscriptions` setter.
@import "stdlib/testing/test"
@import "stdlib/macros/data-kind"
@import "stdlib/macros/host"

@host $counter : live("TestWeb.CounterLive") ;

@type Env { email: string; valid: boolean; }

// FUP-149 (PLAN-119 W3): a NESTED envelope to prove the type-derived zero-value
// seed is built structurally (no balanced-capture parser, so nested works).
@type Errors { email: string; }
@type Envelope { errors: Errors; count: number; ok: boolean; }

// Typed bodyless subscribe (type slot only, no seed).
@data subscribe $typedCount number from $counter : count ;

// FUP-149: typed bodyless subscribe of a NESTED type, NO explicit seed. The
// compiler auto-seeds its zero value ({ errors: { email: "" }, count: 0, ok:
// false }) so nested reads are total from mount.
@data subscribe $env Envelope from $counter : env ;

// Typed + scalar seed.
@data subscribe $seededCount number from $counter : tally {
  initial: 42;
}

// Typed + one-level object seed (the forms-envelope-shaped case, flattened).
@data subscribe $form Env from $counter : form {
  initial: { "email": "seed@x.com", "valid": false };
}

@test "typed bodyless subscribe still registers a real subscription setter" {
  @fixture { <div class="t">x</div> }

  // The degrade bug would make this key absent (fell through to a plain local).
  @eval (window.__fup144_typed = typeof window.__stLiveSubscriptions["TestWeb.CounterLive"].count[0])
  @assert (window.__fup144_typed === "function")
    "a typed subscribe registers a (module, assign) setter, not a degraded local"

  @eval (window.__stLiveSubscriptions["TestWeb.CounterLive"].count[0](5))
  @eval (window.__fup144_typed_val = window.SpacetimeLocal.typedCount)
  @assert (window.__fup144_typed_val === 5)
    "the typed subscribe's setter writes the subscribed local target"
}

@test "explicit scalar seed is the pre-first-diff value, then a diff overwrites it" {
  @fixture { <div class="t">x</div> }

  // BEFORE any server diff, the signal holds its seed.
  @eval (window.__fup144_seed = window.SpacetimeLocal.seededCount)
  @assert (window.__fup144_seed === 42)
    "the seed is the pre-first-diff value"

  // The server's first diff arrives through the registered setter and wins.
  @eval (window.__stLiveSubscriptions["TestWeb.CounterLive"].tally[0](100))
  @eval (window.__fup144_after = window.SpacetimeLocal.seededCount)
  @assert (window.__fup144_after === 100)
    "the first server diff overwrites the seed (seed is never authoritative)"
}

@test "one-level object seed shapes the signal before the first diff" {
  @fixture { <div class="t">x</div> }

  @eval (window.__fup144_obj = window.SpacetimeLocal.form)
  @assert (window.__fup144_obj.email === "seed@x.com")
    "the object seed's fields are present before any server diff"
  @assert (window.__fup144_obj.valid === false)
    "the object seed carries every declared field"

  // A server diff replaces the whole envelope through the same setter.
  @eval (window.__stLiveSubscriptions["TestWeb.CounterLive"].form[0]({ email: "real@x.com", valid: true }))
  @eval (window.__fup144_obj2 = window.SpacetimeLocal.form)
  @assert (window.__fup144_obj2.valid === true)
    "the server envelope overwrites the seed"
}

@test "FUP-149: a typed bodyless subscribe auto-seeds its type's zero value (nested, no BUG-228)" {
  @fixture { <div class="t">x</div> }

  // No explicit seed was written; the compiler derived the zero value from
  // Envelope { errors: Errors { email }, count, ok }.
  @eval (window.__fup149_env = window.SpacetimeLocal.env)
  @assert (window.__fup149_env !== null && typeof window.__fup149_env === "object")
    "a typed bodyless subscribe is seeded with a well-shaped object, not null"

  // The NESTED field is present and reachable WITHOUT a guard — this is the
  // structural-synth win (a hand-written nested `{ initial: {…} }` literal is
  // still blocked by BUG-228, but the type-derived seed sidesteps the parser).
  @assert (window.__fup149_env.errors.email === "")
    "the nested zero value is present: env.errors.email is total from mount"
  @assert (window.__fup149_env.count === 0)
    "a number field zeroes to 0"
  @assert (window.__fup149_env.ok === false)
    "a boolean field zeroes to false"

  // A server diff still overwrites the whole envelope through the same setter.
  @eval (window.__stLiveSubscriptions["TestWeb.CounterLive"].env[0]({ errors: { email: "e" }, count: 3, ok: true }))
  @eval (window.__fup149_env2 = window.SpacetimeLocal.env)
  @assert (window.__fup149_env2.count === 3)
    "the first server diff overwrites the derived seed"
}

