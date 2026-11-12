/// Studio kit (PLAN-154) - the studio's client runtime, as one module primitive.
///
/// Every function hangs off `ST.stu`/`ST.stu*` and is installed once per page
/// through the prelude. The `.st` UI (stdlib/__studio__/index.st) never touches
/// the DOM itself: it derives its views from these pure helpers and calls the
/// actions through the `$stuNoop <- ST.stuX($)` idiom.
///
/// Four layers, bottom up:
///
/// 1. VIEW MODEL (pure): outline JSON -> flat rows for the explorer and the
///    inspector (`stuLayers`, `stuLists`, `stuComponents`, `stuPages`,
///    `stuFields`, `stuRules`, `stuBox`). Flat rows with a depth, because a
///    flat `@each` is what the dock renders without re-rendering siblings.
/// 2. SELECTION: a DOM element -> a descriptor (`describe`): what it is, which
///    list row it was rendered from (found by CONTENT against the outline, so it
///    works through templates and filters), its rect and computed design.
/// 3. TRANSPORT: `apply(op)` POSTs one edit to /__spacetime/studio/apply and
///    publishes `$stuStatus` / `$stuError`. The server writes the source; the
///    file watcher reloads the page; the page is the source again.
/// 4. CANVAS: when the studio is open - hover box, click to select, double-click
///    to edit text in place, drag a row's handle to reorder its list. Links do
///    not navigate while editing. A reload never interrupts typing: the dev
///    live-reload fires a cancellable `st:reload` first, and the canvas defers it.
%primitive studio-kit() {
  %emit prelude-js {
    window.ST = window.ST || {};
    if (!ST.stu) {
      (function () {
        var S = ST.stu = { el: null, selRow: null, editing: null, drag: null, pendingReload: false, inflight: 0, boxes: null, raf: 0, on: false };
        var L = function () { return window.SpacetimeLocal || {}; };
        var put = function (k, v) { if (window.SpacetimeLocal) { window.SpacetimeLocal[k] = v; } };
        var norm = function (s) { return String(s == null ? '' : s).replace(/\s+/g, ' ').trim(); };
        var clip = function (s, n) { s = norm(s); return s.length > n ? s.slice(0, n - 1) + '...' : s; };
        var outline = function () { return L().stuOutline || null; };
        var entry = function () {
          var raw = L().stuEntry || '';
          try { return decodeURIComponent(raw); } catch (e) { return raw; }
        };
        var inUi = function (node) {
          return !!(node && node.closest && node.closest('.st-dev-dock, .stu-ui, .stu-overlay'));
        };

        // ---- 1. view model ------------------------------------------------------

        var SKIP_TAGS = { title: 1, meta: 1, link: 1, br: 1 };

        ST.stuLayers = function (o, q) {
          var rows = [];
          if (!o || !o.markup) { return rows; }
          var query = norm(q).toLowerCase();
          o.markup.forEach(function (f) {
            var nodes = (f.nodes || []).filter(function (n) { return !SKIP_TAGS[n.tag]; });
            if (!nodes.length) { return; }
            var hits = nodes.filter(function (n) {
              return !query || (n.label + ' ' + n.text).toLowerCase().indexOf(query) >= 0;
            });
            if (!hits.length) { return; }
            rows.push({ key: 'f:' + f.file, kind: 'file', label: f.file, text: nodes.length + ' layers', depth: 0, file: f.file, index: -1 });
            hits.forEach(function (n) {
              rows.push({
                key: 'n:' + f.file + ':' + n.index, kind: n.static ? 'node' : 'bound',
                label: n.label, text: clip(n.text, 42), depth: n.depth + 1, file: f.file, index: n.index
              });
            });
          });
          return rows;
        };

        /// A row's readable name: a title-like field, else the first string
        /// field with real content (a short code like "01" names nothing).
        var NAME_KEYS = ['title', 'name', 'label', 'heading', 'line', 'what', 'text'];
        var rowText = function (r) {
          for (var i = 0; i < NAME_KEYS.length; i++) {
            var f = r.fields.filter(function (x) { return x.key === NAME_KEYS[i] && norm(x.value); })[0];
            if (f) { return f.value; }
          }
          var first = r.fields.filter(function (x) { return x.kind === 'string' && norm(x.value).length >= 4; })[0];
          return first ? first.value : r.label;
        };

        ST.stuLists = function (o, q) {
          var rows = [];
          if (!o || !o.lists) { return rows; }
          var query = norm(q).toLowerCase();
          o.lists.forEach(function (l) {
            if (!l.count) { return; }
            var hits = l.rows.filter(function (r) { return !query || norm(r.label).toLowerCase().indexOf(query) >= 0 || l.binding.toLowerCase().indexOf(query) >= 0; });
            if (!hits.length) { return; }
            rows.push({ key: 'l:' + l.binding, kind: 'list', label: '$' + l.binding, text: l.count + ' items', depth: 0, binding: l.binding, index: -1, file: l.file, drag: 'false' });
            hits.forEach(function (r) {
              rows.push({ key: 'r:' + l.binding + ':' + r.index, kind: 'row', label: clip(rowText(r), 46), text: String(r.index + 1), depth: 1, binding: l.binding, index: r.index, file: l.file, drag: 'true' });
            });
          });
          return rows;
        };

        ST.stuComponents = function (o, q) {
          if (!o || !o.templates) { return []; }
          var query = norm(q).toLowerCase();
          return o.templates
            .filter(function (t) { return !query || (t.name + ' ' + t.doc + ' ' + t.group).toLowerCase().indexOf(query) >= 0; })
            .map(function (t) {
              return { key: 't:' + t.name, name: '&' + t.name, params: '(' + t.params + ')', doc: t.doc || 'No doc yet: write a /// line above the template.', group: t.group || 'ungrouped', file: t.file };
            });
        };

        ST.stuPages = function (o) {
          if (!o || !o.pages) { return []; }
          var here = location.pathname.replace(/\/$/, '') || '/';
          return o.pages.map(function (p) {
            var route = p.route.replace(/\/$/, '') || '/';
            return { route: p.route, file: p.file, current: route === here ? 'true' : 'false' };
          });
        };

        ST.stuFields = function (sel) {
          if (!sel || !sel.row) { return []; }
          return sel.row.fields.map(function (f) {
            var v = String(f.value);
            return {
              key: f.key || 'value', field: f.key, value: v, kind: f.kind,
              binding: sel.row.binding, index: sel.row.index,
              rows: Math.min(6, Math.max(1, Math.ceil(v.length / 34)))
            };
          });
        };

        /// The selected element, from the selection itself: the cached element
        /// while it is in the page, else the path it was described by (a hot
        /// patch or a re-render replaces nodes without a new selection).
        var elOf = function (sel) {
          if (!sel) { return null; }
          if (S.el && S.el.isConnected) { return S.el; }
          try { return (S.el = document.querySelector(sel.path)); } catch (e) { return null; }
        };
        /// Forget the page selection without touching the film's (the film calls it).
        S.forget = function () {
          S.el = null; S.selRow = null;
          put('stuSel', null); put('stuSelKey', '');
          S.paint();
        };

        ST.stuRules = function (sel, o) {
          var out = [];
          var el = elOf(sel);
          if (!sel || !el || !o || !o.rules) { return out; }
          o.rules.forEach(function (r) {
            var hit = false;
            try { hit = el.matches(r.selector); } catch (e) { hit = false; }
            if (!hit) { return; }
            out.push({ kind: 'rule', a: r.selector, b: r.file, value: '', file: r.file, start: 0, end: 0, source: '' });
            (r.forms || []).forEach(function (f) {
              out.push({ kind: 'form', a: '--' + f.name.replace(/^--/, ''), b: f.args ? '(' + f.args + ')' : 'brand form', value: '', file: r.file, start: 0, end: 0, source: '' });
            });
            (r.decls || []).forEach(function (d) {
              out.push({ kind: 'decl', a: d.prop, b: '', value: d.value, file: r.file, start: d.span[0], end: d.span[1], source: d.source });
            });
          });
          return out;
        };

        var px = function (v) { var n = parseFloat(v); return isNaN(n) ? v : (Math.round(n * 10) / 10) + ''; };

        ST.stuBox = function (sel) {
          var el = elOf(sel);
          if (!sel || !el || !el.isConnected) { return []; }
          var cs = getComputedStyle(el);
          var r = el.getBoundingClientRect();
          var rows = [
            { group: 'Frame', label: 'W', value: px(r.width), swatch: '' },
            { group: 'Frame', label: 'H', value: px(r.height), swatch: '' },
            { group: 'Frame', label: 'X', value: px(r.left + scrollX), swatch: '' },
            { group: 'Frame', label: 'Y', value: px(r.top + scrollY), swatch: '' },
            { group: 'Layout', label: 'Display', value: cs.display, swatch: '' },
            { group: 'Layout', label: 'Padding', value: cs.padding, swatch: '' },
            { group: 'Layout', label: 'Margin', value: cs.margin, swatch: '' }
          ];
          if (cs.display.indexOf('flex') >= 0 || cs.display.indexOf('grid') >= 0) {
            rows.push({ group: 'Layout', label: 'Gap', value: cs.gap, swatch: '' });
          }
          rows.push(
            { group: 'Type', label: 'Font', value: cs.fontFamily.split(',')[0].replace(/"/g, ''), swatch: '' },
            { group: 'Type', label: 'Size', value: cs.fontSize, swatch: '' },
            { group: 'Type', label: 'Weight', value: cs.fontWeight, swatch: '' },
            { group: 'Type', label: 'Line', value: cs.lineHeight, swatch: '' },
            { group: 'Type', label: 'Tracking', value: cs.letterSpacing, swatch: '' },
            { group: 'Fill', label: 'Text', value: cs.color, swatch: cs.color },
            { group: 'Fill', label: 'Background', value: cs.backgroundColor, swatch: cs.backgroundColor }
          );
          if (parseFloat(cs.borderTopWidth) > 0) {
            rows.push({ group: 'Stroke', label: 'Border', value: cs.borderTopWidth + ' ' + cs.borderTopStyle, swatch: cs.borderTopColor });
          }
          if (cs.boxShadow && cs.boxShadow !== 'none') {
            rows.push({ group: 'Effects', label: 'Shadow', value: cs.boxShadow, swatch: '' });
          }
          return rows;
        };

        // ---- 2. selection ---------------------------------------------------------

        var labelOf = function (el) {
          var cls = el.classList && el.classList[0];
          return el.tagName.toLowerCase() + (cls ? '.' + cls : '');
        };

        var pathOf = function (el) {
          var parts = [];
          for (var n = el; n && n.nodeType === 1 && n !== document.body; n = n.parentElement) {
            var i = 1;
            for (var s = n.previousElementSibling; s; s = s.previousElementSibling) { if (s.tagName === n.tagName) { i++; } }
            parts.unshift(n.tagName.toLowerCase() + ':nth-of-type(' + i + ')');
          }
          return 'body > ' + parts.join(' > ');
        };

        /// The list row an element was rendered from: the nearest ancestor that is
        /// an `@each` row (it carries `data-index`), matched by CONTENT to the
        /// source row whose strings it shows. Every row of every list competes,
        /// so the index is the SOURCE index even when the page renders a
        /// filtered, searched or derived view of the list (whose own index
        /// would point at the wrong item). The rendered index only breaks ties.
        var scoreRow = function (row, text) {
          var score = 0;
          row.fields.forEach(function (f) {
            var v = norm(f.value);
            if (f.kind === 'string' && v.length > 1 && text.indexOf(v) >= 0) { score += 1 + Math.min(v.length, 40) / 40; }
          });
          return score;
        };

        S.rowOf = function (el) {
          var o = outline();
          if (!o || !o.lists) { return null; }
          for (var n = el; n && n !== document.body; n = n.parentElement) {
            if (!n.hasAttribute || !n.hasAttribute('data-index') || inUi(n)) { continue; }
            var shown = parseInt(n.getAttribute('data-index'), 10);
            var text = norm(n.textContent);
            var best = null, bestScore = 0;
            o.lists.forEach(function (l) {
              l.rows.forEach(function (row) {
                var score = scoreRow(row, text) + (row.index === shown ? 0.01 : 0);
                if (score > bestScore) {
                  bestScore = score;
                  best = { binding: l.binding, index: row.index, label: row.label, fields: row.fields, count: l.count, file: l.file };
                }
              });
            });
            if (best && bestScore >= 1) { best.el = n; return best; }
          }
          return null;
        };

        var describe = function (el) {
          var row = S.rowOf(el);
          var crumbs = [];
          for (var n = el; n && n !== document.body && crumbs.length < 4; n = n.parentElement) { crumbs.unshift(labelOf(n)); }
          var d = {
            key: '', tag: el.tagName.toLowerCase(), label: labelOf(el),
            text: clip(el.textContent, 160), crumbs: crumbs.join(' / '), path: pathOf(el),
            row: null
          };
          if (row) {
            d.row = { binding: row.binding, index: row.index, label: row.label, fields: row.fields, count: row.count, file: row.file };
            d.key = 'r:' + row.binding + ':' + row.index;
          }
          return d;
        };

        /// The explorer key of an element: its list row, else the markup node
        /// that declares it (same tag and classes, same text when static).
        S.keyFor = function (el) {
          var row = S.rowOf(el);
          if (row) { return 'r:' + row.binding + ':' + row.index; }
          var o = outline();
          if (!o || !o.markup) { return ''; }
          var tag = el.tagName.toLowerCase(), cls = [].slice.call(el.classList), text = norm(el.textContent);
          var fallback = '';
          for (var i = 0; i < o.markup.length; i++) {
            var f = o.markup[i];
            for (var j = 0; j < f.nodes.length; j++) {
              var n = f.nodes[j];
              if (n.tag !== tag || n.classes.join(' ') !== cls.join(' ')) { continue; }
              var key = 'n:' + f.file + ':' + n.index;
              if (n.static && norm(n.text) === text) { return key; }
              if (!fallback) { fallback = key; }
            }
          }
          return fallback;
        };

        S.select = function (el) {
          S.el = el || null;
          // a page selection clears the film's: the inspector shows one thing
          if (el) { put('stuFilmSel', ''); }
          S.selRow = el ? S.rowOf(el) : null;
          put('stuSel', el ? describe(el) : null);
          put('stuSelKey', el ? S.keyFor(el) : '');
          S.paint();
        };

        /// Mark the selected explorer row and bring it into view (Figma's
        /// "select on canvas, reveal in layers").
        S.markTree = function () {
          var key = L().stuSelKey || '';
          var nodes = document.querySelectorAll('.stu-node');
          var stamp = key + '|' + nodes.length;
          if (stamp === S.marked) { return; }
          S.marked = stamp;
          var hit = null;
          for (var i = 0; i < nodes.length; i++) {
            var on = !!key && nodes[i].dataset.key === key;
            nodes[i].classList.toggle('is-selected', on);
            if (on && nodes[i].offsetParent) { hit = nodes[i]; }
          }
          if (hit) { hit.scrollIntoView({ block: 'nearest' }); }
        };

        // ---- 3. transport -----------------------------------------------------------

        S.queue = Promise.resolve();
        S.apply = function (op) {
          return S.send('/__spacetime/studio/apply', Object.assign({ entry: entry() }, op), 'Saved to ');
        };
        /// Undo the last edit (any tool's: the dev server keeps one journal).
        S.undo = function () { return S.send('/__spacetime/rewrite/undo', {}, 'Undone in '); };

        /// Send a write to the dev server, then take it in place or reload.
        S.send = function (url, body, done) {
          put('stuError', '');
          put('stuStatus', 'Saving...');
          S.inflight++;
          S.queue = S.queue.then(S.hotBaseline).then(function () {
            return fetch(url, {
              method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body)
            }).then(function (r) { return r.json(); }).then(function (res) {
              if (res.ok) {
                put('stuStatus', res.changed && res.changed.length ? done + res.changed.join(', ') : 'Nothing to change');
                if (res.changed && res.changed.length) {
                  S.remember();
                  // An edit the running page can take IN PLACE (PLAN-155: only
                  // hot-keyed arguments moved — a keyframe table, a clip
                  // window, a synth program) is applied without a reload.
                  // Anything else reloads: the file watcher does it, and if it
                  // has not in a moment (it can miss a write), the page goes
                  // back to its source by itself.
                  S.hotting = true;
                  return S.hot().then(function (live) {
                    S.hotting = false;
                    if (live) {
                      S.hotDone = Date.now();
                      S.pendingReload = false;
                      put('stuStatus', done + res.changed.join(', ') + ' - live');
                    } else {
                      setTimeout(function () {
                        if (S.inflight || S.editing) { S.pendingReload = true; } else { location.reload(); }
                      }, 300);
                    }
                    return res;
                  });
                }
              } else {
                put('stuStatus', '');
                put('stuError', res.error || 'The edit was refused');
              }
              return res;
            }).catch(function (e) {
              put('stuStatus', '');
              put('stuError', 'Could not reach the dev server: ' + e.message);
            }).then(function (res) {
              S.inflight--;
              if (!S.inflight && S.pendingReload && !S.editing) { S.pendingReload = false; location.reload(); }
              return res;
            });
          });
          return S.queue;
        };

        // ---- hot edits (PLAN-155) ------------------------------------------------
        var hotUrl = function () { return '/__spacetime/studio/hot.json?entry=' + encodeURIComponent(entry()); };
        /// The page's shape as it was compiled — its fingerprint and arguments —
        /// read once, just before the first edit writes (so it is the page as
        /// loaded, and the studio knows its entry by then).
        S.hotBase = null;
        S.hotBaseline = function () {
          if (S.hotBase || !entry()) { return Promise.resolve(); }
          return fetch(hotUrl()).then(function (r) { return r.json(); }).then(function (h) {
            if (h && !h.error) { S.hotBase = h.fingerprint; if (ST.hotSeen) { ST.hotSeen(h.binds); } }
          }).catch(function () {});
        };
        /// Hand the page what a fresh compile gives its hot-keyed calls; true
        /// when it took them all and nothing else about the page changed.
        S.hot = function () {
          if (!ST.hotApply) { return Promise.resolve(false); }
          return fetch(hotUrl()).then(function (r) { return r.json(); }).then(function (h) {
            if (!h || h.error || !S.hotBase || h.fingerprint !== S.hotBase) { return false; }
            if (!ST.hotApply(h.binds)) { return false; }
            // the page shows the current frame again with its new arguments
            Object.keys(ST._transports || {}).forEach(function (k) {
              var t = ST._transports[k];
              if (t && t.refresh) { t.refresh(); }
            });
            // and the studio re-reads the source (its addresses moved)
            S.refresh();
            return true;
          }).catch(function () { return false; });
        };
        /// Re-read the outline and the film's shape into the panels.
        S.refresh = function () {
          [['stuOutline', '/__spacetime/studio/outline.json?entry='], ['stuScore', '/__spacetime/studio/score.json?entry='], ['stuReflect', '/__spacetime/reflect.json?entry=']].forEach(function (src) {
            fetch(src[1] + encodeURIComponent(entry())).then(function (r) { return r.json(); }).then(function (v) { put(src[0], v); }).catch(function () {});
          });
        };

        /// Keep the place across the reload an edit causes.
        S.remember = function () {
          try {
            sessionStorage.setItem('stu.restore', JSON.stringify({ y: scrollY, path: S.el ? pathOf(S.el) : '', at: Date.now() }));
          } catch (e) {}
        };

        S.restore = function () {
          var saved = null;
          try { saved = JSON.parse(sessionStorage.getItem('stu.restore') || 'null'); sessionStorage.removeItem('stu.restore'); } catch (e) {}
          if (!saved || Date.now() - saved.at > 60000) { return; }
          var go = function () {
            scrollTo(0, saved.y);
            if (saved.path) {
              var el = null;
              try { el = document.querySelector(saved.path); } catch (e) {}
              if (el) { S.select(el); }
            }
          };
          if (document.readyState === 'complete') { setTimeout(go, 120); } else { addEventListener('load', function () { setTimeout(go, 120); }); }
        };

        // Actions the panels call through `$stuNoop <- ST.stuX($)`.
        ST.stuCommitField = function (input) {
          var d = input.dataset;
          if (input.value === input.defaultValue) { return ''; }
          S.apply({ op: 'field', binding: d.binding, index: +d.at, field: d.field || '', value: input.value });
          return '';
        };

        ST.stuCommitDecl = function (input) {
          var d = input.dataset;
          if (input.value === input.defaultValue) { return ''; }
          S.apply({ op: 'decl', file: d.file, start: +d.start, end: +d.end, expect: d.expect, value: input.value });
          return '';
        };

        ST.stuRowAction = function (button) {
          var sel = L().stuSel;
          if (!sel || !sel.row) { return ''; }
          var r = sel.row, act = button.dataset.act;
          if (act === 'up' && r.index > 0) { S.apply({ op: 'move', binding: r.binding, from: r.index, to: r.index - 1 }); }
          if (act === 'down' && r.index < r.count - 1) { S.apply({ op: 'move', binding: r.binding, from: r.index, to: r.index + 1 }); }
          if (act === 'duplicate') { S.apply({ op: 'duplicate', binding: r.binding, index: r.index }); }
          if (act === 'delete' && confirm('Delete this item from $' + r.binding + '?')) { S.apply({ op: 'delete', binding: r.binding, index: r.index }); }
          return '';
        };

        /// Explorer -> page: find what a tree row stands for and select it.
        ST.stuReveal = function (node) {
          var d = node.closest('[data-key]').dataset;
          var o = outline();
          var target = null;
          if (d.kind === 'row' || d.kind === 'list') {
            var idx = d.kind === 'row' ? +d.at : 0;
            // By SOURCE identity: a filtered or derived view renders this row
            // under a different index, or not at all.
            var rows = document.querySelectorAll('[data-index]');
            for (var i = 0; i < rows.length && !target; i++) {
              if (inUi(rows[i])) { continue; }
              var hit = S.rowOf(rows[i]);
              if (hit && hit.el === rows[i] && hit.binding === d.binding && (d.kind === 'list' || hit.index === idx)) { target = rows[i]; }
            }
            if (d.kind === 'list' && target) { target = target.parentElement; }
          } else if (o && d.file) {
            var file = o.markup.filter(function (f) { return f.file === d.file; })[0];
            var n = file && file.nodes.filter(function (x) { return String(x.index) === d.at; })[0];
            if (n) {
              var sel = n.tag + (n.classes.length ? '.' + n.classes.map(CSS.escape).join('.') : '');
              var all = document.querySelectorAll(sel);
              for (var j = 0; j < all.length && !target; j++) {
                if (!inUi(all[j]) && (!n.static || norm(all[j].textContent) === norm(n.text))) { target = all[j]; }
              }
              if (!target) { for (var k = 0; k < all.length && !target; k++) { if (!inUi(all[k])) { target = all[k]; } } }
            }
          }
          if (target) {
            target.scrollIntoView({ block: 'center', behavior: 'smooth' });
            S.select(target);
            put('stuError', '');
          } else {
            put('stuError', 'Not on this page right now (hidden, or rendered elsewhere).');
          }
          put('stuSelKey', d.key);
          return '';
        };

        // ---- 4. canvas -----------------------------------------------------------------

        var overlay = function () {
          if (S.boxes) { return S.boxes; }
          var mk = function (cls, html) {
            var d = document.createElement('div');
            d.className = 'stu-overlay ' + cls;
            if (html) { d.innerHTML = html; }
            // fixed, in viewport coordinates: beside <body>, which a film's
            // studio makes the frame of its page (PLAN-155)
            document.documentElement.appendChild(d);
            return d;
          };
          S.boxes = {
            hover: mk('stu-hover'),
            select: mk('stu-select', '<span class="stu-select__label"></span><button type="button" class="stu-handle" title="Drag to reorder" aria-label="Drag to reorder this item">::</button>'),
            drop: mk('stu-drop')
          };
          return S.boxes;
        };

        var place = function (box, el) {
          if (!el || !el.isConnected) { box.style.display = 'none'; return; }
          var r = el.getBoundingClientRect();
          box.style.display = 'block';
          box.style.transform = 'translate(' + r.left + 'px,' + r.top + 'px)';
          box.style.width = r.width + 'px';
          box.style.height = r.height + 'px';
        };

        S.paint = function () {
          if (!S.on) { return; }
          S.markTree();
          var b = overlay();
          place(b.hover, S.hovered && S.hovered !== S.el && !S.editing ? S.hovered : null);
          place(b.select, S.el);
          var sel = L().stuSel;
          b.select.querySelector('.stu-select__label').textContent = sel ? (sel.row ? '$' + sel.row.binding + ' [' + (sel.row.index + 1) + ']  ' : '') + sel.label : '';
          b.select.classList.toggle('has-row', !!S.selRow);
          b.select.classList.toggle('is-editing', !!S.editing);
        };

        var loop = function () {
          S.paint();
          if (S.on) { S.raf = requestAnimationFrame(loop); }
        };

        // Text editing in place. Every text node is compared before/after; each
        // changed one becomes one `text` op with the hints the page can give.
        var textNodes = function (el) {
          var out = [], w = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
          for (var n = w.nextNode(); n; n = w.nextNode()) { if (n.data.trim()) { out.push(n); } }
          return out;
        };

        S.beginEdit = function (el) {
          if (S.editing) { S.endEdit(true); }
          var nodes = textNodes(el);
          if (!nodes.length) { put('stuError', 'No text to edit here.'); return; }
          S.editing = {
            el: el,
            before: nodes.map(function (n) {
              var p = n.parentElement;
              return { node: n, text: n.data, tag: p.tagName.toLowerCase(), classes: [].slice.call(p.classList), elementText: norm(p.textContent), row: S.rowOf(p) };
            })
          };
          el.setAttribute('contenteditable', 'plaintext-only');
          if (el.contentEditable !== 'plaintext-only') { el.setAttribute('contenteditable', 'true'); }
          el.classList.add('stu-is-editing');
          // Keep the browser's own double-click selection (the word under the
          // pointer, so typing replaces it); from the keyboard, put the caret
          // at the end of the text.
          var keep = getSelection();
          var native = keep.rangeCount && !keep.isCollapsed && el.contains(keep.anchorNode);
          el.focus();
          if (!native) {
            var r = document.createRange();
            r.selectNodeContents(el);
            r.collapse(false);
            keep.removeAllRanges();
            keep.addRange(r);
          }
          put('stuStatus', 'Editing: Enter to save, Esc to cancel');
        };

        S.endEdit = function (save) {
          var ed = S.editing;
          if (!ed) { return; }
          S.editing = null;
          ed.el.removeAttribute('contenteditable');
          ed.el.classList.remove('stu-is-editing');
          var now = textNodes(ed.el);
          var same = now.length === ed.before.length && now.every(function (n, i) { return n === ed.before[i].node; });
          if (!save || !same) {
            ed.before.forEach(function (b) { b.node.data = b.text; });
            if (save && !same) { put('stuError', 'Only text can change here; the structure was restored.'); }
            put('stuStatus', '');
          } else {
            var ops = 0;
            ed.before.forEach(function (b) {
              if (b.node.data === b.text) { return; }
              var hint = { tag: b.tag, classes: b.classes, element_text: b.elementText };
              if (b.row) {
                var f = b.row.fields.filter(function (x) { return norm(x.value) === norm(b.text); })[0];
                if (f) { hint = { binding: b.row.binding, index: b.row.index, field: f.key }; }
              }
              ops++;
              S.apply({ op: 'text', old: b.text, new: b.node.data, hint: hint });
            });
            if (!ops) { put('stuStatus', ''); }
          }
          if (S.pendingReload && !S.inflight) { S.pendingReload = false; location.reload(); }
        };

        // Dragging a row by its handle; a line shows where it will land.
        S.beginDrag = function (ev) {
          var row = S.selRow;
          if (!row || !row.el.isConnected) { return; }
          // Each visible sibling's SOURCE index, by content; a sibling from
          // another list (or none) means this is not one list, so no drag.
          var siblings = [].slice.call(row.el.parentElement.children).filter(function (c) {
            return c.hasAttribute('data-index') && getComputedStyle(c).display !== 'none';
          });
          var source = siblings.map(function (c) { var r = S.rowOf(c); return r && r.binding === row.binding ? r.index : -1; });
          if (source.indexOf(-1) >= 0) { put('stuError', 'This list is computed from another; reorder $' + row.binding + ' in the explorer.'); return; }
          S.drag = { row: row, siblings: siblings, source: source, to: row.index };
          ev.preventDefault();
        };

        S.moveDrag = function (ev) {
          var d = S.drag;
          if (!d) { return; }
          var best = null, before = true;
          d.siblings.forEach(function (c) {
            var r = c.getBoundingClientRect();
            var horizontal = d.siblings.length > 1 && Math.abs(d.siblings[0].getBoundingClientRect().top - d.siblings[1].getBoundingClientRect().top) < 4;
            var inside = horizontal ? (ev.clientX >= r.left && ev.clientX <= r.right) : (ev.clientY >= r.top && ev.clientY <= r.bottom);
            if (inside) { best = c; before = horizontal ? ev.clientX < r.left + r.width / 2 : ev.clientY < r.top + r.height / 2; }
          });
          var drop = overlay().drop;
          if (!best) { drop.style.display = 'none'; return; }
          var target = d.source[d.siblings.indexOf(best)];
          var r = best.getBoundingClientRect();
          drop.style.display = 'block';
          drop.style.transform = 'translate(' + r.left + 'px,' + (before ? r.top - 2 : r.bottom - 2) + 'px)';
          drop.style.width = r.width + 'px';
          var to = before ? target : target + 1;
          if (to > d.row.index) { to -= 1; }
          d.to = Math.max(0, Math.min(d.row.count - 1, to));
        };

        S.endDrag = function () {
          var d = S.drag;
          if (!d) { return; }
          S.drag = null;
          overlay().drop.style.display = 'none';
          if (d.to !== d.row.index) {
            var at = d.source.indexOf(d.to > d.row.index ? d.to + 1 : d.to);
            d.row.el.parentElement.insertBefore(d.row.el, at >= 0 ? d.siblings[at] : null);
            S.apply({ op: 'move', binding: d.row.binding, from: d.row.index, to: d.to });
          }
        };

        // Explorer rows of a list can be dragged onto each other.
        S.treeDrop = function (fromEl, toEl) {
          var a = fromEl.dataset, b = toEl.dataset;
          if (a.kind !== 'row' || b.kind !== 'row' || a.binding !== b.binding || a.at === b.at) { return; }
          S.apply({ op: 'move', binding: a.binding, from: +a.at, to: +b.at });
        };

        /// Interact tool, or Alt held: the page behaves like the site.
        var passThrough = function (ev) { return L().stuToolName === 'Interact' || ev.altKey; };

        var handlers = {
          move: function (ev) {
            if (S.drag) { S.moveDrag(ev); return; }
            S.hovered = inUi(ev.target) || passThrough(ev) ? null : ev.target;
          },
          down: function (ev) {
            if (ev.target.closest && ev.target.closest('.stu-handle')) { S.beginDrag(ev); }
          },
          up: function () { if (S.drag) { S.endDrag(); } },
          click: function (ev) {
            if (inUi(ev.target) || (passThrough(ev) && !S.editing)) { return; }
            // the film's 3D tools (its editor camera, its move tool) own the stage's canvas
            if (ST.stuFilm && ST.stuFilm.ownsPointer && ST.stuFilm.ownsPointer(ev)) { return; }
            if (S.editing && S.editing.el.contains(ev.target)) {
              if (ev.target.closest('a')) { ev.preventDefault(); }
              return;
            }
            ev.preventDefault();
            ev.stopPropagation();
            if (S.editing) { S.endEdit(true); }
            S.select(ev.target);
          },
          dblclick: function (ev) {
            if (inUi(ev.target) || passThrough(ev)) { return; }
            ev.preventDefault();
            S.select(ev.target);
            S.beginEdit(ev.target);
          },
          key: function (ev) {
            if (S.editing) {
              if (ev.key === 'Escape') { ev.preventDefault(); S.endEdit(false); }
              else if (ev.key === 'Enter' && !ev.shiftKey) { ev.preventDefault(); S.endEdit(true); }
              return;
            }
            if (inUi(ev.target) || /INPUT|TEXTAREA|SELECT/.test(ev.target.tagName)) { return; }
            // Ctrl/Cmd+Z: the last edit comes back out of the source (a field
            // being typed in keeps its own undo: the guard above)
            if ((ev.metaKey || ev.ctrlKey) && !ev.shiftKey && (ev.key === 'z' || ev.key === 'Z')) {
              ev.preventDefault();
              S.undo();
              return;
            }
            if (!ev.metaKey && !ev.ctrlKey && !ev.altKey) {
              if (ev.key === 'v' || ev.key === 'V') { put('stuToolName', 'Select'); }
              // with the film open, I marks the loop's in point (the film's keys run first)
              if ((ev.key === 'i' || ev.key === 'I') && !document.documentElement.classList.contains('stu-film-on')) { put('stuToolName', 'Interact'); }
            }
            if (ev.key === 'Escape') { S.select(S.el && S.el.parentElement !== document.body ? S.el.parentElement : null); }
            if (ev.key === 'Enter' && S.el) { ev.preventDefault(); S.beginEdit(S.el); }
            var sel = L().stuSel;
            if (ev.altKey && sel && sel.row && (ev.key === 'ArrowUp' || ev.key === 'ArrowDown')) {
              ev.preventDefault();
              ST.stuRowAction({ dataset: { act: ev.key === 'ArrowUp' ? 'up' : 'down' } });
            }
          },
          focusout: function (ev) {
            if (S.editing && ev.target === S.editing.el) { S.endEdit(true); }
          },
          dragstart: function (ev) {
            var row = ev.target.closest && ev.target.closest('.stu-node[data-kind="row"]');
            if (row) { S.treeDrag = row; ev.dataTransfer.effectAllowed = 'move'; }
          },
          dragover: function (ev) {
            var row = S.treeDrag && ev.target.closest && ev.target.closest('.stu-node[data-kind="row"]');
            if (row && row.dataset.binding === S.treeDrag.dataset.binding) { ev.preventDefault(); }
          },
          drop: function (ev) {
            var row = S.treeDrag && ev.target.closest && ev.target.closest('.stu-node[data-kind="row"]');
            if (row) { ev.preventDefault(); S.treeDrop(S.treeDrag, row); }
            S.treeDrag = null;
          },
          reload: function (ev) {
            // An edit the page has just taken live is not a reload (PLAN-155).
            if (S.hotDone && Date.now() - S.hotDone < 2500) { ev.preventDefault(); return; }
            // Typing, or an edit still on its way: the reload waits for both.
            if (S.editing || S.inflight || S.hotting) { ev.preventDefault(); S.pendingReload = true; return; }
            S.remember();
          }
        };

        S.setOn = function (on) {
          on = !!on;
          if (on === S.on) { return; }
          S.on = on;
          document.documentElement.classList.toggle('stu-on', on);
          var m = on ? 'addEventListener' : 'removeEventListener';
          document[m]('pointermove', handlers.move, true);
          document[m]('pointerdown', handlers.down, true);
          document[m]('pointerup', handlers.up, true);
          document[m]('click', handlers.click, true);
          document[m]('dblclick', handlers.dblclick, true);
          document[m]('keydown', handlers.key, true);
          document[m]('focusout', handlers.focusout, true);
          if (on) {
            S.raf = requestAnimationFrame(loop);
          } else {
            if (S.editing) { S.endEdit(true); }
            cancelAnimationFrame(S.raf);
            S.select(null);
            if (S.boxes) { S.boxes.hover.style.display = 'none'; S.boxes.select.style.display = 'none'; }
          }
        };

        document.addEventListener('dragstart', handlers.dragstart, true);
        document.addEventListener('dragover', handlers.dragover, true);
        document.addEventListener('drop', handlers.drop, true);
        addEventListener('st:reload', handlers.reload);
        S.restore();
      })();
    }
  }
}

/// Binds the canvas to the studio's open state: on while `$on` is true.
%primitive studio-canvas(&el, on: string = "") %uses studio-kit {
  %emit js {
    (() => {
      const host = %&el;
      const sig = %on;
      if (sig) {
        const head = sig.split('.')[0];
        const sync = () => { ST.stu.setOn(!!ST.resolve(host, head)); };
        let stop = null;
        if (ST.watchScoped) { stop = ST.watchScoped(host, head, sync); }
        sync();
        %cleanup {
          if (stop) { stop(); }
          ST.stu.setOn(false);
        }
      }
    })();
  }
}

%macro studio-canvas {
  %form { @studio-canvas(on: $on:binding) }
  %binds { studio-canvas(&self, on: $on) }
}
