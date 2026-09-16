/// Each-Inline Primitive (PLAN-023 W4)
///
/// Renders an INLINE HTML body once per item in a reactive source. Unlike
/// `each-with-templates` (which invokes registered `@template` factories), this
/// takes the raw HTML written directly inside the `@each { … }` body and clones
/// it per item, interpolating `$.field` references (and `$index`) against each
/// item. Backs the HTML-bodied `@each` form (W4).
///
/// The inline HTML is the per-item row markup, e.g.
///   @each($todos as $t) { <li class="todo">`$t.title`</li> }
/// `$.title` / `$t.title` resolve to the current item's field.
///
/// SSG note: for a STATIC source (source_static:true) the unroll happens at
/// compile time (see the macro's emit branch); this primitive is the DYNAMIC
/// (hydrate) path — it re-renders whenever the source signal changes.
///
/// @param &container - element to render rows into
/// @param source     - source signal/data name
/// @param item       - the per-item binding name (e.g. "t" for `$t`)
/// @param html       - raw inline row HTML (string)
///
/// @yields $items - array of rendered row elements
%primitive each-inline(
  &container,
  source: string,
  item: string,
  html: string,
  filter: string? = null
) {
  %emit js {
    (() => {
      const containerEl = %&container;
      const sourceName = %source;
      const itemVar = %item;
      const rowHtml = %html;
      const filterExpr = %filter;

      let items = [];
      let lastData = null; // for reactive re-filtering when filter deps change
      // See each.st for the full rationale: a rebuild destroys live DOM state
      // (focus/caret, <select> choice, open <details>) that the markup cannot
      // re-express, and a polled source re-emits identical data every tick.
      let renderedSig = null;
      const dataSignature = (data) => {
        try {
          return JSON.stringify(data);
        } catch (e) {
          return null;
        }
      };

      const getProperty = (obj, path) => {
        if (!path) return obj;
        let value = obj;
        for (const part of path.split('.')) {
          if (value == null) return '';
          value = value[part];
        }
        return value == null ? '' : value;
      };

      const escapeAttr = (str) => String(str == null ? '' : str)
        .replace(/&/g, '&amp;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#x27;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;');

      // Interpolate $.field, $<itemVar>.field, and $index in a cloned row.
      const interpolate = (el, itemData, index) => {
        const resolveToken = (raw) => {
          // raw is like ".title" ($.title) or "t.title" ($t.title) or "index"
          if (raw === 'index') return String(index);
          let path = raw;
          if (path.startsWith('.')) {
            path = path.slice(1);                 // $.title -> title
          } else if (path === itemVar) {
            return itemData;                       // $t -> whole item
          } else if (path.startsWith(itemVar + '.')) {
            path = path.slice(itemVar.length + 1); // $t.title -> title
          } else {
            return null;                           // not an item ref; leave as-is
          }
          return getProperty(itemData, path);
        };
        const rx = /\$(\.[A-Za-z_][\w.]*|[A-Za-z_]\w*(?:\.[\w.]+)?)/g;
        const walk = (node) => {
          if (node.nodeType === Node.TEXT_NODE) {
            node.textContent = node.textContent.replace(rx, (m, tok) => {
              const v = resolveToken(tok);
              return v == null ? m : v;
            });
          } else if (node.nodeType === Node.ELEMENT_NODE) {
            for (const attr of Array.from(node.attributes)) {
              if (attr.value.includes('$')) {
                attr.value = attr.value.replace(rx, (m, tok) => {
                  const v = resolveToken(tok);
                  return v == null ? m : escapeAttr(v);
                });
              }
            }
            for (const child of node.childNodes) walk(child);
          }
        };
        walk(el);
        el.dataset && (el.dataset.index = index);
        return el;
      };

      // Resolve a `\u0060expr\u0060` backtick hole (W0/W1 HTML hole syntax) or a bare
      // $.field / $item.field token against the current item. Returns the string value,
      // or null when the token is not an item reference (left untouched).
      const resolveHoleExpr = (expr, itemData, index, varName) => {
        const v = varName || itemVar;
        const e = expr.trim();
        if (e === '$index') return String(index);
        if (e === '$' + v) return itemData;
        if (e === '$.') return itemData;
        // ONLY $.field or $<v>.field are item references — a $other.field hole
        // (e.g. a global signal) is NOT an item ref and is left verbatim, matching the
        // SSG resolve_token classifier so unroll == hydrate (W4 reviewer fix).
        if (e.startsWith('$.')) {
          return getProperty(itemData, e.slice(2));
        }
        if (e.startsWith('$' + v + '.')) {
          return getProperty(itemData, e.slice(v.length + 2));
        }
        return null;
      };

      // Replace `\u0060\u2026\u0060` holes in a raw HTML string BEFORE building the row, so a
      // hole that spans text or attribute positions is handled uniformly. Holes that do
      // not reference the item are left verbatim. BUG-188: an optional
      // ` | filter(...)` run right after the hole applies ST.filter to the resolved
      // value (the SAME pipe the `text <-` path honors) instead of leaking as
      // literal text (`` `p.price` | currency("$") `` rendered `2400 | currency("$")`).
      // BUG-203: substituted VALUES are sentinel-protected (\uE000 stands in
      // for the backtick) so a LATER hole pass (the outer fillHoles in a
      // nested @each) can never re-scan data text as holes; makeRow restores
      // them after the final pass. Template holes left verbatim (outer-item
      // refs) still resolve — protection applies to DATA, not the template.
      const fillHolesOnly = (rawHtml, itemData, index, varName) => rawHtml.replace(
        /`([^`]*)`(\s*\|\s*([A-Za-z_][\w-]*(\((?:[^()'"]|"[^"]*"|'[^']*')*\))?))?/g,
        (full, inner, _pipeRun, pipe) => {
          const v = resolveHoleExpr('$' + inner.replace(/^\$/, ''), itemData, index, varName);
          if (v == null) return full;
          const filtered = (pipe && typeof ST !== 'undefined' && ST.filter) ? ST.filter(pipe, v) : v;
          // Remember which FIELD this hole rendered, keyed by the exact text it
          // produced. The hole is substituted into a raw HTML string here, so the
          // field name is only knowable at THIS moment — after `ST.parseHtml` the
          // value is indistinguishable from any other text. Recording it lets the
          // row-level stamping pass below attach `data-st-field` to the element
          // that actually holds the value, which is what makes a click resolve to
          // a single editable field rather than the whole record. Only a plain
          // `$.field` / `$item.field` reference is addressable: a piped or
          // computed hole is a PROJECTION of the data, not the data, and writing
          // to it would be writing to a derived value.
          if (window.__ST_DEV__ && !pipe) {
            const e = inner.trim().replace(/^\$/, '');
            const v2 = varName || itemVar;
            let field = null;
            if (e.startsWith('.')) field = e.slice(1);
            else if (e.startsWith(v2 + '.')) field = e.slice(v2.length + 1);
            if (field && String(filtered) !== '') {
              holeFields.push({ text: String(filtered), field: field });
            }
          }
          return escapeAttr(filtered).replace(/`/g, '\uE000');
        }
      );

      // Resolve a nested-each `src` (`$<var>.path` / `$.path` / `$<var>`) to the
      // inner ARRAY on itemData; [] when absent/not-an-array. Mirrors the Rust
      // SSG `resolve_nested_array` so unroll == hydrate (BUG-081).
      const resolveNestedArray = (src, itemData, itemVarName) => {
        let e = src.trim();
        if (e[0] === '$') e = e.slice(1);
        let path;
        if (e[0] === '.') path = e.slice(1);
        else if (e === itemVarName) path = '';
        else if (e.startsWith(itemVarName + '.')) path = e.slice(itemVarName.length + 1);
        else return [];
        const v = path === '' ? itemData : getProperty(itemData, path);
        return Array.isArray(v) ? v : [];
      };

      const matchDelim = (html, openIdx, openCh, closeCh) => {
        let depth = 0;
        for (let i = openIdx; i < html.length; i++) {
          const c = html[i];
          if (c === openCh) depth++;
          else if (c === closeCh) { depth--; if (depth === 0) return i; }
        }
        return -1;
      };

      // Find the first top-level `@each(<src> as <var>) { <inner> }`, depth-aware
      // over braces. A bounded structural recogniser (NOT a JS parser) matching
      // the Rust `find_nested_each`, so the nested loop hydrates the same DOM the
      // SSG unroll produced (BUG-081).
      const findNestedEach = (html) => {
        let from = 0;
        for (;;) {
          const at = html.indexOf('@each', from);
          if (at < 0) return null;
          const afterKw = at + 5;
          const po = html.indexOf('(', afterKw);
          if (po < 0) return null;
          if (html.slice(afterKw, po).trim() !== '') { from = afterKw; continue; }
          const pc = matchDelim(html, po, '(', ')');
          if (pc < 0) return null;
          const head = html.slice(po + 1, pc).split(',')[0];
          const parts = head.split(' as ');
          if (parts.length < 2) { from = pc + 1; continue; }
          const src = parts[0].trim();
          const v = parts[1].trim().replace(/^\$/, '');
          const bo = html.indexOf('{', pc + 1);
          if (bo < 0) return null;
          if (html.slice(pc + 1, bo).trim() !== '') { from = pc + 1; continue; }
          const bc = matchDelim(html, bo, '{', '}');
          if (bc < 0) return null;
          return { start: at, end: bc + 1, src, var: v, inner: html.slice(bo + 1, bc).trim() };
        }
      };

      // Expand every top-level nested @each against itemData, recursing so deep
      // nesting + inner field holes both resolve.
      const expandNested = (html, itemData, itemVarName) => {
        let out = '';
        let rest = 0;
        for (;;) {
          const ne = findNestedEach(html.slice(rest));
          if (!ne) break;
          out += html.slice(rest, rest + ne.start);
          const inner = resolveNestedArray(ne.src, itemData, itemVarName);
          inner.forEach((innerItem, i) => {
            out += fillHolesOnly(expandNested(ne.inner, innerItem, ne.var), innerItem, i, ne.var);
          });
          rest = rest + ne.end;
        }
        out += html.slice(rest);
        return out;
      };

      // Full row fill: expand nested @each (against this item) THEN fill the
      // surrounding holes (against this item). BUG-081: nested loops render.
      const fillHoles = (rawHtml, itemData, index) =>
        fillHolesOnly(expandNested(rawHtml, itemData, itemVar), itemData, index);
      // Namespace-aware row parse (BUG-082): an @each inside <svg> must parse its
      // rows in SVG context or <circle>/<path>/<text> never paint.
      // BUG-203: restore the sentinel-protected backticks in substituted
      // values AFTER the final hole pass (nested @each) — see fillHolesOnly.
      // Field provenance for ONE row, collected during hole substitution and
      // consumed immediately after that row is parsed. Cleared per row so entries
      // never leak between records.
      let holeFields = [];

      // Attach `data-st-field` to the element that renders each addressable hole.
      // Matching is by exact text content on a leaf element: a hole's value lands
      // as that element's text, so the deepest element whose trimmed text equals
      // the recorded value is the one holding it. A value that appears more than
      // once in a row is AMBIGUOUS and is skipped entirely — stamping the wrong
      // element would make an edit write to the wrong field, which is far worse
      // than leaving that hole unaddressable (the same rule W4R applied to
      // ambiguous invocation spans: no stamp beats a wrong stamp).
      const stampFields = (root, fields) => {
        if (!fields.length) return;
        const counts = new Map();
        for (const f of fields) counts.set(f.text, (counts.get(f.text) || 0) + 1);
        const leaves = root.querySelectorAll ? Array.from(root.querySelectorAll('*')) : [];
        if (root.nodeType === 1) leaves.unshift(root);
        for (const f of fields) {
          if (counts.get(f.text) !== 1) continue;
          const hit = leaves.find((n) =>
            n.children.length === 0 &&
            !n.hasAttribute('data-st-field') &&
            n.textContent.trim() === f.text.trim());
          if (hit) hit.setAttribute('data-st-field', f.field);
        }
      };

      const makeRow = (itemData, index) => {
        holeFields = [];
        const parsed = ST.parseHtml(fillHoles(rowHtml.trim(), itemData, index).replace(/\uE000/g, '`'), containerEl);
        if (window.__ST_DEV__ && parsed) stampFields(parsed, holeFields);
        return parsed;
      };

      // ---- reactive `when` filter (FEAT-094) ----
      // Evaluate the filter expression for one item: $<itemVar>.field reads itemData,
      // any other $stateVar reads SpacetimeLocal / ST.get. Mirrors
      // each-with-templates::evaluateFilter so filtered inline and template lists
      // behave identically.
      const evaluateFilter = (expr, itemData) => {
        if (!expr) return true;
        const cleanExpr = expr.replace(/^when\s+/, '');
        try {
          const resolved = cleanExpr.replace(/\$(\w+)(?:\.(\w+(?:\.\w+)*))?/g, (match, varName, fieldPath) => {
            if (varName === itemVar) {
              if (!fieldPath) return JSON.stringify(itemData);
              return JSON.stringify(getProperty(itemData, fieldPath) ?? '');
            }
            if (typeof window.SpacetimeLocal !== 'undefined' && varName in window.SpacetimeLocal) {
              return JSON.stringify(window.SpacetimeLocal[varName]);
            }
            const stVal = (typeof ST !== 'undefined') ? ST.get(containerEl, varName) : undefined;
            if (stVal !== undefined) return JSON.stringify(stVal);
            return JSON.stringify('');
          });
          return new Function('return (' + resolved + ')')();
        } catch (err) {
          console.warn('[Spacetime] each filter error:', err.message, expr);
          return true; // show on error
        }
      };

      // Toggle each row's display against the current filter + signal state.
      const applyFilter = () => {
        if (!filterExpr) return;
        items.forEach((el) => {
          const matches = evaluateFilter(filterExpr, el.__stItem);
          el.style.display = matches ? '' : 'none';
        });
      };

      const getFilterStateVars = (expr) => {
        if (!expr) return [];
        const vars = [];
        const re = /\$(\w+)/g;
        let m;
        while ((m = re.exec(expr)) !== null) {
          if (m[1] !== itemVar) vars.push(m[1]);
        }
        return [...new Set(vars)];
      };

      const render = (data) => {
        if (!Array.isArray(data)) return;
        lastData = data;
        const sig = dataSignature(data);
        if (sig !== null && sig === renderedSig && containerEl.children.length) {
          return;
        }
        renderedSig = sig;
        containerEl.innerHTML = '';
        items = [];
        data.forEach((itemData, index) => {
          const base = makeRow(itemData, index);
          if (!base) return;
          // Holes are already filled into the HTML; interpolate() then resolves any
          // remaining bare $.field / $item.field tokens (non-hole positions).
          const el = interpolate(base, itemData, index);
          el.__stItem = itemData; // retain for reactive re-filtering
          // DEV SELECTION ADDRESS. A rendered row is the only place that still
          // knows which SOURCE and which INDEX produced it — once it is in the DOM
          // that provenance is gone, so an inspector cannot map a clicked element
          // back to the JSON record behind it. `each.st` has stamped this since
          // PLAN-057; `each-inline.st` (the variant every backtick-hole `@each`
          // compiles to — i.e. the common case) never did, so pages built from
          // `@data` collections carried NO selection identity at all and pick mode
          // had nothing to resolve. Dev-only, exactly like the sibling: production
          // output must stay clean.
          if (window.__ST_DEV__) {
            el.setAttribute('data-st-source', sourceName);
            el.setAttribute('data-st-index', String(index));
          }
          containerEl.appendChild(el);
          items.push(el);
        });
        if (filterExpr) applyFilter();
        %yield items -> $items;
      };

      // BUG-155-class fix: `sourceName` may be a DOTTED PATH (`"wrap.items"` from
      // `@each($wrap.items as $x) { <li>...</li> }`) -- the event channels below
      // are keyed by the HEAD signal (no signal is ever literally named
      // "wrap.items"), and a plain `window.SpacetimeLocal[sourceName]` lookup
      // never matches a dotted name either. Mirrors each-with-templates' fix:
      // resolve/watch the HEAD name, re-derive the full dotted path via
      // ST.resolvePath on every fire.
      const headName = sourceName.split('.')[0];
      const isDotted = sourceName !== headName;
      const resolveSourceData = () => {
        if (typeof ST !== 'undefined' && ST.resolvePath) {
          const v = ST.resolvePath(containerEl, sourceName);
          if (v !== undefined) return v;
        }
        // Registry fallback (ST.setData-published sources, e.g. @data fetch) is
        // keyed by the HEAD name too -- walk the remaining dotted segments as
        // plain property access, same as each-with-templates' equivalent path.
        const headData = (typeof ST !== 'undefined' && ST._dataRegistry) ? ST._dataRegistry.get(headName)?.data : undefined;
        const fromRegistry = headData !== undefined
          ? sourceName.split('.').slice(1).reduce((v, k) => (v == null ? v : v[k]), headData)
          : undefined;
        if (fromRegistry !== undefined) return fromRegistry;
        return (typeof window.SpacetimeLocal !== 'undefined') ? window.SpacetimeLocal[headName] : undefined;
      };

      // React on BOTH channels (data registry + local signal), mirroring
      // each-with-templates, so the list re-renders for @data fetch sources and
      // direct $signal mutations alike. For a plain (non-dotted) source, the
      // channel payload (`e.detail`) already IS the exact new value -- trust it
      // directly, unchanged from before. For a dotted source, `e.detail` is the
      // HEAD signal's whole value (not the nested field this @each wants), so
      // re-derive the full path instead.
      document.addEventListener(`data:${headName}:loaded`, (e) => {
        render(isDotted ? resolveSourceData() : e.detail);
      });
      document.addEventListener(`local:${headName}:updated`, (e) => {
        const data = isDotted ? resolveSourceData() : e.detail;
        if (data != null) render(data);
      });
      // Re-filter (not re-render) when a filter-expression signal changes — live
      // search / category filter, zero custom JS (FEAT-094).
      getFilterStateVars(filterExpr).forEach((varName) => {
        document.addEventListener(`local:${varName}:updated`, applyFilter);
      });
      // Initial paint from the current signal value, if present.
      const initialData = resolveSourceData();
      if (initialData != null) {
        render(initialData);
      }
    })()
  }

  %exports {
    $items: array
  }
}
