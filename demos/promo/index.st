// demos/promo — the Spacetime promo, authored in Spacetime.
//
// Thirty seconds, seven scenes, ONE `@score &.time(30s)`. No JavaScript
// anywhere in this file: every visual is a clip window (`at … for …`) whose
// consumer is an `@on &.clip` keyframe body. Where a studio pipeline reaches
// for a particle shader, this file animates ONE custom property (`--t`) and
// lets CSS do the geometry per element with `sibling-index()` + trig.
//
//   cargo run -- serve demos/promo/                                       # the page
//   cargo run --features cdp -- render demos/promo/ --out scratch/promo-st/video.mp4   # the film
//
// The sibling "studio" cut (scratch/promo-studio/) is the SAME storyboard
// built with Three.js + WebGL + a hand-written CDP capturer. CRITIQUE.md
// compares the two.

// ---------------------------------------------------------------------------
// Stage skeleton. The renderer captures the viewport from the top, so the
// stage is the first (and only) thing in the document.
// ---------------------------------------------------------------------------

<div class="stage">
  <div class="scene s1">
    <div class="cam">
      <div class="stars"><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i></div>
      <div class="dust"><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i></div>
      <div class="orb"></div>
      <h1 class="t1 dsp">CSS describes space.</h1>
    </div>
  </div>

  <div class="scene s2">
    <div class="cam">
      <div class="stars"><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i></div>
      <div class="world">
        <div class="floor"></div>
      </div>
      <div class="ripple"></div>
      <div class="axis"><div class="axis-trail"></div><div class="axis-line"></div></div>
      <h1 class="t2 dsp">Spacetime adds time.</h1>
      <p class="k2 kicker">Dimensions, plural</p>
    </div>
  </div>

  <div class="scene s3">
    <div class="cam">
      <div class="split">
        <div class="win code">
          <div class="bar"><span class="fn">site.st</span><span class="dots"><i></i><i></i><i></i></span></div>
          <pre class="src"><span class="ln"><b class="sel">.card</b> <b class="pn">{</b></span><span class="ln">  <b class="dir">@on</b> <b class="ref">&amp;.visible</b> <b class="pn">{</b></span><span class="ln">    <b class="pr">opacity</b><b class="pn">:</b> <b class="nu">0</b> <b class="ar">-&gt;</b> <b class="nu">1</b><b class="pn">;</b></span><span class="ln">    <b class="pr">translate-y</b><b class="pn">:</b> <b class="nu">40px</b> <b class="ar">-&gt;</b> <b class="nu">0</b><b class="pn">;</b></span><span class="ln">    <b class="pr">easing</b><b class="pn">:</b> <b class="ref">--ease-out-expo</b><b class="pn">;</b></span><span class="ln">  <b class="pn">}</b></span><span class="ln"><b class="pn">}</b></span></pre>
          <i class="caret"></i>
        </div>
        <div class="win live">
          <div class="bar"><span class="fn">browser</span><span class="drv">&amp;.visible</span></div>
          <div class="card">
            <div class="img"><i class="mug"></i></div>
            <div class="row"><span class="nm">Ceramic Mug</span><span class="price">$24.99</span></div>
            <div class="sub">hand-thrown · 12 oz</div>
          </div>
          <div class="ph"><i></i></div>
        </div>
      </div>
      <h1 class="t3 dsp">Declare it once.</h1>
    </div>
  </div>

  <div class="scene s4">
    <div class="cam">
      <div class="panels">
        <div class="panel p1"><div class="bar"><span class="drv">&amp;.time(1.2s)</span><span class="kind">Transport</span><span class="one">@score</span></div><div class="card"><div class="img"><i class="mug"></i></div><div class="row"><span class="nm">Ceramic Mug</span><span class="price">$24.99</span></div><div class="sub">hand-thrown · 12 oz</div></div><div class="ph"><i></i></div></div>
        <div class="panel p2"><div class="bar"><span class="drv">&amp;.scroll(cover)</span><span class="kind">Progress</span><span class="one">@score</span></div><div class="card"><div class="img"><i class="mug"></i></div><div class="row"><span class="nm">Ceramic Mug</span><span class="price">$24.99</span></div><div class="sub">hand-thrown · 12 oz</div></div><div class="ph"><i></i></div></div>
        <div class="panel p3"><div class="bar"><span class="drv">&amp;.click</span><span class="kind">Event</span><span class="one">@score</span></div><div class="card"><div class="img"><i class="mug"></i></div><div class="row"><span class="nm">Ceramic Mug</span><span class="price">$24.99</span></div><div class="sub">hand-thrown · 12 oz</div></div><div class="ph"><i></i></div></div>
        <div class="panel p4"><div class="bar"><span class="drv">&amp;voice.playback</span><span class="kind">Transport</span><span class="one">@score</span></div><div class="card"><div class="img"><i class="mug"></i></div><div class="row"><span class="nm">Ceramic Mug</span><span class="price">$24.99</span></div><div class="sub">hand-thrown · 12 oz</div></div><div class="ph"><i></i></div><div class="wave"><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i></div></div>
      </div>
      <h1 class="t4 dsp">Every clock.</h1>
    </div>
  </div>

  <div class="scene s5">
    <div class="cam">
      <div class="stripg"><div class="strip">
        <div class="fr"><div class="card"><div class="img"><i class="mug"></i></div><div class="row"><span class="nm">Ceramic Mug</span><span class="price">$24.99</span></div></div></div>
        <div class="fr"><div class="card"><div class="img"><i class="mug"></i></div><div class="row"><span class="nm">Ceramic Mug</span><span class="price">$24.99</span></div></div></div>
        <div class="fr"><div class="card"><div class="img"><i class="mug"></i></div><div class="row"><span class="nm">Ceramic Mug</span><span class="price">$24.99</span></div></div></div>
        <div class="fr"><div class="card"><div class="img"><i class="mug"></i></div><div class="row"><span class="nm">Ceramic Mug</span><span class="price">$24.99</span></div></div></div>
        <div class="fr"><div class="card"><div class="img"><i class="mug"></i></div><div class="row"><span class="nm">Ceramic Mug</span><span class="price">$24.99</span></div></div></div>
        <div class="fr"><div class="card"><div class="img"><i class="mug"></i></div><div class="row"><span class="nm">Ceramic Mug</span><span class="price">$24.99</span></div></div></div>
        <div class="fr"><div class="card"><div class="img"><i class="mug"></i></div><div class="row"><span class="nm">Ceramic Mug</span><span class="price">$24.99</span></div></div></div>
        <div class="fr"><div class="card"><div class="img"><i class="mug"></i></div><div class="row"><span class="nm">Ceramic Mug</span><span class="price">$24.99</span></div></div></div>
        <div class="fr"><div class="card"><div class="img"><i class="mug"></i></div><div class="row"><span class="nm">Ceramic Mug</span><span class="price">$24.99</span></div></div></div>
        <div class="fr"><div class="card"><div class="img"><i class="mug"></i></div><div class="row"><span class="nm">Ceramic Mug</span><span class="price">$24.99</span></div></div></div>
        <div class="fr"><div class="card"><div class="img"><i class="mug"></i></div><div class="row"><span class="nm">Ceramic Mug</span><span class="price">$24.99</span></div></div></div>
        <div class="fr"><div class="card"><div class="img"><i class="mug"></i></div><div class="row"><span class="nm">Ceramic Mug</span><span class="price">$24.99</span></div></div></div>
        <div class="fr"><div class="card"><div class="img"><i class="mug"></i></div><div class="row"><span class="nm">Ceramic Mug</span><span class="price">$24.99</span></div></div></div>
        <div class="fr"><div class="card"><div class="img"><i class="mug"></i></div><div class="row"><span class="nm">Ceramic Mug</span><span class="price">$24.99</span></div></div></div>
        <div class="fr"><div class="card"><div class="img"><i class="mug"></i></div><div class="row"><span class="nm">Ceramic Mug</span><span class="price">$24.99</span></div></div></div>
        <div class="fr"><div class="card"><div class="img"><i class="mug"></i></div><div class="row"><span class="nm">Ceramic Mug</span><span class="price">$24.99</span></div></div></div>
      </div></div>
      <div class="ctr"><span class="lbl">frame</span><span class="num"></span><span class="of">/ 1800</span><span class="tm"><span class="tnum"></span></span><span class="lbl2">virtual clock · deterministic</span></div>
      <div class="cmd"><span class="line">$ spacetime render demos/promo/ --out film.mp4</span><i class="caret"></i><span class="ok">✓ 1800 frames · 30.000 s · h264 · bit-identical on every run</span></div>
      <h1 class="t5 dsp">Render it.</h1>
    </div>
  </div>

  <div class="scene s6">
    <div class="cam">
      <div class="tagwrap">
        <span class="tagw mono">&lt;script&gt;</span>
        <span class="tagr mono">&lt;script&gt;</span>
        <span class="tagb mono">&lt;script&gt;</span>
        <div class="burst"><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i></div>
      </div>
      <h1 class="t6 dsp">No JavaScript.</h1>
      <p class="k6 sub6">style · motion · state · data — one declarative language</p>
    </div>
  </div>

  <div class="scene s7">
    <div class="cam">
      <h1 class="mark dsp">SPACETIME</h1>
      <div class="rule"></div>
      <p class="tag7">If CSS describes space, Spacetime adds time.</p>
      <p class="k7 kicker">Declarative · Deterministic · One file</p>
    </div>
  </div>

  <div class="grain"></div>
  <div class="vignette"></div>
  <div class="hit h1"></div><div class="hit h2"></div><div class="hit h3"></div><div class="hit h4"></div><div class="hit h5"></div><div class="hit h6"></div>
  <div class="fadein"></div>
  <div class="fadeout"></div>
</div>

// ---------------------------------------------------------------------------
// The score. Seven scenes on one 30s transport; overlays are clips too.
// Overlapping windows are the cross-dissolves.
// ---------------------------------------------------------------------------

.stage {
  position: relative;
  width: 100vw;
  height: 100vh;
  overflow: hidden;
  background: #06070a;
  color: #e6ecf5;
  font-family: "Space Grotesk", system-ui, sans-serif;

  @score &.time(30s) as &film {
    &s1 at 0s for 3.6s;
    &s2 at 3.2s for 4.6s;
    &s3 at 7.4s for 5.4s;
    &s4 at 12.4s for 6.2s;
    &s5 at 18s for 5s;
    &s6 at 22.8s for 4.4s;
    &s7 at 27s for 3s;
    &h1 at 2.08s for 0.5s;
    &h2 at 6.28s for 0.5s;
    &h3 at 11.48s for 0.5s;
    &h4 at 17.88s for 0.5s;
    &h5 at 22.88s for 0.5s;
    &h6 at 26.88s for 0.5s;
    &grain at 0s for 30s;
    &fadein at 0s for 0.7s;
    &fadeout at 29s for 1s;
  }
}

.scene { position: absolute; inset: 0; opacity: 0; }
.cam { position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; }
.dsp { font-family: Syne, "Space Grotesk", sans-serif; font-weight: 800; letter-spacing: 0.01em; line-height: 1; margin: 0; font-size: 104px; text-align: center; white-space: nowrap; }
.mono { font-family: "JetBrains Mono", ui-monospace, monospace; }
.kicker { font: 600 26px/1 "Space Grotesk", sans-serif; letter-spacing: 0.36em; text-transform: uppercase; color: #8a94a6; margin: 0; }

// ---- overlays -------------------------------------------------------------
.hit { position: absolute; inset: 0; background: #fff8ec; opacity: 0; pointer-events: none; }
.grain {
  position: absolute; inset: -200px; opacity: 0.055; pointer-events: none; mix-blend-mode: screen;
  background-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='300' height='300'><filter id='n'><feTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='2' stitchTiles='stitch'/><feColorMatrix values='0 0 0 0 1 0 0 0 0 1 0 0 0 0 1 0 0 0 1 0'/></filter><rect width='100%' height='100%' filter='url(%23n)'/></svg>");
  background-position: calc(var(--g, 0) * 71px) calc(var(--g, 0) * 37px);
}
.vignette { position: absolute; inset: 0; pointer-events: none; background-image: radial-gradient(ellipse 78% 70% at 50% 50%, transparent 55%, rgba(0, 0, 0, 0.72) 100%); }
.fadein { position: absolute; inset: 0; background: #000; opacity: 1; pointer-events: none; }
.fadeout { position: absolute; inset: 0; background: #000; opacity: 0; pointer-events: none; }

// The hits are audio-locked (2.2 · 6.4 · 11.6 · 18.0 · 23.0 · 27.0 s): one
// short clip each. A clip RESTS at its first stop outside its window, so the
// spike starts at 0; five even stops over 0.5s put the peak 0.125s in — the
// clip is placed 0.12s before the hit so the peak lands on the sound.
@form motion --flash { opacity: 0 -> 0.85 -> 0.18 -> 0.04 -> 0; }
.h1 { @on &.clip: --flash; }
.h2 { @on &.clip: --flash; }
.h3 { @on &.clip: --flash; }
.h4 { @on &.clip: --flash; }
.h5 { @on &.clip: --flash; }
.h6 { @on &.clip: --flash; }
.grain { @on &.clip { --g: 0 -> 300; } }
.fadein { @on &.clip { opacity: 1 -> 0; easing: --ease-out-quad; } }
.fadeout { @on &.clip { opacity: 0 -> 1; easing: --ease-in-quad; } }

// ---- shared components ----------------------------------------------------
.stars { position: absolute; inset: 0; }
.stars i {
  position: absolute; display: block; border-radius: 50%; background: #fff;
  left: calc(mod(sibling-index() * 61, 100) * 1%);
  top: calc(mod(sibling-index() * 37, 100) * 1%);
  width: calc(1px + mod(sibling-index(), 3) * 0.8px);
  height: calc(1px + mod(sibling-index(), 3) * 0.8px);
  opacity: calc(0.25 + 0.6 * abs(sin((var(--tw, 0) + sibling-index()) * 1rad)));
}
.dust { position: absolute; inset: 0; }
.dust i {
  position: absolute; display: block; border-radius: 50%; background: #e6ecf5;
  left: calc(mod(sibling-index() * 83, 100) * 1%);
  top: calc(mod(sibling-index() * 29, 100) * 1%);
  width: calc(2px + mod(sibling-index(), 4) * 1.4px);
  height: calc(2px + mod(sibling-index(), 4) * 1.4px);
  opacity: calc(0.12 + 0.28 * mod(sibling-index(), 3));
  filter: blur(calc(mod(sibling-index(), 3) * 0.7px));
  translate: calc(cos(sibling-index() * 1.7rad) * var(--t, 0) * 90px) calc(sin(sibling-index() * 2.3rad) * var(--t, 0) * 60px);
}
.card {
  width: 560px; background: #0f131a; border: 2px solid rgba(255, 255, 255, 0.14); border-radius: 28px; overflow: hidden;
  box-shadow: 0 30px 80px rgba(0, 0, 0, 0.5);
}
.card .img { height: 300px; background-image: linear-gradient(135deg, #1a2230, #0e1a22); display: flex; align-items: center; justify-content: center; }
.mug { position: relative; display: block; width: 150px; height: 96px; border-radius: 50%; background: rgba(255, 180, 84, 0.85); }
.mug::after { content: ""; position: absolute; left: 118px; top: 8px; width: 54px; height: 80px; border: 12px solid rgba(255, 180, 84, 0.85); border-left: 0; border-radius: 0 40px 40px 0; }
.card .row { display: flex; justify-content: space-between; align-items: baseline; padding: 30px 38px 0; font: 600 34px/1 "Space Grotesk", sans-serif; }
.card .price { color: #ffb454; font-weight: 500; }
.card .sub { padding: 14px 38px 34px; color: #8a94a6; font: 400 24px/1 "Space Grotesk", sans-serif; }
.ph { position: relative; height: 6px; background: rgba(255, 255, 255, 0.1); border-radius: 3px; overflow: hidden; }
.ph i { position: absolute; left: 0; top: 0; bottom: 0; width: 100%; background: #ffb454; transform-origin: left; scale: 0 1; display: block; }
.win { position: relative; background: rgba(11, 14, 20, 0.92); border: 2px solid rgba(255, 255, 255, 0.12); border-radius: 28px; overflow: hidden; }
.bar { display: flex; justify-content: space-between; align-items: center; height: 76px; padding: 0 40px; background: rgba(255, 255, 255, 0.05); font: 500 26px/1 "JetBrains Mono", monospace; color: #8a94a6; }
.dots i { display: inline-block; width: 18px; height: 18px; border-radius: 50%; margin-left: 12px; background: #28c840; }
.dots i:first-child { background: #ff5f57; }
.dots i:nth-child(2) { background: #febc2e; }
.drv { color: #8a94a6; }

// ===========================================================================
// S1 — a point of light. Focus pull on the first line.
// ===========================================================================
.s1 .orb {
  position: absolute; width: 520px; height: 520px; border-radius: 50%;
  background-image: radial-gradient(circle, #fff1d6 0%, rgba(255, 200, 120, 0.85) 8%, rgba(255, 180, 84, 0.35) 28%, rgba(255, 180, 84, 0) 70%);
  filter: blur(6px);
}
.s1 .t1 { position: relative; z-index: 2; opacity: 0; }
.s1 {
  @on &.clip {
    opacity: 0 -> 1 -> 1 -> 1 -> 1 -> 0;
    .stars { --tw: 0 -> 6; }
    .dust  { --t: 0 -> 1; }
    .orb {
      scale: 0.15 -> 0.65 -> 1.9 -> 1.05 -> 1.1 -> 1.0;
      opacity: 0 -> 0.85 -> 1 -> 0.9 -> 0.9 -> 0;
      easing: --ease-out-cubic;
    }
    .t1 {
      opacity: 0 -> 0 -> 1 -> 1 -> 1 -> 0;
      blur: 30px -> 24px -> 0px -> 0px -> 0px -> 14px;
      letter-spacing: 0.32em -> 0.30em -> 0.02em -> 0.01em -> 0.01em -> 0.05em;
      translate-y: 16px -> 12px -> 0px -> 0px -> 0px -> -8px;
      easing: --ease-out-expo;
    }
    .cam {
      translate-x: 0px -> 0px -> 9px -> -7px -> 4px -> -2px -> 0px;
      translate-y: 0px -> 0px -> -6px -> 5px -> -3px -> 1px -> 0px;
      range: 0.60 to 0.76;
    }
  }
}

// ===========================================================================
// S2 — the grid materializes; the time axis sweeps through it.
// ===========================================================================
.s2 .world { position: absolute; inset: 0; perspective: 900px; perspective-origin: 50% calc(var(--po, 30) * 1%); }
.s2 .floor {
  position: absolute; left: -60%; right: -60%; top: 46%; height: 260%;
  transform-origin: 50% 0; transform-style: preserve-3d;
  background-image: linear-gradient(rgba(125, 211, 252, 0.42) 2px, transparent 2px), linear-gradient(90deg, rgba(125, 211, 252, 0.42) 2px, transparent 2px);
  background-size: 96px 96px;
  mask-image: radial-gradient(ellipse 60% 55% at 50% 0%, #000 calc(var(--reveal, 0) * 0.8%), transparent calc(var(--reveal, 0) * 0.8% + 14%));
  rotate-x: 88deg;
}
.s2 .ripple { position: absolute; left: 50%; top: 61%; width: 320px; height: 96px; margin: -48px 0 0 -160px; border-radius: 50%; border: 5px solid #ffb454; box-shadow: 0 0 40px #ffb454, inset 0 0 40px rgba(255, 180, 84, 0.5); opacity: 0; scale: 0.2; }
.s2 .axis { position: absolute; left: 50%; top: -10%; bottom: -10%; width: 10px; margin-left: -5px; translate-x: -1300px; }
.s2 .axis-line { position: absolute; inset: 0; background: #ffe6c2; box-shadow: 0 0 30px 8px #ffb454, 0 0 120px 40px rgba(255, 180, 84, 0.45); }
.s2 .axis-trail { position: absolute; top: 0; bottom: 0; right: 0; width: 520px; background-image: linear-gradient(to left, rgba(255, 180, 84, 0.35), rgba(255, 180, 84, 0)); }
.s2 .t2 { position: relative; z-index: 2; margin-top: 28vh; opacity: 0; }
.s2 .k2 { position: relative; z-index: 2; margin-top: 34px; opacity: 0; }
.s2 {
  @on &.clip {
    opacity: 0 -> 1 -> 1 -> 1 -> 1 -> 0;
    .stars { --tw: 3 -> 9; }
    .world { --po: 10 -> 34; easing: --ease-in-out-cubic; range: 0 to 0.6; }
    .floor { --reveal: 0 -> 100; rotate-x: 88deg -> 74deg; easing: --ease-out-quart; range: 0 to 0.55; }
    .axis { translate-x: -1300px -> 1300px; easing: --ease-in-out-cubic; range: 0.42 to 0.70; }
    .ripple { scale: 0.2 -> 7; opacity: 0.95 -> 0; easing: --ease-out-cubic; range: 0.695 to 0.98; }
    .t2 { opacity: 0 -> 1; blur: 22px -> 0px; letter-spacing: 0.24em -> 0.01em; translate-y: 20px -> 0px; easing: --ease-out-expo; range: 0.30 to 0.56; }
    .k2 { opacity: 0 -> 1; translate-y: 14px -> 0px; easing: --ease-out-expo; range: 0.42 to 0.62; }
    .cam {
      translate-x: 0px -> 0px -> 11px -> -9px -> 5px -> -2px -> 0px;
      translate-y: 0px -> 0px -> -7px -> 6px -> -3px -> 1px -> 0px;
      range: 0.69 to 0.84;
    }
  }
}

// ===========================================================================
// S3 — the code types itself; the element it describes animates beside it.
// The card's motion below IS the snippet on screen: opacity 0 -> 1,
// translate-y 40px -> 0, --ease-out-expo.
// ===========================================================================
.s3 .split { display: flex; gap: 60px; align-items: stretch; perspective: 1800px; margin-top: -60px; }
.s3 .code { width: 1000px; rotate-y: 8deg; }
.s3 .live { width: 820px; rotate-y: -8deg; display: flex; flex-direction: column; }
.s3 .src { margin: 0; padding: 40px 48px; font: 500 46px/1.6 "JetBrains Mono", monospace; color: #e6ecf5; display: flex; flex-direction: column; white-space: pre; }
.s3 .ln { display: block; clip-path: inset(0 calc(100% - clamp(0ch, (var(--chars, 0) - var(--o)) * 1ch, 100%)) 0 0); }
.s3 .ln:nth-child(1) { --o: 0; }
.s3 .ln:nth-child(2) { --o: 8; }
.s3 .ln:nth-child(3) { --o: 27; }
.s3 .ln:nth-child(4) { --o: 48; }
.s3 .ln:nth-child(5) { --o: 76; }
.s3 .ln:nth-child(6) { --o: 106; }
.s3 .ln:nth-child(7) { --o: 110; }
.sel { color: #dbe3ee; font-weight: 500; } .pn { color: #5c6677; font-weight: 500; } .dir { color: #ffb454; font-weight: 500; } .ref { color: #7dd3fc; font-weight: 500; } .pr { color: #c9d3e0; font-weight: 500; } .nu { color: #ffd9a8; font-weight: 500; } .ar { color: #ffb454; font-weight: 500; }
.s3 .caret { position: absolute; left: 48px; top: 108px; width: 24px; height: 52px; background: #ffb454; opacity: calc(0.5 + 0.5 * sign(sin(var(--chars, 0) * 1.3))); translate: calc(clamp(0, var(--chars, 0) - 0, 7) * 1ch) 0; }
.s3 .live .card { margin: 40px auto 0; opacity: 0; }
.s3 .live .ph { margin: auto 40px 40px; }
.s3 .live .drv { color: #8a94a6; }
.s3 .t3 { margin-top: 70px; opacity: 0; font-size: 118px; }
.s3 {
  @on &.clip {
    opacity: 0 -> 1 -> 1 -> 1 -> 1 -> 0;
    .split { opacity: 0 -> 1; translate-y: 40px -> 0px; easing: --ease-out-expo; range: 0 to 0.12; }
    .code { --chars: 0 -> 111; range: 0.05 to 0.70; }
    .card { opacity: 0 -> 1; translate-y: 40px -> 0px; easing: --ease-out-expo; range: 0.62 to 0.78; }
    .ph i { scale-x: 0 -> 1; easing: --ease-out-expo; range: 0.62 to 0.78; }
    .live .drv { color: #8a94a6 -> #ffb454; range: 0.61 to 0.63; }
    .t3 { opacity: 0 -> 1; blur: 18px -> 0px; letter-spacing: 0.2em -> 0.01em; easing: --ease-out-expo; range: 0.78 to 0.90; }
    .cam {
      translate-x: 0px -> 0px -> 8px -> -6px -> 3px -> -1px -> 0px;
      translate-y: 0px -> 0px -> -5px -> 4px -> -2px -> 1px -> 0px;
      range: 0.775 to 0.88;
    }
  }
}

// ===========================================================================
// S4 — four drivers, one composition. Then the playheads become one.
// ===========================================================================
.s4 .panels { position: relative; width: 1800px; height: 640px; }
.s4 .panel { position: absolute; top: 0; width: 420px; height: 620px; border: 3px solid rgba(255, 255, 255, 0.12); border-radius: 26px; background: #0b0e14; transform-origin: 50% 50%; overflow: hidden; }
.s4 .p1 { left: 0; } .s4 .p2 { left: 460px; } .s4 .p3 { left: 920px; } .s4 .p4 { left: 1380px; }
.s4 .panel .bar { background: transparent; padding: 0 28px; height: 70px; font-size: 24px; position: relative; }
.s4 .panel .kind { font: 600 16px/1 "Space Grotesk", sans-serif; letter-spacing: 0.2em; text-transform: uppercase; }
.s4 .panel .one { position: absolute; left: 28px; top: 22px; color: #ffb454; opacity: 0; }
.s4 .panel .card { width: 360px; margin: 10px auto 0; opacity: 0; transform-origin: 50% 50%; }
.s4 .panel .card .img { height: 210px; }
.s4 .panel .card .row { padding: 22px 28px 0; font-size: 26px; }
.s4 .panel .card .sub { padding: 10px 28px 24px; font-size: 18px; }
.s4 .panel .ph { position: absolute; left: 30px; right: 30px; bottom: 44px; }
.s4 .wave { position: absolute; left: 30px; right: 30px; bottom: 16px; height: 18px; display: flex; justify-content: space-between; align-items: center; }
.s4 .wave i { display: block; width: 6px; background: rgba(125, 211, 252, 0.7); height: calc(4px + 12px * abs(sin(sibling-index() * 1.7rad))); }
.s4 .t4 { margin-top: 50px; opacity: 0; font-size: 118px; }
.s4 {
  @on &.clip {
    opacity: 0 -> 1 -> 1 -> 1 -> 1 -> 0;
    .panels { opacity: 0 -> 1; scale: 0.94 -> 1; easing: --ease-out-expo; range: 0 to 0.1; }

    // each panel's card runs the SAME reveal on its own driver's timing
    .p1 .card { opacity: 0 -> 1; translate-y: 40px -> 0px; easing: --ease-out-expo; range: 0.065 to 0.26; }
    .p1 .ph i { scale-x: 0 -> 1; range: 0.065 to 0.26; }
    .p2 .card { opacity: 0 -> 1; translate-y: 40px -> 0px; easing: --ease-out-expo; range: 0.226 to 0.50; }
    .p2 .ph i { scale-x: 0 -> 1; easing: --ease-in-out-cubic; range: 0.226 to 0.50; }
    .p3 .card { opacity: 0 -> 1; translate-y: 40px -> 0px; easing: --ease-out-back; range: 0.387 to 0.48; }
    .p3 .ph i { scale-x: 0 -> 1; range: 0.387 to 0.40; }
    .p4 .card { opacity: 0 -> 1; translate-y: 40px -> 0px; easing: --ease-out-expo; range: 0.548 to 0.77; }
    .p4 .ph i { scale-x: 0 -> 1; range: 0.548 to 0.77; }
    .p1 .drv { color: #8a94a6 -> #ffb454; range: 0.06 to 0.07; }
    .p2 .drv { color: #8a94a6 -> #ffb454; range: 0.22 to 0.23; }
    .p3 .drv { color: #8a94a6 -> #ffb454; range: 0.38 to 0.39; }
    .p4 .drv { color: #8a94a6 -> #ffb454; range: 0.54 to 0.55; }

    // convergence: four windows collapse onto one playhead
    .p1 { translate-x: 0px -> 690px; scale: 1 -> 1.28; easing: --ease-in-out-quart; range: 0.66 to 0.90; }
    .p2 { translate-x: 0px -> 230px; scale: 1 -> 1.28; easing: --ease-in-out-quart; range: 0.66 to 0.90; }
    .p3 { translate-x: 0px -> -230px; scale: 1 -> 1.28; easing: --ease-in-out-quart; range: 0.66 to 0.90; }
    .p4 { translate-x: 0px -> -690px; scale: 1 -> 1.28; easing: --ease-in-out-quart; range: 0.66 to 0.90; }
    .panel { border-color: rgba(255, 255, 255, 0.12) -> #ffb454; range: 0.80 to 0.92; }
    .drv, .kind { opacity: 1 -> 0; range: 0.78 to 0.86; }
    .one { opacity: 0 -> 1; range: 0.86 to 0.92; }
    .t4 { opacity: 0 -> 1; blur: 18px -> 0px; letter-spacing: 0.2em -> 0.01em; easing: --ease-out-expo; range: 0.76 to 0.88; }
    .cam {
      translate-x: 0px -> 0px -> 12px -> -9px -> 5px -> -2px -> 0px;
      translate-y: 0px -> 0px -> -8px -> 6px -> -3px -> 1px -> 0px;
      range: 0.90 to 1;
    }
  }
}

// ===========================================================================
// S5 — render. A filmstrip flies past in perspective; the counter ticks.
// The frame number is a CSS counter reset from the animated --fr.
// ===========================================================================
.s5 .stripg { position: absolute; left: 0; right: 0; top: 50%; height: 0; perspective: 1400px; perspective-origin: 50% 50%; }
.s5 .strip { position: absolute; left: 50%; top: -260px; display: flex; gap: 0; transform-style: preserve-3d; rotate-y: -34deg; translate-x: 1600px; }
.s5 .fr { width: 400px; height: 520px; flex: none; background: #0a0c11; border-right: 3px solid rgba(255, 255, 255, 0.08); position: relative; display: flex; align-items: center; justify-content: center; }
.s5 .fr::before { content: ""; position: absolute; left: 24px; right: 24px; top: 18px; height: 26px; background-image: radial-gradient(circle, transparent 0 8px, #06070a 9px); background-size: 88px 26px; background-repeat: repeat-x; }
.s5 .fr::after { content: ""; position: absolute; left: 24px; right: 24px; bottom: 18px; height: 26px; background-image: radial-gradient(circle, transparent 0 8px, #06070a 9px); background-size: 88px 26px; background-repeat: repeat-x; }
.s5 .fr .card { width: 300px; border-radius: 18px; }
.s5 .fr .card .img { height: 190px; }
.s5 .fr .card .row { padding: 20px 24px 22px; font-size: 22px; }
.s5 .fr .card .mug { width: 110px; height: 70px; }
.s5 .fr .card .mug::after { left: 86px; top: 4px; width: 40px; height: 56px; border-width: 10px; }
.s5 .fr .card { opacity: 0.9; }
.s5 .ctr { position: absolute; left: 120px; top: 110px; display: grid; grid-template-columns: auto auto auto auto; grid-template-rows: auto auto; column-gap: 28px; align-items: baseline; font: 500 44px/1 "JetBrains Mono", monospace; opacity: 0; }
.s5 .ctr .lbl { font: 600 24px/1 "Space Grotesk", sans-serif; letter-spacing: 0.3em; text-transform: uppercase; color: #8a94a6; }
.s5 .ctr .num { font-size: 92px; counter-reset: fr calc(round(var(--fr, 0))); }
.s5 .ctr .num::after { content: counter(fr, decimal-leading-zero); }
.s5 .ctr .of { color: #8a94a6; }
.s5 .ctr .tm { color: #7dd3fc; }
.s5 .ctr .tnum { counter-reset: ms calc(round(var(--fr, 0) * 16.6667)); }
.s5 .ctr .tnum::before { content: "t = "; }
.s5 .ctr .tnum::after { content: counter(ms) "ms"; }
.s5 .ctr .lbl2 { grid-column: 1 / -1; font: 600 22px/1 "Space Grotesk", sans-serif; letter-spacing: 0.3em; text-transform: uppercase; color: #8a94a6; margin-top: 18px; }
.s5 .cmd { position: absolute; left: 120px; bottom: 110px; width: 1500px; padding: 34px 44px; background: rgba(11, 14, 20, 0.88); border: 2px solid rgba(255, 255, 255, 0.1); border-radius: 22px; font: 500 38px/1.5 "JetBrains Mono", monospace; opacity: 0; }
.s5 .cmd .line { display: block; clip-path: inset(0 calc(100% - clamp(0ch, var(--n, 0) * 1ch, 100%)) 0 0); }
.s5 .cmd .caret { position: absolute; left: 44px; top: 40px; width: 22px; height: 46px; background: #ffb454; translate: calc(clamp(0, var(--n, 0), 46) * 1ch) 0; opacity: calc(0.5 + 0.5 * sign(sin(var(--n, 0) * 1.3))); }
.s5 .cmd .ok { display: block; color: #8a94a6; opacity: 0; }
.s5 .t5 { position: relative; z-index: 2; opacity: 0; font-size: 200px; margin-top: -40px; text-shadow: 0 0 60px rgba(255, 255, 255, 0.35); }
.s5 {
  @on &.clip {
    opacity: 0 -> 1 -> 1 -> 1 -> 1 -> 0;
    .strip { translate-x: 1600px -> -6600px; easing: --ease-in-out-sine; }
    .ctr { opacity: 0 -> 1; range: 0 to 0.08; }
    .num { --fr: 0 -> 1800; range: 0.04 to 0.96; }
    .tnum { --fr: 0 -> 1800; range: 0.04 to 0.96; }
    .cmd { opacity: 0 -> 1; translate-y: 20px -> 0px; easing: --ease-out-expo; range: 0.02 to 0.1; }
    .cmd .line { --n: 0 -> 46; range: 0.06 to 0.28; }
    .cmd .caret { --n: 0 -> 46; range: 0.06 to 0.28; }
    .cmd .ok { opacity: 0 -> 1; range: 0.34 to 0.42; }
    .t5 { opacity: 0 -> 1; blur: 24px -> 0px; letter-spacing: 0.2em -> 0.01em; scale: 1.15 -> 1; easing: --ease-out-expo; range: 0.48 to 0.66; }
    .cam { scale: 1 -> 1.06; easing: --ease-in-quad; range: 0.7 to 1; }
    .cam {
      translate-x: 0px -> 12px -> -9px -> 5px -> -2px -> 0px;
      translate-y: 0px -> -8px -> 6px -> -3px -> 1px -> 0px;
      range: 0 to 0.09;
    }
  }
}

// ===========================================================================
// S6 — <script> shatters. 140 particles, one property (--tau); the geometry
// per particle is CSS trig over sibling-index().
// ===========================================================================
.s6 .tagwrap { position: relative; width: 1400px; height: 400px; display: flex; align-items: center; justify-content: center; }
.s6 .tagw, .s6 .tagr, .s6 .tagb { position: absolute; font-size: 250px; font-weight: 700; color: #e6ecf5; opacity: 0; scale: 1.25; }
.s6 .tagr { color: #ff3b3b; mix-blend-mode: screen; opacity: 0; }
.s6 .tagb { color: #3bb0ff; mix-blend-mode: screen; opacity: 0; }
.s6 .burst { position: absolute; left: 50%; top: 50%; width: 0; height: 0; }
.s6 .burst i {
  position: absolute; display: block; border-radius: 50%; background: #ffe6c2;
  width: calc(6px + mod(sibling-index(), 4) * 3px); height: calc(6px + mod(sibling-index(), 4) * 3px);
  left: calc((mod(sibling-index() * 53, 100) - 50) * 12px);
  top: calc((mod(sibling-index() * 31, 100) - 50) * 3.2px);
  translate:
    calc(cos(sibling-index() * 137.5deg) * (200px + mod(sibling-index() * 7, 100) * 6px) * (1 - exp(-2.2 * var(--tau, 0))) / 2.2 * 2.2 + (mod(sibling-index() * 53, 100) - 50) * 12px * var(--tau, 0) * 0.9)
    calc(sin(sibling-index() * 137.5deg) * (140px + mod(sibling-index() * 11, 100) * 5px) * (1 - exp(-2.2 * var(--tau, 0))) - 120px * var(--tau, 0) + 620px * var(--tau, 0) * var(--tau, 0));
  opacity: calc(1 - var(--tau, 0) * var(--tau, 0));
  box-shadow: 0 0 8px rgba(255, 180, 84, 0.8);
}
.s6 .t6 { opacity: 0; font-size: 170px; margin-top: -40px; }
.s6 .sub6 { margin: 30px 0 0; font: 500 48px/1 "Space Grotesk", sans-serif; letter-spacing: 0.12em; color: #8a94a6; opacity: 0; }
.s6 {
  @on &.clip {
    opacity: 0 -> 1 -> 1 -> 1 -> 1 -> 1 -> 0;
    .tagw { opacity: 0 -> 1 -> 1 -> 0; scale: 1.25 -> 1 -> 1 -> 1; easing: --ease-out-back; range: 0 to 0.06; }
    .burst { --tau: 0 -> 1; range: 0.045 to 0.80; }
    .burst i { opacity: 0 -> 1 -> 1; range: 0.04 to 0.05; }
    // rgb-split glitch stutter after the shatter
    .tagr { opacity: 0 -> 0.8 -> 0 -> 0.9 -> 0 -> 0.7 -> 0; translate-x: -40px -> 30px -> -20px -> 14px -> -8px -> 0px; range: 0.14 to 0.25; }
    .tagb { opacity: 0 -> 0.7 -> 0 -> 0.9 -> 0 -> 0.6 -> 0; translate-x: 40px -> -30px -> 20px -> -14px -> 8px -> 0px; range: 0.14 to 0.25; }
    .cam { translate-x: 0px -> 14px -> -10px -> 6px -> -3px -> 0px; skew-x: 0deg -> 3deg -> -2deg -> 1deg -> 0deg -> 0deg; hue-rotate: 0deg -> 40deg -> -30deg -> 10deg -> 0deg -> 0deg; range: 0.14 to 0.26; }
    .t6 { opacity: 0 -> 1; blur: 18px -> 0px; letter-spacing: 0.2em -> 0.01em; easing: --ease-out-expo; range: 0.34 to 0.52; }
    .sub6 { opacity: 0 -> 1; blur: 10px -> 0px; letter-spacing: 0.22em -> 0.12em; easing: --ease-out-expo; range: 0.52 to 0.70; }
  }
}

// ===========================================================================
// S7 — the wordmark. A gradient clipped to text; --pos sweeps the highlight.
// ===========================================================================
.s7 .mark {
  font-size: 150px; letter-spacing: 0.14em; opacity: 0;
  background-image: linear-gradient(100deg, #e6ecf5 42%, #fffaf0 50%, #e6ecf5 58%);
  background-size: 260% 100%;
  background-position: var(--pos, -120%) 0;
  -webkit-background-clip: text; background-clip: text; color: transparent;
  text-shadow: none;
}
.s7 .rule { width: 1040px; height: 4px; margin-top: 34px; background: #7dd3fc; box-shadow: 0 0 24px #7dd3fc, 0 0 60px rgba(125, 211, 252, 0.6); transform-origin: 50% 50%; scale: 0 1; }
.s7 .tag7 { margin: 52px 0 0; font: 500 58px/1 "Space Grotesk", sans-serif; letter-spacing: 0.01em; opacity: 0; }
.s7 .k7 { margin-top: 44px; opacity: 0; }
.s7 {
  @on &.clip {
    opacity: 0 -> 1 -> 1;
    .mark { opacity: 0 -> 1; blur: 30px -> 0px; letter-spacing: 0.30em -> 0.14em; scale: 1.06 -> 1; easing: --ease-out-expo; range: 0 to 0.27; }
    .mark { --pos: -120% -> 230%; easing: --ease-in-out-sine; range: 0.13 to 0.63; }
    .rule { scale-x: 0 -> 1; easing: --ease-out-quart; range: 0.07 to 0.37; }
    .tag7 { opacity: 0 -> 1; blur: 10px -> 0px; letter-spacing: 0.12em -> 0.01em; easing: --ease-out-expo; range: 0.33 to 0.60; }
    .k7 { opacity: 0 -> 1; translate-y: 10px -> 0px; easing: --ease-out-expo; range: 0.47 to 0.67; }
  }
}
