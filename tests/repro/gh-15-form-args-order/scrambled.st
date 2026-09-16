// GH-15 stress: heavily reordered named args — shape NOT first, metalness
// pulled way forward. All named, none in %form declaration order.
@import "stdlib/3d"

body { margin: 0; }

.hero {
  @stage(camZ: 5) {
    @object(metalness: 0.9, shape: "sphere", detail: 6, size: 1.4, roughness: 0.15)
  }
}
