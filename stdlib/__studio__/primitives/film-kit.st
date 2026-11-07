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
            out.push({ kind: 'name', row: r + 1, col: 1, text: row.prop, on: false, file: '', start: 0, end: 0, expect: '' });
            (row.cells || []).forEach(function (c, k) {
              out.push(Object.assign({ kind: 'cell', row: r + 1, col: k + 2, text: c.text, on: c.text !== '.', file: fileOf(s, c.file) }, addr(c)));
            });
          });
          return out;
        };

        /// The scene: the stage and its parts.
        ST.stuFilmParts = function (s) {
          return ((s && s.parts) || []).map(function (p, i) {
            return { key: 'part:' + i, kind: p.kind, label: p.name ? '&' + p.name : p.kind, text: p.kind, depth: p.kind === 'stage' ? 0 : 1 };
          });
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
          var d = el.dataset, next = d.text === '.' ? 'x' : '.';
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
