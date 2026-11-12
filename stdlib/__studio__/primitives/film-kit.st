/// Film kit (PLAN-155) - the film studio's client runtime, as one module primitive.
///
/// The studio's Film mode is a PROJECTION of the page's score
/// (`/__spacetime/studio/score.json`): every clip, keyframe, sound, drum cell
/// and stage param in it is a word of the source with its span. A gesture
/// rewrites that one word through PLAN-154's edit rail (`ST.stu.apply`, ops
/// `span` / `insert` / `source`); the page reloads and the film is put back
/// where it was - its playhead, play state, loop and selection.
///
/// Layers, bottom up (the same shape as studio-kit):
///
/// 1. VIEW MODEL (pure): score -> flat rows for the timeline (`stuFilmRows`),
///    keyframe diamonds (`stuFilmKeys`), ruler ticks (`stuFilmTicks`), the
///    selection's editable words (`stuFilmFields`), a pattern's step grid
///    (`stuFilmGrid`), the scene's parts (`stuFilmParts`).
/// 2. TRANSPORT: the score's own timeline (`ST._transports[name]`, PLAN-155 F1)
///    - play/pause/seek/loop, the playhead and the timecode.
/// 3. GESTURES: drag a clip (its `at`), its right edge (its `for`), a diamond
///    (its stop's `at p%`); click a cell; scrub the ruler; keys like an editor's
///    (Space, J/K/L, arrows, I/O, Home/End).
/// 4. SOUND: each sound lane's waveform, from the same synthesizer that plays
///    and renders it (`STSynth.render` over the score's program).
%primitive film-kit() %uses studio-kit {
  %emit prelude-js {
    window.ST = window.ST || {};
    if (!ST.stuFilm) {
      (function () {
        var F = ST.stuFilm = { drag: null, loopIn: null, loopOut: null, waves: {}, raf: 0, lastTime: '' };
        var L = function () { return window.SpacetimeLocal || {}; };
        var put = function (k, v) { if (window.SpacetimeLocal) { window.SpacetimeLocal[k] = v; } };
        var score = function () { return L().stuScore || null; };
        var main = function (s) { return s && s.scores && s.scores[0] || null; };
        var total = function (s) { var m = main(s); return (m && m.total_ms) || 1; };
        /// The film selection. Setting one clears the page selection: the
        /// inspector shows one thing at a time.
        F.select = function (key) {
          put('stuFilmSel', key || '');
          if (key && ST.stu && ST.stu.forget) { ST.stu.forget(); }
        };
        var FPS = 30;

        // ---- 1. view model ----------------------------------------------------------

        /// Stops without a position spread as CSS keyframes do: first 0, last 1,
        /// the rest evenly between their positioned neighbours.
        var positions = function (stops) {
          var n = stops.length, at = stops.map(function (s) { return s.at; });
          if (n === 0) { return at; }
          if (at[0] == null) { at[0] = 0; }
          if (at[n - 1] == null) { at[n - 1] = 1; }
          var i = 0;
          while (i < n) {
            if (at[i] != null) { i++; continue; }
            var a = i - 1, b = i;
            while (at[b] == null) { b++; }
            for (var k = a + 1; k < b; k++) { at[k] = at[a] + (at[b] - at[a]) * (k - a) / (b - a); }
            i = b;
          }
          return at;
        };

        // a word's address in its file (the file itself, as a path, travels beside it)
        var addr = function (w) { return w ? { start: w.start, end: w.end, expect: w.text } : null; };
        var fileOf = function (s, i) { return (s && s.files && s.files[i]) || ''; };

        /// Is a row (or a key) the selection, or the row a selected keyframe sits on?
        var picked = function (key, sel) {
          if (!sel) { return false; }
          if (key === sel) { return true; }
          var p = sel.split(':');
          return p[0] === 'stop' && key === 'chan:' + p[1] + ':' + p[2];
        };

        /// The timeline: a row per clip, its channels under it, then the sounds.
        /// A channel's bar carries no text: its keyframes are drawn on it (their
        /// values show on hover and in the inspector), and a chain of words under
        /// the diamonds read as noise.
        ST.stuFilmRows = function (s, sel) {
          var rows = [], m = main(s);
          if (!m) { return rows; }
          var T = total(s);
          var y = 0;
          m.entries.forEach(function (e, i) {
            if (e.kind !== 'clip') { return; }
            var at = e.at_ms || 0, len = e.for_ms != null ? e.for_ms : T - at;
            rows.push({
              key: 'clip:' + i, kind: 'clip', label: '&' + e.subject, depth: 0, y: y++, selected: picked('clip:' + i, sel),
              left: at / T * 100, width: len / T * 100, text: (e.at ? e.at.text : '0s') + ' for ' + (e.for ? e.for.text : '...'),
              entry: i
            });
            var body = s.clips && s.clips[e.subject];
            (body ? body.channels : []).forEach(function (c, j) {
              rows.push({
                key: 'chan:' + i + ':' + j, kind: 'channel', label: (c.scope ? c.scope + ' ' : '') + c.prop, depth: 1, y: y++,
                left: at / T * 100, width: len / T * 100, text: '', entry: i, selected: picked('chan:' + i + ':' + j, sel)
              });
            });
          });
          m.entries.forEach(function (e, i) {
            if (e.kind !== 'sound') { return; }
            var at = e.at_ms || 0;
            var form = s.forms && e.form && s.forms[e.form.text];
            var len = e.for_ms != null ? e.for_ms : Math.min(T - at, 2000);
            rows.push({
              key: 'sound:' + i, kind: form && form.kind === 'pattern' ? 'pattern' : 'sound', selected: picked('sound:' + i, sel),
              label: (e.name ? e.name.text + ' ' : '') + (e.form ? e.form.text : '@sound'), depth: 0, y: y++,
              left: at / T * 100, width: Math.max(0.6, len / T * 100), text: e.notes ? e.notes.text : '', entry: i
            });
          });
          return rows;
        };

        /// Keyframe diamonds, placed in score time on their channel's row.
        ST.stuFilmKeys = function (s, sel, many) {
          var held = String(many || '').split(' ');
          var keys = [], m = main(s);
          if (!m) { return keys; }
          var T = total(s), y = 0;
          m.entries.forEach(function (e, i) {
            if (e.kind !== 'clip') { return; }
            y++;
            var at = e.at_ms || 0, len = e.for_ms != null ? e.for_ms : T - at;
            var body = s.clips && s.clips[e.subject];
            (body ? body.channels : []).forEach(function (c, j) {
              var pos = positions(c.stops);
              c.stops.forEach(function (st, k) {
                keys.push({
                  key: 'stop:' + i + ':' + j + ':' + k, y: y, left: (at + pos[k] * len) / T * 100,
                  value: st.value, selected: sel === 'stop:' + i + ':' + j + ':' + k || held.indexOf('stop:' + i + ':' + j + ':' + k) >= 0, held: !!(st.easing && st.easing.indexOf('steps') >= 0), fixed: k === 0 || k === c.stops.length - 1
                });
              });
              y++;
            });
          });
          return keys;
        };

        /// Ruler ticks: whole seconds, labelled every few.
        /// Ruler ticks for what the timeline shows (`zoom` = film / view): a
        /// tick about every 40px, a label every few; finer as you zoom in.
        ST.stuFilmTicks = function (s, zoom) {
          var T = total(s), ticks = [], z = Math.max(1, +zoom || 1);
          var visible = T / z;
          var steps = [50, 100, 250, 500, 1000, 2000, 5000, 10000, 30000, 60000];
          var tick = steps.find(function (st) { return visible / st <= 30; }) || 60000;
          var label = steps.find(function (st) { return st >= tick * 2 && visible / st <= 12; }) || tick * 5;
          for (var ms = 0; ms <= T + 1; ms += tick) {
            var major = Math.abs(ms / label - Math.round(ms / label)) < 1e-6;
            var text = ms % 1000 === 0 ? (ms / 1000) + 's' : (ms / 1000).toFixed(ms % 100 === 0 ? 1 : 2) + 's';
            ticks.push({ left: ms / T * 100, label: major ? text : '', major: major });
          }
          return ticks;
        };

        var find = function (s, key) {
          var m = main(s), p = String(key || '').split(':');
          if (!m) { return null; }
          if (p[0] === 'part') { return s.parts[+p[1]] ? { kind: 'part', part: s.parts[+p[1]] } : null; }
          var e = m.entries[+p[1]];
          if (!e) { return null; }
          if (p[0] === 'clip' || p[0] === 'sound') { return { kind: p[0], entry: e }; }
          var body = s.clips && s.clips[e.subject];
          var c = body && body.channels[+p[2]];
          if (p[0] === 'chan') { return c ? { kind: 'channel', entry: e, channel: c } : null; }
          if (p[0] === 'stop') { return c ? { kind: 'stop', entry: e, channel: c, stop: c.stops[+p[3]], index: +p[3] } : null; }
          return null;
        };
        F.find = find;

        /// The words the inspector edits for what is selected.
        ST.stuFilmFields = function (key, s) {
          var f = find(s, key), out = [];
          if (!f) { return out; }
          var word = function (label, w, hint) {
            if (w) { out.push(Object.assign({ label: label, value: w.text, hint: hint || '', mode: 'span', file: fileOf(s, w.file) }, addr(w), { at: '', before: '' })); }
          };
          if (f.kind === 'clip') {
            word('at', f.entry.at, 'when it starts');
            word('for', f.entry.for, 'how long');
          } else if (f.kind === 'sound') {
            word('sound', f.entry.form, 'an instrument or a pattern');
            word('notes', f.entry.notes, 'a chord');
            word('at', f.entry.at);
            word('for', f.entry.for);
          } else if (f.kind === 'channel') {
            word((f.channel.scope ? f.channel.scope + ' ' : '') + f.channel.prop, f.channel.value, 'every stop: a -> b at p%');
          } else if (f.kind === 'stop') {
            word('stop ' + (f.index + 1), f.stop.span, 'value [at p%] [--easing]');
          }
          // a part's params are widgets, typed by their form: ST.stuPartFields
          return out;
        };

        /// A CSS colour as `#rrggbb`, for a colour input (the canvas normalises it).
        var hexOf = function (c) {
          try {
            var x = document.createElement('canvas').getContext('2d');
            x.fillStyle = '#000000';
            x.fillStyle = String(c).replace(/^"|"$/g, '');
            return /^#[0-9a-f]{6}$/i.test(x.fillStyle) ? x.fillStyle : '#000000';
          } catch (e) { return '#000000'; }
        };
        var WIDGETS = { number: 1, range: 1, color: 1, toggle: 1, choice: 1, text: 1 };

        /// The edit that makes a driven param read `v` at the playhead
        /// (`drive` = "entry:channel"): the stop that sits there is rewritten;
        /// between stops, a new stop goes in after the one before.
        F.stopEdit = function (drive, v) {
          var s = score(), mn = main(s), t = F.transport();
          var p = String(drive).split(':'), e = mn && mn.entries[+p[0]];
          var body = e && s.clips && s.clips[e.subject], c = body && body.channels[+p[1]];
          if (!c || !t) { return null; }
          var T = total(s), at = e.at_ms || 0, len = e.for_ms != null ? e.for_ms : T - at;
          var pct = Math.round(Math.max(0, Math.min(1, (t.position - at) / len)) * 1000) / 10;
          var pos = positions(c.stops), k = -1, before = 0;
          pos.forEach(function (q, i) { if (Math.abs(q * 100 - pct) < 0.5) { k = i; } if (q * 100 < pct) { before = i; } });
          var st, text;
          if (k >= 0) {
            st = c.stops[k];
            var tail = st.span.text.slice(String(st.value).length);
            text = v + tail;
          } else {
            st = c.stops[before];
            text = st.span.text + ' -> ' + v + ' at ' + pct + '%';
          }
          return { file: fileOf(s, st.span.file), start: st.span.start, end: st.span.end, expect: st.span.text, text: text };
        };

        /// A part's params as widget fields (PLAN-156), from reflect.json: the
        /// type its form declares picks the widget (its scalar row's `%widget`,
        /// `choice` when it has choices: a keyword set's words, a model file's
        /// clips), its `///` doc is the label's tooltip, the row gives the range,
        /// step and unit. The score's part is found in reflect where it starts.
        ST.stuPartFields = function (key, s, r) {
          var f = find(s, key);
          if (!f || f.kind !== 'part' || !r || !r.matches) { return []; }
          var path = fileOf(s, f.part.span.file);
          var fi = (r.files || []).findIndex(function (x) { return x.path === path; });
          var m = r.matches.find(function (x) { return x.file === fi && x.start === f.part.span.start; });
          if (!m) { return []; }
          // a param still to write goes after the one written last
          var last = null;
          m.params.forEach(function (p) { if (p.written && !p.positional && (!last || p.written.end > last.end)) { last = p.written; } });
          // the channels a score animates this part's params with: `&lamp2 { intensity: … }`
          var driven = {}, mn = main(s), name = f.part.name ? '&' + f.part.name : null;
          if (name && mn) {
            mn.entries.forEach(function (e, i) {
              var body = e.kind === 'clip' && s.clips && s.clips[e.subject];
              (body ? body.channels : []).forEach(function (c, j) {
                if (c.scope === name && !driven[c.prop]) { driven[c.prop] = i + ':' + j; }
              });
            });
          }
          var fields = m.params.filter(function (p) { return !p.positional; }).map(function (p) {
            var sc = p.scalar || {}, range = sc.range || [null, null];
            var words = p.written ? p.written.text : (p.default == null ? '' : String(p.default));
            var bare = words.replace(/^"(.*)"$/, '$1');
            var choices = Array.isArray(p.choices) && p.choices.length ? p.choices : null;
            var widget = choices ? 'choice' : (sc.widget || { color: 'color', bool: 'toggle', number: 'number' }[p.type] || 'text');
            if (widget === 'number' && range[0] != null && range[1] != null) { widget = 'range'; }
            if (!WIDGETS[widget] || (widget === 'choice' && !choices)) { widget = 'text'; }
            return {
              tpl: 'w-' + widget, label: p.name, doc: p.doc || '', written: !!p.written,
              value: widget === 'text' || widget === 'choice' ? bare : words,
              unit: sc.unit || '', min: range[0] == null ? '' : range[0], max: range[1] == null ? '' : range[1],
              step: sc.step || (p.type === 'number' ? 0.1 : ''), hex: hexOf(words), on: bare === 'true',
              // the author's spelling stays: quoted stays quoted, a bare word stays bare
              quote: p.written ? /^"/.test(p.written.text) : /^(string|model_clip|expr)$/.test(p.type),
              choices: (choices || []).map(function (c) { return { word: c.word, doc: c.doc || '', on: c.word === bare }; }),
              file: path,
              start: p.written ? p.written.start : (last ? last.start : 0),
              end: p.written ? p.written.end : (last ? last.end : 0),
              expect: p.written ? p.written.text : (last ? last.text : ''),
              at: last ? 'after' : '', before: last ? last.text : '',
              drive: driven[p.name] || ''
            };
          });
          // a shorthand's longhands share one row (`pos`: X Y Z)
          (m.shorthands || []).forEach(function (sh) {
            var parts = sh[1].map(function (l) { return fields.find(function (x) { return x.label === l; }); });
            if (parts.some(function (x) { return !x; })) { return; }
            var at = fields.indexOf(parts[0]);
            fields = fields.filter(function (x) { return parts.indexOf(x) < 0; });
            fields.splice(at, 0, {
              tpl: 'w-vec3', label: sh[0], doc: parts[0].doc, written: parts.some(function (x) { return x.written; }),
              parts: parts.map(function (x) {
                return Object.assign({}, x, { axis: x.label.slice(sh[0].length) || x.label });
              })
            });
          });
          return fields;
        };

        /// Any widget's change, as one guarded edit through the one writer
        /// (`/__spacetime/rewrite`): a written param's words are replaced; a
        /// param left at its default is written after the last written one.
        ST.stuWidgetCommit = function (el) {
          var field = el.closest && el.closest('.w-field');
          if (!field) { return ''; }
          // a template writes a true flag as a present attribute and a false one
          // not at all; an @each inside a template writes "true"/"false"
          var flag = function (x) { return x !== undefined && x !== 'false'; };
          var d = field.dataset, v, written = flag(d.written), quote = flag(d.quote);
          if (el.classList.contains('w-choice')) { v = el.dataset.word; }
          else if (el.type === 'checkbox') { v = el.checked ? 'true' : 'false'; }
          else { v = el.value; }
          if (quote) { v = '"' + String(v).replace(/^"|"$/g, '').replace(/"/g, '\\"') + '"'; }
          var edit;
          if (d.drive) {
            // a param a score drives: the value is the stop at the playhead
            edit = F.stopEdit(d.drive, v);
            if (!edit) { return ''; }
          } else if (written) {
            if (v === d.expect) { return ''; }
            edit = { file: d.file, start: +d.start, end: +d.end, expect: d.expect, text: v };
          } else if (d.at === 'after') {
            edit = { file: d.file, start: +d.start, end: +d.end, expect: d.expect, text: d.expect + ', ' + d.label + ': ' + v };
          } else {
            return '';
          }
          ST.stu.send('/__spacetime/rewrite', { entry: L().stuEntry, edits: [edit] }, 'Saved to ');
          return '';
        };

        /// A pattern's step grid: a row per instrument, a cell per step.
        ST.stuFilmGrid = function (key, s) {
          var f = find(s, key), out = [];
          if (!f || f.kind !== 'sound' || !f.entry.form) { return out; }
          var form = s.forms && s.forms[f.entry.form.text];
          if (!form || form.kind !== 'pattern') { return out; }
          // one flat list (an element renders one `@each`): each row's name, then its cells
          form.props.forEach(function (row, r) {
            // a row that plays notes is melodic: its cells open the note picker
            var melodic = (row.cells || []).some(function (c) { return NOTE.test(c.text); });
            out.push({ kind: 'name', row: r + 1, col: 1, step: -1, rc: '', melodic: melodic, text: row.prop, on: false, file: '', start: 0, end: 0, expect: '' });
            (row.cells || []).forEach(function (c, k) {
              out.push(Object.assign({ kind: 'cell', row: r + 1, col: k + 2, step: k, rc: r + ':' + k, melodic: melodic, text: c.text, on: c.text !== '.', file: fileOf(s, c.file) }, addr(c)));
            });
          });
          return out;
        };

        /// The scene: the stage and its parts.
        ST.stuFilmParts = function (s, sel) {
          return ((s && s.parts) || []).map(function (p, i) {
            return { key: 'part:' + i, kind: p.kind, label: p.name ? '&' + p.name : p.kind, text: p.kind, depth: p.kind === 'stage' ? 0 : 1, selected: sel === 'part:' + i };
          });
        };

        // ---- the scene: a score.json part <-> the stage's live part ---------------------
        // A stage part remembers the compiled call that made it (its hot key,
        // `three-object@canvas.world#3`); score.json lists the parts in source
        // order. The n-th call of a primitive on a stage is the n-th part of
        // that kind in the source.
        var PRIM = { object: 'three-object', light: 'three-light', gltf: 'three-gltf' };
        var KIND = { 'three-object': 'object', 'three-light': 'light', 'three-gltf': 'gltf' };
        F.stage = function () {
          var cs = document.querySelectorAll('canvas');
          for (var c = 0; c < cs.length; c++) { if (cs[c]._stThree && cs[c]._stThree.parts) { return cs[c]._stThree; } }
          return null;
        };
        var keyOf = function (bind) {
          var at = bind.indexOf('@'), hash = bind.lastIndexOf('#');
          return at > 0 && hash > at ? { prim: bind.slice(0, at), n: +bind.slice(hash + 1) } : null;
        };
        /// The live part a score.json part (by index) is drawn as.
        F.stagePart = function (s, i) {
          var p = s && s.parts && s.parts[i], st = F.stage();
          if (!p || !st || !PRIM[p.kind]) { return null; }
          var n = 0;
          for (var j = 0; j < i; j++) { if (s.parts[j].kind === p.kind) { n++; } }
          return st.parts().filter(function (pt) {
            var k = pt.bind && keyOf(pt.bind);
            return k && k.prim === PRIM[p.kind] && k.n === n;
          })[0] || null;
        };
        /// The score.json part (its index) a live part is.
        F.partIndexOf = function (s, pt) {
          var k = pt && pt.bind && keyOf(pt.bind);
          if (!k || !KIND[k.prim] || !s || !s.parts) { return -1; }
          var n = -1;
          for (var i = 0; i < s.parts.length; i++) {
            if (s.parts[i].kind === KIND[k.prim] && ++n === k.n) { return i; }
          }
          return -1;
        };

        // ---- outlines: a box drawn round a part in the 3D itself ---------------------------
        F.outlines = {};
        F.outline = function (slot, pt, color) {
          var st = F.stage(), old = F.outlines[slot];
          if (!st || !st.THREE || !st.scene) { return; }
          // the selected part already has its outline: hovering it adds none
          if (slot === 'hover' && pt && F.outlines.sel && F.outlines.sel.pt === pt) { pt = null; }
          if (slot === 'sel' && pt && F.outlines.hover && F.outlines.hover.pt === pt) { F.outline('hover', null); }
          if (old && old.pt === pt) { return; }
          if (old) { st.scene.remove(old.helper); if (old.helper.geometry) { old.helper.geometry.dispose(); } }
          F.outlines[slot] = null;
          if (pt && pt.obj) {
            var helper = new st.THREE.BoxHelper(pt.obj, color);
            helper.userData.stuOutline = true;
            // drawn as it is (not through the film's exposure and tone curve),
            // and over everything
            helper.material.toneMapped = false;
            helper.material.depthTest = false;
            helper.renderOrder = 999;
            st.scene.add(helper);
            F.outlines[slot] = { pt: pt, helper: helper };
          }
          st.draw();
        };
        var followOutlines = function () {
          var moved = false;
          Object.keys(F.outlines).forEach(function (k) { var o = F.outlines[k]; if (o) { o.helper.update(); moved = true; } });
          return moved;
        };

        // ---- the Select tool: point at the 3D, pick a part --------------------------------
        // the 3D tools' hit layer is the stage, not the studio's UI
        var inStudioUi = function (node) { return !!(node && node.closest && !node.closest('.stu-3d-hit') && node.closest('.stu-ui, .st-dev-dock, .stu-overlay')); };
        F.pickAt = function (ev) {
          var st = F.stage();
          if (!st || !st.THREE || !st.camera || !st.renderer) { return null; }
          var r = st.renderer.domElement.getBoundingClientRect();
          if (ev.clientX < r.left || ev.clientX > r.right || ev.clientY < r.top || ev.clientY > r.bottom) { return null; }
          var ndc = new st.THREE.Vector2((ev.clientX - r.left) / r.width * 2 - 1, -((ev.clientY - r.top) / r.height) * 2 + 1);
          F.ray = F.ray || new st.THREE.Raycaster();
          F.ray.setFromCamera(ndc, st.looking ? st.looking() : st.camera);
          var parts = st.parts().filter(function (pt) { return pt.bind && pt.obj; });
          var hits = F.ray.intersectObjects(st.scene.children, true);
          for (var h = 0; h < hits.length; h++) {
            if (hits[h].object.userData.stuOutline || hits[h].object.isLine) { continue; }
            for (var o = hits[h].object; o; o = o.parent) {
              for (var q = 0; q < parts.length; q++) { if (parts[q].obj === o) { return parts[q]; } }
            }
          }
          return null;
        };
        F.pickMove = function (ev) {
          if (!F.pickOn || inStudioUi(ev.target)) { return; }
          var pt = F.pickAt(ev);
          F.outline('hover', pt, 0x0d99ff);
          document.documentElement.classList.toggle('stu-picking-hit', !!pt);
        };
        F.pickClick = function (ev) {
          if (!F.pickOn || inStudioUi(ev.target)) { return; }
          ev.preventDefault();
          ev.stopImmediatePropagation();
          var pt = F.pickAt(ev), i = F.partIndexOf(score(), pt);
          if (i < 0) { return; }
          put('stuTabName', 'Scene');
          F.select('part:' + i);
        };
        ST.stuFilmPickTool = function () {
          F.pickOn = !F.pickOn;
          put('stuFilmPickOn', F.pickOn);
          document.documentElement.classList.toggle('stu-picking', F.pickOn);
          if (!F.pickOn) { F.outline('hover', null); }
          return '';
        };
        // the Scene list: hovering a part outlines it in the 3D
        F.listOver = function (ev) {
          var row = ev.target.closest && ev.target.closest('.stu-part');
          if (!row) { return; }
          F.outline('hover', F.stagePart(score(), +String(row.dataset.key).split(':')[1]), 0x0d99ff);
        };
        F.listOut = function (ev) {
          var row = ev.target.closest && ev.target.closest('.stu-part');
          if (row && !(ev.relatedTarget && row.contains(ev.relatedTarget))) { F.outline('hover', null); }
        };

        // ---- music: where the playhead is in the selected sound ---------------------------
        var NAMES = ['C', 'Db', 'D', 'Eb', 'E', 'F', 'Gb', 'G', 'Ab', 'A', 'Bb', 'B'];
        var NOTE = /^([A-G])([#b]?)(-?\d)$/;
        var midiOf = function (n) {
          var m = NOTE.exec(String(n));
          if (!m) { return null; }
          var pc = { C: 0, D: 2, E: 4, F: 5, G: 7, A: 9, B: 11 }[m[1]] + (m[2] === '#' ? 1 : m[2] === 'b' ? -1 : 0);
          return (+m[3] + 1) * 12 + pc;
        };
        var nameOf = function (mi) { return NAMES[((mi % 12) + 12) % 12] + (Math.floor(mi / 12) - 1); };
        var ms = function (t) { var v = parseFloat(t); return /ms$/.test(String(t)) ? v : v * 1000; };
        /// The selected sound's own clock: its window, and for a pattern its step.
        F.soundClock = function (s, key, at) {
          var f = find(s, key);
          if (!f || f.kind !== 'sound' || !f.entry.form) { return null; }
          var e = f.entry, prog = s.programs && s.programs[0];
          var form = prog && prog.forms && prog.forms[e.form.text];
          var start = e.at_ms || 0;
          var len = e.for_ms != null ? e.for_ms : (form && form.props && form.props['for'] ? ms(form.props['for'][0]) : 1000);
          var c = { start: start, len: len, t: at - start, step: -1, steps: 0 };
          if (form && form.kind === 'pattern') {
            var bpm = parseFloat((form.params && form.params.tempo) || '120');
            var per = parseFloat((form.params && form.params.step) || '16');
            var shape = s.forms && s.forms[e.form.text];
            c.steps = shape ? Math.max.apply(null, shape.props.map(function (r) { return (r.cells || []).length; })) : 16;
            c.stepMs = 60000 / bpm * 4 / per;
            if (c.t >= 0 && c.t < len) { c.step = Math.floor(c.t / c.stepMs) % c.steps; c.bar = Math.floor(c.t / (c.stepMs * c.steps)) + 1; }
          }
          return c;
        };
        var secs = function (v) { return (v / 1000).toFixed(2) + ' s'; };
        F.showSound = function (t) {
          var s = score(), sel = L().stuFilmSel || '';
          var c = F.soundClock(s, sel, t.position), text = '';
          if (c) {
            if (c.t < 0) { text = 'starts in ' + secs(-c.t); }
            else if (c.t >= c.len) { text = 'ended ' + secs(c.t - c.len) + ' ago'; }
            else { text = '\u25b6 ' + secs(c.t) + ' of ' + secs(c.len) + (c.step >= 0 ? '  \u00b7  step ' + (c.step + 1) + '/' + c.steps + '  \u00b7  bar ' + c.bar : ''); }
          }
          if (text !== F.lastSound) { F.lastSound = text; put('stuFilmSoundNow', text); }
          var step = c ? c.step : -1;
          if (step !== F.lastStep) {
            F.lastStep = step;
            document.querySelectorAll('.stu-cell.is-now').forEach(function (el) { el.classList.remove('is-now'); });
            if (step >= 0) {
              document.querySelectorAll('.stu-cell[data-step="' + step + '"]').forEach(function (el) { el.classList.add('is-now'); });
            }
          }
        };

        // ---- notes: a keyboard for a chord or a melodic cell -------------------------------
        var cellAt = function (s, key, rc) {
          var f = find(s, key);
          if (!f || f.kind !== 'sound' || !f.entry.form || !rc) { return null; }
          var form = s.forms && s.forms[f.entry.form.text], p = String(rc).split(':');
          var row = form && form.props[+p[0]];
          return row && row.cells && row.cells[+p[1]] ? { cell: row.cells[+p[1]], row: row } : null;
        };
        /// The keys of the note picker: two octaves round what is being edited.
        ST.stuFilmPiano = function (key, s, rc) {
          var f = find(s, key), notes = [], target = '';
          var c = cellAt(s, key, rc);
          if (c) { notes = NOTE.test(c.cell.text) ? [c.cell.text] : []; target = 'cell'; }
          else if (f && f.kind === 'sound' && f.entry.notes) {
            notes = f.entry.notes.text.replace(/^["']|["']$/g, '').split(/\s+/).filter(Boolean); target = 'chord';
          } else { return []; }
          var on = {}, lo = 48;
          var mids = notes.map(midiOf).filter(function (m) { return m != null; });
          mids.forEach(function (m) { on[m] = true; });
          if (mids.length) { lo = Math.floor(Math.min.apply(null, mids) / 12) * 12; }
          var hi = Math.max(lo + 24, mids.length ? Math.max.apply(null, mids) + 1 : 0);
          var keys = [], white = 0;
          for (var m = lo; m < hi; m++) {
            var black = [1, 3, 6, 8, 10].indexOf(m % 12) >= 0;
            keys.push({ note: nameOf(m), black: black, on: !!on[m], label: m % 12 === 0 ? nameOf(m) : '', target: target, i: black ? white - 1 : white++ });
          }
          return keys;
        };
        ST.stuFilmNote = function (el) {
          var s = score(), key = L().stuFilmSel || '', note = el.dataset.note;
          var c = cellAt(s, key, L().stuFilmNoteCell);
          if (c) {
            var w = c.cell;
            ST.stu.apply({ op: 'span', file: fileOf(s, w.file), start: w.start, end: w.end, expect: w.text, value: w.text === note ? '.' : note });
            return '';
          }
          var f = find(s, key);
          if (!f || !f.entry.notes) { return ''; }
          var n = f.entry.notes, q = n.text.charAt(0) === "'" ? "'" : '"';
          var words = n.text.replace(/^["']|["']$/g, '').split(/\s+/).filter(Boolean);
          var at = words.indexOf(note);
          if (at >= 0) { words.splice(at, 1); } else { words.push(note); words.sort(function (a, b) { return midiOf(a) - midiOf(b); }); }
          if (!words.length) { return ''; }
          ST.stu.apply({ op: 'span', file: fileOf(s, n.file), start: n.start, end: n.end, expect: n.text, value: q + words.join(' ') + q });
          return '';
        };

        /// The film's own file, for the code pane (one item: an `@each` source).
        ST.stuFilmSources = function (s) {
          return ((s && s.sources) || []).filter(function (x) { return x.path === s.entry; });
        };

        ST.stuFilmLabel = function (key, s) {
          var f = find(s, key);
          if (!f) { return ''; }
          if (f.kind === 'clip') { return 'Clip &' + f.entry.subject; }
          if (f.kind === 'sound') { return 'Sound ' + (f.entry.form ? f.entry.form.text : ''); }
          if (f.kind === 'channel') { return 'Channel ' + f.channel.prop + ' of &' + f.entry.subject; }
          if (f.kind === 'stop') { return 'Keyframe ' + (f.index + 1) + ' of ' + f.channel.prop; }
          if (f.kind === 'part') { return (f.part.name ? '&' + f.part.name + ' ' : '') + '(' + f.part.kind + ')'; }
          return '';
        };

        // ---- 2. transport ---------------------------------------------------------------

        F.transport = function () {
          var ts = ST._transports || {}, m = main(score());
          return (m && m.name && ts[m.name]) || ts[Object.keys(ts)[0]] || null;
        };
        var timecode = function (ms) {
          var s = Math.max(0, ms) / 1000, m = Math.floor(s / 60), sec = Math.floor(s % 60), fr = Math.floor((s * FPS) % FPS);
          var two = function (n) { return (n < 10 ? '0' : '') + n; };
          return two(m) + ':' + two(sec) + ':' + two(fr);
        };
        F.seek = function (ms) { var t = F.transport(); if (t) { t.seek(Math.max(0, Math.min(t.duration, ms))); } };
        F.toggle = function () { var t = F.transport(); if (t) { if (t.playing) { t.pause(); } else { t.play(); } } };
        F.step = function (frames) { var t = F.transport(); if (t) { t.pause(); F.seek(t.position + frames * 1000 / FPS); } };
        F.setLoop = function () {
          var t = F.transport();
          if (!t) { return; }
          if (F.loopIn != null && F.loopOut != null && F.loopOut > F.loopIn) { t.loop(F.loopIn, F.loopOut); } else { t.loop(); }
          put('stuFilmLoop', F.loopIn != null && F.loopOut != null ? timecode(F.loopIn) + ' - ' + timecode(F.loopOut) : '');
        };

        var tick = function () {
          F.raf = requestAnimationFrame(tick);
          var t = F.transport(), root = document.querySelector('.stu-film');
          if (!t || !root) { return; }
          root.style.setProperty('--stu-t', String(t.position / t.duration));
          var tc = timecode(t.position) + (t.playing ? '' : '  paused');
          if (tc !== F.lastTime) { F.lastTime = tc; put('stuFilmTime', tc); put('stuFilmPlaying', t.playing); }
          // the selected part keeps its outline; a new selection forgets a picked note
          var sel = L().stuFilmSel || '';
          if (sel !== F.lastSel) {
            F.lastSel = sel;
            put('stuFilmNoteCell', '');
            F.outline('sel', sel.indexOf('part:') === 0 ? F.stagePart(score(), +sel.split(':')[1]) : null, 0xffcf5a);
            F.lastStep = null;
          }
          if (followOutlines() && !t.playing) { var st = F.stage(); if (st) { st.draw(); } }
          F.showSound(t);
          // the picked note cell is marked in the grid
          var rc = L().stuFilmNoteCell || '';
          if (rc !== F.lastRc) {
            F.lastRc = rc;
            document.querySelectorAll('.stu-cell.is-picked').forEach(function (el) { el.classList.remove('is-picked'); });
            if (rc) { document.querySelectorAll('.stu-cell[data-rc="' + rc + '"]').forEach(function (el) { el.classList.add('is-picked'); }); }
          }
        };

        // ---- 3. gestures ----------------------------------------------------------------

        var fmtTime = function (ms, like) {
          if (like && /ms$/.test(like)) { return Math.round(ms) + 'ms'; }
          var s = Math.round(ms / 10) / 100;
          return String(s) + 's';
        };
        var snap = function (ms, free) { return free ? ms : Math.round(ms / 100) * 100; };
        var lane = function () { return document.querySelector('.stu-film__lanes'); };
        // ---- the view window (PLAN-156 P7) ----------------------------------------------
        // What the timeline shows: from `from` ms, `span` ms of film. Zoom and pan
        // change only this. The view model stays in percent of the film; the
        // track is widened (--stu-zoom) and shifted (--stu-from) under the lanes.
        F.view = { from: 0, span: 0 };
        var viewSpan = function () { var T = total(score()); return F.view.span > 0 ? Math.min(F.view.span, T) : T; };
        var viewFrom = function () { var T = total(score()); return Math.max(0, Math.min(T - viewSpan(), F.view.from)); };
        F.setView = function (from, span) {
          var T = total(score());
          span = Math.max(Math.min(T, 200), Math.min(T, span));
          from = Math.max(0, Math.min(T - span, from));
          F.view = { from: from, span: span };
          var root = document.querySelector('.stu-film');
          if (root) {
            root.style.setProperty('--stu-zoom', String(T / span));
            root.style.setProperty('--stu-from', String(from / T));
          }
          put('stuFilmZoom', Math.round(T / span * 100) / 100);
          put('stuFilmView', (from / 1000).toFixed(2) + 's - ' + ((from + span) / 1000).toFixed(2) + 's');
        };
        /// Zoom by `factor`, keeping the moment under `clientX` where it is.
        F.zoomAt = function (clientX, factor) {
          var r = lane().getBoundingClientRect();
          var x = Math.max(0, Math.min(1, (clientX - r.left) / r.width));
          var sp = viewSpan(), at = viewFrom() + x * sp, ns = sp / factor;
          F.setView(at - x * ns, ns);
        };
        F.fit = function () { F.setView(0, total(score())); };
        F.wheel = function (ev) {
          if (!ev.target.closest || !ev.target.closest('.stu-film__lanes')) { return; }
          var r = lane().getBoundingClientRect();
          if (ev.ctrlKey || ev.metaKey) {
            ev.preventDefault();
            F.zoomAt(ev.clientX, ev.deltaY < 0 ? 1.25 : 0.8);
          } else if (ev.shiftKey || Math.abs(ev.deltaX) > Math.abs(ev.deltaY)) {
            ev.preventDefault();
            var d = ev.deltaX || ev.deltaY;
            F.setView(viewFrom() + d / r.width * viewSpan(), viewSpan());
          }
        };

        var msAt = function (clientX) {
          var r = lane().getBoundingClientRect();
          return viewFrom() + Math.max(0, Math.min(1, (clientX - r.left) / r.width)) * viewSpan();
        };
        var spanOp = function (w, value) {
          return { op: 'span', file: fileOf(score(), w.file), start: w.start, end: w.end, expect: w.text, value: value };
        };

        /// The keyframes a box drag holds (PLAN-156 P6): a set of stop keys.
        var multi = function () { return String(L().stuFilmMulti || '').split(' ').filter(Boolean); };
        var marquee = function () { return document.querySelector('.stu-film__marquee'); };

        F.down = function (ev) {
          if (ev.button !== 0) { return; }
          var el = ev.target.closest && ev.target.closest('[data-film]');
          var lanes = ev.target.closest && ev.target.closest('.stu-film__lanes');
          // a drag from empty lane (or a channel's bar) draws a box around keyframes
          if (lanes && (!el || (el.dataset.film === 'clip' && el.dataset.kind === 'channel'))) {
            F.drag = { kind: 'marquee', x0: ev.clientX, y0: ev.clientY, add: ev.shiftKey, row: el && el.dataset.key, moved: false };
            ev.preventDefault();
            return;
          }
          if (!el || !el.closest('.stu-film')) { return; }
          var kind = el.dataset.film, s = score();
          if (kind === 'ruler') {
            var t = F.transport();
            F.drag = { kind: 'scrub', was: t && t.playing };
            if (t) { t.pause(); }
            F.seek(msAt(ev.clientX));
          } else if (kind === 'clip' || kind === 'edge' || kind === 'key') {
            var key = el.dataset.key;
            F.select(key);
            F.drag = { kind: kind, key: key, el: el, x0: ev.clientX, moved: false };
          } else if (kind === 'row') {
            F.select(el.dataset.key);
          } else {
            return;
          }
          ev.preventDefault();
          try { el.setPointerCapture(ev.pointerId); } catch (e) {}
        };
        F.move = function (ev) {
          var d = F.drag;
          if (!d) { return; }
          if (d.kind === 'scrub') { F.seek(msAt(ev.clientX)); return; }
          var dx = ev.clientX - d.x0;
          if (Math.abs(dx) > 2 || (d.kind === 'marquee' && Math.abs(ev.clientY - d.y0) > 2)) { d.moved = true; }
          if (d.kind === 'marquee') {
            var box = marquee(), r = lane().getBoundingClientRect();
            if (box && d.moved) {
              box.style.left = (Math.min(d.x0, ev.clientX) - r.left) + 'px';
              box.style.top = (Math.min(d.y0, ev.clientY) - r.top) + 'px';
              box.style.width = Math.abs(dx) + 'px';
              box.style.height = Math.abs(ev.clientY - d.y0) + 'px';
              box.classList.add('is-on');
            }
            return;
          }
          // a selected keyframe moves the whole selection
          var many = d.kind === 'key' && multi().indexOf(d.key) >= 0 ? multi() : [];
          if (many.length) {
            document.querySelectorAll('.stu-film__keys .stu-key').forEach(function (k) {
              if (many.indexOf(k.dataset.key) >= 0) { k.style.setProperty('--stu-dx', dx + 'px'); }
            });
          } else {
            d.el.style.setProperty('--stu-dx', dx + 'px');
          }
        };
        F.up = function (ev) {
          var d = F.drag;
          F.drag = null;
          if (!d) { return; }
          if (d.kind === 'scrub') { var t = F.transport(); if (t && d.was) { t.play(); } return; }
          if (d.kind === 'marquee') {
            var box = marquee();
            if (box) { box.classList.remove('is-on'); }
            if (!d.moved) { put('stuFilmMulti', ''); if (d.row) { F.select(d.row); } return; }
            var x1 = Math.min(d.x0, ev.clientX), x2 = Math.max(d.x0, ev.clientX);
            var y1 = Math.min(d.y0, ev.clientY), y2 = Math.max(d.y0, ev.clientY);
            var got = d.add ? multi() : [];
            document.querySelectorAll('.stu-film__keys .stu-key').forEach(function (k) {
              var b = k.getBoundingClientRect(), cx = b.left + b.width / 2, cy = b.top + b.height / 2;
              if (cx >= x1 && cx <= x2 && cy >= y1 && cy <= y2 && got.indexOf(k.dataset.key) < 0) { got.push(k.dataset.key); }
            });
            put('stuFilmMulti', got.join(' '));
            if (got.length) { F.select(got[got.length - 1]); }
            return;
          }
          document.querySelectorAll('.stu-film__keys .stu-key').forEach(function (k) { k.style.removeProperty('--stu-dx'); });
          d.el.style.removeProperty('--stu-dx');
          if (!d.moved) { return; }
          var s = score(), T = total(s), dms = (ev.clientX - d.x0) / lane().getBoundingClientRect().width * viewSpan();
          var f = find(s, d.key);
          if (!f) { return; }
          var free = ev.shiftKey;
          if (d.kind === 'clip' && f.entry.at) {
            ST.stu.apply(spanOp(f.entry.at, fmtTime(snap(Math.max(0, (f.entry.at_ms || 0) + dms), free), f.entry.at.text)));
          } else if (d.kind === 'edge' && f.entry.for) {
            ST.stu.apply(spanOp(f.entry.for, fmtTime(snap(Math.max(100, (f.entry.for_ms || 0) + dms), free), f.entry.for.text)));
          } else if (d.kind === 'key' && f.kind === 'stop') {
            // one keyframe, or every one the box holds: one transaction
            var keys = multi().indexOf(d.key) >= 0 ? multi() : [d.key];
            var edits = keys.map(function (k) {
              var g = find(s, k);
              if (!g || g.kind !== 'stop') { return null; }
              var e = g.entry, len = e.for_ms != null ? e.for_ms : T - (e.at_ms || 0);
              var pos = positions(g.channel.stops)[g.index] + dms / len;
              var pct = Math.round(Math.max(0, Math.min(1, pos)) * 1000) / 10;
              var st = g.stop;
              return { file: fileOf(s, st.span.file), start: st.span.start, end: st.span.end, expect: st.span.text,
                       text: st.value + ' at ' + pct + '%' + (st.easing ? ' ' + st.easing : '') };
            }).filter(Boolean);
            ST.stu.send('/__spacetime/rewrite', { entry: L().stuEntry, edits: edits }, 'Saved to ');
          }
        };

        F.key = function (ev) {
          if (!document.documentElement.classList.contains('stu-film-on')) { return; }
          var tag = (ev.target && ev.target.tagName) || '';
          if (/INPUT|TEXTAREA|SELECT/.test(tag) || (ev.target && ev.target.isContentEditable)) { return; }
          var t = F.transport();
          if (!t) { return; }
          var k = ev.key, hit = true;
          if (k === ' ' || k === 'k' || k === 'K') { if (k === ' ') { F.toggle(); } else { t.pause(); } }
          else if (k === 'l' || k === 'L') { t.play(); }
          else if (k === 'j' || k === 'J') { t.pause(); F.seek(t.position - 1000); }
          else if (k === 'ArrowRight') { F.step(ev.shiftKey ? FPS : 1); }
          else if (k === 'ArrowLeft') { F.step(ev.shiftKey ? -FPS : -1); }
          else if (k === 'Home') { F.seek(0); }
          else if (k === 'End') { F.seek(t.duration); }
          else if (k === 'i' || k === 'I') { F.loopIn = t.position; F.setLoop(); }
          else if (k === 'o' || k === 'O') { F.loopOut = t.position; F.setLoop(); }
          else if (k === 'p' || k === 'P') { ST.stuFilmPickTool(); }
          else if (k === 'c' || k === 'C') { F.editorCam(!F.ed); }
          else if (k === 'f' || k === 'F') { F.frame(); }
          else if (k === 'm' || k === 'M') { F.moveTool(!F.moveOn); }
          // the view: \ fits the film, = and - zoom about the playhead
          else if (k === '\\') { F.fit(); }
          else if (k === '=' || k === '+' || k === '-') {
            var r = lane().getBoundingClientRect();
            F.zoomAt(r.left + (t.position - viewFrom()) / viewSpan() * r.width, k === '-' ? 0.8 : 1.25);
          }
          // Escape leaves the most transient thing first: the tool, the loop, the selection
          else if (k === 'Escape' && F.pickOn) { ST.stuFilmPickTool(); }
          else if (k === 'Escape' && (F.loopIn != null || F.loopOut != null)) { F.loopIn = F.loopOut = null; F.setLoop(); }
          else if (k === 'Escape' && multi().length) { put('stuFilmMulti', ''); }
          else if (k === 'Escape' && L().stuFilmSel) { F.select(''); }
          else { hit = false; }
          if (hit) { ev.preventDefault(); ev.stopPropagation(); }
        };

        // Actions the panels call through `$stuNoop <- ST.stuFilmX($)`.
        ST.stuFilmToggle = function () { F.toggle(); return ''; };
        ST.stuFilmFit = function () { F.fit(); return ''; };

        // ---- the editor camera and the move tool (PLAN-156 P8) ---------------------------
        // The stage is seen through a free camera (orbit: drag; pan: right-drag;
        // dolly: wheel) while the film's camera keeps following the score. C
        // toggles it, F frames the selected part. With M, dragging the selected
        // part moves it over the ground (Shift: up and down); on release its
        // position is written, as words (or as the stop at the playhead when the
        // score drives it).
        F.editorCam = function (on) {
          var st = F.stage();
          if (!st || !st.THREE || !st.renderer || !st.view) { return; }
          if (on && !F.ed) {
            var cam = st.camera.clone();
            // the film's own layers (its titles) sit over the canvas: the camera
            // listens on the studio's hit layer, laid over the canvas while a 3D
            // tool is on
            var ctl = new st.THREE.addons.OrbitControls(cam, document.querySelector('.stu-3d-hit') || st.renderer.domElement);
            ctl.enablePan = true; ctl.enableZoom = true; ctl.enableDamping = false;
            var dir = new st.THREE.Vector3();
            st.camera.getWorldDirection(dir);
            ctl.target.copy(st.camera.position).addScaledVector(dir, 8);
            ctl.update();
            ctl.addEventListener('change', function () { st.draw(); });
            F.ed = { cam: cam, ctl: ctl };
            st.view(cam);
          } else if (!on && F.ed) {
            F.ed.ctl.dispose();
            F.ed = null;
            st.view(null);
          }
          put('stuFilmCam', F.ed ? 'Editor camera' : 'Film camera');
          F.syncHit();
        };
        /// Lay the hit layer over the stage's canvas while a 3D tool is on.
        F.syncHit = function () {
          var on = !!(F.ed || F.moveOn), hit = document.querySelector('.stu-3d-hit'), st = F.stage();
          document.documentElement.classList.toggle('stu-3d-tools', on);
          if (!on || !hit || !st || !st.renderer) { return; }
          var r = st.renderer.domElement.getBoundingClientRect();
          hit.style.left = r.left + 'px'; hit.style.top = r.top + 'px';
          hit.style.width = r.width + 'px'; hit.style.height = r.height + 'px';
        };
        ST.stuFilmCamToggle = function () { F.editorCam(!F.ed); return ''; };
        /// Bring the selected part into the editor camera's view.
        F.frame = function () {
          var st = F.stage(), sel = L().stuFilmSel || '';
          if (!st || sel.indexOf('part:') !== 0) { return; }
          var pt = F.stagePart(score(), +sel.split(':')[1]);
          if (!pt || !pt.obj) { return; }
          if (!F.ed) { F.editorCam(true); }
          var T3 = st.THREE, box = new T3.Box3().setFromObject(pt.obj);
          var c = box.getCenter(new T3.Vector3()), size = Math.max(0.5, box.getSize(new T3.Vector3()).length());
          var dir = new T3.Vector3().subVectors(F.ed.cam.position, F.ed.ctl.target).normalize();
          F.ed.ctl.target.copy(c);
          F.ed.cam.position.copy(c).addScaledVector(dir, size * 1.8);
          F.ed.ctl.update();
          st.draw();
        };

        /// Does a 3D tool of the film own this pointer event (a click on the stage)?
        F.ownsPointer = function (ev) {
          var t = ev.target && ev.target.closest && (ev.target.closest('.stu-3d-hit') || ev.target.closest('canvas'));
          return !!((F.ed || F.moveOn) && t && (t._stThree || t.classList.contains('stu-3d-hit')));
        };
        F.moveTool = function (on) {
          F.moveOn = !!on;
          put('stuFilmMoveOn', F.moveOn);
          document.documentElement.classList.toggle('stu-moving', F.moveOn);
          F.syncHit();
        };
        ST.stuFilmMoveToggle = function () { F.moveTool(!F.moveOn); return ''; };
        var selectedPart = function () {
          var sel = L().stuFilmSel || '';
          return sel.indexOf('part:') === 0 ? F.stagePart(score(), +sel.split(':')[1]) : null;
        };
        F.moveDown = function (ev) {
          if (!F.moveOn || ev.button !== 0 || inStudioUi(ev.target)) { return; }
          var st = F.stage(), pt = selectedPart(), hit = F.pickAt(ev);
          if (!st || !pt || hit !== pt) { return; }
          var T3 = st.THREE, cam = st.looking();
          // the plane the part moves in: the ground through it, or (Shift) the
          // upright plane facing the camera
          var normal = ev.shiftKey ? cam.getWorldDirection(new T3.Vector3()).setY(0).normalize().negate() : new T3.Vector3(0, 1, 0);
          var plane = new T3.Plane().setFromNormalAndCoplanarPoint(normal, pt.obj.position);
          var at = F.ray.ray.intersectPlane(plane, new T3.Vector3());
          if (!at) { return; }
          F.moving = { pt: pt, plane: plane, at: at, from: pt.obj.position.clone(), up: ev.shiftKey };
          if (F.ed) { F.ed.ctl.enabled = false; }
          ev.preventDefault();
          ev.stopImmediatePropagation();
        };
        F.moveMove = function (ev) {
          var m = F.moving;
          if (!m) { return; }
          var st = F.stage();
          F.pickAt(ev); // aims F.ray through the pointer
          var now = F.ray.ray.intersectPlane(m.plane, new st.THREE.Vector3());
          if (!now) { return; }
          var d = now.sub(m.at);
          if (m.up) { d.set(0, d.y, 0); } else { d.y = 0; }
          m.pt.obj.position.copy(m.from).add(d);
          m.delta = d;
          st.draw();
          ev.preventDefault();
          ev.stopImmediatePropagation();
        };
        F.moveUp = function (ev) {
          var m = F.moving;
          if (!m) { return; }
          F.moving = null;
          if (F.ed) { F.ed.ctl.enabled = true; }
          ev.preventDefault();
          ev.stopImmediatePropagation();
          if (!m.delta || m.delta.length() < 1e-4) { return; }
          var round = function (v) { return Math.round(v * 1000) / 1000; };
          var axes = { posX: m.delta.x, posY: m.delta.y, posZ: m.delta.z };
          var fields = ST.stuPartFields(L().stuFilmSel, score(), L().stuReflect);
          var flat = [];
          fields.forEach(function (f) { if (f.parts) { flat = flat.concat(f.parts); } else { flat.push(f); } });
          var edits = [], inserts = [], last = null;
          Object.keys(axes).forEach(function (axis) {
            var d = axes[axis], f = flat.find(function (x) { return x.label === axis; });
            if (!f || Math.abs(d) < 1e-4) { return; }
            var v = String(round(Number(f.value) + d));
            if (f.drive) {
              var e = F.stopEdit(f.drive, String(round(m.from[axis.slice(-1).toLowerCase()] + d)));
              if (e) { edits.push(e); }
            } else if (f.written) {
              edits.push({ file: f.file, start: f.start, end: f.end, expect: f.expect, text: v });
            } else if (f.at === 'after') {
              inserts.push(axis + ': ' + v);
              last = f;
            }
          });
          if (inserts.length && last) {
            // the params still to write go after the last written one, in one edit
            // (folded into that param's own edit when it moves too)
            var mine = edits.find(function (e) { return e.start === last.start && e.end === last.end; });
            if (mine) { mine.text += ', ' + inserts.join(', '); }
            else { edits.push({ file: last.file, start: last.start, end: last.end, expect: last.expect, text: last.expect + ', ' + inserts.join(', ') }); }
          }
          if (edits.length) { ST.stu.send('/__spacetime/rewrite', { entry: L().stuEntry, edits: edits }, 'Saved to '); }
        };
        ST.stuFilmGo = function (el) {
          var t = F.transport(), a = el.dataset.go;
          if (a === 'start') { F.seek(0); } else if (a === 'end') { F.seek(t ? t.duration : 0); }
          else if (a === 'back') { F.step(-1); } else if (a === 'fwd') { F.step(1); }
          return '';
        };
        ST.stuFilmCommit = function (input) {
          var d = input.dataset;
          if (input.value === input.defaultValue) { return ''; }
          if (d.mode === 'insert') {
            ST.stu.apply({ op: 'insert', file: d.file, at: +d.at, before: d.before, text: ', ' + d.label + ': ' + input.value });
          } else {
            ST.stu.apply({ op: 'span', file: d.file, start: +d.start, end: +d.end, expect: d.expect, value: input.value });
          }
          return '';
        };
        ST.stuFilmCell = function (el) {
          if (el.dataset.kind !== 'cell') { return ''; }
          var d = el.dataset;
          // a melodic cell is a note: pick it, and the keyboard edits it
          if (d.melodic === 'true') { put('stuFilmNoteCell', L().stuFilmNoteCell === d.rc ? '' : d.rc); return ''; }
          var next = d.text === '.' ? 'x' : '.';
          ST.stu.apply({ op: 'span', file: d.file, start: +d.start, end: +d.end, expect: d.text, value: next });
          return '';
        };
        ST.stuFilmSave = function (area) {
          var d = area.dataset;
          if (area.value === area.defaultValue) { return ''; }
          ST.stu.apply({ op: 'source', file: d.path, base: d.hash, text: area.value });
          return '';
        };
        ST.stuFilmPick = function (el) { F.select(el.dataset.key); return ''; };

        // ---- 4. sound: waveforms from the synthesizer itself ----------------------------

        F.paintWaves = function () {
          var s = score(), m = main(s);
          if (!m || typeof STSynth === 'undefined' || !s.programs || !s.programs[0]) { return; }
          var prog = s.programs[0], sig = JSON.stringify(prog);
          if (F.waveSig !== sig) {
            F.waveSig = sig;
            try { F.wave = STSynth.render(prog, { sr: 4000 }); } catch (e) { F.wave = null; }
          }
          if (!F.wave) { return; }
          var sr = F.wave.sr || 4000, left = F.wave.left;
          document.querySelectorAll('.stu-wave').forEach(function (cv) {
            var e = m.entries[+cv.dataset.entry];
            if (!e) { return; }
            var w = cv.clientWidth, h = cv.clientHeight;
            if (!w || !h) { return; }
            if (cv.width !== w) { cv.width = w; cv.height = h; }
            var from = Math.floor((e.at_ms || 0) / 1000 * sr);
            var len = Math.floor((e.for_ms != null ? e.for_ms : 2000) / 1000 * sr);
            var g = cv.getContext('2d');
            g.clearRect(0, 0, w, h);
            g.fillStyle = getComputedStyle(cv).color;
            for (var x = 0; x < w; x++) {
              var a = from + Math.floor(x / w * len), b = from + Math.floor((x + 1) / w * len), peak = 0;
              for (var i = a; i < b && i < left.length; i++) { var v = Math.abs(left[i]); if (v > peak) { peak = v; } }
              var bar = Math.max(1, peak * h);
              g.fillRect(x, (h - bar) / 2, 1, bar);
            }
          });
        };

        // ---- across the reload an edit causes -------------------------------------------

        F.remember = function () {
          var t = F.transport();
          try {
            sessionStorage.setItem('stu.film', JSON.stringify({
              at: Date.now(), t: t ? t.position : 0, playing: t ? t.playing : false,
              loopIn: F.loopIn, loopOut: F.loopOut, sel: L().stuFilmSel || ''
            }));
          } catch (e) {}
        };
        F.restore = function () {
          var saved = null;
          try { saved = JSON.parse(sessionStorage.getItem('stu.film') || 'null'); sessionStorage.removeItem('stu.film'); } catch (e) {}
          if (!saved || Date.now() - saved.at > 60000) { return; }
          var tries = 0;
          F.restoring = true;
          var go = function () {
            // the film's timeline and the studio's own state (its score) are both up
            var t = F.transport();
            if ((!t || !L().stuScore) && tries++ < 200) { setTimeout(go, 50); return; }
            F.restoring = false;
            if (!t) { return; }
            if (!saved.playing) { t.pause(); }
            t.seek(saved.t);
            F.loopIn = saved.loopIn; F.loopOut = saved.loopOut; F.setLoop();
            if (saved.sel) { F.select(saved.sel); }
          };
          go();
        };

        /// Opening the studio on a film that has played to its end shows its
        /// first frame, not its last: a restore (after an edit's reload) wins.
        F.rewind = function () {
          var tries = 0;
          var go = function () {
            var t = F.transport();
            if (!t && tries++ < 100) { setTimeout(go, 50); return; }
            if (!t || F.restoring) { return; }
            if (!t.playing && t.position >= t.duration - 1) { t.seek(0); }
          };
          go();
        };

        F.setOn = function (on) {
          on = !!on;
          if (on === !!F.on) { return; }
          F.on = on;
          document.documentElement.classList.toggle('stu-film-on', on);
          // The VIEWER: a film fills its viewport (a fixed, full-frame reel), so
          // the page itself becomes the frame between the panels - `<body>` is
          // made the containing block of its fixed children - and the dock,
          // which must stay on the screen's edges, steps out of it.
          var dock = document.querySelector('.st-dev-dock');
          if (dock) {
            if (on && dock.parentNode === document.body) { F.dockHome = document.body; document.documentElement.appendChild(dock); }
            if (!on && F.dockHome && dock.parentNode !== F.dockHome) { F.dockHome.appendChild(dock); }
          }
          dispatchEvent(new Event('resize'));
          var m = on ? 'addEventListener' : 'removeEventListener';
          document[m]('pointerdown', F.down, true);
          document[m]('wheel', F.wheel, { capture: true, passive: false });
          document[m]('pointermove', F.move, true);
          document[m]('pointerup', F.up, true);
          document[m]('keydown', F.key, true);
          // the Select tool runs before the page's own studio canvas (window capture)
          window[m]('pointermove', F.pickMove, true);
          window[m]('pointerdown', F.moveDown, true);
          window[m]('pointermove', F.moveMove, true);
          window[m]('pointerup', F.moveUp, true);
          window[m]('click', F.pickClick, true);
          document[m]('pointerover', F.listOver, true);
          document[m]('pointerout', F.listOut, true);
          if (!on && F.pickOn) { ST.stuFilmPickTool(); }
          if (on) {
            F.rewind();
            F.raf = requestAnimationFrame(tick);
            setTimeout(F.paintWaves, 60);
          } else {
            cancelAnimationFrame(F.raf);
          }
        };
        // every way the page goes (a deferred reload included) passes here
        addEventListener('pagehide', function () { if (F.on) { F.remember(); } });
        addEventListener('resize', function () { if (F.on) { F.paintWaves(); F.syncHit(); } });
        F.restore();
      })();
    }
  }
}

/// Binds the timeline to the studio: on while the studio is open and the page
/// has a score; waveforms repaint when the score's shape arrives.
%primitive film-panel(&el, on: string = "", shape: string = "") %uses film-kit {
  %emit js {
    (() => {
      const host = %&el;
      const on = %on, shape = %shape;
      const sync = () => {
        const open = !!ST.resolve(host, on.split('.')[0]);
        const s = ST.resolve(host, shape.split('.')[0]);
        ST.stuFilm.setOn(open && !!(s && s.scores && s.scores.length));
        if (open) { setTimeout(ST.stuFilm.paintWaves, 30); }
      };
      const stops = [];
      if (ST.watchScoped) {
        stops.push(ST.watchScoped(host, on.split('.')[0], sync));
        stops.push(ST.watchScoped(host, shape.split('.')[0], sync));
      }
      sync();
      %cleanup {
        stops.forEach((s) => s && s());
        ST.stuFilm.setOn(false);
      }
    })();
  }
}

%macro film-panel {
  %form { @film-panel(on: $on:binding, shape: $shape:binding) }
  %binds { film-panel(&self, on: $on, shape: $shape) }
}
