/// Behavior gate: the EFFECT GALLERY's snippets move (FEAT-172).
///
/// demos/effect-gallery/ redoes HyperFrames registry blocks as literate
/// snippets — one `@score` placement + one keyframe body each. These probes
/// mirror four representative fences VERBATIM (karaoke windows, pixelate's
/// container-level stagger, the ticker's linear wrap, morph's opposed
/// phases) — if a snippet's fence changes, update its mirror here.
///
/// Two compiler bugs this file would have caught on arrival:
/// - consumers written as DESCENDANT selectors (`.g-kara .w1`) fell back to
///   synthetic `__drive_clip_N` names nothing publishes (selector_element
///   rejected the space) — every gallery consumer was silently dead;
/// - multi-file literate merges collided on `data-lit-seg="0"` and every
///   prose section rendered the last import's prose.
///
/// Sampler discipline as in score-launch-film.test.st: mount lands anywhere
/// in a loop's cycle, so claims sample traces instead of trusting @wait.
/// NB `--cdp` (real Chromium): timing claims refuse on --headless.

@import "stdlib/testing/test"

// --- Mirrored fences (see demos/effect-gallery/snippets/*.st.md) ------------
// Selectors are single-class here (the test-file grammar's rule shape); the
// snippets scope the same rules under .g-<name>. The motion program — score,
// windows, keyframes — is verbatim.

.gk {
  @score &.loop(3.2s) {
    &w1 at 0.0s for 0.8s; &w2 at 0.8s for 0.8s;
    &w3 at 1.6s for 0.8s; &w4 at 2.4s for 0.8s;
  }
}
.w1 { @on &.clip { opacity: 0.25 -> 1 -> 0.25; } }
.w2 { @on &.clip { opacity: 0.25 -> 1 -> 0.25; } }
.w3 { @on &.clip { opacity: 0.25 -> 1 -> 0.25; } }
.w4 { @on &.clip { opacity: 0.25 -> 1 -> 0.25; } }

.grid {
  @score &.loop(3.6s) { &grid at 0s for 3.6s; }
  @on &.clip {
    .cell { stagger: 300ms; opacity: 0 -> 1; }
  }
}

.gt { @score &.loop(8s) { &strip at 0s for 8s; } }
.strip { @on &.clip { translate: 0% 0% -> -50% 0%; easing: --linear; } }

.gm { @score &.loop(4s) { &a at 0s for 4s; &b at 0s for 4s; } }
.a { @on &.clip { opacity: 1 -> 0 -> 0 -> 1; } }
.b { @on &.clip { opacity: 0 -> 1 -> 1 -> 0; } }

// =============================================================================
// 1. KARAOKE. Four windows, four words — over a full loop EVERY word must
//    reach full opacity, and no two words may share the highlight.
// =============================================================================

@test "gallery: karaoke lights every word, one at a time" needs timing {
  @mount {
    <section class="gk">
      <p class="pill"><span class="w1">THE</span><span class="w2">FUTURE</span><span class="w3">IS</span><span class="w4">COMPILED</span></p>
    </section>
  }

  @eval (window.__g1s = [], window.__g1t = setInterval(function () {
    var ws = document.querySelectorAll('.gk .pill span');
    window.__g1s.push({
      w1: parseFloat(getComputedStyle(ws[0]).opacity),
      w2: parseFloat(getComputedStyle(ws[1]).opacity),
      w3: parseFloat(getComputedStyle(ws[2]).opacity),
      w4: parseFloat(getComputedStyle(ws[3]).opacity)
    });
  }, 150))

  @wait 3600ms

  @eval (clearInterval(window.__g1t))
  @eval (window.__g1litw1 = Math.max.apply(null, window.__g1s.map(function (s) { return s.w1; })))
  @eval (window.__g1litw2 = Math.max.apply(null, window.__g1s.map(function (s) { return s.w2; })))
  @eval (window.__g1litw3 = Math.max.apply(null, window.__g1s.map(function (s) { return s.w3; })))
  @eval (window.__g1litw4 = Math.max.apply(null, window.__g1s.map(function (s) { return s.w4; })))
  @eval (window.__g1crowd = window.__g1s.filter(function (s) {
    var n = 0;
    if (s.w1 > 0.8) { n++; }
    if (s.w2 > 0.8) { n++; }
    if (s.w3 > 0.8) { n++; }
    if (s.w4 > 0.8) { n++; }
    return n > 1;
  }).length)

  @assert (window.__g1litw1 > 0.8 && window.__g1litw2 > 0.8 && window.__g1litw3 > 0.8 && window.__g1litw4 > 0.8)
    "some word never rose from the 0.25 baseline in a 3.2s loop — its window is not being driven (dead consumer or wrong clip name)"
  @assert (window.__g1crowd === 0)
    "two words were lit in the same sample — the windows overlap and the karaoke reads as a wave, not a pointer"
}

// =============================================================================
// 2. PIXELATE. The container owns the window; stagger spreads the cells'
//    local starts. A trace must show 0 on, then a PARTIAL count, then all 25.
// =============================================================================

@test "gallery: pixelate spreads 25 cells across the window" needs timing {
  @mount {
    <section class="gp">
      <div class="grid">
        <div class="cell"></div><div class="cell"></div><div class="cell"></div><div class="cell"></div><div class="cell"></div>
        <div class="cell"></div><div class="cell"></div><div class="cell"></div><div class="cell"></div><div class="cell"></div>
        <div class="cell"></div><div class="cell"></div><div class="cell"></div><div class="cell"></div><div class="cell"></div>
        <div class="cell"></div><div class="cell"></div><div class="cell"></div><div class="cell"></div><div class="cell"></div>
        <div class="cell"></div><div class="cell"></div><div class="cell"></div><div class="cell"></div><div class="cell"></div>
      </div>
    </section>
  }

  @eval (window.__g2s = [], window.__g2t = setInterval(function () {
    var cs = document.querySelectorAll('.gp .cell');
    var n = 0;
    for (var i = 0; i < cs.length; i++) {
      if (parseFloat(getComputedStyle(cs[i]).opacity) > 0.5) { n++; }
    }
    window.__g2s.push(n);
  }, 120))

  @wait 4000ms

  @eval (clearInterval(window.__g2t))
  @eval (window.__g2full = window.__g2s.filter(function (n) { return n === 25; }).length)
  @eval (window.__g2partial = window.__g2s.filter(function (n) { return n > 0 && n < 25; }).length)

  @assert (window.__g2full > 0)
    "never saw all 25 cells lit — the nested .cell scope is not consuming the grid's window"
  @assert (window.__g2partial > 0)
    "cells flipped all-at-once — stagger: 45ms is not spreading their local starts"
}

// =============================================================================
// 3. TICKER. --linear, 0% -> -50% over the loop: the strip's translateX must
//    travel steadily left (a real range, no easing stall at the wrap).
// =============================================================================

@test "gallery: the ticker travels left at constant speed" needs timing {
  @mount {
    <section class="gt">
      <div class="rail"><p class="strip">SPACETIME COMPILES TO A FILM ✦ SCORES ARE DATA ✦ SPACETIME COMPILES TO A FILM ✦ SCORES ARE DATA ✦ </p></div>
    </section>
  }

  @eval (window.__g3s = [], window.__g3first = -99999, window.__g3last = 0, window.__g3t = setInterval(function () {
    var m = new DOMMatrix(getComputedStyle(document.querySelector('.gt .strip')).transform);
    if (window.__g3first < -9999) { window.__g3first = m.e; }
    window.__g3last = m.e;
    window.__g3s.push(m.e);
  }, 100))

  @wait 2600ms

  @eval (clearInterval(window.__g3t))
  @eval (window.__g3range = Math.max.apply(null, window.__g3s) - Math.min.apply(null, window.__g3s))
  @eval (window.__g3net = window.__g3first - window.__g3last)

  @assert (window.__g3range > 80)
    "strip translateX barely moved in 2.6s — the linear clip is not driving the strip"
  @assert (window.__g3net > 40)
    "no net leftward travel over the window — the ticker is easing in place or palindroming"
}

// =============================================================================
// 4. MORPH. Two placements, opposed phases: a owns the front half, b the
//    back, and they trade.
// =============================================================================

@test "gallery: morph trades two words through opposed phases" needs timing {
  @mount {
    <section class="gm">
      <p class="stack"><span class="a">chaos</span><span class="b">clarity</span></p>
    </section>
  }

  @eval (window.__g4s = [], window.__g4t = setInterval(function () {
    window.__g4s.push({
      a: parseFloat(getComputedStyle(document.querySelector('.gm .a')).opacity),
      b: parseFloat(getComputedStyle(document.querySelector('.gm .b')).opacity)
    });
  }, 150))

  @wait 4400ms

  @eval (clearInterval(window.__g4t))
  @eval (window.__g4a = window.__g4s.filter(function (s) { return s.a > 0.8 && s.b < 0.2; }).length)
  @eval (window.__g4b = window.__g4s.filter(function (s) { return s.b > 0.8 && s.a < 0.2; }).length)

  @assert (window.__g4a > 0)
    "'chaos' never owned the stage — the &a window is not driving its consumer"
  @assert (window.__g4b > 0)
    "'clarity' never owned the stage — the &b window is not driving its consumer"
}
