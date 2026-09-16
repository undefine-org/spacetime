/// Behavior gate: the LAUNCH DEMO's film + catalog surfaces move (FEAT-171)
///
/// Split from score-launch.test.st: the CDP runner gives each FILE a 30s
/// budget, and this demo's cycles are long (the film loops at 12s), so the
/// film/transition/color gates live here while the reel/rate gates stay in
/// score-launch.test.st. Same discipline: the demo's fences are mounted
/// verbatim (see that file's header for why @mount and not @drive), and
/// long-window claims SAMPLE a trace instead of trusting a fixed @wait —
/// mount + warm-Chromium overhead lands anywhere in a loop's cycle, so a
/// fixed wait cannot know the driver's startTime.
///
/// NB `--cdp` (real Chromium): timing-dependent claims refuse on --headless.

@import "stdlib/testing/test"

// --- The demo's section-0 film, verbatim ------------------------------------
// The renderable hero: a 12s loop, four scenes. Scene consumers animate
// opacity plus, for sc2, the CUSTOM PROPERTY --iris (clip-path reads it) —
// the mechanism every var()-driven wipe in the catalog is built on.

.stage {
  @score &.loop(12s) as &film {
    &sc1 at 0s for 3s;
    &sc2 at 3s for 3s;
    &sc3 at 6s for 3s;
    &sc4 at 9s for 3s;
  }
}

.scene {
  position: absolute;
  inset: 0;
  opacity: 0;
}

.sc1 {
  @on &.clip {
    opacity: 0 -> 1 -> 1 -> 0;
    .fh {
      blur: 14px -> 0 -> 0 -> 8px;
      letter-spacing: 0.3em -> 0.04em;
      translate-y: 18px -> 0;
      easing: --ease-out-expo;
    }
    .fk { opacity: 0 -> 1; translate-y: 10px -> 0; }
  }
}

.sc2 {
  clip-path: circle(calc(var(--iris, 0) * 1%) at 50% 50%);
  @on &.clip {
    --iris: 0 -> 75 -> 75 -> 0;
    opacity: 0 -> 1 -> 1 -> 0;
    .fh { translate-y: 26px -> 0; easing: --ease-out-expo; }
  }
}

.sc3 {
  @on &.clip {
    opacity: 0 -> 1 -> 1 -> 1 -> 0;
    .fg {
      translate-x: 0 -> 12px -> -9px -> 5px -> 0 -> 0;
      skew-x: 0deg -> 7deg -> -5deg -> 2deg -> 0deg -> 0deg;
      hue-rotate: 0deg -> 110deg -> -60deg -> 25deg -> 0deg -> 0deg;
    }
  }
}

.sc4 {
  @on &.clip {
    opacity: 0 -> 1 -> 1 -> 1;
    .fs { scale: 1.7 -> 1; easing: --ease-out-expo; --pos: -120% -> 230%; }
    .fk { opacity: 0 -> 1; range: 0.4 to 1; }
  }
}

// --- The demo's section-2 cover transition, verbatim -------------------------

.cov { @score &.loop(4s) { &cova at 0s for 2s; &covb at 2s for 2s; } }
.cova { @on &.clip { opacity: 0 -> 1 -> 1 -> 1; } }
.covb { @on &.clip { opacity: 1; translate-x: 100% -> 0; easing: --ease-out-expo; } }

// --- The demo's section-3 tint chip, verbatim --------------------------------

.luxrow {
  @score &.loop(6s) {
    &tint at 0s for 6s;
  }
}

.tint { @on &.clip { background: #0e131b -> #134e4a -> #5eead4 -> #134e4a -> #0e131b;
                     color: #5eead4 -> #e8eef7 -> #06251f -> #e8eef7 -> #5eead4; } }

// =============================================================================
// 6. THE FILM MOVES SCENE BY SCENE. Negative-first: at 500ms sc1 (0s..3s) is
//    mid-window and must have moved; sc3 (6s..9s) and sc2's --iris (3s..6s)
//    must be UNTOUCHED. A nonzero sc3 here means the film's clips read the
//    loop's raw progress instead of their own windows.
// =============================================================================

@test "score-launch: the film's first scene moves while later scenes are still" needs timing {
  @mount {
    <div class="stage">
      <div class="scene sc1"><span class="fk">SIP-001</span><h1 class="fh">One score</h1></div>
      <div class="scene sc2"><h1 class="fh">Every clock</h1></div>
      <div class="scene sc3"><h1 class="fh fg">No JavaScript</h1></div>
      <div class="scene sc4"><h1 class="fh fs">Spacetime</h1><span class="fk">render</span></div>
    </div>
  }

  @wait 500ms

  @eval (window.__sl6a = parseFloat(getComputedStyle(document.querySelector('.sc1')).opacity))
  @eval (window.__sl6b = parseFloat(getComputedStyle(document.querySelector('.sc3')).opacity))
  @eval (window.__sl6c = parseFloat(getComputedStyle(document.querySelector('.sc2')).getPropertyValue('--iris')) || 0)

  @assert (window.__sl6a > 0)
    "at 500ms sc1 is ~1/6 through its 0s..3s window and must have moved — 0 means the film's score publishes windows nothing consumes"
  @assert (window.__sl6b === 0)
    "at 500ms sc3's window (6s..9s) has not opened — nonzero means the film's clips are driven by the loop's raw progress, not their windows"
  @assert (window.__sl6c === 0)
    "at 500ms sc2's window (3s..6s) has not opened, so --iris must still be 0 — nonzero means the custom-property channel reads the wrong progress"
}

// =============================================================================
// 7. THE IRIS OPENS. The custom-property keyframe is the catalog's wipe
//    mechanism: at 3.6s sc2 is 1/5 through its window, so --iris must be
//    mid-sweep — and sc1, whose window closed at 3s, must rest at its END
//    value (0), proving windows clamp rather than wrap.
// =============================================================================

@test "score-launch: the iris wipe's custom property sweeps inside its window" needs timing {
  @mount {
    <div class="stage">
      <div class="scene sc1"><span class="fk">SIP-001</span><h1 class="fh">One score</h1></div>
      <div class="scene sc2"><h1 class="fh">Every clock</h1></div>
      <div class="scene sc3"><h1 class="fh fg">No JavaScript</h1></div>
      <div class="scene sc4"><h1 class="fh fs">Spacetime</h1><span class="fk">render</span></div>
    </div>
  }

  // Sampled trace — see test 4 for why fixed waits cannot carry this claim.
  @eval (window.__sl7s = [], window.__sl7t = setInterval(function () {
    window.__sl7s.push({
      sc1: parseFloat(getComputedStyle(document.querySelector('.sc1')).opacity),
      iris: parseFloat(getComputedStyle(document.querySelector('.sc2')).getPropertyValue('--iris')) || 0
    });
  }, 200))

  @wait 13000ms

  @eval (clearInterval(window.__sl7t))
  @eval (window.__sl7moved = window.__sl7s.filter(function (s) { return s.sc1 > 0.5; }).length)
  @eval (window.__sl7iris = window.__sl7s.filter(function (s) { return s.iris > 30; }).length)
  @eval (window.__sl7rest = window.__sl7s.filter(function (s) { return s.iris > 30 && s.sc1 < 0.1; }).length)

  @assert (window.__sl7moved > 0)
    "sc1 never reached mid-window opacity — the film's first scene is not being driven"
  @assert (window.__sl7iris > 0)
    "--iris never swept open in ANY sample over a full 12s cycle — custom-property keyframes are emitted but never applied"
  @assert (window.__sl7rest > 0)
    "whenever the iris was open, sc1 was still visible — sc1's window (0s..3s) never closes, or sc2's (3s..6s) opens early: the scenes do not own their windows"
}

// =============================================================================
// 8. THE COVER TRANSITION. Negative: covb's window opens at 2s, so at 500ms
//    its clip progress must be 0. Positive: at 2.4s it must be mid-slide.
// =============================================================================

@test "score-launch: a cover panel is still before its window, slides inside it" needs timing {
  @mount {
    <div class="txs cov">
      <div class="panel pa cova">Cover</div>
      <div class="panel pb covb">Push</div>
    </div>
  }

  @wait 500ms

  // tx of the computed matrix vs the panel's own width: translate-x starts
  // at 100% (fully offstage right). Still there = window not yet read.
  @eval (window.__sl8el = document.querySelector('.covb'))
  @eval (window.__sl8m = new DOMMatrixReadOnly(getComputedStyle(window.__sl8el).transform))
  @eval (window.__sl8w = window.__sl8el.offsetWidth || 1)

  @assert (window.__sl8m.m41 > 0.9 * window.__sl8w)
    "at 500ms covb's window (2s..4s) has not opened, so it must still be offstage at translate-x 100% — having slid in means the transition reads the loop's raw clock instead of its window"
}

@test "score-launch: the cover panel slides when its window opens" needs timing {
  @mount {
    <div class="txs cov">
      <div class="panel pa cova">Cover</div>
      <div class="panel pb covb">Push</div>
    </div>
  }

  @wait 2400ms

  @eval (window.__sl8el = document.querySelector('.covb'))
  @eval (window.__sl8m = new DOMMatrixReadOnly(getComputedStyle(window.__sl8el).transform))
  @eval (window.__sl8w = window.__sl8el.offsetWidth || 1)

  @assert (window.__sl8m.m41 < 0.95 * window.__sl8w)
    "at 2400ms covb is 1/5 through its 2s..4s window with an expo-out ease, so it must have slid most of the way in — still at translate-x ≈100% means the emitted keyframes never move the panel (a consumer watching a signal nobody publishes)"
}

// =============================================================================
// 9. COLOR INTERPOLATES. tint's background keyframes #0e131b -> #134e4a ->
//    #5eead4 -> ... over a 6s loop; at 1.5s (progress 0.25, linear) it sits
//    at the second stop — anything still at the FIRST stop means color
//    keyframes are emitted but never interpolated.
// =============================================================================

@test "score-launch: a color keyframe leaves its first stop" needs timing {
  @mount {
    <div class="luxrow">
      <div class="lux tint">Tint</div>
    </div>
  }

  @wait 1500ms

  @eval (window.__sl9 = getComputedStyle(document.querySelector('.tint')).backgroundColor)
  @assert (window.__sl9 !== 'rgb(14, 19, 27)')
    "at 1500ms tint is 1/4 through its color sweep and must have left #0e131b — still being there means color keyframes never interpolate"
}

// --- The demo's section-1 kinetic stagger, verbatim --------------------------

.kine {
  @score &.loop(6s) {
    &kine at 0s for 6s;
  }

  @on &.clip {
    .wk {
      opacity: 0 -> 1 -> 1 -> 0;
      translate-y: 0.6em -> 0;
      blur: 6px -> 0;
      easing: --ease-out-expo;
      stagger: 120ms;
      staggerFrom: first;
    }
  }
}

.wk {
  display: inline-block;
  opacity: 0;
}

// =============================================================================
// 10. STAGGER SPREADS ONE WINDOW. The five words share the container's clip;
//     stagger spreads their local starts, so there must be a moment where the
//     FIRST word is up while the LAST is still down. Without it, stagger is a
//     control prop that parses, emits, and does nothing (the delay: 0 class).
// =============================================================================

@test "score-launch: stagger spreads the words across the window" needs timing {
  @mount {
    <div class="kine">
      <span class="wk">type</span>
      <span class="wk">moves</span>
      <span class="wk">because</span>
      <span class="wk">data</span>
      <span class="wk">does</span>
    </div>
  }

  @eval (window.__sl10s = [], window.__sl10t = setInterval(function () {
    var w = document.querySelectorAll('.wk');
    window.__sl10s.push({
      first: parseFloat(getComputedStyle(w[0]).opacity),
      last: parseFloat(getComputedStyle(w[4]).opacity)
    });
  }, 100))

  @wait 6500ms

  @eval (clearInterval(window.__sl10t))
  @eval (window.__sl10spread = window.__sl10s.filter(function (s) { return s.first > 0.3 && s.last < 0.05; }).length)
  @eval (window.__sl10last = window.__sl10s.filter(function (s) { return s.last > 0.3; }).length)

  @assert (window.__sl10spread > 0)
    "no sample ever saw the first word up while the last was still down — the words move as one block, which means the scope's stagger delay never reaches calcStagger"
  @assert (window.__sl10last > 0)
    "the last word never came up at all — stagger is not spreading the window, it is eating the tail of it"
}

