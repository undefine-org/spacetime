// GH-15: same intent as in-order.st but `detail` written BEFORE `size`, both
// still explicitly named. Issue claims `size: 1.4` silently drops to default 1
// purely because size is declared before detail in the %form.
@import "stdlib/3d"

body { margin: 0; }

.hero {
  @stage(camZ: 5) {
    @object(shape: "sphere", detail: 6, size: 1.4)
  }
}
