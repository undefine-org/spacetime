/// Signal-call FIRE-by-name runtime tests (PLAN-053).
///
/// The documented fire surface is `@on click { $sig(args) }`
/// (docs/language/reactive-output.md), which lowers to
/// `ST.runMutations(el, { js_statements: ["$sig(args)"] })`. For that to work,
/// `ST.runMutations` MUST recognize a bare signal call `$name(args)` and invoke
/// the callable the signal-call primitive registered for `name`.
///
/// Before PLAN-053 this silently dropped: a `$sig(...)` statement matched
/// neither the `$coll.method()` nor the `$x <- expr` branch, so the signal
/// never fired. These tests pin the by-name invocation rail.

@import "stdlib/testing/test"
@import "stdlib/testing/assertions/binding"

// =============================================================================
// ST.runMutations — fire a registered signal call by name
// =============================================================================

@test "runMutations fires a registered signal call by name" {
  @fixture {
    <div class="sc-fire">x</div>
  }

  @let (el = document.querySelector('.sc-fire'))

  // Register a fake fire fn under the signal registry, exactly as the
  // signal-call primitive does (it stores a callable invokable by signal name).
  // The fire fn records the received payload into a signal on the element.
  @eval (ST.set(el, 'last', null))
  @eval (ST.registerSignalCall('ping', function(payload){ ST.set(el, 'last', payload); }))

  // The documented @on body form: a bare signal call. runMutations must invoke
  // the registered fire fn with the evaluated argument object.
  @eval (ST.runMutations(el, { js_statements: ['$ping({ msg: "hi" })'] }))

  @then $last on .sc-fire should equal (JSON.parse('{"msg":"hi"}'))
}

@test "runMutations fires a signal call with no arguments" {
  @fixture {
    <div class="sc-noarg">x</div>
  }

  @let (el = document.querySelector('.sc-noarg'))

  @eval (ST.set(el, 'count', 0))
  @eval (ST.registerSignalCall('refresh', function(){ ST.set(el, 'count', ST.get(el, 'count') + 1); }))
  @eval (ST.runMutations(el, { js_statements: ['$refresh()'] }))

  @then $count on .sc-noarg should equal 1
}

@test "a signal call resolves $signal args from page state" {
  @fixture {
    <div class="sc-arg">x</div>
  }

  @let (el = document.querySelector('.sc-arg'))

  // A signal-call argument that references local state must resolve through the
  // same transformExpr rail as a mutation RHS.
  @eval (ST.set(el, 'arg', null))
  @eval (ST.registerSignalCall('send', function(payload){ ST.set(el, 'arg', payload); }))
  @eval (window.SpacetimeLocal = (window.SpacetimeLocal || {}))
  @eval (window.SpacetimeLocal.draft = "typed text")
  @eval (ST.runMutations(el, { js_statements: ['$send({ text: $draft })'] }))

  @then $arg on .sc-arg should equal (JSON.parse('{"text":"typed text"}'))
}

@test "an unregistered signal call is a no-op (no throw)" {
  @fixture {
    <div class="sc-missing">x</div>
  }

  @let (el = document.querySelector('.sc-missing'))

  // Firing an unknown signal must not throw — it is a benign no-op (the signal
  // may simply not exist on this page). If runMutations threw, the @eval below
  // would error and fail the test; reaching the assertion proves the no-op.
  @eval (ST.set(el, 'ok', false))
  @eval (ST.runMutations(el, { js_statements: ['$nonexistent({ a: 1 })'] }))
  @eval (ST.set(el, 'ok', true))

  @then $ok on .sc-missing should equal true
}

@test "runMutations fires a signal call whose name carries a dash" {
  @fixture {
    <div class="sc-dash">x</div>
  }

  @let (el = document.querySelector('.sc-dash'))

  // The compiler admits dashed signal names (`:new-session` becomes a
  // data-signal, registered as "new-session"), but the runMutations parse was
  // /^\$(\w+)\s*\(/ — \w excludes the dash, so `$new-session()` matched NO
  // branch and dropped SILENTLY: the bind compiled, the handler bound, the
  // click did nothing. Found live in spell's chat (2026-08-29): a "New
  // session" button that never fired. The parse now reads [\w-]+.
  @eval (ST.set(el, 'fired', false))
  @eval (ST.registerSignalCall('new-session', function(){ ST.set(el, 'fired', true); }))
  @eval (ST.runMutations(el, { js_statements: ['$new-session()'] }))

  @then $fired on .sc-dash should equal true
}

@test "runMutations assigns a dashed signal name ($foo-bar <- x)" {
  @fixture {
    <div class="sc-dashset">x</div>
  }

  @let (el = document.querySelector('.sc-dashset'))

  @eval (ST.runMutations(el, { js_statements: ['$my-flag <- 41 + 1'] }))

  @then $my-flag on .sc-dashset should equal 42
}
