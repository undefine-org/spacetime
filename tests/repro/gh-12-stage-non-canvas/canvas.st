// GH-12 CONTROL: `@stage` on a `<canvas>` selector is the CORRECT spelling and
// must still compile (a constraint that rejects everything is not a fix). This
// page must produce ZERO element-scope diagnostics after the fix.
@import "stdlib/3d"

body { margin: 0; }

canvas.hero {
  width: 100%;
  height: 100vh;

  @stage(camZ: 5) {
    @light(type: directional, intensity: 2)
  }
}
