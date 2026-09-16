// GH-14: documented bareword-shape call style `@object torusknot(size: 1.2, ...)`
// — the shape token is a leading bareword, but the %form declares a flat
// paren-arg list with no leading bareword. Issue claims: silent drop to
// default "icosahedron" geometry, zero diagnostic.
@import "stdlib/3d"

body { margin: 0; }

.hero {
  @stage(camZ: 5) {
    @object torusknot(size: 1.2, color: #c0c5ff, roughness: 0.15, metalness: 1, spinY: 0.4)
  }
}
