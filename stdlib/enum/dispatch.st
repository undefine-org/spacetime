/// @match / @view — unified template dispatch (PLAN-077 W5)
///
/// ONE primitive, ONE arm-resolution code path, reactivity as a parameter:
///
///   @match $subject { pat => &tpl(args); }   — render ONCE at mount
///   @view  $subject { pat => &tpl(args); }   — re-mount on every signal change
///
/// The two share the `arm` grammar (stdlib/enum/capture-types/match-arms.st),
/// the instance-first subject resolution (ST.resolve — it falls through to the
/// global page store per FUP-094, so a `@match $kind` inside a @template body
/// resolves the instance param while a page-level @view reads global state),
/// the dotted-path walk (BUG-155-class: `$wrap.kind` — the HEAD signal publishes
/// updates, the tail is property access), the `$`-dynamic template-name
/// resolution (FEAT-126 region templates), and the arm-arg resolution (dotted
/// `$` refs, quoted literals).
///
/// Merged from the retired view-mount and match-render primitives,
/// both deleted this wave —
/// they were near-duplicates differing only in "subscribe + re-mount" vs
/// "mount once". Semantic notes on the merge:
///   * Subject/template-name/arg resolution uses match-render's INSTANCE-FIRST
///     order (ST.resolve then SpacetimeLocal) — a strict superset of view.st's
///     global-first order, since ST.resolve itself falls back to the global
///     store when no element scope owns the name (FUP-094).
///   * The reactive mode keeps view-mount's rendered-guard (ST.watch fires
///     immediately for element-scoped values — that IS the initial render) and
///     its teardown (dispose mounted subtree before re-mount) and the M4
///     region-input seeding (inert outside region mounts).

%macro match {
  %order 700

  %form {
    @match $subject:binding {
      $arms:arm+
    }
  }

  %binds {
    dispatch-mount(&self, subject: $subject, arms: $arms, reactive: false)
  }
}

%macro view {
  %order 700

  %form {
    @view $subject:binding {
      $arms:arm+
    }
  }

  %binds {
    dispatch-mount(&self, subject: $subject, arms: $arms, reactive: true)
  }
}

/// Dispatch Mount Primitive (PLAN-077 W5)
///
/// @param &el      - the host element (the matched arm's template mounts as a child)
/// @param subject  - the subject signal name (with or without `$`, dotted paths ok)
/// @param arms     - raw grammar arms: [{ pat: {lit|wild}, inv: [{name,args}] }]
/// @param reactive - false: mount once (@match); true: re-mount on change (@view)
%primitive dispatch-mount(
  &el,
  subject: string = "",
  arms: expr,
  reactive: bool = false
) {
  %emit js {
    (() => {
      const host = %&el;
      const sig = %subject;
      const arms = %arms;
      const reactive = %reactive;
      if (!host || !sig || !Array.isArray(arms)) return;

      // BUG-155-class: `sig` may be a DOTTED PATH (`@view $wrap.kind { … }`) —
      // resolve/watch the HEAD signal, walk the remaining segments as property
      // access. For a plain subject sigHead === sig (a superset, not a special
      // case). Subject resolution is INSTANCE-FIRST (ST.resolve — itself falls
      // through to the global page store when no element scope owns the name,
      // FUP-094), then the page-global store.
      const sigName = (typeof sig === 'string' && sig.charAt(0) === '$') ? sig.slice(1) : sig;
      const sigHead = String(sigName).split('.')[0];
      const sigTail = String(sigName).split('.').slice(1);
      const resolveValue = () => {
        let base;
        if (typeof ST !== 'undefined' && ST.resolve) base = ST.resolve(host, sigHead);
        if (base === undefined && typeof window !== 'undefined' && window.SpacetimeLocal && sigHead in window.SpacetimeLocal) {
          base = window.SpacetimeLocal[sigHead];
        }
        for (const k of sigTail) { if (base == null) break; base = base[k]; }
        return base;
      };

      // Normalize one raw grammar arm `{ pat: {lit|wild}, inv: [{name,args}] }`
      // into `{ pattern, ref }`. `_` (wild) -> null wildcard.
      const armPattern = (a) => {
        if (!a || !a.pat) return null;
        if (typeof a.pat.lit === 'string') return a.pat.lit;
        if (typeof a.pat.wild === 'string') return a.pat.wild === '_' ? null : a.pat.wild;
        return null;
      };
      const armRef = (a) => (a && Array.isArray(a.inv) && a.inv[0]) ? a.inv[0] : null;

      // Resolve a (possibly `$`-dynamic) template NAME against the same scopes
      // as the subject (FEAT-126: a region's `_ => &$stageTpl()` names the
      // template by a runtime value; a bare name passes through as static).
      const resolveName = (name) => {
        if (typeof name !== 'string' || name.charAt(0) !== '$') return name;
        const rn = name.slice(1);
        let rv;
        if (typeof ST !== 'undefined' && ST.resolve) rv = ST.resolve(host, rn);
        if (rv === undefined && typeof window !== 'undefined' && window.SpacetimeLocal && rn in window.SpacetimeLocal) {
          rv = window.SpacetimeLocal[rn];
        }
        return (rv == null || rv === '') ? '' : String(rv);
      };

      // Resolve one arm arg: a `$ref` (dotted) against the scopes, a quoted
      // literal unwrapped, everything else passed through.
      const resolveArg = (arg) => {
        if (typeof arg === 'string' && arg.charAt(0) === '$') {
          const an = arg.slice(1).split('.');
          let av;
          if (typeof ST !== 'undefined' && ST.resolve) av = ST.resolve(host, an[0]);
          if (av === undefined && typeof window !== 'undefined' && window.SpacetimeLocal && an[0] in window.SpacetimeLocal) {
            av = window.SpacetimeLocal[an[0]];
          }
          for (let i = 1; i < an.length && av != null; i++) av = av[an[i]];
          return av !== undefined ? av : arg;
        }
        if (typeof arg === 'string' &&
            ((arg.charAt(0) === '"' && arg.endsWith('"')) || (arg.charAt(0) === "'" && arg.endsWith("'")))) {
          return arg.slice(1, -1);
        }
        return arg;
      };

      let mounted = null;

      const render = () => {
        const value = resolveValue();
        // First arm whose pattern equals the value (null pattern = `_` wildcard).
        const arm = arms.find(a => { const p = armPattern(a); return p == null || String(p) === String(value); });
        // Tear down the currently-mounted pane (dispose cleanup, then remove) —
        // only meaningful in reactive mode; `mounted` is always null on the
        // once-mount path.
        if (mounted) {
          if (typeof ST !== 'undefined' && ST.dispose) {
            ST.dispose(mounted);
            mounted.querySelectorAll && mounted.querySelectorAll('*').forEach(n => ST.dispose(n));
          }
          mounted.remove();
          mounted = null;
        }
        const ref = armRef(arm);
        if (!ref) return;
        const tname = resolveName(ref.name);
        if (!tname) { console.warn('[ST] dispatch-mount: template name resolved empty for', ref.name); return; }
        const factory = (window.Spacetime && window.Spacetime.templates && window.Spacetime.templates.get(tname))
                     || (window.__pendingTemplates && window.__pendingTemplates[tname]);
        if (!factory) { console.warn('[ST] dispatch-mount: template not found:', tname); return; }
        try {
          const el = factory(...(ref.args || []).map(resolveArg));
          if (el) {
            host.appendChild(el);
            mounted = el;
            // M4 (PLAN-046): seed region param overrides onto the freshly-mounted
            // guest root so its `$param` holes reflect inspector edits (inert
            // outside region mounts — a normal pane swap has no __stRegionInput).
            if (host.__stRegionInput && typeof ST !== 'undefined' && ST.set) {
              const input = host.__stRegionInput;
              Object.keys(input).forEach(function(k) { ST.set(el, k, input[k]); });
            }
          }
        } catch (err) {
          console.error('[ST] dispatch-mount error (' + tname + '):', err);
        }
      };

      // Initial mount + (reactive mode only) re-mount on the subject signal.
      // ST.watch fires immediately if an element-scoped value exists — that IS
      // the initial render for the element-scope case; the rendered-guard
      // prevents a double initial render. The watched value itself is never
      // used directly: render() re-derives the full dotted path regardless.
      let rendered = false;
      const renderOnce = () => { rendered = true; render(); };
      let unwatch = null;
      if (reactive) {
        // ST.watchScoped (not ST.watch): the subject resolves via ST.resolve
        // (nearest owner — host, an ANCESTOR scope, or the global store), so
        // the watch must cover the same scopes or an ancestor-scoped subject
        // renders once and never re-mounts (SWARM GATE 5 finding; the FUP-094
        // watch-mirrors-resolve doctrine). watchScoped also subscribes to the
        // global `local:<head>:updated` channel internally, so no separate
        // document listener is needed. The callback ignores the delivered
        // value — render() re-derives via ST.resolve, so a fire from ANY
        // scope re-renders the nearest-owner value.
        if (typeof ST !== 'undefined' && ST.watchScoped) {
          unwatch = ST.watchScoped(host, sigHead, () => renderOnce());
        }
      }
      if (!rendered) render();

      if (reactive && typeof ST !== 'undefined' && ST.onCleanup) {
        ST.onCleanup(host, () => {
          if (unwatch) unwatch();
        });
      }
    })();
  }
}
