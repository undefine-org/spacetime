// GH-12 UNKNOWABLE CASE: `.hero { @stage }` — the selector carries NO element
// evidence, so the required element cannot be determined at compile time. Per
// the W6 decision (DOCUMENTED in evidence.txt): this is a WARNING (W0963), not
// an error — the element may be a `<canvas>` inserted at runtime. It must NOT
// pass silently.
@import "stdlib/3d"

body { margin: 0; }

.hero {
  width: 100%;
  height: 100vh;

  @stage(camZ: 5) {
    @light(type: directional, intensity: 2)
  }
}
