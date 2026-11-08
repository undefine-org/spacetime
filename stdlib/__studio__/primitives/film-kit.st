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

        /// The timeline: a row per clip, its channels under it, then the sounds.
        ST.stuFilmRows = function (s) {
          var rows = [], m = main(s);
          if (!m) { return rows; }
          var T = total(s);
          var y = 0;
          m.entries.forEach(function (e, i) {
            if (e.kind !== 'clip') { return; }
            var at = e.at_ms || 0, len = e.for_ms != null ? e.for_ms : T - at;
            rows.push({
              key: 'clip:' + i, kind: 'clip', label: '&' + e.subject, depth: 0, y: y++,
              left: at / T * 100, width: len / T * 100, text: (e.at ? e.at.text : '0s') + ' for ' + (e.for ? e.for.text : '...'),
              entry: i
            });
            var body = s.clips && s.clips[e.subject];
            (body ? body.channels : []).forEach(function (c, j) {
              rows.push({
                key: 'chan:' + i + ':' + j, kind: 'channel', label: (c.scope ? c.scope + ' ' : '') + c.prop, depth: 1, y: y++,
                left: at / T * 100, width: len / T * 100, text: c.value.text, entry: i
              });
            });
          });
          m.entries.forEach(function (e, i) {
            if (e.kind !== 'sound') { return; }
            var at = e.at_ms || 0;
            var form = s.forms && e.form && s.forms[e.form.text];
            var len = e.for_ms != null ? e.for_ms : Math.min(T - at, 2000);
            rows.push({
              key: 'sound:' + i, kind: form && form.kind === 'pattern' ? 'pattern' : 'sound',
              label: (e.name ? e.name.text + ' ' : '') + (e.form ? e.form.text : '@sound'), depth: 0, y: y++,
              left: at / T * 100, width: Math.max(0.6, len / T * 100), text: e.notes ? e.notes.text : '', entry: i
            });
          });
          return rows;
        };

        /// Keyframe diamonds, placed in score time on their channel's row.
        ST.stuFilmKeys = function (s) {
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
                  value: st.value, held: !!(st.easing && st.easing.indexOf('steps') >= 0), fixed: k === 0 || k === c.stops.length - 1
                });
              });
              y++;
            });
          });
          return keys;
        };

        /// Ruler ticks: whole seconds, labelled every few.
        ST.stuFilmTicks = function (s) {
          var T = total(s), ticks = [], step = T > 60000 ? 10 : T > 20000 ? 2 : 1;
          for (var sec = 0; sec * 1000 <= T; sec += 1) {
            ticks.push({ left: sec * 1000 / T * 100, label: sec % step === 0 ? sec + 's' : '', major: sec % step === 0 });
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
          } else if (f.kind === 'part') {
            // a new param goes after the one written last in the source
            var last = null;
            f.part.params.forEach(function (p) { if (p.written && (!last || p.written.end > last.end)) { last = p.written; } });
            f.part.params.forEach(function (p) {
              if (p.written) {
                word(p.name, p.written);
              } else if (last) {
                out.push({ label: p.name, value: String(p.value), hint: 'default', mode: 'insert', file: fileOf(s, last.file), start: 0, end: 0, expect: '', at: last.end, before: last.text });
              }
            });
          }
          return out;
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
        var inStudioUi = function (node) { return !!(node && node.closest && node.closest('.stu-ui, .st-dev-dock, .stu-overlay')); };
        F.pickAt = function (ev) {
          var st = F.stage();
          if (!st || !st.THREE || !st.camera || !st.renderer) { return null; }
          var r = st.renderer.domElement.getBoundingClientRect();
          if (ev.clientX < r.left || ev.clientX > r.right || ev.clientY < r.top || ev.clientY > r.bottom) { return null; }
          var ndc = new st.THREE.Vector2((ev.clientX - r.left) / r.width * 2 - 1, -((ev.clientY - r.top) / r.height) * 2 + 1);
          F.ray = F.ray || new st.THREE.Raycaster();
          F.ray.setFromCamera(ndc, st.camera);
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
          put('stuFilmSel', 'part:' + i);
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
        var msAt = function (clientX) {
          var r = lane().getBoundingClientRect();
          return Math.max(0, Math.min(1, (clientX - r.left) / r.width)) * total(score());
        };
        var spanOp = function (w, value) {
          return { op: 'span', file: fileOf(score(), w.file), start: w.start, end: w.end, expect: w.text, value: value };
        };

        F.down = function (ev) {
          if (ev.button !== 0) { return; }
          var el = ev.target.closest && ev.target.closest('[data-film]');
          if (!el || !el.closest('.stu-film')) { return; }
          var kind = el.dataset.film, s = score();
          if (kind === 'ruler') {
            var t = F.transport();
            F.drag = { kind: 'scrub', was: t && t.playing };
            if (t) { t.pause(); }
            F.seek(msAt(ev.clientX));
          } else if (kind === 'clip' || kind === 'edge' || kind === 'key') {
            var key = el.dataset.key;
            put('stuFilmSel', key);
            F.drag = { kind: kind, key: key, el: el, x0: ev.clientX, moved: false };
          } else if (kind === 'row') {
            put('stuFilmSel', el.dataset.key);
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
          if (Math.abs(dx) > 2) { d.moved = true; }
          d.el.style.setProperty('--stu-dx', dx + 'px');
        };
        F.up = function (ev) {
          var d = F.drag;
          F.drag = null;
          if (!d) { return; }
          if (d.kind === 'scrub') { var t = F.transport(); if (t && d.was) { t.play(); } return; }
          d.el.style.removeProperty('--stu-dx');
          if (!d.moved) { return; }
          var s = score(), T = total(s), dms = (ev.clientX - d.x0) / lane().getBoundingClientRect().width * T;
          var f = find(s, d.key);
          if (!f) { return; }
          var free = ev.shiftKey;
          if (d.kind === 'clip' && f.entry.at) {
            ST.stu.apply(spanOp(f.entry.at, fmtTime(snap(Math.max(0, (f.entry.at_ms || 0) + dms), free), f.entry.at.text)));
          } else if (d.kind === 'edge' && f.entry.for) {
            ST.stu.apply(spanOp(f.entry.for, fmtTime(snap(Math.max(100, (f.entry.for_ms || 0) + dms), free), f.entry.for.text)));
          } else if (d.kind === 'key' && f.kind === 'stop') {
            var e = f.entry, len = e.for_ms != null ? e.for_ms : T - (e.at_ms || 0);
            var pos = positions(f.channel.stops)[f.index] + dms / len;
            var pct = Math.round(Math.max(0, Math.min(1, pos)) * 1000) / 10;
            var st = f.stop;
            ST.stu.apply(spanOp(st.span, st.value + ' at ' + pct + '%' + (st.easing ? ' ' + st.easing : '')));
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
          else if (k === 'Escape' && (F.loopIn != null || F.loopOut != null)) { F.loopIn = F.loopOut = null; F.setLoop(); }
          else if (k === 'p' || k === 'P') { ST.stuFilmPickTool(); }
          else if (k === 'Escape' && F.pickOn) { ST.stuFilmPickTool(); }
          else { hit = false; }
          if (hit) { ev.preventDefault(); ev.stopPropagation(); }
        };

        // Actions the panels call through `$stuNoop <- ST.stuFilmX($)`.
        ST.stuFilmToggle = function () { F.toggle(); return ''; };
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
        ST.stuFilmPick = function (el) { put('stuFilmSel', el.dataset.key); return ''; };

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
          var go = function () {
            // the film's timeline and the studio's own state (its score) are both up
            var t = F.transport();
            if ((!t || !L().stuScore) && tries++ < 200) { setTimeout(go, 50); return; }
            if (!t) { return; }
            if (!saved.playing) { t.pause(); }
            t.seek(saved.t);
            F.loopIn = saved.loopIn; F.loopOut = saved.loopOut; F.setLoop();
            if (saved.sel) { put('stuFilmSel', saved.sel); }
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
          document[m]('pointermove', F.move, true);
          document[m]('pointerup', F.up, true);
          document[m]('keydown', F.key, true);
          // the Select tool runs before the page's own studio canvas (window capture)
          window[m]('pointermove', F.pickMove, true);
          window[m]('click', F.pickClick, true);
          document[m]('pointerover', F.listOver, true);
          document[m]('pointerout', F.listOut, true);
          if (!on && F.pickOn) { ST.stuFilmPickTool(); }
          if (on) {
            F.raf = requestAnimationFrame(tick);
            setTimeout(F.paintWaves, 60);
          } else {
            cancelAnimationFrame(F.raf);
          }
        };
        // every way the page goes (a deferred reload included) passes here
        addEventListener('pagehide', function () { if (F.on) { F.remember(); } });
        addEventListener('resize', function () { if (F.on) { F.paintWaves(); } });
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
