/// Template Registration Primitive
///
/// Registers a template factory function with the global Spacetime.templates registry.
/// Templates are compiled to factory functions that create DOM elements from data.
///
/// @param name - Template name (without &)
/// @param html - HTML template string with $param interpolation
/// @param params - Array of parameter names (may include optional params with `?` suffix)
/// @param animations - Optional keyframes for scoped template animations
///
/// @yields $factory - The registered factory function
///
/// ## Optional Parameters (`?` modifier)
///
/// Parameters can be marked as optional using the `?` suffix in the @template definition.
/// This affects how the factory function handles missing arguments:
///
/// ### Behavior:
/// - Optional value params (`$param?`): Receive `undefined` when not provided
/// - Optional element params (`&param?`): Receive `null` when not provided
/// - The factory function does NOT throw when optional params are omitted
/// - HTML interpolation treats undefined/null as empty string
///
/// ### Example factory generation:
/// For `@template &card($title, $subtitle?)`:
/// ```javascript
/// // Generated factory handles both:
/// factory("Hello")           // $title="Hello", $subtitle=undefined
/// factory("Hello", "World")  // $title="Hello", $subtitle="World"
/// ```
///
/// ### Default value handling in templates:
/// Use JavaScript's OR (||) or nullish coalescing (??) for defaults:
/// ```javascript
/// el.innerHTML = '<h2>' + ($title || 'Untitled') + '</h2>';
/// el.innerHTML = '<span>' + ($count ?? 0) + '</span>';
/// ```

%primitive register-template(
  name: string,
  body: string,
  params: array = [],
  animations: keyframes? = null
) {
  %emit js {
    (() => {
    // Strip & prefix from template name if present (macro captures it with prefix)
    // FEAT-126: factory construction is shared with the region-mount registerBundle
    // primitive via Spacetime.buildTemplateFactory (public/runtime/templates.js).
    // One implementation turns a body payload into a factory; both the page-load
    // register-template (here) and the bundle-arrival registerBundle call it.
    // FUP-134: capture the owner NOW, at registration, and bind it into the
    // factory as its 5th argument (the same slot region mounts use). The global is
    // only set while this bundle EVALUATES; a factory invoked later — from an event
    // handler, an `@each` re-render, any async callback — would read `undefined`
    // and fall back to bare lookup, which is precisely the collision being fixed.
    // Binding it at build time makes owner identity a property of the FACTORY
    // rather than of the moment it happens to run.
    const owner = window.__ST_BUNDLE_OWNER__;
    const built = window.Spacetime && window.Spacetime.buildTemplateFactory
      ? window.Spacetime.buildTemplateFactory(%name, %body, %params || [], %animations, owner)
      : null;
    if (!built) return;
    const templateName = built.name;
    const factory = built.factory;

    // Register with Spacetime, under this bundle's OWNER (FUP-134).
    //
    // `window.__ST_BUNDLE_OWNER__` is set by the dev server for each DOCK bundle
    // it injects; an ordinary page leaves it undefined. An owned bundle registers
    // `owner/name` and claims the bare name only if it is FREE, so a dock widget
    // can no longer evict a page's same-named template — which is exactly what
    // happened to a page declaring `&field-text`: its factory vanished and its
    // `@each` rendered nothing, with no error.
    if (window.Spacetime && window.Spacetime.registerOwnedTemplate) {
      Spacetime.registerOwnedTemplate(owner, templateName, factory);
    } else if (window.Spacetime && window.Spacetime.templates) {
      Spacetime.templates.set(templateName, factory);
    } else {
      // Fallback: store for later registration
      window.__pendingTemplates = window.__pendingTemplates || {};
      window.__pendingTemplates[templateName] = factory;
    }
    // Note: factory is registered globally, no element-scoped yield needed
    })();
  }

  %exports {
    // Factory is available via Spacetime.templates.get(name)
  }
}


/// Template Invocation Primitive
///
/// Invokes a registered template with arguments and appends to container.
///
/// @param &container - Element to append rendered template to
/// @param name - Template name to invoke
/// @param args - Arguments to pass to factory
/// @param ref - Optional ref name to store created element under
///
/// @yields $element - The created element

%primitive invoke-template(
  &container,
  name: string,
  args: array = [],
  body: string? = null,
  ref: string? = null,
  node: string? = null
) {
  %emit js {
    const container = %&container;
    const templateName = %name;
    // Resolve each invocation arg (PLAN-039 W3-fn): a `$`-ref arg like `&fld($doc)` is
    // emitted as the string "$doc" and resolved here to its reactive VALUE against global
    // state (SpacetimeLocal), so the factory receives the data OBJECT, not the literal
    // "$doc". String/number literals pass through unchanged (resolveTemplateArg no-ops them).
    const rawTemplateArgs = %args;
    // BUG-131: split named args (`title: "A"`) from positional and bind by name,
    // resolving `$`-refs against global state. ST.resolveRawInvocationArgs centralizes
    // the split + branding so the factory binds named args order-independently; a
    // purely positional list passes through unchanged (legacy behaviour).
    const __stData = (typeof SpacetimeLocal !== 'undefined' ? SpacetimeLocal : {});
    const templateArgs = (typeof ST !== 'undefined' && ST.resolveRawInvocationArgs)
      ? ST.resolveRawInvocationArgs(rawTemplateArgs, __stData)
      : (Array.isArray(rawTemplateArgs) ? rawTemplateArgs : []).map(function (a) {
          return (typeof a === 'string' && a.charAt(0) === '$' && typeof ST !== 'undefined' && ST.resolveTemplateArg)
            ? ST.resolveTemplateArg(a, __stData)
            : a;
        });
    const bodyHtml = %body;

    let element = null;

    // FUP-134: resolve OWNER-first, then bare, so a dock widget invoking its own
    // template gets its own even when a page has claimed the bare name.
    //
    // The owner comes from `__ST_BUNDLE_OWNER__` when this invocation runs during
    // its bundle's evaluation, and otherwise from `__ST_INVOKE_OWNER__`, which the
    // enclosing factory sets while it builds. A deferred invocation — an event
    // handler, an `@each` re-render, any async callback — happens long after the
    // bundle-evaluation global has been restored, so relying on it alone silently
    // degraded to bare lookup: exactly the collision this fixes.
    const invokeOwner = (typeof window.__ST_BUNDLE_OWNER__ !== 'undefined' && window.__ST_BUNDLE_OWNER__)
      ? window.__ST_BUNDLE_OWNER__
      : window.__ST_INVOKE_OWNER__;
    const factory = (window.Spacetime && window.Spacetime.resolveTemplate)
      ? window.Spacetime.resolveTemplate(templateName, invokeOwner)
      : (window.Spacetime?.templates?.get(templateName)
         || window.__pendingTemplates?.[templateName]);

    if (factory) {
      try {
        // If body HTML is provided, append it as the last argument
        // so it fills the first unbound &param in the template
        const allArgs = bodyHtml != null
          ? [...templateArgs, bodyHtml]
          : templateArgs;
        element = factory(...allArgs);
        if (element && container) {
          // FEAT-154: a provably-static invocation may have been SSG-unrolled at
          // compile time — the container already holds a `data-st-ssg="1"` marked
          // ROOT element with real (but non-reactive: no listeners, no watchScoped)
          // markup for SEO/no-JS. The factory-built `element` IS the reactive
          // instance (event handlers wired, holes watching their signals) — it must
          // REPLACE the static placeholder, not sit ALONGSIDE it (which would
          // duplicate visible content). This mirrors `each-inline`'s own
          // unroll-then-hydrate pattern: the SSG pass buys a real first paint, the
          // runtime pass buys full interactivity, and the DOM converges to the
          // SAME shape either way (`unroll == hydrate`).
          const __ssgPlaceholder = container.querySelector(':scope > [data-st-ssg="1"]');
          if (__ssgPlaceholder) {
            container.replaceChild(element, __ssgPlaceholder);
          } else {
            container.appendChild(element);
          }
          const refName = %ref;
          if (refName) {
            // Two registries, two jobs. `ST.set` publishes a SIGNAL on the
            // container; `window.__stRefs` maps a ref NAME to its element and
            // is what `ST.ref(name)` — and therefore every `&name` subject,
            // `$bounds` capture and derives read — resolves against.
            // Registering only the signal met the letter of "stored under the
            // ref name" while leaving the name unfindable by every reader.
            if (typeof window !== 'undefined') {
              window.__stRefs = window.__stRefs || {};
              window.__stRefs[refName] = element;
            }
            ST.set(container, refName, element);
          }
        }
      } catch (err) {
        console.error(`Template invocation error (${templateName}):`, err);
      }
    } else {
      console.error(`Template not found: ${templateName}`);
    }

    %yield element -> $element;
  }

  %if $node {
    %emit js {
      if (element) {
        element.setAttribute('data-st-node', %node);
      }
    }
  }

  %exports {
    $element: element
  }
}


/// Each With Templates Primitive
///
/// Iterates over a data source and invokes template(s) for each item.
/// Uses Spacetime.templates registry for template lookup.
///
/// @param &container - Element to render items into
/// @param source - Data source name (dispatches data:name:loaded events)
/// @param item - Variable name for each item (used by compiler)
/// @param invocations - Array of template invocations [{name, args}]
/// @param key - Property to use as unique key for diffing
///
/// @yields $items - Array of rendered item elements
/// @yields $update - Function to trigger re-render

%primitive each-with-templates(
  &container,
  source: string,
  item: string,
  invocations: array,
  key: string = "id",
  filter: string? = null
) {
  %emit js {
    (() => {
    const containerEl = %&container;
    const sourceName = %source;
    const itemVar = %item;
    const templateInvocations = %invocations;
    const keyProp = %key;
    const filterExpr = %filter;

    let items = [];
    const itemMap = new Map(); // key -> elements[]
    let lastData = null; // Store last rendered data for re-filtering

    // Invoke a template with item data
    const invokeTemplate = (invocation, itemData, index) => {
      const { name, args } = invocation;

      // Resolve args - replace item variable references with actual data
      const resolvedArgs = args.map(arg => {
        if (typeof arg === 'string' && arg.startsWith('$')) {
          // It's a variable reference
          const varPath = arg.slice(1); // Remove $
          if (varPath === itemVar || varPath.startsWith(itemVar + '.')) {
            // Reference to current item
            const path = varPath.slice(itemVar.length);
            if (path === '' || path === '.') {
              return itemData;
            }
            // Property access: $item.name -> itemData.name
            let value = itemData;
            for (const part of path.slice(1).split('.')) {
              if (value == null) return '';
              value = value[part];
            }
            return value;
          }
        }
        return arg;
      });

      // Resolve a DYNAMIC template name (FEAT-073 dynamic dispatch in @each):
      // `&$item.tpl(…)` names the template by a runtime value on the current item.
      // `$item` / `$item.field` walks the item; a bare `$other` reads the global
      // signal store. A static name passes through unchanged.
      let resolvedName = name;
      if (typeof name === 'string' && name.startsWith('$')) {
        const np = name.slice(1);
        if (np === itemVar || np.startsWith(itemVar + '.')) {
          const rest = np.slice(itemVar.length);
          let v = itemData;
          if (rest && rest[0] === '.') {
            for (const part of rest.slice(1).split('.')) { if (v == null) break; v = v[part]; }
          }
          resolvedName = (v == null) ? '' : String(v);
        } else {
          const g = (typeof window !== 'undefined' && window.SpacetimeLocal) ? window.SpacetimeLocal[np.split('.')[0]] : undefined;
          resolvedName = (g == null) ? '' : String(g);
        }
        if (!resolvedName) { console.warn('[ST] @each dynamic dispatch: name', name, 'resolved empty'); return null; }
      }

      // Get factory from registry
      const factory = window.Spacetime?.templates?.get(resolvedName);
      if (!factory) {
        console.warn(`Template not found: ${resolvedName}`);
        return null;
      }

      try {
        return factory(...resolvedArgs);
      } catch (err) {
        console.error(`Template error (${resolvedName}):`, err);
        return null;
      }
    };

    // Evaluate a filter expression for a given item
    const evaluateFilter = (expr, itemData) => {
      if (!expr) return true;
      const cleanExpr = expr.replace(/^when\s+/, '');
      try {
        // Replace $itemVar.field references with item data
        let resolved = cleanExpr.replace(/\$(\w+)(?:\.(\w+(?:\.\w+)*))?/g, (match, varName, fieldPath) => {
          if (varName === itemVar) {
            if (!fieldPath) return JSON.stringify(itemData);
            let val = itemData;
            for (const part of fieldPath.split('.')) {
              if (val == null) return JSON.stringify('');
              val = val[part];
            }
            return JSON.stringify(val != null ? val : '');
          }
          // State variable reference — read from SpacetimeLocal or ST.get
          if (typeof window.SpacetimeLocal !== 'undefined' && varName in window.SpacetimeLocal) {
            return JSON.stringify(window.SpacetimeLocal[varName]);
          }
          const stVal = ST.get(containerEl, varName);
          if (stVal !== undefined) return JSON.stringify(stVal);
          return JSON.stringify('');
        });
        return new Function('return (' + resolved + ')')();
      } catch (err) {
        console.warn('Filter expression error:', err.message, expr);
        return true; // Show item on error
      }
    };

    // Apply filter to all rendered items (toggle display)
    const applyFilter = () => {
      if (!filterExpr || !lastData) return;
      let idx = 0;
      lastData.forEach((itemData) => {
        const itemKey = itemData[keyProp] ?? idx;
        const elements = itemMap.get(itemKey);
        if (elements) {
          const matches = evaluateFilter(filterExpr, itemData);
          for (const el of elements) {
            el.style.display = matches ? '' : 'none';
          }
        }
        idx++;
      });
    };

    // Extract state variable names from filter expression for reactive listening
    const getFilterStateVars = (expr) => {
      if (!expr) return [];
      const vars = [];
      const re = /\$(\w+)/g;
      let m;
      while ((m = re.exec(expr)) !== null) {
        const varName = m[1];
        // Skip the item variable itself
        if (varName !== itemVar) {
          vars.push(varName);
        }
      }
      // Deduplicate
      return [...new Set(vars)];
    };

    // Render all items
    const render = (data) => {
      if (!Array.isArray(data)) {
        console.warn('Each: expected array, got', typeof data);
        return;
      }

      // Idempotent dual-arrival guard: a source can publish through BOTH channels
      // (local:<name>:updated AND the @data registry) with the SAME array value.
      // Skip a redundant rebuild when the incoming data is reference-identical to
      // what we last rendered — collapses the double-arrival to one paint without
      // suppressing genuine updates (a real change is a new array reference).
      if (data === lastData) return;

      // Build all items into a fragment (off-DOM) so MutationObserver fires
      // once for the entire batch instead of once per appendChild.
      const fragment = document.createDocumentFragment();
      items = [];
      itemMap.clear();
      lastData = data;

      data.forEach((itemData, index) => {
        const itemKey = itemData[keyProp] ?? index;
        const itemElements = [];

        // Invoke each template for this item
        for (const invocation of templateInvocations) {
          const el = invokeTemplate(invocation, itemData, index);
          if (el) {
            el.dataset.key = itemKey;
            el.dataset.index = index;
            fragment.appendChild(el);  // off-DOM — no MutationObserver callback
            itemElements.push(el);
            items.push(el);
          }
        }

        itemMap.set(itemKey, itemElements);
      });

      // Single DOM mutation: clear + insert atomically.
      // replaceChildren fires one MutationObserver record vs. N+1 (innerHTML + N appends).
      containerEl.replaceChildren(fragment);

      // Apply filter after render
      if (filterExpr) {
        applyFilter();
      }

      %yield items -> $items;
    };

    // A source can reach @each through EITHER channel, and the producing channel
    // may resolve AFTER this init runs (e.g. a `@data derive` that recomputes
    // once its fetched dependency lands). So we wire BOTH channels unconditionally
    // rather than branching on a one-shot snapshot of `sourceName in SpacetimeLocal`
    // — a derived/late source that was absent at init time would otherwise bind to
    // the @data branch only and never render when it later publishes via
    // local:<name>:updated (BUG-076). Both channels are idempotent: render() diffs
    // against lastData, so a double-arrival collapses to one paint.

    // Instance-scope source resolution (PLAN-039): when @each lives inside a
    // `@template` body, its `$source` is a TEMPLATE PARAM seeded by the factory as an
    // element signal on the instance root (`ST.set(instanceRoot, source, value)`), NOT a
    // global @data field. Walk containerEl + ancestors via ST.get first (instance scope),
    // then fall back to the global channels (SpacetimeLocal / @data registry / inline
    // data). Mirrors the editable surface's resolveBoundDoc ancestor-walk. Returns an
    // array or undefined.
    //
    // `sourceName` may be a DOTTED PATH (`"wrap.items"` from `@each($wrap.items as
    // $x)` -- BindingExtractor bundles the whole path into one token, by design).
    // ST.resolve does an EXACT signal-name lookup and never splits on `.`, so a
    // dotted source silently resolved to undefined forever (BUG-155). ST.resolvePath
    // is the existing, already-correct primitive for this: it resolves the HEAD name
    // in scope, then walks the remaining segments as plain property access -- the
    // same path `@data derive`/interpolation already use correctly. `headName` is the
    // signal that actually publishes updates; for a non-dotted sourceName headName ===
    // sourceName, so this is a superset, not a special case.
    const headName = sourceName.split('.')[0];
    const resolveSourceData = () => {
      const scoped = (typeof ST !== 'undefined' && ST.resolvePath) ? ST.resolvePath(containerEl, sourceName) : undefined;
      if (scoped !== undefined) return scoped;
      // Registry/window fallbacks are keyed by the HEAD name (ST.setData registers
      // under 'wrap', never a literal 'wrap.items') -- read the head entry, then
      // walk the remaining dotted segments as plain property access, same as
      // ST.resolvePath does for the primary (scoped) case above.
      const headData = ST._dataRegistry.get(headName)?.data ?? window.__spacetimeData?.[headName]?.data;
      if (headData === undefined) return undefined;
      const parts = sourceName.split('.').slice(1);
      let v = headData;
      for (let i = 0; i < parts.length && v != null; i++) v = v[parts[i]];
      return v;
    };

    // Channel 1: local signal updates (mutations, @data derive/fold/source writes).
    // For a PLAIN (non-dotted) source, `e.detail` already IS the exact new value
    // (SpacetimeLocal's proxy dispatches it post-write) -- trust it directly, same
    // as always. For a DOTTED source (BUG-155), `e.detail` is the HEAD signal's
    // whole value (not the nested field this @each wants), so re-derive the full
    // dotted path via resolveSourceData() instead of trusting the raw detail.
    const isDotted = sourceName !== headName;
    const localHandler = (e) => {
      const data = isDotted ? resolveSourceData() : e.detail;
      if (Array.isArray(data)) render(data);
    };
    document.addEventListener(`local:${headName}:updated`, localHandler);

    // Channel 2: the @data registry (remote sources; replays immediately if ready).
    // Same dotted-path rationale as Channel 1: the registry is keyed by the HEAD
    // name (ST.setData('wrap', ...) registers under 'wrap', never the literal
    // dotted string) -- subscribe under headName, re-derive the full path for a
    // dotted source, trust the registry value directly for a plain one.
    ST.afterData(headName, (data) => {
      const resolved = isDotted ? resolveSourceData() : data;
      if (Array.isArray(resolved)) render(resolved);
    });

    // Explicit empty state (backwards compatibility) -- keyed by headName, same reason.
    document.addEventListener(`data:${headName}:empty`, () => {
      containerEl.innerHTML = '';
      items = [];
      itemMap.clear();
      lastData = null;
      %yield items -> $items;
    });

    // Initial render from whatever is already present (no-op if neither is ready;
    // the listeners above catch the later arrival). resolveSourceData checks the
    // instance scope (template param) before the global channels.
    const initialData = resolveSourceData();
    if (Array.isArray(initialData)) {
      render(initialData);
    }

    // Instance-param channel (PLAN-039): when the source is a template param, the factory
    // may seed it a tick AFTER this selector-init runs (the container is in the DOM, the
    // observer fires init, but `$xs` is ST.set just after). Watch containerEl + ancestors
    // for the source signal so the list renders the moment the param lands; ST.watch
    // fires immediately if already present, covering both the race and the seeded case.
    if (typeof ST !== 'undefined' && ST.watchScoped) {
      // Watch the HEAD signal in scope (same dotted-path rationale as Channel 1): a
      // PLAIN source's watched value IS the array (trust it directly, unchanged
      // behavior); a DOTTED source's watched value is the HEAD object, so re-derive
      // the full path via resolveSourceData() instead.
      ST.watchScoped(containerEl, headName, (v) => {
        const data = isDotted ? resolveSourceData() : v;
        if (Array.isArray(data)) render(data);
      });
    }

    // Set up reactive re-filtering when filter state variables change
    const filterStateVars = getFilterStateVars(filterExpr);
    for (const varName of filterStateVars) {
      document.addEventListener(`local:${varName}:updated`, () => {
        applyFilter();
      });
    }

    // Manual update function: re-render from whichever channel currently holds data
    // (instance scope first, then global).
    const update = () => {
      const data = resolveSourceData();
      if (Array.isArray(data)) render(data);
    };

    %yield update -> $update;
    })();

    %cleanup {
      // Event listeners are cleaned up when element is removed
    }
  }

  %exports {
    $items: array
    $update: fn() ~> effect          // Trigger re-render
  }
}
