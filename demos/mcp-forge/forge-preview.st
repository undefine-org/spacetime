/// Forge Deck — GUEST stage (FEAT-151).
///
/// Composed into the mission-control host's "stage" region via
/// `st_mount { host: <mission-control instance>, region: "stage", input: {...} }`.
/// Region-compose is TEMPLATES-ONLY (FUP-061 v1) — the guest's whole surface is
/// one `@template &main(...)` whose named params ARE the mount `input` fields
/// (BUG-113: mount_into_host seeds region_mount_input from `input`, coerced to
/// strings, and the bundle renders the template with those as bound args).
/// Re-mounting with a new `glbPath` (cache-busted) hot-swaps the viewport to the
/// latest Blender export IN PLACE — one DOM, one three.js runtime, no reload.
///
/// Mount input shape: { "glbPath": "models/step-3.glb?v=3", "stepLabel": "Raise Boss" }
///
/// No hand-authored JavaScript: the entire viewport is stdlib/3d's @-surface.

@import "stdlib/3d";

@template &main($glbPath string = "models/step-0.glb", $stepLabel string = "Blank Disc") {
  <main class="forge-preview">
    <canvas class="forge-stage">
      @stage(camY: 1.2, camZ: 3.6, fov: 42, tone: "aces", exposure: 0.75) {
        @env(preset: "studio", intensity: 0.55)
        @light(type: "directional", intensity: 1.0, color: #ffdfb0, posX: 4, posY: 3, posZ: 3)
        @light(type: "directional", intensity: 0.5, color: #8fb4ff, posX: -4, posY: 1, posZ: -3)
        @light(type: "ambient", intensity: 0.16)
        @gltf(src: $glbPath, size: 2.2, spinY: 0.18)
        @orbit(damping: 0.09, autoRotate: 0.12, enableZoom: true, minDistance: 2, maxDistance: 8)
        @post(bloomStrength: 0.05, bloomRadius: 0.2, bloomThreshold: 0.97)
      }
    </canvas>
    <p class="forge-step-label">`$stepLabel`</p>
  </main>
}

.forge-preview {
  position: relative;
  width: 100%;
  height: 100%;
  min-height: 480px;
  background: radial-gradient(120% 100% at 50% 38%, #17130c 0%, #0d0b07 55%, #0a0906 100%);
}

.forge-stage {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
}

.forge-step-label {
  position: absolute;
  left: 20px;
  bottom: 16px;
  margin: 0;
  padding: 6px 14px;
  border-radius: 999px;
  background: rgba(36, 30, 25, 0.55);
  color: #f0e6d6;
  font-family: Inter, ui-sans-serif, system-ui, sans-serif;
  font-size: 12px;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  pointer-events: none;
}
