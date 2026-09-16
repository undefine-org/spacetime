/// Signal-handler primitive — the CONSUMER runtime for `@handle` (PLAN-038 W1,
/// pillar 4). Subscribes to a signal's reply and runs the scope's effect clauses.
///
/// `@handle $sig { optimistic{} receive{} final{} }` binds this per element/scope.
/// It listens for the signal's `signal:<name>:reply` event (dispatched by the
/// signal-call primitive), and on each reply runs the matching `receive` arm's
/// effect statements, then the `final` clause for a terminal outcome. The
/// `optimistic` clause runs at fire time (a follow-up wires the fire hook).
///
/// This W1 implementation establishes the subscription + lifecycle wiring. The
/// per-arm variant DISPATCH (matching the decoded variant to its arm + binding
/// the carried payload) lands with the decode codegen (deferred FUP) — until
/// then the handler exposes the raw reply to the scope and runs the final clause.
///
/// @param signal     - the consumed signal binding name (`$add` → key `add`)
/// @param optimistic - the optimistic clause statements (text; may be empty)
/// @param arms       - the receive arms (variant => effect stmts; may be empty)
/// @param sum        - the response sum NAME (the `to <Sum>` through-line; optional)
/// @param final      - the final-clause statements (text; may be empty)
///
/// @yields $handled - whether a reply has been handled at least once
%primitive signal-handler(
  &el,
  signal: binding,
  optimistic: any?,
  arms: any?,
  sum: ident?,
  final: any?
) {
  %emit js {
    (function() {
      // NB: a DISTINCT name — `var el = %&el;` lowers to `var el = el;` at any
      // splice site whose closure already names the element `el`, self-shadowing
      // to `undefined` under var-hoisting. Same trap documented in drag-zones.st.
      var node = %&el;
      var signalKey = (%signal || '').replace(/^\$/, '');
      var handled = false;
      // The most recent `latest` fire owns the scope's optimistic snapshot.
      // Older LiveView replies are legitimate network completions but must not
      // run their confirm/rollback arm over that newer local state.
      var latestGeneration = 0;
      // The receive arms for THIS scope: [{ ctor, bind?, body }] — each maps a
      // decoded variant tag to its effect statements. `optimisticStmts` runs at
      // fire time (before the reply); `finalStmts` runs on a terminal outcome.
      var handleArms = %arms || [];
      var optimisticStmts = %optimistic || '';
      var finalStmts = %final || '';

      function yieldState() {
        %yield handled -> $handled;
      }

      // Run a raw mutation-statement string against this scope, with an optional
      // lexically-bound payload (`bind` => value). Splits on `;` so a multi-
      // statement arm body runs each mutation through ST.runMutations (the shared
      // `@on`/`@effect` mutation runtime), which resolves `$signals` + the local.
      // Run a raw mutation-statement string against this scope. A clause can
      // carry TWO disjoint kinds of binding:
      //   - `bindName` (string) + `bindValue`: a BARE-NAME receive-arm carry
      //     (`Created(todo) => $rows.replace(todo)` -> local `todo`).
      //   - `params` (object): the $-SIGIL signal-call payload (`$move(card,to)`
      //     -> `$card`/`$to` in the optimistic clause).
      // They go in separate runMutations channels so a param value never
      // clobbers a same-named object key / identifier.
      function runStmts(text, bindName, bindValue, params) {
        if (!text || typeof ST === 'undefined' || !ST.runMutations) return;
        var stmts = String(text).split(';').map(function(s) { return s.trim(); }).filter(Boolean);
        if (!stmts.length) return;
        var locals = null;
        if (bindName) { locals = {}; locals[bindName] = bindValue; }
        ST.runMutations(node, { js_statements: stmts }, locals, params || null);
      }

      // Subscribe to the signal's reply. signal-call dispatches
      // `signal:<name>:reply` with the DECODED reply { variant, value, final }.
      // Dispatch to the arm whose `ctor` matches `variant` (or a `_` wildcard),
      // binding the carried `value` to the arm's payload name, then run the
      // `final` clause when the outcome is terminal.
      function onReply(ev) {
        var reply = ev.detail || {};
        if (reply.latest && Number(reply.generation) < latestGeneration) return;
        handled = true;
        if (node) { node.__stLastReply = reply; }
        var arm = null;
        for (var i = 0; i < handleArms.length; i++) {
          var a = handleArms[i];
          if (!a) continue;
          if (a.ctor === reply.variant || a.ctor === '_') { arm = a; break; }
        }
        if (arm) { runStmts(arm.body, arm.bind, reply.value); }
        if (reply.final) { runStmts(finalStmts, null, null); }
        yieldState();
      }

      // Fire hook: run the optimistic clause the instant the signal is invoked,
      // BEFORE the reply lands (an immediate UI update that the reply reconciles).
      // The fire event carries the call payload (`detail.payload` = the signal's
      // named args, e.g. { card, to }); bind it as locals so the optimistic
      // clause's `$card`/`$to` resolve to the call arguments.
      function onFire(ev) {
        var detail = (ev && ev.detail) || {};
        var payload = detail.payload;
        if (detail.latest) latestGeneration = Number(detail.generation) || latestGeneration + 1;
        var params = (payload && typeof payload === 'object' && !Array.isArray(payload)) ? payload : null;
        runStmts(optimisticStmts, null, null, params);
      }

      if (typeof document !== 'undefined') {
        document.addEventListener('signal:' + signalKey + ':reply', onReply);
        document.addEventListener('signal:' + signalKey + ':fire', onFire);
      }
      yieldState();

      %cleanup {
        if (typeof document !== 'undefined') {
          document.removeEventListener('signal:' + signalKey + ':reply', onReply);
          document.removeEventListener('signal:' + signalKey + ':fire', onFire);
        }
      }
    })();
  }

  %exports {
    $handled: bool
  }
}
