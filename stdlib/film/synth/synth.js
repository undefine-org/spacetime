// Spacetime's score synthesizer (PLAN-152) — first-party, EPL-2.0 like the stdlib.
//
// ONE implementation, three hosts: a browser plays the score's sound live from
// it (stdlib/primitives/score-synth.st), and `spacetime render` runs this same
// file in V8 (src/render/soundtrack.rs) to write the film's soundtrack — so a
// website and its video hear the same samples. No Web Audio, no DOM: a pure
// function of the program, deterministic (noise is seeded).
//
// The program (built by src/pipeline/score.rs from `@sound` entries and the
// `@form instrument` / `@form pattern` declarations they name):
//   { total: ms,
//     sounds: [{ form, at: ms, dur: ms|null, notes: "C3 Eb3"|null, name }],
//     forms: { "--kick": { kind: "instrument", props: { wave: ["sine"], gain: ["1", "0"], … } },
//              "--beat": { kind: "pattern", params: { tempo: "118bpm", step: "1/16" }, props: { "--kick": ["x . . ."] } } } }
//
// render(program, { sr, from, to }) -> { sr, left: Float32Array, right: Float32Array }
// wav(program, opts) -> base64 of a 16-bit stereo WAV
(function (global) {
  "use strict";

  // ---------------------------------------------------------------- values
  // A stop is `value [unit] [at p% | at t ms|s]` — the film surface's own stop
  // grammar (PLAN-150 W1), read here from its text.
  function stop(text) {
    const t = String(text).trim();
    const m = /^(-?[\d.]+)\s*([a-z%]*)?(?:\s+at\s+(-?[\d.]+)\s*(%|ms|s)?)?/i.exec(t);
    if (!m) return { word: t.split(/\s+/)[0] };
    let v = parseFloat(m[1]);
    const unit = (m[2] || "").toLowerCase();
    if (unit === "khz") v *= 1000;
    if (unit === "s") v *= 1000;
    let at = null;
    if (m[3] != null) {
      const a = parseFloat(m[3]);
      at = { v: a, unit: (m[4] || "%").toLowerCase() };
    }
    return { v, unit, at };
  }

  // An envelope from stops: f(u, lenMs) -> value, u in 0..1 of the note's life.
  function envelope(stops, dflt) {
    if (!stops || !stops.length) return () => dflt;
    const ss = stops.map(stop).filter((s) => s.v != null);
    if (!ss.length) return () => dflt;
    if (ss.length === 1) return () => ss[0].v;
    return (u, lenMs) => {
      // positions: explicit `at`, else spread evenly between neighbours
      const pos = ss.map((s, i) => (s.at ? (s.at.unit === "%" ? s.at.v / 100 : (s.at.unit === "s" ? s.at.v * 1000 : s.at.v) / Math.max(1, lenMs)) : null));
      pos[0] = pos[0] == null ? 0 : pos[0];
      pos[pos.length - 1] = pos[pos.length - 1] == null ? 1 : pos[pos.length - 1];
      for (let i = 1; i < pos.length - 1; i++) {
        if (pos[i] == null) {
          let j = i + 1;
          while (pos[j] == null) j++;
          const a = pos[i - 1], b = pos[j];
          for (let k = i; k < j; k++) pos[k] = a + ((b - a) * (k - i + 1)) / (j - i + 1);
        }
      }
      if (u <= pos[0]) return ss[0].v;
      for (let i = 1; i < ss.length; i++) {
        if (u <= pos[i]) {
          const span = pos[i] - pos[i - 1];
          const k = span > 0 ? (u - pos[i - 1]) / span : 1;
          return ss[i - 1].v + (ss[i].v - ss[i - 1].v) * k;
        }
      }
      return ss[ss.length - 1].v;
    };
  }

  const NOTE = { c: 0, d: 2, e: 4, f: 5, g: 7, a: 9, b: 11 };
  function noteHz(name) {
    const m = /^([a-g])(#|b|-flat|-sharp)?(-?\d)$/i.exec(String(name).trim());
    if (!m) return null;
    let n = NOTE[m[1].toLowerCase()];
    if (m[2] === "#" || m[2] === "-sharp") n += 1;
    if (m[2] === "b" || m[2] === "-flat") n -= 1;
    const midi = (parseInt(m[3], 10) + 1) * 12 + n;
    return 440 * Math.pow(2, (midi - 69) / 12);
  }

  function mulberry(seed) {
    let a = seed >>> 0;
    return () => {
      a = (a + 0x6d2b79f5) >>> 0;
      let t = a;
      t = Math.imul(t ^ (t >>> 15), t | 1);
      t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  }

  // ---------------------------------------------------------------- voices
  // The waves, as data: phase (cycles) -> sample, with PolyBLEP on the
  // discontinuous ones so a saw doesn't alias.
  function blep(t, dt) {
    if (t < dt) { t /= dt; return t + t - t * t - 1; }
    if (t > 1 - dt) { t = (t - 1) / dt; return t * t + t + t + 1; }
    return 0;
  }
  const WAVES = {
    sine: (p) => Math.sin(2 * Math.PI * p),
    triangle: (p) => 1 - 4 * Math.abs(p - 0.5),
    saw: (p, dt) => 2 * p - 1 - blep(p, dt),
    square: (p, dt) => (p < 0.5 ? 1 : -1) + blep(p, dt) - blep((p + 0.5) % 1, dt),
  };

  // One note into the buses: an instrument's props over [start, start+len).
  function note(bus, sr, inst, startMs, lenMs, hz, seed, gainScale) {
    const pr = inst.props || {};
    const word = (k, d) => (pr[k] && pr[k][0] != null ? String(pr[k][0]).trim().split(/\s+/)[0] : d);
    const wave = word("wave", "sine");
    const gain = envelope(pr.gain, 1);
    const hasPitch = !!pr.pitch;
    const pitch = envelope(pr.pitch, hz || 220);
    const pitch0 = hasPitch ? stop(pr.pitch[0]).v : null;
    const cutoff = pr.cutoff || pr.lowpass ? envelope(pr.cutoff || pr.lowpass, 20000) : null;
    const hp = pr.highpass ? stop(pr.highpass[0]).v : 0;
    const detune = pr.detune ? stop(pr.detune[0]).v : 0;
    const voices = Math.max(1, Math.round(pr.voices ? stop(pr.voices[0]).v : detune ? 2 : 1));
    const pan = pr.pan ? Math.max(-1, Math.min(1, stop(pr.pan[0]).v)) : 0;
    const send = pr.reverb ? Math.max(0, Math.min(1, stop(pr.reverb[0]).v)) : 0;
    const a = Math.round((startMs / 1000) * sr);
    const n = Math.max(1, Math.round((lenMs / 1000) * sr));
    const rnd = mulberry(seed);
    const phases = new Float64Array(voices);
    for (let v = 0; v < voices; v++) phases[v] = rnd();
    let lp = 0, hpPrev = 0, hpOut = 0;
    const lr = Math.cos(((pan + 1) * Math.PI) / 4), rr = Math.sin(((pan + 1) * Math.PI) / 4);
    const hpK = hp > 0 ? 1 / (1 + (2 * Math.PI * hp) / sr) : 0;
    for (let i = 0; i < n; i++) {
      const k = a + i;
      if (k < 0) continue;
      if (k >= bus.left.length) break;
      const u = i / n;
      let f = pitch(u, lenMs);
      if (hasPitch && hz && pitch0) f = (f / pitch0) * hz; // the envelope's shape, at the note's pitch
      let s = 0;
      if (wave === "noise") s = rnd() * 2 - 1;
      else {
        for (let v = 0; v < voices; v++) {
          const cents = voices > 1 ? detune * ((v / (voices - 1)) * 2 - 1) : 0;
          const dt = (f * Math.pow(2, cents / 1200)) / sr;
          phases[v] = (phases[v] + dt) % 1;
          s += (WAVES[wave] || WAVES.sine)(phases[v], dt);
        }
        s /= Math.sqrt(voices);
      }
      if (cutoff) {
        const fc = Math.max(20, Math.min(sr / 2.2, cutoff(u, lenMs)));
        const kk = 1 - Math.exp((-2 * Math.PI * fc) / sr);
        lp += (s - lp) * kk;
        s = lp;
      }
      if (hpK) { hpOut = hpK * (hpOut + s - hpPrev); hpPrev = s; s = hpOut; }
      // a 2 ms ramp at both ends: no click
      const edge = Math.min(1, i / (0.002 * sr), (n - i) / (0.002 * sr));
      s *= gain(u, lenMs) * gainScale * edge;
      bus.left[k] += s * lr;
      bus.right[k] += s * rr;
      if (send) bus.wet[k] += s * send;
    }
  }

  // ---------------------------------------------------------------- reverb
  // Schroeder: four combs and two all-passes per channel, offset for width.
  function reverb(wet, sr, outL, outR) {
    const combs = [1557, 1617, 1491, 1422], aps = [225, 556];
    for (const [out, spread] of [[outL, 0], [outR, 23]]) {
      const lines = combs.map((d) => ({ b: new Float32Array(Math.round(((d + spread) * sr) / 44100)), i: 0, f: 0 }));
      const alls = aps.map((d) => ({ b: new Float32Array(Math.round(((d + spread) * sr) / 44100)), i: 0 }));
      for (let k = 0; k < wet.length; k++) {
        const x = wet[k] * 0.015;
        let y = 0;
        for (const c of lines) {
          const o = c.b[c.i];
          c.f = o * 0.8 + c.f * 0.2;
          c.b[c.i] = x + c.f * 0.84;
          c.i = (c.i + 1) % c.b.length;
          y += o;
        }
        for (const ap of alls) {
          const o = ap.b[ap.i];
          ap.b[ap.i] = y + o * 0.5;
          ap.i = (ap.i + 1) % ap.b.length;
          y = o - y * 0.5;
        }
        out[k] += y;
      }
    }
  }

  // ---------------------------------------------------------------- the score
  function render(program, opts) {
    const o = opts || {};
    const sr = o.sr || 48000;
    const total = program.total || 0;
    const from = o.from || 0, to = o.to == null ? total : o.to;
    const len = Math.max(1, Math.round(((to - from) / 1000) * sr));
    const bus = { left: new Float32Array(len), right: new Float32Array(len), wet: new Float32Array(len) };
    const forms = program.forms || {};
    let seed = 1;
    const instrumentLen = (inst) => (inst.props && inst.props.for ? stop(inst.props.for[0]).v : 500);
    for (const s of program.sounds || []) {
      const f = forms[s.form];
      if (!f) continue;
      const at = s.at - from;
      const gainScale = s.gain != null ? s.gain : 1;
      if (f.kind === "instrument") {
        const notes = s.notes ? String(s.notes).trim().split(/\s+/) : [null];
        const lenMs = s.dur != null ? s.dur : instrumentLen(f);
        for (const nm of notes) note(bus, sr, f, at, lenMs, nm ? noteHz(nm) : null, seed++, gainScale / Math.sqrt(notes.length));
      } else if (f.kind === "pattern") {
        const p = f.params || {};
        const bpm = parseFloat(p.tempo) || 120;
        const stepFrac = (() => { const m = /^(\d+)\s*\/\s*(\d+)$/.exec(String(p.step || "1/16")); return m ? parseInt(m[1], 10) / parseInt(m[2], 10) : 1 / 16; })();
        const stepMs = (60000 / bpm) * 4 * stepFrac;
        const rows = Object.entries(f.props || {}).map(([inst, cells]) => [inst, String(cells[0] || "").trim().split(/\s+/)]);
        const steps = Math.max(1, ...rows.map(([, c]) => c.length));
        const windowMs = s.dur != null ? s.dur : steps * stepMs;
        for (let t = 0, k = 0; t < windowMs - 1e-6; t += stepMs, k++) {
          for (const [inst, cells] of rows) {
            const cell = cells[k % cells.length];
            const def = forms[inst];
            if (!def || !cell || cell === "." || cell === "-") continue;
            const hz = cell === "x" ? null : noteHz(cell);
            note(bus, sr, def, at + t, instrumentLen(def), hz, seed++, gainScale);
          }
        }
      }
    }
    reverb(bus.wet, sr, bus.left, bus.right);
    // the master: to a -1 dBFS peak, a gentle tanh above it
    let peak = 0;
    for (let k = 0; k < len; k++) peak = Math.max(peak, Math.abs(bus.left[k]), Math.abs(bus.right[k]));
    const g = peak > 0 ? 0.89 / peak : 1;
    for (let k = 0; k < len; k++) { bus.left[k] = Math.tanh(bus.left[k] * g * 1.1) / 1.1; bus.right[k] = Math.tanh(bus.right[k] * g * 1.1) / 1.1; }
    return { sr, left: bus.left, right: bus.right };
  }

  function wav(program, opts) {
    const r = render(program, opts);
    const n = r.left.length, bytes = 44 + n * 4;
    const buf = new Uint8Array(bytes);
    const dv = new DataView(buf.buffer);
    const str = (o, s) => { for (let i = 0; i < s.length; i++) buf[o + i] = s.charCodeAt(i); };
    str(0, "RIFF"); dv.setUint32(4, bytes - 8, true); str(8, "WAVE"); str(12, "fmt ");
    dv.setUint32(16, 16, true); dv.setUint16(20, 1, true); dv.setUint16(22, 2, true);
    dv.setUint32(24, r.sr, true); dv.setUint32(28, r.sr * 4, true); dv.setUint16(32, 4, true); dv.setUint16(34, 16, true);
    str(36, "data"); dv.setUint32(40, n * 4, true);
    for (let i = 0; i < n; i++) {
      dv.setInt16(44 + i * 4, Math.max(-1, Math.min(1, r.left[i])) * 32767, true);
      dv.setInt16(46 + i * 4, Math.max(-1, Math.min(1, r.right[i])) * 32767, true);
    }
    // base64 without btoa (V8 without a browser has none)
    const T = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    const out = [];
    for (let i = 0; i < bytes; i += 3) {
      const a = buf[i], b = i + 1 < bytes ? buf[i + 1] : 0, c = i + 2 < bytes ? buf[i + 2] : 0;
      out.push(T[a >> 2] + T[((a & 3) << 4) | (b >> 4)] + (i + 1 < bytes ? T[((b & 15) << 2) | (c >> 6)] : "=") + (i + 2 < bytes ? T[c & 63] : "="));
    }
    return out.join("");
  }

  global.STSynth = { render, wav, noteHz, envelope };
})(typeof globalThis !== "undefined" ? globalThis : this);
