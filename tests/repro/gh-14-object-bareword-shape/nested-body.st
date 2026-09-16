// GH-14 (part 2): the nested-body form from MODULE.st's doc example:
// `@object icosahedron(detail: 6, size: 1.4) { material: glass(...) spin: [...] }`
// The %form reserves `{ $children* }` for child DIRECTIVES, not key:value lines.
@import "stdlib/3d"

body { margin: 0; }

.hero {
  @stage(camZ: 5) {
    @object icosahedron(detail: 6, size: 1.4) {
      material: glass(roughness: 0.04, transmission: 1, ior: 1.5, thickness: 1.2)
      spin: [0, 0.3, 0]
    }
  }
}
