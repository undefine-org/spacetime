@import "stdlib/film"

// ═══════════════════════════════════════════════════════════════════════════
// FILM SHOWCASE — the whole film surface, one 24-second score, zero JavaScript.
//
// Five beats on ONE `@score &.time(24s)`. Windows ABUT (no overlap): like the
// studio cut, each scene owns its slice of time exclusively and fades to black
// at the cut, so two scenes never double-expose. Per-scene display sizes (the
// studio uses 150–215px, never one size) keep every word readable inside frame.
//
//   cargo run -- serve demos/film-showcase/
//   cargo run --features cdp -- render demos/film-showcase/ --out film.mp4 --width 1280 --height 720 --fps 30
// ═══════════════════════════════════════════════════════════════════════════

<div class="stage">

  <div class="scene s1">
    <div class="cam">
      <div class="dust"><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i></div>
      <div class="orb"></div>
      <h1 class="t1 dsp">SPACETIME</h1>
      <p class="k1 kicker">a language that describes time</p>
    </div>
  </div>

  <div class="scene s2">
    <div class="cam">
      <div class="world"><div class="floor"></div></div>
      <div class="axis"><div class="axis-trail"></div><div class="axis-line"></div></div>
      <h1 class="t2 dsp">ONE SOURCE</h1>
      <p class="k2 kicker">every driver · one clock</p>
    </div>
  </div>

  <div class="scene s3">
    <div class="cam">
      <div class="cardwrap"></div>
      <p class="k3 kicker">a component with a timeline</p>
    </div>
  </div>

  <div class="scene s4">
    <div class="cam">
      <div class="burst">
        <span class="word dsp">MOTION</span>
        <i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i>
        <i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i>
        <i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i>
        <i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i>
      </div>
      <p class="k4 kicker">one property · every element</p>
    </div>
  </div>

  <div class="scene s5">
    <div class="cam">
      <div class="dust d2"><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i></div>
      <h1 class="mark dsp">NO JAVASCRIPT</h1>
      <div class="rule"></div>
      <p class="k5 kicker">just Spacetime</p>
    </div>
  </div>

  <div class="grain"></div>
  <div class="vignette"></div>
  <div class="fadein"></div>
  <div class="fadeout"></div>

</div>

// ── A card shot: markup + motion in one form (W9) ──────────────────────────
@form shot --product-card($title, $price) {
  :markup {
    <div class="card">
      <div class="img"><i class="mug"></i></div>
      <div class="row"><span class="nm">`$title`</span><span class="price">`$price`</span></div>
      <div class="tag">hand-thrown · 12 oz</div>
    </div>
  }
  :motion {
    translate-y: 24px -> 0;
  }
}

// ═══ Base styling ═══════════════════════════════════════════════════════════
body { margin: 0; }
.stage {
  position: fixed; inset: 0; overflow: hidden;
  background: #06070a;
  font-family: Syne, "Space Grotesk", system-ui, sans-serif;
  color: #eaf0ff; -webkit-font-smoothing: antialiased;

  // ═══ THE SCORE — one transport, five beats, windows ABUT (no double-expose) ═
  @score &.time(24s) as &reel {
    &s1 at 0s   for 5s;
    &s2 at 5s   for 4.5s;
    &s3 at 9.5s for 4.5s;
    &s4 at 14s  for 4.5s;
    &s5 at 18.5s for 5.5s;
    &grain  at 0s for 24s;
    &fadein  at 0s for 0.8s;
    &fadeout at 23s for 1s;
  }
}

.scene { position: absolute; inset: 0; opacity: 0; }
.cam { position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; }

// Display type. ONE family, but PER-SCENE sizes (the studio uses 150–215px per
// line, never one size) so a 6-letter word and a 13-letter word both sit inside
// the frame. Base is conservative; each hero overrides to fill ~75% width.
.dsp { font-weight: 800; letter-spacing: 0.01em; line-height: 0.98; margin: 0;
  font-size: 8vw; text-align: center; white-space: nowrap;
  color: #eef3ff; text-shadow: 0 0 60px rgba(138,180,255,0.35); }
.kicker { font: 600 clamp(11px, 1.35vw, 20px)/1 "Space Grotesk", sans-serif; letter-spacing: 0.34em;
  text-transform: uppercase; color: #9aa4d0; margin: 1.8rem 0 0; }

// ── The lens overlays ──────────────────────────────────────────────────────
.grain {
  position: absolute; inset: -200px; opacity: 0.05; pointer-events: none; mix-blend-mode: screen; z-index: 40;
  background-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='300' height='300'><filter id='n'><feTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='2' stitchTiles='stitch'/><feColorMatrix values='0 0 0 0 1 0 0 0 0 1 0 0 0 0 1 0 0 0 1 0'/></filter><rect width='100%' height='100%' filter='url(%23n)'/></svg>");
  background-position: calc(var(--g, 0) * 71px) calc(var(--g, 0) * 37px);
}
.vignette { position: absolute; inset: 0; pointer-events: none; z-index: 41;
  background-image: radial-gradient(ellipse 82% 74% at 50% 48%, transparent 52%, rgba(0,0,0,0.82) 108%); }
.fadein  { position: absolute; inset: 0; background: #000; opacity: 1; pointer-events: none; z-index: 42; }
.fadeout { position: absolute; inset: 0; background: #000; opacity: 0; pointer-events: none; z-index: 42; }
.grain   { @on &.clip { --g: 0 -> 300; } }
.fadein  { @on &.clip { opacity: 1 -> 0; easing: --ease-out-quad; } }
.fadeout { @on &.clip { opacity: 0 -> 1; easing: --ease-in-quad; } }

// ═══ Scene 1 — a point of light. Dust placed with random() (W6). ══════════════
.dust { position: absolute; inset: 0; }
.dust i {
  position: absolute;
  --seed: random(); --seed2: random(2); --seed3: random(3);
  left: calc(var(--seed) * 100%); top: calc(var(--seed2) * 100%);
  width: calc(1px + var(--seed3) * 2px); height: calc(1px + var(--seed3) * 2px);
  border-radius: 50%; background: #cdd8ff;
  opacity: calc(0.1 + 0.55 * var(--seed3));
  box-shadow: 0 0 6px rgba(170,190,255,0.6);
}
.s1 .t1 { font-size: 8vw; }
.s1 .orb {
  position: absolute; width: 44vw; height: 44vw; border-radius: 50%;
  background-image: radial-gradient(circle, #fff1d6 0%, rgba(255,200,120,0.85) 8%, rgba(255,180,84,0.32) 30%, rgba(255,180,84,0) 70%);
  filter: blur(6px);
}
.s1 .t1 { position: relative; z-index: 2; opacity: 0; }
.s1 .k1 { position: relative; z-index: 2; opacity: 0; }
.s1 {
  @on &.clip {
    opacity: 0 -> 1 at 7% -> 1 -> 1 -> 0 at 93%;
    .orb {
      scale: 0.14 -> 0.62 -> 1.8 -> 1.05 -> 1.06 -> 1.0;
      opacity: 0 -> 0.85 -> 1 -> 0.92 -> 0.9 -> 0.5;
      easing: --ease-out-cubic;
    }
    .t1 {
      opacity: 0 -> 0 -> 1 -> 1 -> 1 -> 1;
      blur: 34px -> 26px -> 0px -> 0px -> 0px -> 0px;
      letter-spacing: 0.30em -> 0.26em -> 0.01em -> 0.01em -> 0.01em -> 0.03em;
      translate-y: 18px -> 12px -> 0px -> 0px -> 0px -> -6px;
      easing: --ease-out-expo;
    }
    .k1 { opacity: 0 -> 1; translate-y: 14px -> 0px; easing: --ease-out-expo; range: 0.42 to 0.64; }
    .cam { translate-x: 0px -> 6px -> -5px -> 2px -> 0px; range: 0.58 to 0.78; }
  }
}

// ═══ Scene 2 — the world gets depth: a perspective grid + a time-axis sweep. ══
.s2 .t2 { font-size: 7vw; }
.s2 .world { position: absolute; inset: 0; perspective: 900px; perspective-origin: 50% calc(var(--po, 30) * 1%); }
.s2 .floor {
  position: absolute; left: -60%; right: -60%; top: 50%; height: 260%;
  transform-origin: 50% 0; transform-style: preserve-3d;
  background-image: linear-gradient(rgba(125,211,252,0.42) 2px, transparent 2px), linear-gradient(90deg, rgba(125,211,252,0.42) 2px, transparent 2px);
  background-size: 92px 92px;
  mask-image: radial-gradient(ellipse 62% 56% at 50% 0%, #000 calc(var(--reveal, 0) * 0.8%), transparent calc(var(--reveal, 0) * 0.8% + 14%));
  rotate-x: 88deg;
}
.s2 .axis { position: absolute; left: 50%; top: -10%; bottom: -10%; width: 10px; margin-left: -5px; translate-x: -1200px; }
.s2 .axis-line { position: absolute; inset: 0; background: #ffe6c2; box-shadow: 0 0 30px 8px #ffb454, 0 0 120px 40px rgba(255,180,84,0.45); }
.s2 .axis-trail { position: absolute; top: 0; bottom: 0; right: 0; width: 520px; background-image: linear-gradient(to left, rgba(255,180,84,0.35), rgba(255,180,84,0)); }
.s2 .t2 { position: relative; z-index: 2; opacity: 0; text-shadow: 0 0 40px rgba(125,211,252,0.4); }
.s2 .k2 { position: relative; z-index: 2; opacity: 0; }
.s2 {
  @on &.clip {
    opacity: 0 -> 1 at 8% -> 1 -> 1 -> 0 at 92%;
    .world { --po: 10 -> 34; easing: --ease-in-out-cubic; range: 0 to 0.6; }
    .floor { --reveal: 0 -> 100; rotate-x: 88deg -> 74deg; easing: --ease-out-quart; range: 0 to 0.58; }
    .axis  { translate-x: -1200px -> 1200px; easing: --ease-in-out-cubic; range: 0.36 to 0.68; }
    .t2 { opacity: 0 -> 1; blur: 26px -> 0px; letter-spacing: 0.24em -> 0.01em; translate-y: 20px -> 0px; easing: --ease-out-expo; range: 0.26 to 0.54; }
    .k2 { opacity: 0 -> 1; translate-y: 14px -> 0px; easing: --ease-out-expo; range: 0.44 to 0.64; }
  }
}

// ═══ Scene 3 — a card shot (W9): markup + motion in one form, in perspective. ══
.s3 .cardwrap { perspective: 1600px; display: grid; place-content: center; }
.card {
  width: 34vw; min-width: 380px; max-width: 460px; border-radius: 22px; overflow: hidden;
  background: #0d1024; border: 1px solid #2b3260; box-shadow: 0 40px 120px rgba(0,0,0,0.7);
  text-align: left;
}
.card .img { height: 17vw; min-height: 210px; max-height: 270px; background: radial-gradient(circle at 50% 38%, #2f3680, #12152e); display: grid; place-content: center; }
.card .mug { width: 96px; height: 74px; border-radius: 0 0 44px 44px; background: #e9edff; display: block; box-shadow: inset 0 -9px 0 #cfd6ff; }
.card .row { display: flex; justify-content: space-between; align-items: baseline; padding: 1.4rem 1.6rem 0.3rem; }
.card .nm { font-weight: 700; font-size: 1.9rem; white-space: nowrap; }
.card .price { font-weight: 800; font-size: 1.9rem; color: #ffb454; white-space: nowrap; }
.card .tag { padding: 0.2rem 1.6rem 1.5rem; color: #8892c0; font-size: 1.05rem; }
.s3 .k3 { opacity: 0; margin-top: 2.6rem; }
.s3 .cardwrap { opacity: 0; }
.s3 {
  @on &.clip {
    opacity: 0 -> 1 at 8% -> 1 -> 1 -> 0 at 92%;
    .cardwrap { opacity: 0 -> 1; scale: 0.9 -> 1; rotate-y: 20deg -> 0deg; translate-y: 44px -> 0px; easing: --ease-out-expo; range: 0.05 to 0.34; }
    .k3 { opacity: 0 -> 1; translate-y: 14px -> 0px; easing: --ease-out-expo; range: 0.42 to 0.64; }
    .cam { scale: 1 -> 1.05; easing: --ease-in-out-sine; range: 0.1 to 1; }
  }
}
.s3 .cardwrap { &product-card("Ceramic Mug", "$24.99"); }

// ═══ Scene 4 — a word that shatters into its own letters (@scatter, W5). ══════
// The particles sit at their sampled glyph points (--x,--y from @scatter, in px
// relative to the burst box, from left:0/top:0) so they FORM the word; then
// --tau flings each out on a golden-angle spiral. The <span> word reads first,
// fades as the dots take over.
.s4 .burst { position: relative; display: inline-grid; place-content: center; margin: auto; }
.s4 .word { grid-area: 1 / 1; font-size: 11.3vw; color: #cfe0ff; text-shadow: 0 0 40px rgba(138,180,255,0.55); }
.s4 .burst i {
  position: absolute; left: 0; top: 0; width: 11px; height: 11px; border-radius: 50%;
  background: #bcd4ff; box-shadow: 0 0 16px rgba(150,190,255,1), 0 0 4px #fff;
  translate:
    calc(var(--x, 0px) + cos(sibling-index() * 137.5deg) * 34vw * var(--tau, 0))
    calc(var(--y, 0px) + sin(sibling-index() * 137.5deg) * 22vh * var(--tau, 0) + 26vh * var(--tau, 0) * var(--tau, 0));
  opacity: calc(clamp(0, var(--tau, 0) * 6, 1) * (1 - var(--tau, 0) * var(--tau, 0)));
}
.s4 .k4 { opacity: 0; }
.s4 .burst {
  @scatter(from: &word, count: 40) { :shape { --x: sample-x; --y: sample-y; } }
}
.s4 {
  @on &.clip {
    opacity: 0 -> 1 at 8% -> 1 -> 1 -> 0 at 92%;
    .word { opacity: 0 -> 1 -> 1 -> 0 at 52%; scale: 1.1 -> 1 -> 1 -> 1.04; blur: 24px -> 0px -> 0px -> 8px; easing: --ease-out-expo; }
    .burst { --tau: 0 -> 0 -> 1 at 46% -> 1; easing: --ease-out-cubic; }
    .k4 { opacity: 0 -> 1; translate-y: 14px -> 0px; easing: --ease-out-expo; range: 0.5 to 0.7; }
  }
}

// ═══ Scene 5 — the wordmark: a gradient clipped to text; the highlight sweeps. ═
.s5 .mark { font-size: 6vw; }
.s5 .mark {
  opacity: 0;
  background-image: linear-gradient(100deg, #e6ecf5 42%, #fffaf0 50%, #e6ecf5 58%);
  background-size: 260% 100%;
  background-position: var(--pos, -120%) 0;
  -webkit-background-clip: text; background-clip: text; color: transparent;
  text-shadow: none;
}
.s5 .rule { width: 46vw; height: 3px; margin-top: 2rem; background: #7dd3fc; box-shadow: 0 0 24px #7dd3fc, 0 0 60px rgba(125,211,252,0.6); transform-origin: 50% 50%; scale: 0 1; }
.s5 .k5 { opacity: 0; margin-top: 2.2rem; }
.s5 .d2 i { background: #ffd9a8; box-shadow: 0 0 8px rgba(255,200,120,0.7); }
.s5 {
  @on &.clip {
    opacity: 0 -> 1 at 7% -> 1 -> 1;
    .mark { opacity: 0 -> 1; blur: 30px -> 0px; letter-spacing: 0.26em -> 0.01em; scale: 1.06 -> 1; easing: --ease-out-expo; range: 0 to 0.28; }
    .mark { --pos: -120% -> 230%; easing: --ease-in-out-sine; range: 0.14 to 0.64; }
    .rule { scale-x: 0 -> 1; easing: --ease-out-quart; range: 0.08 to 0.38; }
    .k5 { opacity: 0 -> 1; translate-y: 12px -> 0px; easing: --ease-out-expo; range: 0.46 to 0.68; }
  }
}
