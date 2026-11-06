@import "stdlib/3d"
@import "stdlib/film"

// ═══════════════════════════════════════════════════════════════════════════
// ROBOT DAY — a 24-second film: a lit 3D plaza, a rigged character, a score
// with music, and not one line of JavaScript (PLAN-152).
//
//   cargo run -- render demos/film-robot-day/ --engine raea -o robot-day.mp4
//   cargo run --features cdp -- render demos/film-robot-day/ -o robot-day.mp4   # Chromium
//   cargo run -- serve demos/film-robot-day/                                   # live, with sound
//
// The robot is RobotExpressive (Tomás Laulhé / Quaternius, CC0 1.0).
// ═══════════════════════════════════════════════════════════════════════════

<div class="reel">
  <div class="shot">
    <canvas class="world"></canvas>
  </div>
  <div class="card"><h1>ROBOT DAY</h1><p>a film in Spacetime</p></div>
  <p class="sub s1">Every evening it crossed the plaza.</p>
  <p class="sub s2">It said hello to whoever was there.</p>
  <p class="sub s3">Some days the ground came up to meet it.</p>
  <p class="sub s4">Then it got back up, and watched the lights come on.</p>
  <div class="end"><h2>Spacetime</h2><p>one source · rendered by raea</p></div>
</div>

body { margin: 0; background: #000; }
.reel { position: fixed; inset: 0; overflow: hidden; font-family: "Space Grotesk", "Helvetica Neue", sans-serif; }
.shot, .world { position: absolute; inset: 0; width: 100%; height: 100%; }

.card { position: absolute; left: 7vw; top: 30vh; color: #fff8ec; opacity: 0; }
.card h1 { margin: 0; font: 800 11vw/0.9 "Syne", "Space Grotesk", sans-serif; letter-spacing: 0.02em; }
.card p { margin: 1.4vw 0 0; font: 500 1.5vw/1 "Space Grotesk", sans-serif; letter-spacing: 0.4em; text-transform: uppercase; color: #ffcf8a; }

.sub { position: absolute; left: 0; right: 0; bottom: 7vh; margin: 0; text-align: center; color: #fff; font: 500 2.1vw/1.3 "Space Grotesk", sans-serif; text-shadow: 0 2px 18px rgba(0,0,0,0.6); opacity: 0; }

.end { position: absolute; inset: 0; display: grid; place-content: center; text-align: center; color: #fff; background: #07080c; opacity: 0; }
.end h2 { margin: 0; font: 800 7vw/1 "Syne", "Space Grotesk", sans-serif; }
.end p { margin: 1.2vw 0 0; font: 500 1.4vw/1 "Space Grotesk", sans-serif; letter-spacing: 0.3em; text-transform: uppercase; color: #8ab4ff; }

// ═══ The world ═══════════════════════════════════════════════════════════════
canvas.world {
  @stage(fov: 34, camX: 0, camY: 3.2, camZ: 14, shadows: "soft", background: "#1d1a2a", bloom: 0.6) {
    @light &sky (type: "ambient", intensity: 0.35, color: #b9b3ff)
    @light &sun (type: "directional", intensity: 2.4, color: #ffb47a, posX: -6, posY: 5, posZ: 3)
    @fog(color: #1d1a2a, near: 12, far: 34)
    @object &ground (shape: "plane", size: 18, rotateX: -90, color: #3a3448, roughness: 0.92, metalness: 0)
    @object &path (shape: "plane", size: 1.4, rotateX: -90, posY: 0.01, color: #6c6278, roughness: 0.8, metalness: 0)
    // two rows of lamps: a post, a globe and its light, each named so the score can light it
    @object (shape: "cylinder", size: 0.06, scaleY: 20, posX: -2.6, posY: 1.2, posZ: -4, color: #2b2833)
    @object (shape: "cylinder", size: 0.06, scaleY: 20, posX: 2.6, posY: 1.2, posZ: -4, color: #2b2833)
    @object (shape: "cylinder", size: 0.06, scaleY: 20, posX: -2.6, posY: 1.2, posZ: 1, color: #2b2833)
    @object (shape: "cylinder", size: 0.06, scaleY: 20, posX: 2.6, posY: 1.2, posZ: 1, color: #2b2833)
    @object &globe1 (shape: "sphere", size: 0.22, posX: -2.6, posY: 2.5, posZ: -4, color: #fff1d0, emissive: #ffd58a, emissiveIntensity: 0)
    @object &globe2 (shape: "sphere", size: 0.22, posX: 2.6, posY: 2.5, posZ: -4, color: #fff1d0, emissive: #ffd58a, emissiveIntensity: 0)
    @object &globe3 (shape: "sphere", size: 0.22, posX: -2.6, posY: 2.5, posZ: 1, color: #fff1d0, emissive: #ffd58a, emissiveIntensity: 0)
    @object &globe4 (shape: "sphere", size: 0.22, posX: 2.6, posY: 2.5, posZ: 1, color: #fff1d0, emissive: #ffd58a, emissiveIntensity: 0)
    @light &lamp1 (type: "point", intensity: 0, color: #ffcf8a, posX: -2.6, posY: 2.3, posZ: -4, distance: 9)
    @light &lamp2 (type: "point", intensity: 0, color: #ffcf8a, posX: 2.6, posY: 2.3, posZ: -4, distance: 9)
    @light &lamp3 (type: "point", intensity: 0, color: #ffcf8a, posX: -2.6, posY: 2.3, posZ: 1, distance: 9)
    @light &lamp4 (type: "point", intensity: 0, color: #ffcf8a, posX: 2.6, posY: 2.3, posZ: 1, distance: 9)
    @gltf &robot (src: "models/robot.glb", clip: "Idle", scale: 0.42, posZ: -7)
  }
}

// ═══ The score: one clock — picture, words and music on it ═══════════════════
@form instrument --pad   { wave: saw; voices: 3; detune: 14; gain: 0 -> 0.45 at 25% -> 0.45 at 80% -> 0; cutoff: 700 -> 1600 at 60% -> 900; reverb: 0.7; }
@form instrument --kick  { wave: sine; pitch: 150hz -> 42hz at 25%; gain: 1 -> 0; for: 420ms; }
@form instrument --hat   { wave: noise; highpass: 7500; gain: 0.25 -> 0; for: 70ms; }
@form instrument --pluck { wave: square; gain: 0 -> 0.5 at 3% -> 0.1 at 30% -> 0; cutoff: 2600 -> 400; for: 360ms; reverb: 0.4; }
@form instrument --thud  { wave: noise; cutoff: 260 -> 60; gain: 1 -> 0; for: 700ms; reverb: 0.3; }
@form instrument --chime { wave: sine; gain: 0 -> 0.4 at 2% -> 0; for: 2600ms; reverb: 0.9; }

@form pattern --beat($tempo = 96bpm, $step = 16) {
  --kick:  x . . . . . . . x . x . . . . . ;
  --hat:   . . x . . . x . . . x . . . x x ;
  --pluck: C3 . . G3 . . Bb3 . . G3 . . F3 . Eb3 . ;
}

.reel {
  @score &.time(24s) as &film {
    &shot at 0s for 24s;
    &card at 0.4s for 5.2s;
    &s1   at 6.2s for 4.4s;
    &s2   at 11s for 3.6s;
    &s3   at 14.8s for 3.2s;
    &s4   at 18.4s for 4s;
    &end  at 22.4s for 1.6s;
    @sound(--pad, notes: "C3 Eb3 G3 Bb3") at 0s for 24s;
    @sound(--beat) as &drums at 5.8s for 9.2s;
    @sound(--thud) at 15.3s;
    @sound(--chime, notes: "G5") at 18.9s;
    @sound(--chime, notes: "Eb5") at 19.6s;
    @sound(--chime, notes: "C6") at 20.3s;
  }
}

// ═══ The shot: the camera, the light, the character ══════════════════════════
.shot {
  @on &.clip {
    &camera { posZ: 14 -> 9 at 25% -> 6.5 at 62% -> 7.8; posY: 3.2 -> 2.2 at 25% -> 1.6 at 62% -> 2.6; posX: 0 -> 0.12 at 64% -> -0.1 at 64.6% -> 0 at 65.4% -> 1.4; lookY: 1 -> 1.1 at 62% -> 0.6; fov: 34 -> 30 at 60% -> 36; }
    &robot  { clip: "Idle" -> "Walking" at 25% --steps(1) -> "Wave" at 46% --steps(1) -> "Walking" at 58% --steps(1) -> "Death" at 63.5% --steps(1) -> "Idle" at 77% --steps(1) -> "ThumbsUp" at 86% --steps(1); posZ: -7 -> -7 at 25% -> -1 at 46% -> -1 at 58% -> 0.4 at 63.5%; }
    &sun    { intensity: 2.4 -> 0.25 at 80%; }
    &sky    { intensity: 0.35 -> 0.22 at 80%; }
    &lamp1  { intensity: 0 -> 0 at 78% -> 2.6 at 80%; }
    &globe1 { emissive: 0 -> 0 at 78% -> 2.5 at 80%; }
    &lamp2  { intensity: 0 -> 0 at 81% -> 2.6 at 83%; }
    &globe2 { emissive: 0 -> 0 at 81% -> 2.5 at 83%; }
    &lamp3  { intensity: 0 -> 0 at 84% -> 2.6 at 86%; }
    &globe3 { emissive: 0 -> 0 at 84% -> 2.5 at 86%; }
    &lamp4  { intensity: 0 -> 0 at 84% -> 2.6 at 86%; }
    &globe4 { emissive: 0 -> 0 at 84% -> 2.5 at 86%; }
  }
}

.card { @on &.clip { opacity: 0 -> 1 at 18% -> 1 at 78% -> 0; translate-y: 24px -> 0 at 30%; easing: --ease-out-expo; } }
@form motion --subtitle { opacity: 0 -> 1 at 15% -> 1 at 82% -> 0; translate-y: 10px -> 0 at 20%; }
.s1 { @on &.clip --subtitle; }
.s2 { @on &.clip --subtitle; }
.s3 { @on &.clip --subtitle; }
.s4 { @on &.clip --subtitle; }
.end  { @on &.clip { opacity: 0 -> 1 at 40%; } }
