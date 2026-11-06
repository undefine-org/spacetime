@import "stdlib/3d"
@import "stdlib/film"

// ═══════════════════════════════════════════════════════════════════════════
// DISCO — 20 seconds on a light-up floor: a dancing character, rings on the
// bar, sweeping colour, an orbiting camera and a 118 bpm score. No JavaScript.
//
//   cargo run -- render demos/film-disco/ --engine raea -o disco.mp4
//   cargo run -- serve demos/film-disco/
//
// The dancer is RobotExpressive (Tomás Laulhé / Quaternius, CC0 1.0).
// ═══════════════════════════════════════════════════════════════════════════


// Fonts ship with the film (SIL OFL 1.1, Google Fonts), so every engine draws the same letters.
@font("Space Grotesk") { src: url("fonts/SpaceGrotesk.woff2") format("woff2"); font-weight: 300 700; font-style: normal; font-display: block; }
@font("Syne") { src: url("fonts/Syne.woff2") format("woff2"); font-weight: 400 800; font-style: normal; font-display: block; }

<div class="reel">
  <div class="shot"><canvas class="world"></canvas></div>
  <h1 class="t1">ONE MORE</h1>
  <h1 class="t2">SONG</h1>
  <div class="end"><h2>Spacetime</h2><p>notes · light · motion — one score</p></div>
</div>

body { margin: 0; background: #000; }
.reel { position: fixed; inset: 0; overflow: hidden; }
.shot, .world { position: absolute; inset: 0; width: 100%; height: 100%; }
.t1, .t2 { position: absolute; left: 0; right: 0; margin: 0; text-align: center; color: #fff; font: 800 13vw/0.9 "Syne", "Space Grotesk", sans-serif; letter-spacing: 0.04em; opacity: 0; }
.t1 { top: 16vh; }
.t2 { bottom: 16vh; color: #ff3ea5; }
.end { position: absolute; inset: 0; display: grid; place-content: center; text-align: center; color: #fff; background: #07040c; opacity: 0; }
.end h2 { margin: 0; font: 800 7vw/1 "Syne", "Space Grotesk", sans-serif; }
.end p { margin: 1.2vw 0 0; font: 500 1.4vw/1 "Space Grotesk", sans-serif; letter-spacing: 0.3em; text-transform: uppercase; color: #3ef0ff; }

canvas.world {
  @stage(fov: 40, camX: 0, camY: 2.6, camZ: 9, shadows: "soft", background: "#07040c", bloom: 0.8, exposure: 4.8) {
    @light(type: "ambient", intensity: 0.039, color: #8a7bff)
    @light &key (type: "spot", intensity: 268, color: #ffffff, posX: 0, posY: 9, posZ: 2, angle: 28, castShadow: true)
    @light &pink (type: "point", intensity: 125, color: #ff3ea5, posX: -4, posY: 2.5, posZ: 2, distance: 14)
    @light &cyan (type: "point", intensity: 125, color: #3ef0ff, posX: 4, posY: 2.5, posZ: -2, distance: 14)
    @fog(color: #07040c, near: 10, far: 26)
    @grid &floor (cols: 12, rows: 12, tile: 0.9, gap: 0.08, pattern: "ripple", glow: 0.14, speed: 1.96)
    @object &ring1 (shape: "torus", size: 0.2, rotateX: 90, posY: 0.08, color: #000000, emissive: #ff3ea5, emissiveIntensity: 0.375, roughness: 0.4, metalness: 0)
    @object &ring2 (shape: "torus", size: 0.2, rotateX: 90, posY: 0.08, color: #000000, emissive: #3ef0ff, emissiveIntensity: 0.375, roughness: 0.4, metalness: 0)
    @object &ring3 (shape: "torus", size: 0.2, rotateX: 90, posY: 0.08, color: #000000, emissive: #ffe23e, emissiveIntensity: 0.375, roughness: 0.4, metalness: 0)
    @gltf &dancer (src: "models/robot.glb", clip: "Idle", scale: 0.5)
  }
}

@form instrument --kick  { wave: sine; pitch: 165hz -> 45hz at 22%; gain: 1 -> 0; for: 380ms; }
@form instrument --clap  { wave: noise; cutoff: 2400 -> 900; gain: 0.6 -> 0; for: 160ms; reverb: 0.35; }
@form instrument --hat   { wave: noise; highpass: 8000; gain: 0.22 -> 0; for: 50ms; }
@form instrument --bass  { wave: saw; cutoff: 900 -> 160; gain: 0 -> 0.8 at 3% -> 0.5 at 40% -> 0; for: 230ms; }
@form instrument --stab  { wave: square; voices: 2; detune: 10; cutoff: 3200 -> 700; gain: 0 -> 0.32 at 2% -> 0; for: 320ms; reverb: 0.5; }
@form instrument --riser { wave: noise; highpass: 400 -> 4000; gain: 0 -> 0.5 at 90% -> 0; }

@form pattern --four($tempo = 118bpm, $step = 16) {
  --kick: x . . . x . . . x . . . x . . . ;
  --clap: . . . . x . . . . . . . x . . . ;
  --hat:  . . x . . . x . . . x . . . x x ;
  --bass: F2 . F2 F3 . F2 Ab2 . F2 . F2 F3 . Eb2 C3 . ;
}
@form pattern --chords($tempo = 118bpm, $step = 8) {
  --stab: . F4 . . . Ab4 . . . C5 . . . Eb5 . . ;
}

.reel {
  @score &.time(20s) as &film {
    &shot at 0s for 20s;
    &t1   at 0.5s for 3.3s;
    &t2   at 1.2s for 2.6s;
    &end  at 18.4s for 1.6s;
    @sound(--riser) at 0s for 4.07s;
    @sound(--four) as &drums at 4.07s for 14.24s;
    @sound(--chords) as &keys at 8.14s for 10.17s;
    @sound(--kick) at 4.07s;
  }
}

.shot {
  @on &.clip {
    &camera { posX: 0 -> 0 at 20% -> 7 at 40% -> 0 at 60% -> -7 at 80% -> 0; posZ: 9 -> 6 at 20% -> 2 at 40% -> -6 at 60% -> 2 at 80% -> 7; posY: 2.6 -> 1.7 at 20% -> 3.4 at 60% -> 2.2; lookY: 1 -> 1.1; fov: 40 -> 34 at 20% -> 44 at 60% -> 36; }
    &dancer { clip: "Idle" -> "Dance" at 20.35% --steps(1) -> "Wave" at 88% --steps(1); }
    &floor  { pulse: 0 -> 0.15 at 19% -> 1 at 20.4%; }
    &ring1  { scale: 0.2 -> 0.2 at 20.35% -> 5.5 at 28.99% -> 0.2 at 30.52% -> 5.5 at 39.16% -> 0.2 at 40.69% -> 5.5 at 49.33% -> 0.2 at 50.86% -> 5.5 at 59.50% -> 0.2 at 61.03% -> 5.5 at 69.67% -> 0.2 at 71.20% -> 5.5 at 79.84% -> 0.2 at 81.37% -> 5.5 at 90.01%; opacity: 0 -> 1 at 20.35% -> 0 at 28.99% -> 1 at 30.52% -> 0 at 39.16% -> 1 at 40.69% -> 0 at 49.33% -> 1 at 50.86% -> 0 at 59.50% -> 1 at 61.03% -> 0 at 69.67% -> 1 at 71.20% -> 0 at 79.84% -> 1 at 81.37% -> 0 at 90.01%; }
    &ring2  { scale: 0.2 -> 0.2 at 23.74% -> 5.5 at 32.38% -> 0.2 at 33.91% -> 5.5 at 42.55% -> 0.2 at 44.08% -> 5.5 at 52.72% -> 0.2 at 54.25% -> 5.5 at 62.89% -> 0.2 at 64.42% -> 5.5 at 73.06% -> 0.2 at 74.59% -> 5.5 at 83.23% -> 0.2 at 84.76% -> 5.5 at 93.40%; opacity: 0 -> 1 at 23.74% -> 0 at 32.38% -> 1 at 33.91% -> 0 at 42.55% -> 1 at 44.08% -> 0 at 52.72% -> 1 at 54.25% -> 0 at 62.89% -> 1 at 64.42% -> 0 at 73.06% -> 1 at 74.59% -> 0 at 83.23% -> 1 at 84.76% -> 0 at 93.40%; }
    &ring3  { scale: 0.2 -> 0.2 at 27.13% -> 5.5 at 35.77% -> 0.2 at 37.30% -> 5.5 at 45.94% -> 0.2 at 47.47% -> 5.5 at 56.11% -> 0.2 at 57.64% -> 5.5 at 66.28% -> 0.2 at 67.81% -> 5.5 at 76.45% -> 0.2 at 77.98% -> 5.5 at 86.62% -> 0.2 at 88.15% -> 5.5 at 96.79%; opacity: 0 -> 1 at 27.13% -> 0 at 35.77% -> 1 at 37.30% -> 0 at 45.94% -> 1 at 47.47% -> 0 at 56.11% -> 1 at 57.64% -> 0 at 66.28% -> 1 at 67.81% -> 0 at 76.45% -> 1 at 77.98% -> 0 at 86.62% -> 1 at 88.15% -> 0 at 96.79%; }
    &pink   { posX: -4 -> 4 at 50% -> -4; posZ: 2 -> -2 at 50% -> 2; }
    &cyan   { posX: 4 -> -4 at 50% -> 4; posZ: -2 -> 2 at 50% -> -2; }
    &key    { intensity: 27 -> 27 at 19% -> 143 at 20.4%; }
  }
}

.t1 { @on &.clip { opacity: 0 -> 1 at 20% -> 1 at 80% -> 0; letter-spacing: 0.3em -> 0.04em at 40%; easing: --ease-out-expo; } }
.t2 { @on &.clip { opacity: 0 -> 1 at 25% -> 1 at 75% -> 0; translate-y: 30px -> 0 at 40%; easing: --ease-out-expo; } }
.end { @on &.clip { opacity: 0 -> 1 at 40%; } }
