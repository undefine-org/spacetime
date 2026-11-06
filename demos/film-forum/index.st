@import "stdlib/3d"
@import "stdlib/film"

// ═══════════════════════════════════════════════════════════════════════════
// FORUM — 26 seconds in Rome at dusk, built from primitives: a colonnade, a
// temple, firelight, a gold denarius. Title cards, subtitles and a score of
// low drums and a held chord. No JavaScript, no assets.
//
//   cargo run -- render demos/film-forum/ --engine raea -o forum.mp4
// ═══════════════════════════════════════════════════════════════════════════


// Fonts ship with the film (SIL OFL 1.1, Google Fonts), so every engine draws the same letters.
@font("Space Grotesk") { src: url("fonts/SpaceGrotesk.woff2") format("woff2"); font-weight: 300 700; font-style: normal; font-display: block; }
@font("Cormorant Garamond") { src: url("fonts/CormorantGaramond.woff2") format("woff2"); font-weight: 300 700; font-style: normal; font-display: block; }
@font("Cormorant Garamond") { src: url("fonts/CormorantGaramond-Italic.woff2") format("woff2"); font-weight: 300 700; font-style: italic; font-display: block; }

<div class="reel">
  <div class="shot"><canvas class="world"></canvas></div>
  <div class="card"><p class="k">Rome · the fifteenth of March, 44 BC</p><h1>IDES OF MARCH</h1></div>
  <p class="sub s1">The senate met in the Theatre of Pompey that morning.</p>
  <p class="sub s2">He had been warned. He walked in anyway.</p>
  <p class="sub s3">Within months, his killers struck a coin to celebrate.</p>
  <div class="coin-card"><p>EID · MAR</p><span>denarius of Brutus, 43–42 BC</span></div>
</div>

body { margin: 0; background: #000; }
.reel { position: fixed; inset: 0; overflow: hidden; }
.shot, .world { position: absolute; inset: 0; width: 100%; height: 100%; }
.card { position: absolute; left: 0; right: 0; top: 34vh; text-align: center; color: #f6ead4; opacity: 0; }
.card h1 { margin: 0; font: 400 8vw/1 "Cormorant Garamond", "Playfair Display", Georgia, serif; letter-spacing: 0.12em; }
.card .k { margin: 0 0 1.6vw; font: 500 1.3vw/1 "Space Grotesk", sans-serif; letter-spacing: 0.45em; text-transform: uppercase; color: #e0b46a; }
.sub { position: absolute; left: 0; right: 0; bottom: 8vh; margin: 0; text-align: center; color: #fff; font: italic 400 2.3vw/1.3 "Cormorant Garamond", Georgia, serif; text-shadow: 0 2px 18px rgba(0,0,0,0.7); opacity: 0; }
.coin-card { position: absolute; right: 7vw; top: 16vh; text-align: right; color: #f6ead4; opacity: 0; }
.coin-card p { margin: 0; font: 400 4vw/1 "Cormorant Garamond", Georgia, serif; letter-spacing: 0.2em; }
.coin-card span { font: italic 400 1.4vw/1 "Cormorant Garamond", Georgia, serif; color: #e0b46a; }

canvas.world {
  @stage(fov: 36, camX: 0, camY: 1.7, camZ: 6, shadows: "soft", background: "#2b1c22", bloom: 0.5, exposure: 4.8) {
    @light &sky (type: "ambient", intensity: 0.047, color: #9fa7ff)
    @light &sun (type: "directional", intensity: 2.2, color: #ff9d5c, posX: 7, posY: 3.2, posZ: -6)
    @fog(color: #2b1c22, near: 10, far: 40)
    @object (shape: "plane", size: 30, rotateX: -90, posZ: -12, color: #8d7b66, roughness: 0.95, metalness: 0)
    @object (shape: "cylinder", size: 0.34, scaleY: 7, posX: -3.2, posY: 2.38, posZ: -2.0, color: #d9cbb1, roughness: 0.85, metalness: 0)
    @object (shape: "box", size: 0.9, scaleY: 0.25, posX: -3.2, posY: 0.11, posZ: -2.0, color: #cbbb9e, roughness: 0.9, metalness: 0)
    @object (shape: "box", size: 0.85, scaleY: 0.22, posX: -3.2, posY: 4.86, posZ: -2.0, color: #cbbb9e, roughness: 0.9, metalness: 0)
    @object (shape: "cylinder", size: 0.34, scaleY: 7, posX: -3.2, posY: 2.38, posZ: -5.0, color: #d9cbb1, roughness: 0.85, metalness: 0)
    @object (shape: "box", size: 0.9, scaleY: 0.25, posX: -3.2, posY: 0.11, posZ: -5.0, color: #cbbb9e, roughness: 0.9, metalness: 0)
    @object (shape: "box", size: 0.85, scaleY: 0.22, posX: -3.2, posY: 4.86, posZ: -5.0, color: #cbbb9e, roughness: 0.9, metalness: 0)
    @object (shape: "cylinder", size: 0.34, scaleY: 7, posX: -3.2, posY: 2.38, posZ: -8.0, color: #d9cbb1, roughness: 0.85, metalness: 0)
    @object (shape: "box", size: 0.9, scaleY: 0.25, posX: -3.2, posY: 0.11, posZ: -8.0, color: #cbbb9e, roughness: 0.9, metalness: 0)
    @object (shape: "box", size: 0.85, scaleY: 0.22, posX: -3.2, posY: 4.86, posZ: -8.0, color: #cbbb9e, roughness: 0.9, metalness: 0)
    @object (shape: "cylinder", size: 0.34, scaleY: 7, posX: -3.2, posY: 2.38, posZ: -11.0, color: #d9cbb1, roughness: 0.85, metalness: 0)
    @object (shape: "box", size: 0.9, scaleY: 0.25, posX: -3.2, posY: 0.11, posZ: -11.0, color: #cbbb9e, roughness: 0.9, metalness: 0)
    @object (shape: "box", size: 0.85, scaleY: 0.22, posX: -3.2, posY: 4.86, posZ: -11.0, color: #cbbb9e, roughness: 0.9, metalness: 0)
    @object (shape: "cylinder", size: 0.34, scaleY: 7, posX: -3.2, posY: 2.38, posZ: -14.0, color: #d9cbb1, roughness: 0.85, metalness: 0)
    @object (shape: "box", size: 0.9, scaleY: 0.25, posX: -3.2, posY: 0.11, posZ: -14.0, color: #cbbb9e, roughness: 0.9, metalness: 0)
    @object (shape: "box", size: 0.85, scaleY: 0.22, posX: -3.2, posY: 4.86, posZ: -14.0, color: #cbbb9e, roughness: 0.9, metalness: 0)
    @object (shape: "cylinder", size: 0.34, scaleY: 7, posX: -3.2, posY: 2.38, posZ: -17.0, color: #d9cbb1, roughness: 0.85, metalness: 0)
    @object (shape: "box", size: 0.9, scaleY: 0.25, posX: -3.2, posY: 0.11, posZ: -17.0, color: #cbbb9e, roughness: 0.9, metalness: 0)
    @object (shape: "box", size: 0.85, scaleY: 0.22, posX: -3.2, posY: 4.86, posZ: -17.0, color: #cbbb9e, roughness: 0.9, metalness: 0)
    @object (shape: "cylinder", size: 0.34, scaleY: 7, posX: -3.2, posY: 2.38, posZ: -20.0, color: #d9cbb1, roughness: 0.85, metalness: 0)
    @object (shape: "box", size: 0.9, scaleY: 0.25, posX: -3.2, posY: 0.11, posZ: -20.0, color: #cbbb9e, roughness: 0.9, metalness: 0)
    @object (shape: "box", size: 0.85, scaleY: 0.22, posX: -3.2, posY: 4.86, posZ: -20.0, color: #cbbb9e, roughness: 0.9, metalness: 0)
    @object (shape: "box", size: 1, scaleX: 1.1, scaleY: 0.55, scaleZ: 21, posX: -3.2, posY: 5.25, posZ: -11, color: #c4b293, roughness: 0.9, metalness: 0)
    @object (shape: "cylinder", size: 0.34, scaleY: 7, posX: 3.2, posY: 2.38, posZ: -2.0, color: #d9cbb1, roughness: 0.85, metalness: 0)
    @object (shape: "box", size: 0.9, scaleY: 0.25, posX: 3.2, posY: 0.11, posZ: -2.0, color: #cbbb9e, roughness: 0.9, metalness: 0)
    @object (shape: "box", size: 0.85, scaleY: 0.22, posX: 3.2, posY: 4.86, posZ: -2.0, color: #cbbb9e, roughness: 0.9, metalness: 0)
    @object (shape: "cylinder", size: 0.34, scaleY: 7, posX: 3.2, posY: 2.38, posZ: -5.0, color: #d9cbb1, roughness: 0.85, metalness: 0)
    @object (shape: "box", size: 0.9, scaleY: 0.25, posX: 3.2, posY: 0.11, posZ: -5.0, color: #cbbb9e, roughness: 0.9, metalness: 0)
    @object (shape: "box", size: 0.85, scaleY: 0.22, posX: 3.2, posY: 4.86, posZ: -5.0, color: #cbbb9e, roughness: 0.9, metalness: 0)
    @object (shape: "cylinder", size: 0.34, scaleY: 7, posX: 3.2, posY: 2.38, posZ: -8.0, color: #d9cbb1, roughness: 0.85, metalness: 0)
    @object (shape: "box", size: 0.9, scaleY: 0.25, posX: 3.2, posY: 0.11, posZ: -8.0, color: #cbbb9e, roughness: 0.9, metalness: 0)
    @object (shape: "box", size: 0.85, scaleY: 0.22, posX: 3.2, posY: 4.86, posZ: -8.0, color: #cbbb9e, roughness: 0.9, metalness: 0)
    @object (shape: "cylinder", size: 0.34, scaleY: 7, posX: 3.2, posY: 2.38, posZ: -11.0, color: #d9cbb1, roughness: 0.85, metalness: 0)
    @object (shape: "box", size: 0.9, scaleY: 0.25, posX: 3.2, posY: 0.11, posZ: -11.0, color: #cbbb9e, roughness: 0.9, metalness: 0)
    @object (shape: "box", size: 0.85, scaleY: 0.22, posX: 3.2, posY: 4.86, posZ: -11.0, color: #cbbb9e, roughness: 0.9, metalness: 0)
    @object (shape: "cylinder", size: 0.34, scaleY: 7, posX: 3.2, posY: 2.38, posZ: -14.0, color: #d9cbb1, roughness: 0.85, metalness: 0)
    @object (shape: "box", size: 0.9, scaleY: 0.25, posX: 3.2, posY: 0.11, posZ: -14.0, color: #cbbb9e, roughness: 0.9, metalness: 0)
    @object (shape: "box", size: 0.85, scaleY: 0.22, posX: 3.2, posY: 4.86, posZ: -14.0, color: #cbbb9e, roughness: 0.9, metalness: 0)
    @object (shape: "cylinder", size: 0.34, scaleY: 7, posX: 3.2, posY: 2.38, posZ: -17.0, color: #d9cbb1, roughness: 0.85, metalness: 0)
    @object (shape: "box", size: 0.9, scaleY: 0.25, posX: 3.2, posY: 0.11, posZ: -17.0, color: #cbbb9e, roughness: 0.9, metalness: 0)
    @object (shape: "box", size: 0.85, scaleY: 0.22, posX: 3.2, posY: 4.86, posZ: -17.0, color: #cbbb9e, roughness: 0.9, metalness: 0)
    @object (shape: "cylinder", size: 0.34, scaleY: 7, posX: 3.2, posY: 2.38, posZ: -20.0, color: #d9cbb1, roughness: 0.85, metalness: 0)
    @object (shape: "box", size: 0.9, scaleY: 0.25, posX: 3.2, posY: 0.11, posZ: -20.0, color: #cbbb9e, roughness: 0.9, metalness: 0)
    @object (shape: "box", size: 0.85, scaleY: 0.22, posX: 3.2, posY: 4.86, posZ: -20.0, color: #cbbb9e, roughness: 0.9, metalness: 0)
    @object (shape: "box", size: 1, scaleX: 1.1, scaleY: 0.55, scaleZ: 21, posX: 3.2, posY: 5.25, posZ: -11, color: #c4b293, roughness: 0.9, metalness: 0)
    @object (shape: "box", size: 1, scaleX: 9.0, scaleY: 0.3, scaleZ: 3.0, posY: 0.15, posZ: -24.0, color: #d6c7aa, roughness: 0.9, metalness: 0)
    @object (shape: "box", size: 1, scaleX: 8.4, scaleY: 0.3, scaleZ: 2.6, posY: 0.45, posZ: -23.8, color: #d6c7aa, roughness: 0.9, metalness: 0)
    @object (shape: "box", size: 1, scaleX: 7.8, scaleY: 0.3, scaleZ: 2.2, posY: 0.75, posZ: -23.6, color: #d6c7aa, roughness: 0.9, metalness: 0)
    @object (shape: "box", size: 1, scaleX: 7.2, scaleY: 0.3, scaleZ: 1.8, posY: 1.05, posZ: -23.4, color: #d6c7aa, roughness: 0.9, metalness: 0)
    @object (shape: "box", size: 1, scaleX: 8, scaleY: 6, scaleZ: 1.2, posY: 4.2, posZ: -25.6, color: #cdbd9f, roughness: 0.9, metalness: 0)
    @object (shape: "box", size: 1, scaleX: 3, scaleY: 4, scaleZ: 1.3, posY: 2.6, posZ: -25.0, color: #20161a, roughness: 1, metalness: 0)
    @object (shape: "cylinder", size: 0.3, scaleY: 1.6, posX: -1.6, posY: 0.5, posZ: -8, color: #3b2c22, roughness: 0.8, metalness: 0.3)
    @object (shape: "cylinder", size: 0.3, scaleY: 1.6, posX: 1.6, posY: 0.5, posZ: -8, color: #3b2c22, roughness: 0.8, metalness: 0.3)
    @object &fire1 (shape: "sphere", size: 0.22, posX: -1.6, posY: 1.1, posZ: -8, color: #000000, emissive: #ff8a2a, emissiveIntensity: 0.625)
    @object &fire2 (shape: "sphere", size: 0.22, posX: 1.6, posY: 1.1, posZ: -8, color: #000000, emissive: #ff8a2a, emissiveIntensity: 0.625)
    @light &blaze1 (type: "point", intensity: 27, color: #ff8a3c, posX: -1.6, posY: 1.4, posZ: -8, distance: 8)
    @light &blaze2 (type: "point", intensity: 27, color: #ff8a3c, posX: 1.6, posY: 1.4, posZ: -8, distance: 8)
    @object &coin (shape: "coin", size: 0.42, rotateX: 90, posY: 1.5, posZ: 3.2, color: #d6a73c, roughness: 0.22, metalness: 1)
    @light &glint (type: "point", intensity: 0, color: #fff0c8, posX: 0.8, posY: 2.2, posZ: 4.6, distance: 5)
  }
}

@form instrument --drum  { wave: sine; pitch: 95hz -> 40hz at 30%; gain: 1 -> 0; for: 900ms; reverb: 0.5; }
@form instrument --strings { wave: saw; voices: 3; detune: 9; cutoff: 500 -> 1300 at 50% -> 600; gain: 0 -> 0.42 at 30% -> 0.42 at 85% -> 0; reverb: 0.8; }
@form instrument --ting  { wave: sine; gain: 0 -> 0.5 at 1% -> 0; for: 3000ms; reverb: 0.8; }
@form pattern --march($tempo = 72bpm, $step = 8) {
  --drum: x . . . x . x . ;
}

.reel {
  @score &.time(26s) as &film {
    &shot at 0s for 26s;
    &card at 0.6s for 5.4s;
    &s1 at 6.6s for 4.6s;
    &s2 at 11.6s for 4.4s;
    &s3 at 16.6s for 4.4s;
    &coin-card at 20.4s for 5.6s;
    @sound(--strings, notes: "D3 A3 F4") at 0s for 26s;
    @sound(--march) as &drums at 6s for 13.4s;
    @sound(--ting, notes: "A5") at 20.4s;
    @sound(--ting, notes: "D6") at 21.1s;
  }
}

.shot {
  @on &.clip {
    &camera { posZ: 6 -> 2 at 22% -> -12 at 62% -> 6.4 at 76% -> 5.6; posY: 1.7 -> 2.6 at 22% -> 1.4 at 62% -> 1.55 at 76%; posX: 0 -> 0.8 at 40% -> 0 at 76%; lookZ: -10 -> -14 at 40% -> -24 at 62% -> 3.2 at 76%; lookY: 2 -> 3 at 40% -> 2.4 at 62% -> 1.5 at 76%; fov: 36 -> 32 at 62% -> 24 at 78% -> 20; }
    &coin   { rotateZ: 0 -> 1080 at 88% -> 1080; posY: 1.5 -> 1.5 at 70% -> 1.55; easing: --ease-out-cubic; }
    &glint  { intensity: 0 -> 0 at 74% -> 54 at 80% -> 27; }
    &sun    { intensity: 2.2 -> 1.2; }
    &fire1  { emissive: 0.625 -> 0.812 at 5% -> 0.525 at 11% -> 0.75 at 17% -> 0.575 at 26% -> 0.775 at 34% -> 0.55 at 45% -> 0.8 at 58% -> 0.6 at 70% -> 0.75 at 85% -> 0.625; }
    &fire2  { emissive: 0.625 -> 0.55 at 7% -> 0.775 at 14% -> 0.575 at 22% -> 0.8 at 31% -> 0.6 at 41% -> 0.75 at 52% -> 0.525 at 63% -> 0.787 at 77% -> 0.625; }
    &blaze1 { intensity: 27 -> 34 at 5% -> 23 at 11% -> 32 at 17% -> 25 at 26% -> 33 at 34% -> 24 at 45% -> 34 at 58% -> 26 at 70% -> 31 at 85% -> 27; }
    &blaze2 { intensity: 27 -> 23 at 7% -> 33 at 14% -> 25 at 22% -> 34 at 31% -> 26 at 41% -> 32 at 52% -> 23 at 63% -> 33 at 77% -> 27; }
  }
}

@form motion --subtitle { opacity: 0 -> 1 at 14% -> 1 at 84% -> 0; translate-y: 10px -> 0 at 20%; }
.card { @on &.clip { opacity: 0 -> 1 at 22% -> 1 at 78% -> 0; letter-spacing: 0.3em -> 0.12em at 60%; easing: --ease-out-expo; } }
.s1 { @on &.clip --subtitle; }
.s2 { @on &.clip --subtitle; }
.s3 { @on &.clip --subtitle; }
.coin-card { @on &.clip { opacity: 0 -> 1 at 20% -> 1 at 85% -> 0; translate-x: 30px -> 0 at 30%; easing: --ease-out-expo; } }
