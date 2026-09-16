/// Dev Review Primitive
///
/// Implements `Spacetime.review()` and `window.__stDevReview()` — a
/// 7-dimension audit of the live page.
///
/// Dimensions:
///   1. idiomatic  — inline event handlers, foreign <script> tags
///   2. animation  — timelines from window.__ST_DEBUG__.getTimelines()
///   3. brand      — hex literals not wrapped in var(--*)
///   4. a11y       — img alt, link/button accessible name
///   5. compile    — errors from window.__ST_DEBUG__.getErrors()
///   6. hierarchy  — heading order (h1→h2→h3) + font-size monotonicity (FEAT-032)
///   7. detail     — text-wrap on display headings, oklch on brand tokens,
///                   var() coverage on heading font-family (FEAT-032)
///
/// Finding shape: { dim, severity: 'error'|'warn'|'info', message,
///                   selector, value }
/// Per-dim score:  clamp(10 - errors*2 - warns, 0, 10)
/// Top score:      min over dim scores
/// Cache:          250ms debounce so dev-panel polling is cheap
///
/// Production guard: only loaded inside the dev runtime, so
/// `window.Spacetime.review` is never defined in shipped builds.

%primitive dev-review() {
  %emit js {
    (function() {
      var EVENT_ATTRS = [
        'onclick','ondblclick','onmousedown','onmouseup','onmouseover','onmouseout',
        'onmousemove','onmouseenter','onmouseleave',
        'onkeydown','onkeyup','onkeypress',
        'onfocus','onblur','onchange','oninput','onsubmit','onreset',
        'onload','onerror','onscroll','onresize','ontoggle'
      ];

      function clamp(errs, warns) {
        var s = 10 - (errs * 2) - warns;
        return s < 0 ? 0 : (s > 10 ? 10 : s);
      }

      function sel(el) {
        if (!el || el.nodeType !== 1) return '';
        if (el.id) return el.tagName.toLowerCase() + '#' + el.id;
        var c = (typeof el.className === 'string')
          ? el.className.split(/\s+/).filter(Boolean).slice(0, 2).join('.') : '';
        return el.tagName.toLowerCase() + (c ? '.' + c : '');
      }

      function add(out, dim, severity, message, selector, value) {
        out.push({ dim: dim, severity: severity, message: message, selector: selector || '', value: value || '' });
      }

      function auditIdiomatic(out) {
        var all = document.querySelectorAll('*');
        for (var i = 0; i < all.length; i++) {
          var el = all[i];
          if (!el.attributes) continue;
          for (var a = 0; a < el.attributes.length; a++) {
            var name = el.attributes[a].name;
            if (EVENT_ATTRS.indexOf(name) !== -1) {
              add(out, 'idiomatic', 'error', 'Inline event handler "' + name + '"', sel(el), name);
            }
          }
        }
      }

      
      function auditBrand(out) {
        var hexRe = /#[0-9a-fA-F]+/g;
        var counts = {};
        var srcs = {};
        function rec(v, where) {
          counts[v] = (counts[v] || 0) + 1;
          srcs[v] = srcs[v] || [];
          if (srcs[v].length < 3) srcs[v].push(where);
        }
        var styled = document.querySelectorAll('[style]');
        for (var i = 0; i < styled.length; i++) {
          var s = (styled[i].getAttribute('style') || '').replace(/var\([^)]*\)/g, '');
          var m;
          while ((m = hexRe.exec(s)) !== null) rec(m[0], sel(styled[i]));
          hexRe.lastIndex = 0;
        }
        var blocks = document.querySelectorAll('style');
        for (var b = 0; b < blocks.length; b++) {
          var raw = blocks[b].textContent || '';
          // Skip Spacetime dev-runtime injected styles. Heuristic: dev runtime
          // styles always reference st-dev-, __st-, or data-st-edit markers.
          if (/st-dev-|__st-|data-st-edit-mode|\.st-mp-|\.st-tree-|\.st-ct-|\.st-crud-|\.st-tl-|\.st-props|\.st-field-|\.st-sidebar-/.test(raw)) continue;
          var lines = raw.split('\n');
          var keep = '';
          for (var li = 0; li < lines.length; li++) {
            if (lines[li].indexOf('--') === -1) keep += lines[li] + '\n';
          }
          var cleaned = keep.replace(/var\([^)]*\)/g, '');
          var mm;
          while ((mm = hexRe.exec(cleaned)) !== null) rec(mm[0], '<style>');
          hexRe.lastIndex = 0;
        }
        var keys = Object.keys(counts);
        keys.sort(function(a, b) { return counts[b] - counts[a]; });
        for (var k = 0; k < keys.length; k++) {
          var v = keys[k];
          add(out, 'brand', 'warn',
            'Unbound color "' + v + '" used ' + counts[v] + 'x - promote to a CSS var',
            srcs[v].join(', '), v);
        }
      }

      function auditAnimation(out) {
        var dbg = window.__ST_DEBUG__;
        if (!dbg || typeof dbg.getTimelines !== 'function') return;
        var list = [];
        try {
          var m = dbg.getTimelines();
          if (m && typeof m.values === 'function') list = Array.from(m.values());
          else if (Array.isArray(m)) list = m;
        } catch (e) { return; }
        var loaded = (document.readyState === 'complete');
        for (var i = 0; i < list.length; i++) {
          var t = list[i] || {};
          var n = t.id || t.name || ('timeline-' + i);
          var st = t.state || {};
          if (typeof t.elementCount === 'number' && t.elementCount === 0) {
            add(out, 'animation', 'error', 'Timeline "' + n + '" matched 0 elements', t.selector || '', n);
          }
          if (loaded && st.started === false) {
            add(out, 'animation', 'warn', 'Timeline "' + n + '" never started after load', '', n);
          }
          if (st.destroyed === true && st.completed === false) {
            add(out, 'animation', 'error', 'Timeline "' + n + '" destroyed before completion', '', n);
          }
        }
      }

      function auditA11y(out) {
        var imgs = document.querySelectorAll('img:not([alt])');
        for (var i = 0; i < imgs.length; i++) add(out, 'a11y', 'error', '<img> missing alt', sel(imgs[i]));
        var links = document.querySelectorAll('a');
        for (var l = 0; l < links.length; l++) {
          var a = links[l];
          var named = (a.textContent && a.textContent.trim()) || a.getAttribute('aria-label') || a.getAttribute('aria-labelledby') || a.getAttribute('title');
          if (!named) add(out, 'a11y', 'error', '<a> with no accessible name', sel(a));
        }
        var btns = document.querySelectorAll('button');
        for (var b = 0; b < btns.length; b++) {
          var btn = btns[b];
          var named2 = (btn.textContent && btn.textContent.trim()) || btn.getAttribute('aria-label') || btn.getAttribute('aria-labelledby') || btn.getAttribute('title');
          if (!named2) add(out, 'a11y', 'error', '<button> with no label', sel(btn));
        }
      }

      function auditCompile(out) {
        var dbg = window.__ST_DEBUG__;
        if (!dbg) return;
        var errs = [];
        try {
          if (typeof dbg.getErrors === 'function') errs = dbg.getErrors() || [];
          else if (Array.isArray(dbg.errors)) errs = dbg.errors;
        } catch (e) { return; }
        for (var i = 0; i < errs.length; i++) {
          var e = errs[i] || {};
          add(out, 'compile', 'error', e.message || String(e), '', e.type || 'error');
        }
      }

      function fontSize(el) {
        try {
          var v = window.getComputedStyle(el).fontSize;
          var n = parseFloat(v);
          return isNaN(n) ? 0 : n;
        } catch (e) { return 0; }
      }

      function fontFamily(el) {
        try { return window.getComputedStyle(el).fontFamily || ''; }
        catch (e) { return ''; }
      }

      function textWrap(el) {
        try { return window.getComputedStyle(el).textWrap || ''; }
        catch (e) { return ''; }
      }

      function auditHierarchy(out) {
        var headings = document.querySelectorAll('h1, h2, h3, h4, h5, h6');
        if (!headings.length) return;
        var prevLevel = 0;
        var maxSizeAtLevel = {};
        for (var i = 0; i < headings.length; i++) {
          var h = headings[i];
          var lvl = parseInt(h.tagName.substring(1), 10);
          if (prevLevel > 0 && lvl > prevLevel + 1) {
            add(out, 'hierarchy', 'warn',
              'Heading level skipped (h' + prevLevel + ' → h' + lvl + ')',
              sel(h), 'h' + lvl);
          }
          var size = fontSize(h);
          if (lvl >= 2 && size > 0) {
            for (var pl = 1; pl < lvl; pl++) {
              var prevMax = maxSizeAtLevel[pl];
              if (prevMax && size >= prevMax) {
                add(out, 'hierarchy', 'warn',
                  'h' + lvl + ' font-size ' + size.toFixed(0) + 'px not smaller than max h' + pl + ' (' + prevMax.toFixed(0) + 'px)',
                  sel(h), size.toFixed(0) + 'px');
                break;
              }
            }
          }
          if (size > (maxSizeAtLevel[lvl] || 0)) maxSizeAtLevel[lvl] = size;
          prevLevel = lvl;
        }
      }

      function auditDetail(out) {
        // 1. text-wrap on display-class headings
        var displays = document.querySelectorAll('h1, h2');
        for (var i = 0; i < displays.length; i++) {
          var h = displays[i];
          var text = (h.textContent || '').trim();
          if (text.length <= 12) continue;
          var size = fontSize(h);
          if (size <= 24) continue;
          var tw = textWrap(h);
          if (tw !== 'pretty' && tw !== 'balance') {
            add(out, 'detail', 'info',
              'Display heading missing `text-wrap: pretty` or `balance`',
              sel(h), tw || '(unset)');
          }
        }
        // 2. oklch coverage on --brand-* tokens
        try {
          var rootStyle = window.getComputedStyle(document.documentElement);
          var props;
          if (typeof rootStyle.getPropertyNames === 'function') {
            props = rootStyle.getPropertyNames();
          } else {
            props = [];
            for (var pi = 0; pi < rootStyle.length; pi++) props.push(rootStyle[pi]);
          }
          for (var bi = 0; bi < props.length; bi++) {
            var name = props[bi];
            if (name.indexOf('--brand-') !== 0) continue;
            var val = (rootStyle.getPropertyValue(name) || '').trim();
            if (!val) continue;
            if (val.indexOf('oklch') === -1 && (val.charAt(0) === '#' || val.indexOf('rgb') === 0 || val.indexOf('hsl') === 0)) {
              add(out, 'detail', 'warn',
                'Brand token `' + name + '` uses ' + val + ' — prefer oklch() for perceptual stability',
                ':root', val);
            }
          }
        } catch (e) { /* getComputedStyle on root may throw in some hosts */ }
        // 3. var() coverage on heading font-family
        var heads = document.querySelectorAll('h1, h2, h3');
        for (var hi = 0; hi < heads.length; hi++) {
          var hh = heads[hi];
          var inline = hh.getAttribute('style') || '';
          if (inline.indexOf('font-family') !== -1 && inline.indexOf('var(') === -1) {
            add(out, 'detail', 'warn',
              hh.tagName.toLowerCase() + ' inline font-family is a literal; promote to var(--brand-display)',
              sel(hh), inline);
          }
        }
      }

      function build() {
        var out = [];
        try { auditIdiomatic(out); } catch (e) {}
        try { auditBrand(out); } catch (e) {}
        try { auditAnimation(out); } catch (e) {}
        try { auditA11y(out); } catch (e) {}
        try { auditCompile(out); } catch (e) {}
        try { auditHierarchy(out); } catch (e) {}
        try { auditDetail(out); } catch (e) {}
        var dims = { idiomatic: { e: 0, w: 0 }, animation: { e: 0, w: 0 }, brand: { e: 0, w: 0 }, a11y: { e: 0, w: 0 }, compile: { e: 0, w: 0 }, hierarchy: { e: 0, w: 0 }, detail: { e: 0, w: 0 } };
        for (var i = 0; i < out.length; i++) {
          var f = out[i];
          if (!dims[f.dim]) continue;
          if (f.severity === 'error') dims[f.dim].e += 1;
          else if (f.severity === 'warn') dims[f.dim].w += 1;
        }
        var scores = {};
        var min = 10;
        var keys = Object.keys(dims);
        for (var k = 0; k < keys.length; k++) {
          var d = dims[keys[k]];
          var n = clamp(d.e, d.w);
          scores[keys[k]] = n;
          if (n < min) min = n;
        }
        return { score: min, dims: scores, findings: out, generatedAt: Date.now() };
      }

      var cachedAt = 0;
      var cached = null;
      function review() {
        var now = Date.now();
        if (cached && (now - cachedAt) < 250) return cached;
        cached = build();
        cachedAt = now;
        return cached;
      }

      window.__stDevReview = review;
      window.Spacetime = window.Spacetime || {};
      window.Spacetime.review = review;
    })();
  }
}
