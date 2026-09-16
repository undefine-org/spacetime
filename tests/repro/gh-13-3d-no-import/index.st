// GH-13: @stage/@object in selector scope WITHOUT `@import "stdlib/3d"`.
// Expectation (BUG-268): unknown primitives are now E0956 build failures.
.hero {
  width: 100%;
  height: 100vh;

  @stage(camZ: 5) {
    @object(shape: "icosahedron", size: 1)
  }
}
