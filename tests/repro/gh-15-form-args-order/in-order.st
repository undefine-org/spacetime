// GH-15: named args to a %form macro must appear in declaration order —
// otherwise earlier-declared args silently default. %form order for @object:
// shape, size, detail, ... This file passes args IN declaration order.
@import "stdlib/3d"

body { margin: 0; }

.hero {
  @stage(camZ: 5) {
    @object(shape: "sphere", size: 1.4, detail: 6)
  }
}
