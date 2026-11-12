/// Score Synth Primitive — a score's `@sound` notes, played live (PLAN-152).
///
/// The score's sounds and the `@form instrument` / `@form pattern` forms they
/// name arrive as ONE program (src/pipeline/score.rs). The vendored-first-party
/// synthesizer `STSynth` (stdlib/film/synth/synth.js — the same file `spacetime
/// render` runs to write the film's soundtrack) renders it to a buffer once;
/// the buffer plays in step with the score's progress, seeking when the
/// transport jumps, silent while it is paused. Under a virtual clock (render, `@clock`) nothing plays: the
/// film's sound is written by the renderer from the same program instead.

%primitive score-synth(
  /// the score's element
  &el,
  /// the synth program (JSON)
  program: string = "",
  /// the score's length (ms)
  total: number = 0,
  /// the score's progress signal
  progress: string = "time",
  bind: string = ""
) {
  %emit js {
    const root = %&el;
    const totalMs = %total;
    let prog = null;
    try { prog = JSON.parse(%program); } catch (_e) { console.warn('[Spacetime] @sound: bad program'); }
    if (typeof STSynth === 'undefined') console.warn('[Spacetime] @sound needs `@import "stdlib/film"` (its synthesizer)');
    if (!root || !prog || typeof STSynth === 'undefined') { %yield true -> $ready; return; }
    // a film being rendered hears nothing here: its soundtrack is written by
    // the renderer from this same program
    const virtual = () => typeof ST !== 'undefined' && ST._clock && ST._clock.virtual;
    let ctx = null, buffer = null, src = null, startedAt = 0, offset = 0;
    const ensure = () => {
      if (buffer || virtual()) return !!buffer;
      const AC = typeof AudioContext !== 'undefined' ? AudioContext : (typeof webkitAudioContext !== 'undefined' ? webkitAudioContext : null);
      if (!AC) return false;
      ctx = ctx || new AC();
      const r = STSynth.render(prog, { sr: ctx.sampleRate });
      buffer = ctx.createBuffer(2, r.left.length, r.sr);
      buffer.copyToChannel(r.left, 0);
      buffer.copyToChannel(r.right, 1);
      return true;
    };
    const play = (sec) => {
      if (src) { try { src.stop(); } catch (_e) {} src = null; }
      if (!ensure() || sec >= buffer.duration) return;
      src = ctx.createBufferSource();
      src.buffer = buffer;
      src.connect(ctx.destination);
      src.start(0, Math.max(0, sec));
      startedAt = ctx.currentTime; offset = sec;
    };
    if (typeof ST === 'undefined' || !ST.watchScoped) { %yield true -> $ready; return; }
    let held = false;
    const silence = () => { if (src) { try { src.stop(); } catch (_e) {} src = null; } };
    ST.watchScoped(root, %progress, (p) => {
      if (virtual()) return;
      const sec = ((typeof p === 'number' ? p : 0) * totalMs) / 1000;
      root._stSynthAt = sec;
      if (held) return;
      const heard = src ? offset + (ctx.currentTime - startedAt) : -1;
      if (!src || Math.abs(heard - sec) > 0.2) play(sec);
    });
    // PLAN-155 hot bind: new notes / a new pattern / a new instrument is a new
    // program; it is rendered again and heard from where the score is.
    ST.hot(%bind, (a) => {
      let next = null;
      try { next = JSON.parse(a.program); } catch (_e) { return false; }
      prog = next;
      root._stSynthProgram = prog;
      if (buffer) {
        buffer = null;
        silence();
        if (!held && !virtual() && ensure()) play(root._stSynthAt || 0);
      }
      return true;
    });

    // a paused transport is silent; it sounds again from where it is
    ST.watchScoped(root, %progress + '__playing', (on) => {
      held = on === false;
      if (held) silence();
    });
    %yield true -> $ready;
  }

  %exports {
    $ready: bool
  }
}
