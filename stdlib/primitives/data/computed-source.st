/// Computed Source Primitive
///
/// Backs `@data query`: an array pipeline (where / sort / limit / map / reduce)
/// over a SOURCE signal. Per-item expressions are `sexpr-row` params — compiled
/// at BUILD time (PLAN-133): `$`/`$.field` is the current row (`item`), unbound
/// `$sig` is a watched dependency, arrow params bind locally, the query-helper
/// vocabulary rewrites to `ST.filters.*`. The runtime receives real functions
/// and dep lists — the old `rewriteExpr` char-scan (no lambda exemption: the
/// BUG-273 class), the pipe desugar, the constant-vs-expr leaf heuristic, and
/// every `new Function` are deleted.
///
/// The lifecycle (pipeline, dual-channel subscription, publish) lives ONCE in
/// the prelude (`__stComputed`); each usage emits a single config object.
///
/// @param name - Name for the computed value
/// @param source - Source signal NAME to derive from (root or member path)
/// @param filter - Optional `where` predicate (row expr: `$.field` / `$sig`)
/// @param sortBy - Optional sort key expr (e.g., $.price)
/// @param sortDir - Optional sort direction (asc or desc)
/// @param limit - Optional limit count
/// @param transform - Optional `map` projection — an expr or the declarative
///   object form `{ label: $.name, kind: "photo" }` (leaves compile as exprs,
///   quoted constants survive verbatim)
/// @param reduce - Optional reduce step — a bare step expr (`acc + item.price`)
///   or an arrow (`(s, $) => s + $.price`); `$` binds the row
/// @param initial - Optional initial accumulator for reduce
///
/// @yields $value - The computed result
%primitive computed-source(
  name: ident,
  source: string,
  filter: sexpr-row?,
  sortBy: sexpr-row?,
  sortDir: ident?,
  limit: number?,
  transform: sexpr-row?,
  reduce: sexpr-row?,
  initial: expr?
) %uses data-channel {
  %emit prelude-js {
    // The query-helper vocabulary: pure data-shaping fns callable bare inside
    // where/sort/map exprs (the compiler rewrites known calls to
    // ST.filters.<name>). Case-insensitive string predicates by design
    // (search UX). Installed on ST.filters — ONE namespace with page @fns;
    // guarded so a page-defined filter of the same name wins (fn-registry
    // assigns unconditionally, after preludes run). Keep in sync with
    // QUERY_HELPERS in src/emit/metasystem_codegen.rs.
    window.__stQueryHelpers = window.__stQueryHelpers || {
      includes: (s, sub) => String(s == null ? '' : s).toLowerCase().indexOf(String(sub == null ? '' : sub).toLowerCase()) !== -1,
      contains: (s, sub) => String(s == null ? '' : s).toLowerCase().indexOf(String(sub == null ? '' : sub).toLowerCase()) !== -1,
      starts: (s, p) => String(s == null ? '' : s).toLowerCase().startsWith(String(p == null ? '' : p).toLowerCase()),
      ends: (s, p) => String(s == null ? '' : s).toLowerCase().endsWith(String(p == null ? '' : p).toLowerCase()),
      basename: (s) => String(s == null ? '' : s).split('/').pop(),
      titleize: (s) => String(s == null ? '' : s).replace(/[-_]+/g, ' ').replace(/(^|\s)\w/g, (c) => c.toUpperCase()),
      lower: (s) => String(s == null ? '' : s).toLowerCase(),
      upper: (s) => String(s == null ? '' : s).toUpperCase(),
    };
    if (typeof ST !== 'undefined') {
      ST.filters = ST.filters || {};
      for (var __q in window.__stQueryHelpers) {
        if (!Object.prototype.hasOwnProperty.call(ST.filters, __q)) {
          ST.filters[__q] = window.__stQueryHelpers[__q];
        }
      }
    }

    // The whole query-pipeline lifecycle, once per page. Semantics moved here
    // VERBATIM from the per-usage template — including the load-bearing
    // dual-channel subscription note below. cfg slots: { f, deps } pairs for
    // filter/sortKey/map/reduce (f = compiled fn, deps = watched signals).
    window.__stComputed = window.__stComputed || function (cfg) {
      const computedName = cfg.name;
      const sourcePath = cfg.source;
      // The source may be a bare signal (`from $products`) or a member path
      // (`from $assetDoc.images`). Resolve the root signal + walk the tail.
      const sourceRoot = String(sourcePath).split('.')[0];
      const sourceTail = String(sourcePath).split('.').slice(1);
      const walkSource = (rootData) => sourceTail.reduce((o, k) => (o == null ? o : o[k]), rootData);
      let value = null;
      let lastSource = [];

      // Dep values are read from the global signal store at COMPUTE time
      // (the compiled fns take them as trailing params).
      const depVals = (deps) => deps.map((d) => (typeof SpacetimeLocal !== 'undefined') ? SpacetimeLocal[d] : undefined);
      const allDeps = [];
      [cfg.filter, cfg.sortKey, cfg.map, cfg.reduce].forEach((slot) => {
        if (slot && slot.deps) slot.deps.forEach((d) => { if (!allDeps.includes(d)) allDeps.push(d); });
      });

      const compute = (sourceData) => {
        lastSource = sourceData;
        let result = sourceData;

        // Ensure we have an array to work with
        if (!Array.isArray(result)) {
          result = result != null ? [result] : [];
        }

        // Apply filter (where clause)
        if (cfg.filter && cfg.filter.f) {
          const f = cfg.filter.f;
          const dv = depVals(cfg.filter.deps);
          result = result.filter((item) => { try { return f(item, ...dv); } catch (e) { return false; } });
        }

        // Apply sort
        if (cfg.sortKey && cfg.sortKey.f) {
          const key = cfg.sortKey.f;
          const kv = depVals(cfg.sortKey.deps);
          const ascending = cfg.sortDir !== 'desc';
          result = [...result].sort((a, b) => {
            let aVal, bVal;
            try { aVal = key(a, ...kv); } catch (e) { aVal = undefined; }
            try { bVal = key(b, ...kv); } catch (e) { bVal = undefined; }
            if (aVal < bVal) return ascending ? -1 : 1;
            if (aVal > bVal) return ascending ? 1 : -1;
            return 0;
          });
        }

        // Apply limit
        if (cfg.limit != null && cfg.limit > 0) {
          result = result.slice(0, cfg.limit);
        }

        // Apply transform (map)
        if (cfg.map && cfg.map.f) {
          const f = cfg.map.f;
          const dv = depVals(cfg.map.deps);
          result = result.map((item) => { try { return f(item, ...dv); } catch (e) { return item; } });
        }

        // Apply reduce (collapses array to scalar)
        if (cfg.reduce && cfg.reduce.f) {
          const rf = cfg.reduce.f;
          const dv = depVals(cfg.reduce.deps);
          const step = (acc, item) => rf(acc, item, ...dv);
          result = cfg.initial !== null && cfg.initial !== undefined
            ? result.reduce(step, cfg.initial)
            : result.reduce(step);
        }

        value = result;

        // Publish the computed result as a first-class global signal so that
        // file-scope bindings (`.sel { text: $computedName }`) and downstream
        // computed-of-computed chains can react — ONE implementation in the
        // data-channel module (`%uses data-channel`).
        window.__stPublish(computedName, value);
      };

      // Re-run the pipeline against the LAST source data (for dep-triggered
      // recomputes, where only an external $sig changed, not the source).
      const recompute = () => compute(lastSource);

      // Subscribe to BOTH data channels unconditionally — same protocol as
      // each-with-templates (stdlib/primitives/data/each.st). Branching on a
      // single channel at IIFE-execution time is fragile: if the source signal
      // is not yet populated when this primitive runs, the unmatched branch is
      // skipped and later mutations on the other channel are never observed.
      // Registering both listeners makes the computed react regardless of init
      // order, supporting remote (@data src:"/api") and local (@data src:inline
      // / $state) sources alike.
      //
      // ST.afterData replays immediately if the remote source is already
      // loaded, so it doubles as the remote initial-compute. For local sources
      // we read the current SpacetimeLocal value directly for the first paint.
      // The source is addressed by its ROOT signal; the member tail (if any) is
      // walked off whatever that root currently holds. So `from $assetDoc.images`
      // recomputes whenever $assetDoc updates, reading `.images` each time.
      ST.afterData(sourceRoot, (data) => {
        compute(walkSource(data));
      });

      document.addEventListener(`local:${sourceRoot}:updated`, (e) => {
        if (e.detail != null) compute(walkSource(e.detail));
      });

      // Watch every $sig dependency discovered at COMPILE time so an external
      // signal (e.g. a live search box) re-runs the pipeline.
      allDeps.forEach((d) => {
        if (d === sourceRoot) return; // already wired above
        // Bridge element-scoped primitive exports (BUG-069 rail; BUG-197) —
        // no-op for global deps.
        if (typeof ST !== 'undefined' && ST._bridgeExport) ST._bridgeExport(d);
        document.addEventListener(`local:${d}:updated`, recompute);
        if (typeof ST !== 'undefined' && ST.afterData) ST.afterData(d, recompute);
      });

      if (typeof window.SpacetimeLocal !== 'undefined'
          && window.SpacetimeLocal[sourceRoot] != null) {
        compute(walkSource(window.SpacetimeLocal[sourceRoot]));
      }
    };
  }

  %emit js {
    (() => {
      const value = null;
      window.__stComputed({
        name: '%name',
        source: %source,
        sortDir: '%sortDir',
        limit: %limit,
        initial: %initial,
        filter: { f: %filter__fn, deps: %filter__deps },
        sortKey: { f: %sortBy__fn, deps: %sortBy__deps },
        map: { f: %transform__fn, deps: %transform__deps },
        reduce: { f: %reduce__step_fn, deps: %reduce__deps },
      });

      %yield value -> $value;
    })()
  }

  %exports {
    $value: any
  }
}
