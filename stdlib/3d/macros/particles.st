/// Particles Macro — a morphing GPU particle field.
///
/// `@particles(…)` adds a glowing point cloud to the stage that morphs between
/// built-in shapes (sphere → torus-knot → galaxy → wave grid) as its `morph`
/// value moves from 0 to 3. Drive `morph` from scroll with `@scroll-3d` to get a
/// scroll-controlled particle transition. Pair with `@post(bloomStrength: …)` for
/// the signature glow.
///
/// @example a scroll-morphing particle hero
/// @stage(camZ: 6) {
///   @particles(count: 30000, color: #88aaff, spin: 0.04)
///   @scroll-3d(drive: morph, to: 3)
///   @post(bloomStrength: 0.9, bloomThreshold: 0.2)
/// }
///
/// @param count   - particle count (default 24000)
/// @param size    - point size (default 0.02)
/// @param radius  - cloud radius (default 2)
/// @param color   - particle color (default white)
/// @param morph   - initial morph position 0..3 (default 0)
/// @param spin    - auto-rotation speed (radians/second, default 0.05)

%macro particles {

  %scope selector

  %form {
    @particles(
      count: $count:number = 24000,
      size: $size:number = 0.02,
      radius: $radius:number = 2,
      color: $color:color = white,
      morph: $morph:number = 0,
      spin: $spin:number = 0.05
    )
  }

  %binds {
    three-particles(&self,
      count: $count,
      size: $size,
      radius: $radius,
      color: $color,
      morph: $morph,
      spin: $spin
    ) -> {
      $ready,
      $shapes
    }
  }
}
