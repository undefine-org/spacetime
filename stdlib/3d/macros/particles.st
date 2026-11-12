/// Particles Macro — a morphing GPU particle field.
///
/// `@particles(…)` adds a glowing point cloud to the stage that morphs between
/// built-in shapes (sphere → torus-knot → galaxy → wave grid) as its `morph`
/// value moves from 0 to 3. Drive `morph` from scroll with `@scroll-3d` to get a
/// scroll-controlled particle transition. Pair with `@post(bloomStrength: …)` for
/// the signature glow.

%macro particles {

  %scope selector

  %form {
    @particles(
      /// particle count
      count: $count:number = 24000,
      /// point size
      size: $size:number = 0.02,
      /// cloud radius
      radius: $radius:number = 2,
      /// particle color
      color: $color:color = white,
      /// initial morph position 0..3
      morph: $morph:number = 0,
      /// auto-rotation speed (radians/second, default 0.05)
      spin: $spin:number = 0.05
    )
  }

  %binds {
    three-particles(&self, ...) -> {
      $ready,
      $shapes
    }
  }

  %example "a scroll-morphing particle hero" {
    @stage(camZ: 6) {
      @particles(count: 30000, color: #88aaff, spin: 0.04)
      @scroll-3d(drive: morph, to: 3)
      @post(bloomStrength: 0.9, bloomThreshold: 0.2)
    }
  }
}
