# Primitive Authoring Rules (PLAN-133)

The rules that keep `%emit` templates from re-growing the bug classes
PLAN-133 deleted (BUG-273's runtime regex rewriter, BUG-274's
branch-scoped error-path crash). These are *generation* rules, not gates:
follow them and the bug class is unrepresentable — no validator needed.

## 1. Expressions are `sexpr`, never `string`

A param that authors write as an expression — `expr: "total($rates)"`,
`with: "item.price * 2"` — must be declared `sexpr`, not `string`.

`sexpr` is compiled at build time: the compiler parses the expression
(SWC), binds arrow params as locals, turns unbound `$name` reads into
dependencies, rewrites known page filters to `ST.filters.*`, and injects
synthetic params (`expr__fn`, `expr__deps`, `expr__step_fn`, …) your
template splices verbatim.

A `string` param means the *runtime* must re-derive those semantics from
raw text — char scans, regexes, `new Function`. That machinery is where
BUG-273 lived (an arrow param `$r` rewritten to a signal read →
SyntaxError on every recompute). It is deleted. Do not rebuild it.

```spacetime
// BAD  — runtime must guess what this text means
%primitive derived-signal(name: ident, expr: string, ...) { … }

// GOOD — compiler owns meaning; runtime gets a callable
%primitive derived-signal(name: ident, expr: sexpr, ...) {
  %emit js {
    __stDerive('%name', %expr__deps, %expr__fn);
  }
}
```

## 2. The runtime owns behavior; per-usage code is calls + data

A lifecycle (fetch, watch, publish, error reporting, scheduling) belongs
in the primitive's `%emit prelude-js` as ONE function, guarded with
`window.__stX = window.__stX || …`. The per-usage `%emit js` emits a
single call carrying config — and declares **no locals**.

Locals in per-usage code are the habitat of BUG-274: a `const` declared
in a fetch success-branch, read from the `catch` → ReferenceError that
crashed the error reporter itself. A config call cannot have that shape.

```spacetime
// GOOD — data-source today
%emit prelude-js {
  window.__stDataSource = window.__stDataSource || function (cfg) { /* the whole lifecycle */ };
}
%emit js {
  window.__stDataSource({ name: %name, src: %src, default: %default, refresh: %refresh });
}
```

When migrating a lifecycle into a prelude, move the code **verbatim** —
load-bearing ordering comments travel with it (e.g. the `setData` →
`SpacetimeLocal` microtask ordering that `@each` depends on).

## 3. Shared prelude code goes through `%uses`, never copy-paste

When two primitives need the same prelude code (a module, a store init, a
resolution chain), put it in a small param-free *module primitive* and
declare `%uses`. User ruling (2026-08-05): this applies even with **one**
consumer — the architecture is cleaner and the next consumer never copies.
Do not keep a watch list of "single-owner today" helpers.

```spacetime
%primitive locale() {
  %emit prelude-js {
    window.__stResolveLocale = window.__stResolveLocale || function () { … };
  }
}

%primitive data-source(...) %uses locale, data-store { … }
```

The named primitive's prelude reaches any page expanding the consumer —
emitted once, dependency-before-dependent, prelude-only (its per-usage
code, exports and html never run). Unknown targets and cycles are
compile errors.

**The `%uses` target MUST be param-free** (or all-defaults): it is invoked
with no arguments, so a target with required params silently yields no
prelude. If the helper you want lives inside a full primitive's prelude
(the way `__stWatchSignals` lived inside derived-signal), EXTRACT it to a
module (`signal-watch`) and have the original owner `%uses` it too.

Copy-pasting the code instead *works on day one* because prelude dedup is
per primitive name — and then the copies drift. `dev-editable.st` shipped
a two-step locale chain while `source.st` shipped three steps; that drift
is exactly the BUG-274 pattern re-forming. `%uses` makes the shared code
exist once, so it cannot drift. Watch for the drift SHAPE too: a
defensive re-init (`window.__stX = window.__stX || …`) inside per-usage
`%emit js` is a second copy — delete it; the module prelude is the
guarantee.

Do **not** `%uses` when the read means something different in the
consumer — e.g. `dev-inspect.st` reads `window.__st_locale` to ask "is a
locale active?" (bails when falsy), which is not "resolve the current
locale". Wrapping it in the fallback chain would change behavior.

## 4. The sexpr family is data, declared by the param type

- `sexpr` — global context: bare `$` is E0957; unbound `$sig` is a dep.
- `sexpr-row` — row context: `$`/`$.field` is the row (`item`); a param
  named exactly `$` binds the row (the reduce shape `(s, $) => …`); the
  query-helper vocabulary (QUERY_HELPERS ↔ computed-source's prelude
  table — keep the two lists in sync) rewrites to `ST.filters.*`. An
  object-literal value (`map: { label: $.name, kind: "photo" }`) lowers
  per-leaf; a top-level arrow keeps the author's arrow with deps appended
  (one calling convention: `step(acc, item, …deps)`).
- `sexpr-arms` — a cond_block record (derive-match): every embedded
  `expr:`/`arg:` string is lowered and the record gains `f` + `deps`
  fields; the runtime evaluates `f(...depVals)`.

## Reference

- Mechanism: `src/emit/st_expr.rs` (sexpr lowering),
  `src/pipeline/expand.rs::order_with_uses` (%uses closure).
- Examples: `stdlib/primitives/data/derived-signal.st` (sexpr + prelude
  lifecycle), `stdlib/primitives/data/source.st` (prelude lifecycle +
  `%uses locale, data-store`), `stdlib/primitives/locale.st` and
  `stdlib/primitives/data/data-store.st` (module primitives).
- Plan & rulings: `!tasks/plans/PLAN-133-generation-not-gates-compiled-expression.org`.
- Follow-ups: FUP-175 (remaining cross-primitive dedup roster).
