// GH-12: `@stage` can bind to ANY selector, including one that only ever
// matches a `<div>`, with ZERO compile-time diagnostic.
//
// The `%scope element(canvas)` constraint exists to refuse this at compile
// time. TODAY (before W6) this page compiles clean with `check`/`build` and
// fails only at runtime: three.js throws inside three-stage's try/catch
// because `canvas.getContext` does not exist on a div, and nothing renders.
//
// AFTER the fix: `check`/`build` on this page MUST emit a compile error
// (E0963) naming the required element `<canvas>`.
@import "stdlib/3d"

body { margin: 0; }

div.hero {
  width: 100%;
  height: 100vh;

  @stage(camZ: 5) {
    @light(type: directional, intensity: 2)
  }
}
