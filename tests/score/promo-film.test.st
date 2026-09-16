/// Behavior gate: the PROMO FILM (demos/promo) moves, scene by scene, and its
/// first frame is RIGHT (BUG-382 / FEAT-175).
///
/// `spacetime render` captures frame 0 the instant the virtual clock is
/// installed, so the FOUC initial-state CSS is not a nicety for this page — it
/// IS frame 0 of the film. BUG-382 flattened every nested-scope keyframe's
/// first value onto the ROOT (`.orb { scale: 0.15 -> 1 }` became
/// `transform: scale(0.15)` on `.s1`), so the whole first scene rendered at
/// 15% scale for the entire film. The runtime never rewrites a root transform
/// it does not own, so the page stayed wrong. Test 1 pins that shape.
///
/// The demo's S1 and S6 blocks are mounted verbatim (the same discipline as
/// score-launch-film.test.st: a committed gate cannot depend on a served
/// page). The score is shortened to the two scenes under test so a fixed
/// `@wait` lands inside a known window.
///
/// NB `--cdp` (real Chromium): timing + paint-adjacent; `--headless` refuses.

@import "stdlib/testing/test"

.stage {
  position: relative; width: 100vw; height: 100vh; overflow: hidden;
  @score &.time(8s) as &film {
    &s1 at 0s for 3.6s;
    &s6 at 3s for 4.4s;
  }
}

.scene { position: absolute; inset: 0; opacity: 0; }
.cam { position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; }

// --- demos/promo S1, verbatim ------------------------------------------------
.s1 .orb {
  position: absolute; width: 520px; height: 520px; border-radius: 50%;
  background-image: radial-gradient(circle, #fff1d6 0%, rgba(255, 200, 120, 0.85) 8%, rgba(255, 180, 84, 0.35) 28%, rgba(255, 180, 84, 0) 70%);
}
.s1 .t1 { position: relative; z-index: 2; opacity: 0; font-size: 104px; }
.s1 {
  @on &.clip {
    opacity: 0 -> 1 -> 1 -> 1 -> 1 -> 0;
    .orb {
      scale: 0.15 -> 0.65 -> 1.9 -> 1.05 -> 1.1 -> 1.0;
      opacity: 0 -> 0.85 -> 1 -> 0.9 -> 0.9 -> 0;
      easing: --ease-out-cubic;
    }
    .t1 {
      opacity: 0 -> 0 -> 1 -> 1 -> 1 -> 0;
      blur: 30px -> 24px -> 0px -> 0px -> 0px -> 14px;
      letter-spacing: 0.32em -> 0.30em -> 0.02em -> 0.01em -> 0.01em -> 0.05em;
      easing: --ease-out-expo;
    }
  }
}

// --- demos/promo S6 burst, verbatim (the CSS-trig particle system) ----------
.s6 .burst { position: absolute; left: 50%; top: 50%; width: 0; height: 0; }
.s6 .burst i {
  position: absolute; display: block; width: 6px; height: 6px; border-radius: 50%; background: #ffe6c2;
  left: calc((mod(sibling-index() * 53, 100) - 50) * 12px);
  top: calc((mod(sibling-index() * 31, 100) - 50) * 3.2px);
  translate:
    calc(cos(sibling-index() * 137.5deg) * (200px + mod(sibling-index() * 7, 100) * 6px) * (1 - exp(-2.2 * var(--tau, 0))) + (mod(sibling-index() * 53, 100) - 50) * 12px * var(--tau, 0) * 0.9)
    calc(sin(sibling-index() * 137.5deg) * (140px + mod(sibling-index() * 11, 100) * 5px) * (1 - exp(-2.2 * var(--tau, 0))) - 120px * var(--tau, 0) + 620px * var(--tau, 0) * var(--tau, 0));
  opacity: calc(1 - var(--tau, 0) * var(--tau, 0));
}
.s6 {
  @on &.clip {
    opacity: 0 -> 1 -> 1 -> 1 -> 1 -> 1 -> 0;
    .burst { --tau: 0 -> 1; range: 0.045 to 0.80; }
  }
}

// =============================================================================
// 1. FRAME 0 IS RIGHT. Before the clock moves, the ROOT scene must carry NO
//    transform/filter (its keyframes are opacity only), while the ORB carries
//    scale(0.15) and the TITLE blur(30px). BUG-382's flattening put all three
//    on `.s1` — this is the negative that would have caught it at build time.
// =============================================================================

@test "promo: frame 0 puts each child's initial value on the child, not the scene" needs timing {
  @mount {
    <div class="stage">
      <div class="scene s1"><div class="cam"><div class="orb"></div><h1 class="t1">CSS describes space.</h1></div></div>
      <div class="scene s6"><div class="cam"><div class="burst"><i></i><i></i><i></i><i></i><i></i><i></i></div></div></div>
    </div>
  }

  @wait 60ms

  @eval (window.__pf1root = getComputedStyle(document.querySelector('.s1')).transform)
  @eval (window.__pf1rootf = getComputedStyle(document.querySelector('.s1')).filter)
  @eval (window.__pf1orb = new DOMMatrixReadOnly(getComputedStyle(document.querySelector('.s1 .orb')).transform).a)
  @eval (window.__pf1t1 = getComputedStyle(document.querySelector('.s1 .t1')).filter)

  @assert (window.__pf1root === 'none')
    "BUG-382: the scene root has only opacity keyframes, so its transform must be `none` — a scale here means a CHILD's initial value was flattened onto the parent (the whole film rendered at 15% scale)"
  @assert (window.__pf1rootf === 'none')
    "BUG-382: the scene root must carry no filter — a blur here is the title's initial value flattened onto the parent"
  @assert (window.__pf1orb < 0.3)
    "the orb must START at scale 0.15 (its own first stop) — 1 means the child's initial state was lost, so frame 0 shows a full-size orb"
  @assert (window.__pf1t1.indexOf('blur(') === 0)
    "the title must START blurred (blur 30px is its first stop) — the focus pull has nothing to pull from otherwise"
}

// =============================================================================
// 2. SCENES OWN THEIR WINDOWS. At 1.5s s1 is mid-window and s6 (3s..7.4s) has
//    not opened: the orb must have grown past its first stop, the burst's
//    --tau must still be 0.
// =============================================================================

@test "promo: the first scene moves while the burst is still" needs timing {
  @mount {
    <div class="stage">
      <div class="scene s1"><div class="cam"><div class="orb"></div><h1 class="t1">CSS describes space.</h1></div></div>
      <div class="scene s6"><div class="cam"><div class="burst"><i></i><i></i><i></i><i></i><i></i><i></i></div></div></div>
    </div>
  }

  @wait 1500ms

  @eval (window.__pf2orb = new DOMMatrixReadOnly(getComputedStyle(document.querySelector('.s1 .orb')).transform).a)
  @eval (window.__pf2s1 = parseFloat(getComputedStyle(document.querySelector('.s1')).opacity))
  @eval (window.__pf2tau = parseFloat(getComputedStyle(document.querySelector('.s6 .burst')).getPropertyValue('--tau')) || 0)

  @assert (window.__pf2orb > 0.5)
    "at 1.5s the orb is ~40% through its window and must have grown well past scale 0.15 — still there means the scene's clip publishes a window nothing consumes"
  @assert (window.__pf2s1 > 0.9)
    "at 1.5s s1 is inside its window's flat middle stops (opacity 1) — anything else means the root's multi-stop chain is not being evaluated"
  @assert (window.__pf2tau === 0)
    "at 1.5s s6's window (3s..7.4s) has not opened, so --tau must be 0 — nonzero means the burst reads the score's raw progress instead of its own window"
}

// =============================================================================
// 3. A CSS-TRIG PARTICLE SYSTEM MOVES ON ONE PROPERTY. Over the burst's
//    window --tau sweeps 0..1; two particles with different sibling-index()
//    must sit at DIFFERENT displacements, both away from rest. This is the
//    whole bet of S6: per-particle geometry is CSS, the score drives one
//    number. Sampled trace, not a fixed wait: after two mounts in this file
//    the warm page's wall clock has drifted, so a fixed `@wait` cannot know
//    where the 8s transport is (the same lesson score-launch-film records).
//    `translate` is the INDIVIDUAL transform property — read it, not
//    `transform`, which stays `none`.
// =============================================================================

@test "promo: sibling-index() particles fan out when --tau moves" needs timing {
  @mount {
    <div class="stage">
      <div class="scene s1"><div class="cam"><div class="orb"></div><h1 class="t1">CSS describes space.</h1></div></div>
      <div class="scene s6"><div class="cam"><div class="burst"><i></i><i></i><i></i><i></i><i></i><i></i></div></div></div>
    </div>
  }

  @eval (window.__pf3xy = function (el) { var t = getComputedStyle(el).translate; var p = t === 'none' ? [0, 0] : t.split(' ').map(parseFloat); return { x: p.at(0) || 0, y: p.at(1) || 0 }; })
  @eval (window.__pf3s = [], window.__pf3t = setInterval(function () {
    var b = document.querySelector('.s6 .burst');
    var i = document.querySelectorAll('.s6 .burst i');
    var pa = window.__pf3xy(i.item(1));
    var pb = window.__pf3xy(i.item(4));
    window.__pf3s.push({ tau: parseFloat(getComputedStyle(b).getPropertyValue('--tau')) || 0, ax: pa.x, ay: pa.y, bx: pb.x, by: pb.y });
  }, 100))

  @wait 7000ms

  @eval (clearInterval(window.__pf3t))
  @eval (window.__pf3mid = window.__pf3s.filter(function (s) { return s.tau > 0.2 && s.tau < 0.9; }))
  @eval (window.__pf3moved = window.__pf3mid.filter(function (s) { return Math.abs(s.ax) + Math.abs(s.ay) > 20; }).length)
  @eval (window.__pf3spread = window.__pf3mid.filter(function (s) { return Math.abs(s.ax - s.bx) > 20 || Math.abs(s.ay - s.by) > 20; }).length)

  @assert (window.__pf3mid.length > 0)
    "no sample over 7s ever saw --tau strictly inside (0,1) — the custom-property keyframe never applied, or the burst's range clamps it to an endpoint"
  @assert (window.__pf3moved > 0)
    "particle 2 never LEFT its rest position while --tau was mid-sweep — a zero translate means the browser rejected the sibling-index()/trig calc, so the whole particle system is a static dot cloud"
  @assert (window.__pf3spread > 0)
    "particles 2 and 5 never fanned out to DIFFERENT places — identical displacement means sibling-index() resolved to one value for every particle"
}
