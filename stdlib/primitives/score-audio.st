/// Score Audio Primitive — sound ON the score (PLAN-150 W7, film.st.md §8).
///
/// Creates a hidden `<audio>` element for a `@audio(src:) as &name at <t>;`
/// score entry and binds it to the score's ONE transport: the audio SEEKS to
/// follow the score's progress signal, and PLAYS/PAUSES with it. The score is
/// truth — the audio never runs its own clock. `gain`/`rate`/`pan` are keyframed
/// on the named element in an ordinary `@on &.clip` block (the element is
/// registered under its `&name` so `@on` can address it, the W4 stage-object
/// pattern reused for a media entity).
///
/// Under the browser's `.time` transport the audio plays in real time and seeks
/// on scrub; under a virtual clock (render) it stays muted geometry — the mux is
/// a render-side follow-up. Either way the score, not the audio, owns the clock.
///
/// @param &el      - the score's scope owner (the transport signal resolves here)
/// @param src      - the audio source URL
/// @param name     - the entity name (`bed` from `&bed`), for @on addressing
/// @param at       - the start offset in the score's domain (ms)
/// @param total    - the score's full duration (ms), to map progress -> time
/// @param progress - the score's own progress signal name (`time`)
///
/// @yields $ready  - true once the audio element is wired

%primitive score-audio(
  &el,
  src: string = "",
  name: string = "",
  at: number = 0,
  total: number = 0,
  progress: string = "time"
) {
  %emit js {
    const root = %&el;
    const src = %src, entName = String(%name || '').replace(/^&/, '');
    const atMs = %at, totalMs = %total;
    if (!root || !src) { %yield true -> $ready; return; }

    const audio = document.createElement('audio');
    audio.src = src;
    audio.preload = 'auto';
    audio.setAttribute('aria-hidden', 'true');
    audio.style.cssText = 'position:absolute;width:0;height:0;opacity:0;pointer-events:none;';
    // A channel proxy so `@on &.clip { gain: … }` can address the audio by name
    // (the W4 stage-object rail): apply-animations writes gain/rate/pan here.
    const channels = {};
    const proxy = {
      _stChannels: channels,
      style: new Proxy({}, { set: (o, k, v) => { channels[k] = v; return true; }, get: (o, k) => o[k] }),
    };
    root._stStageObjects = root._stStageObjects || {};
    root._stStageObjects['&' + entName] = proxy;
    root._stStageObjects[entName] = proxy;
    root.appendChild(audio);

    const num = (v, d) => { const n = parseFloat(v); return isNaN(n) ? d : n; };

    if (typeof ST === 'undefined' || !ST.watchScoped) { %yield true -> $ready; return; }

    // The score's progress (0..1) maps to a wall-clock time in the audio's own
    // seconds: audioTime = (progress * total - at) / 1000. Seek to it whenever
    // the score advances; play while the audio's window is live.
    let lastSeek = -1;
    ST.watchScoped(root, %progress, (p) => {
      const prog = typeof p === 'number' ? p : 0;
      const scoreMs = prog * totalMs;
      const audioSec = (scoreMs - atMs) / 1000;
      // Record the intended playhead for tests/tools: currentTime clamps to the
      // real clip duration, but this is the transport-derived seek target.
      audio._stSeekTarget = audioSec;
      // Apply keyframed channels.
      if ('gain' in channels) audio.volume = Math.max(0, Math.min(1, num(channels.gain, 1)));
      if ('rate' in channels) audio.playbackRate = Math.max(0.0625, num(channels.rate, 1));
      if (audioSec < 0) {
        // Before this audio's start: keep it paused at 0.
        if (!audio.paused) audio.pause();
        return;
      }
      // Seek to follow the transport (only when it drifts, to avoid stutter).
      if (Math.abs(audio.currentTime - audioSec) > 0.15 && isFinite(audioSec)) {
        try { audio.currentTime = audioSec; } catch (_e) {}
      }
      // Play while live and the page is actually advancing (a scrub sets
      // progress without playing; a real transport advances continuously).
      if (audio.paused && audioSec < (audio.duration || Infinity)) {
        const pr = audio.play();
        if (pr && pr.catch) pr.catch(() => {}); // autoplay policy: ignore rejection
      }
    });

    %yield true -> $ready;
  }

  %exports {
    $ready: bool
  }
}
