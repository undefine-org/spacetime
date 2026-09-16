/// Derived Signal Primitive (PLAN-023 W3, FEAT-046; PLAN-133 structural rebuild)
///
/// Backs `@data derive` and `@data fold`. The derivation is a `sexpr`: a
/// Spacetime signal expression parsed and lowered AT COMPILE TIME (SWC parse,
/// scope-aware name resolution — see src/emit/st_expr.rs). The compiler emits
/// a positional-parameter function and the dependency list as data:
///
///   $comRecords.filter(($r) => $r.status == "open").length
///     → fn:   (comRecords) => comRecords.filter(($r) => $r.status == "open").length
///       deps: ["comRecords"]
///
/// An arrow param (`$r`, `item`) is a LOCAL — structurally, because it is a
/// binding node in the parse; there is no runtime rewriting to corrupt it
/// (BUG-273's entire mechanism is deleted: the deps regex, the quote-aware
/// char-scan, the lambdaBound heuristic, and `new Function` runtime eval).
/// A typo'd `$signal` is a compile-time unknown, not a silent never-fire.
///
/// The runtime lifecycle (watch deps → recompute → republish through the dual
/// channel) lives ONCE in the prelude below; each usage emits only a call.
///
/// @param name  - signal name to publish under
/// @param expr  - derivation expression (sexpr), may reference $signals
/// @param fold  - when true, the expr is a reducer body over `acc`/`item`
///                and accumulates statefully; when false, stateless recompute
/// @param initial - initial accumulator value for fold
///
/// @yields $value - the derived result
%primitive derived-signal(
  name: ident,
  expr: sexpr,
  fold: bool = false,
  source: ident?,
  initial: expr? = null
) %uses data-channel, signal-watch {
  %emit prelude-js {
    // Publish a derived value as a first-class global signal (dual channel:
    // SpacetimeLocal proxy, else _localState + event). ONE implementation of
    // Stateless derived signal: watch the compiler-discovered deps, call the
    // compiler-lowered compute with dep values POSITIONALLY, republish.
    window.__stDerive = window.__stDerive || function (name, deps, compute) {
      window.__stWatchSignals(deps, function () {
        var vals = deps.map(function (d) {
          return (typeof SpacetimeLocal !== 'undefined') ? SpacetimeLocal[d] : undefined;
        });
        var value;
        try {
          value = compute.apply(null, vals);
        } catch (e) {
          console.error('[derived-signal ' + name + ']', e);
          return;
        }
        window.__stPublish(name, value);
      });
    };

    // Stateful fold: reduce the source array through the compiler-lowered
    // step (acc, item, ...deps); watches the source AND any $signals the
    // step reads (e.g. `acc + item * $rate` recomputes when $rate changes).
    window.__stFold = window.__stFold || function (name, source, initial, deps, stepFn) {
      var watched = [source].concat(deps).filter(Boolean);
      window.__stWatchSignals(watched, function () {
        var value;
        try {
          var arr = (typeof SpacetimeLocal !== 'undefined') ? SpacetimeLocal[source] : undefined;
          var seed = (initial != null) ? initial : 0;
          var extra = deps.map(function (d) {
            return (typeof SpacetimeLocal !== 'undefined') ? SpacetimeLocal[d] : undefined;
          });
          value = Array.isArray(arr)
            ? arr.reduce(function (acc, item) { return stepFn.apply(null, [acc, item].concat(extra)); }, seed)
            : seed;
        } catch (e) {
          console.error('[derived-signal ' + name + ']', e);
          return;
        }
        window.__stPublish(name, value);
      });
    };
  }

  %emit js {
    (() => {
      const dn = '%name';
      const isFold = %fold;
      const foldSource = '%source' || '';
      const initialVal = %initial;
      const value = isFold ? initialVal : null;

      if (isFold) {
        window.__stFold(dn, foldSource, initialVal, %expr__deps, %expr__step_fn);
      } else {
        window.__stDerive(dn, %expr__deps, %expr__fn);
      }

      %yield value -> $value;
    })()
  }

  %exports {
    $value: any
  }
}
