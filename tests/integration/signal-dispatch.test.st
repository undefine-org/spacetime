/// Signal decode + @handle dispatch runtime tests (PLAN-038 FUP-080).
///
/// Verifies the reactive-output runtime end to end at the JS layer:
///   1. ST.runMutations honors a lexically-bound `locals` payload (the carried
///      value of a decoded variant, e.g. `Created(todo)`), so an arm body like
///      `$rows <- $rows.concat(todo)` resolves `todo` to the reply value.
///   2. The decode → dispatch round-trip: a simulated `signal:<name>:reply`
///      event carrying a decoded `{ variant, value }` runs the matching arm's
///      mutation against page state.
///
/// These exercise the SHARED mutation runtime (ST.runMutations) the @handle
/// signal-handler primitive lowers to — the same rail as @on / @effect.

@import "stdlib/testing/test"
@import "stdlib/testing/assertions/binding"

// =============================================================================
// ST.runMutations — local payload binding (the @handle arm carry)
// =============================================================================

@test "runMutations binds a local payload into the mutation RHS" {
  @fixture {
    <div class="rm-local">x</div>
  }

  @let (el = document.querySelector('.rm-local'))

  // Seed an array signal, then run a mutation whose RHS references a LOCAL
  // (`item`) — the carried payload of a decoded variant. The locals map makes
  // `item` resolve to the JSON-encoded value inside the eval'd RHS.
  @eval (ST.set(el, 'rows', []))
  @eval (ST.runMutations(el, { js_statements: ['$rows <- $rows.concat(item)'] }, { item: { id: 7, title: 'hi' } }))

  @then $rows on .rm-local should have_length 1
}

@test "runMutations local payload survives as a structured object" {
  @fixture {
    <div class="rm-obj">x</div>
  }

  @let (el = document.querySelector('.rm-obj'))

  @eval (ST.set(el, 'last', null))
  @eval (ST.runMutations(el, { js_statements: ['$last <- payload'] }, { payload: { ok: true, n: 42 } }))

  @then $last on .rm-obj should equal (JSON.parse('{"ok":true,"n":42}'))
}

@test "runMutations without locals still resolves $signals (no regression)" {
  @fixture {
    <div class="rm-noloc">x</div>
  }

  @let (el = document.querySelector('.rm-noloc'))

  @eval (ST.set(el, 'n', 1))
  @eval (ST.runMutations(el, { js_statements: ['$n <- $n + 1'] }))

  @then $n on .rm-noloc should equal 2
}

@test "runMutations binds a $-sigil signal param local over a missing global" {
  @fixture {
    <div class="rm-sig">x</div>
  }

  @let (el = document.querySelector('.rm-sig'))

  // The @handle optimistic clause references the signal's call params with the
  // $-sigil (`$card`/`$to`). When the fire payload is bound as locals, a `$card`
  // hole must resolve to the payload value, NOT to a nonexistent
  // `SpacetimeLocal.card`. Regression: an unbound `$card` matched nothing, so a
  // drag-and-drop optimistic move silently no-op'd (demos/liveview-kanban).
  @eval (ST.set(el, 'cards', JSON.parse('[{"id":"c1","col":"todo"},{"id":"c2","col":"todo"}]')))
  @eval (ST.runMutations(el, { js_statements: ['$cards <- $cards.map(c => c.id == $card ? Object.assign({}, c, { col: $to }) : c)'] }, null, { card: 'c1', to: 'doing' }))

  @then $cards on .rm-sig should equal (JSON.parse('[{"id":"c1","col":"doing"},{"id":"c2","col":"todo"}]'))
}

@test "a $-sigil param local does not clobber a member access of the same name" {
  @fixture {
    <div class="rm-sig2">x</div>
  }

  @let (el = document.querySelector('.rm-sig2'))

  // `$to` substitutes, but a member access `.to` on an object must be left
  // alone (the substitution is anchored to the `$` sigil).
  @eval (ST.set(el, 'out', null))
  @eval (ST.runMutations(el, { js_statements: ['$out <- ({ to: $to, kept: ($to) })'] }, null, { to: 'doing' }))

  @then $out on .rm-sig2 should equal (JSON.parse('{"to":"doing","kept":"doing"}'))
}

// =============================================================================
// Decode → dispatch round-trip (variant → arm → mutation)
// =============================================================================

@test "a decoded variant reply runs the matching handle arm" {
  @fixture {
    <div class="hd-app">app</div>
  }

  @let (app = document.querySelector('.hd-app'))

  // Mirror the emitted signal-handler dispatch: a reply listener that matches
  // the decoded `variant` to an arm and runs its body with the carried payload.
  @eval (ST.set(app, 'rows', []))
  @eval (eval("(function(){ var el = document.querySelector('.hd-app'); var arms = [{ ctor: 'Created', bind: 'todo', body: '$rows <- $rows.concat(todo);' }]; el.__on = function(reply){ var arm = null; for (var i=0;i<arms.length;i++){ if(arms[i].ctor===reply.variant||arms[i].ctor==='_'){arm=arms[i];break;} } if(arm){ var st=arm.body.split(';').map(function(s){return s.trim();}).filter(Boolean); var loc={}; loc[arm.bind]=reply.value; ST.runMutations(el, {js_statements: st}, loc); } }; })()"))

  // Fire a decoded Created reply carrying a todo.
  @eval (document.querySelector('.hd-app').__on({ variant: 'Created', value: { id: 1, title: 'milk' }, final: true }))

  @then $rows on .hd-app should have_length 1
}

@test "ws message-type pattern decodes to its variant" {
  @fixture {
    <div class="ws-app">app</div>
  }

  @let (app = document.querySelector('.ws-app'))

  // Mirror the signal-call ws decode: arms keyed by message TYPE (string `lit`
  // pattern). armMatches resolves a string pattern against env.type.
  @eval (ST.set(app, 'preview', null))
  @eval (eval("(function(){ var arms=[{pat:{lit:'previewed'},cons:{variant:{ctor:'Previewed',payload:'$.payload'}}},{pat:{lit:'chosen'},cons:{variant:{ctor:'Chosen',payload:'$.payload'}}}]; function armMatches(pat,env){ if(pat.lit!=null) return String(env.type!=null?env.type:env.status)===String(pat.lit); return false; } function evalPayload(expr,env){ var s=String(expr).replace(/\\bas\\s+[A-Za-z_][\\w.<>\\[\\]]*\\s*$/,'').trim().replace(/\\$\\./g,'__env.'); try{ return (new Function('__env','return ('+s+');'))(env); }catch(e){ return s; } } window.__wsDecode=function(env){ for(var i=0;i<arms.length;i++){ if(armMatches(arms[i].pat,env)){ var c=arms[i].cons.variant; return {variant:c.ctor, value:evalPayload(c.payload,env)}; } } return {variant:null,value:env}; }; })()"))

  // A ws envelope { type:'previewed', payload:{html:'<b>hi</b>'} } decodes to Previewed.
  @let (decoded = window.__wsDecode({ type: 'previewed', payload: { html: '<b>hi</b>' } }))
  @eval (ST.set(app, 'variant', window.__wsDecode({ type: 'previewed', payload: { html: 'x' } }).variant))

  @then $variant on .ws-app should equal "Previewed"
}

@test "a non-matching variant falls through to the wildcard arm" {
  @fixture {
    <div class="hd-wild">app</div>
  }

  @let (app = document.querySelector('.hd-wild'))

  @eval (ST.set(app, 'err', ''))
  @eval (eval("(function(){ var el = document.querySelector('.hd-wild'); var arms = [{ ctor: 'Created', bind: 'todo', body: '$rows <- $rows.concat(todo);' }, { ctor: '_', bind: 'msg', body: '$err <- msg;' }]; el.__on = function(reply){ var arm = null; for (var i=0;i<arms.length;i++){ if(arms[i].ctor===reply.variant||arms[i].ctor==='_'){arm=arms[i];break;} } if(arm){ var st=arm.body.split(';').map(function(s){return s.trim();}).filter(Boolean); var loc={}; loc[arm.bind]=reply.value; ST.runMutations(el, {js_statements: st}, loc); } }; })()"))

  @eval (document.querySelector('.hd-wild').__on({ variant: 'Failed', value: 'boom', final: true }))

  @then $err on .hd-wild should equal "boom"
}


// `latest` is stronger than transport cancellation: LiveView and websocket
// replies may arrive after a later fire. The handler admits only the generation
// owning its optimistic snapshot, so an old rejection cannot roll UI back.
@test "latest reply cannot roll back a later optimistic fire" {
  @fixture {
    <div class="hd-latest">app</div>
  }

  @let (app = document.querySelector('.hd-latest'))
  @eval (ST.set(app, 'count', 2))
  @eval (eval("(function(){ var el=document.querySelector('.hd-latest'), latest=0; el.__reply=function(reply){ if(reply.latest && Number(reply.generation)<latest)return; if(reply.variant==='Failed')ST.runMutations(el,{js_statements:['$count <- $before']}); }; el.__fire=function(generation){ latest=generation; ST.set(el,'before',ST.get(el,'count')); ST.runMutations(el,{js_statements:['$count <- $count + 1']}); }; })()"))

  @eval (app.__fire(1))
  @eval (app.__fire(2))
  @eval (app.__reply({ variant: 'Failed', generation: 1, latest: true }))

  @then $count on .hd-latest should equal 4
}

