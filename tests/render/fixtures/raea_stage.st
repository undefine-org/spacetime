// tests/render/fixtures/raea_stage.st — a 3D stage scored like any film
// (PLAN-152): a named red box slides across the frame, driven by a score
// channel. Under raea the stage's parts are `data-3d` markup drawn by Bevy
// under the page; before PLAN-152 a stage drew nothing there (no WebGL), so
// the frame stayed empty.

@version 2026-06-09;
@import "stdlib/3d"

<div class="film"><canvas class="world"></canvas></div>

body { margin: 0; }
.film, .world { position: absolute; inset: 0; width: 100%; height: 100%; }
canvas.world {
  @stage(fov: 40, camZ: 8, background: "#ffffff") {
    @light(type: "ambient", intensity: 2)
    @object &box (shape: "box", size: 1.4, color: #ff0000, emissive: #ff0000, emissiveIntensity: 1, roughness: 1, metalness: 0, posX: -3)
  }
}
.film { @score &.time(1s) { &film at 0s for 1s; } }
.film { @on &.clip { &box { posX: -3 -> 3; } } }
