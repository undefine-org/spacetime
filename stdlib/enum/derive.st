/// @data derive … : @match — cond-mode value-producing match (PLAN-077 W2)
///
/// A derive whose value is computed by a GUARD-COND @match: first truthy guard
/// wins, the arm's variant constructor becomes the published value
/// (`{type: 'Variant', ...payload}`), and the result republishes through the
/// SAME channel every derive uses (`window.SpacetimeLocal[name]` +
/// `local:<name>:updated`). `ST.watchTypedUnion` (the `@state(when: $x is V)`
/// runtime half) consumes it via its global-channel fallthrough (st.js,
/// PLAN-077 gate-2 fix: nearest-owner-wins — an element-scoped signal of the
/// same name shadows the global one).
///
/// @example
///   @type BadgeState { Blue | Orange | Turquoise | Green }
///   @data derive $badge BadgeState : @match {
///     ($isOrange)       => Orange;
///     ($isUnauthorized) => Blue;
///     ($isTurquoise)    => Turquoise;
///     _                 => Green;
///   }
///
///   // Inline anonymous union (nullary tags only) — no @type needed:
///   @data derive $badge (Blue | Orange | Green) : @match {
///     ($isOrange) => Orange;
///     _           => Green;
///   }
///
/// Grammar (stdlib/enum/capture-types/cond-arms.st): guards are PARENTHESIZED
/// (the lexer has no `=>` token, so a bare expr would swallow the arrow), every
/// arm's `;` is REQUIRED (a payload-less consequence would gobble the next
/// arm's guard as payload args), and the `_` catch-all terminates the chain
/// (E0931 hard-errors when absent — W3). The subject token's absence is the
/// grammar-level discriminator vs dispatch-mode `@match $subject { … }` —
/// exactly as `@type`'s product/sum split works.
///
/// Form discrimination vs plain `@data derive $x T : <expr>;`: this form's
/// `$match:cond_block` value capture can only match a real `@match {…}` arg — but
/// the plain form's `balanced(';')` value can ALSO swallow that same arg as an
/// opaque expr, so shadowing is decided by form SPECIFICITY (element-count
/// scoring, src/syntax/registry.rs). The trailing `;` literal is therefore
/// LOAD-BEARING: it lifts this form to 33 > plain data-derive's 31, so the more
/// specific form is tried first and a non-@match value falls through to the
/// plain form (the @type product/sum doctrine — the grammar is the
/// discriminator, specificity only orders the attempt).

%macro data-derive-match {
  %pipeline-consumed
  %scope file | selector
  %order 250

  // The `: @match { … }` value is ONE inline arg (the events/CST parsers route a
  // value-colon-led `@match {` as a single ARG — PLAN-077 W2 twin routing), so the
  // form captures it with the cond_block grammar rather than a directive body.
  %form {
    @data derive $name:binding $type:typeref? $uitype:inline_variant_type? : $match:cond_block ;
  }

  %binds {
    derive-match(name: $name, arms: $match) -> { $value as $name }
  }

  %registers binding($name, type: $type) {
    value: $name
    computed: true
    source_kind: "derive"
    source_static: false
  }
}

/// Derive Match Primitive (PLAN-077 W2; PLAN-133 sexpr-arms migration)
///
/// Arms arrive wrapped in the cond_block record (the form's `$match:cond_block`
/// capture): `{ arms: [{ guard: "_" | {expr: "…"}, cons: { name: "Variant",
/// payload: [{arg: "…"}] } }] }`. Since PLAN-133 the param is `sexpr-arms`:
/// at BUILD time the compiler lowers every embedded guard/payload expression
/// (real parse — dep discovery, `$sig` → dep, arrow params local, pipes,
/// known-filter rewrite) and the record gains `f` + `deps` fields per arm.
/// The runtime evaluates `f(...depVals)` — the dep-discovery regex, the
/// quote-aware char-scan, the pipe desugar, and `new Function` eval are
/// deleted (the third verbatim copy of the BUG-273 machinery).
///
/// The lifecycle lives ONCE in the prelude (`__stDeriveMatch`); the watch and
/// publish channels are the shared ones (`%uses signal-watch` for
/// `__stWatchSignals`, `%uses data-channel` for `__stPublish`). First truthy
/// guard wins; the arm's variant constructor becomes the published value
/// (`{type: 'Variant', ...payload}`). `ST.watchTypedUnion` consumes it via
/// its global-channel fallthrough (st.js, PLAN-077 gate-2 fix:
/// nearest-owner-wins — an element-scoped signal of the same name shadows
/// the global one).
///
/// Payload field naming (W2 convention): a constructor arg that is a BARE
/// `$signal` (`Failed($error)`) publishes under the signal's name
/// (`{type:'Failed', error: …}`) so `@state(when: $fs is Failed { $error })`
/// destructures it by the same name. A single non-signal arg publishes as
/// `value`; positional args in a multi-arg construct publish as `arg0`,
/// `arg1`, …
///
/// @param name - signal name to publish under
/// @param arms - cond_block arms record (compiled: f + deps per arm)
///
/// @yields $value - the derived `{type, ...}` union value
%primitive derive-match(
  name: ident,
  arms: sexpr-arms
) %uses data-channel, signal-watch {
  %emit prelude-js {
    // The whole derive-match lifecycle, once per page. Semantics moved here
    // VERBATIM from the per-usage template — including the per-guard
    // containment rule (a throwing guard logs and counts as FALSY so the scan
    // continues; the REQUIRED trailing `_` — E0931 — keeps the chain total).
    window.__stDeriveMatch = window.__stDeriveMatch || function (dn, arms) {
      // --- guard/arg normalization -----------------------------------------
      // cond_guard yields "_" (bare catch-all) or {expr: "…"} (paren guard).
      const guardText = (g) => (typeof g === 'string') ? g : (g && typeof g.expr === 'string' ? g.expr : '');
      const isCatchAll = (g) => guardText(g).trim() === '_';
      const consArgs = (c) => (c && Array.isArray(c.payload)) ? c.payload : [];
      const depVals = (deps) => deps.map((d) => (typeof SpacetimeLocal !== 'undefined') ? SpacetimeLocal[d] : undefined);

      let value = null;

      const recompute = () => {
        const arm = arms.find(a => {
          if (isCatchAll(a.guard)) return true;
          const g = a.guard;
          if (!g || typeof g.f !== 'function') return false;
          try {
            return !!g.f.apply(null, depVals(g.deps || []));
          } catch (e) {
            console.error('[derive-match ' + dn + ' guard]', e);
            return false;
          }
        });
        if (!arm) return; // no guard truthy and no catch-all (E0931 catches at compile)
        const cons = arm.cons || {};
        if (!cons.name) return;
        value = { type: cons.name };
        const cargs = consArgs(cons);
        if (cargs.length) {
          try {
            cargs.forEach((x, i) => {
              const argText = (x && x.arg) || '';
              const v = (typeof x.f === 'function') ? x.f.apply(null, depVals(x.deps || [])) : undefined;
              // Field naming: bare $sig -> the signal's name (destructure by
              // the same name); single non-signal -> 'value'; else 'arg{i}'.
              const bare = /^\$([A-Za-z_][A-Za-z0-9_]*)$/.exec(argText.trim());
              const field = bare ? bare[1] : (cargs.length === 1 ? 'value' : 'arg' + i);
              value[field] = v;
            });
          } catch (e) {
            console.error('[derive-match ' + dn + ' payload]', e);
            return;
          }
        }
        // Republish as a first-class global signal — the shared data-channel
        // (`%uses data-channel`; watchTypedUnion consumes it via its
        // global-channel fallthrough, st.js PLAN-077 gate-2 fix).
        window.__stPublish(dn, value);
      };

      // Watch every dep discovered at COMPILE time (exported on each arm's
      // guard/payload fields) through the shared watcher (`%uses
      // signal-watch`): export bridging + data-registry sources included,
      // initial compute after init settles.
      const deps = [];
      arms.forEach((a) => {
        ((a.guard && a.guard.deps) || []).forEach((d) => { if (!deps.includes(d)) deps.push(d); });
        consArgs(a.cons).forEach((x) => ((x && x.deps) || []).forEach((d) => { if (!deps.includes(d)) deps.push(d); }));
      });
      window.__stWatchSignals(deps, recompute);
    };
  }

  %emit js {
    (() => {
      const value = null;
      window.__stDeriveMatch('%name', (%arms && Array.isArray(%arms.arms)) ? %arms.arms : []);

      %yield value -> $value;
    })()
  }

  %exports {
    $value: any
  }
}
