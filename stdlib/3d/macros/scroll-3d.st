/// Scroll-3D Macro — drive the stage from page scroll.
///
/// `@scroll-3d(drive: …)` maps page-scroll progress onto a stage target with
/// eased smoothing. Place inside `@stage`.
///
/// drive targets:
///   morph   - the @particles field's shape morph (0 → to)
///   dollyZ  - camera Z position (from → to): fly toward/through the scene
///   spinY   - the most recent @object's Y rotation (0 → to)
///   camY    - camera Y position (from → to)
///
/// @example scroll morphs the particle field
/// @stage { @particles(count: 30000) @scroll-3d(drive: morph, to: 3) }
///
/// @example scroll flies the camera through a tunnel
/// @stage(camZ: 0) { @scroll-3d(drive: dollyZ, from: 0, to: -40) }
///
/// @param drive  - morph|dollyZ|spinY|camY (default morph)
/// @param from   - value at scroll 0 (dollyZ/camY); default 0
/// @param to     - value at scroll 1; default 1
/// @param ease   - per-frame smoothing 0..1 (default 0.08)

%macro scroll-3d {

  %scope selector

  %form {
    @scroll-3d(
      drive: $drive:string = "morph",
      from: $from:number = 0,
      to: $to:number = 1,
      ease: $ease:number = 0.08
    )
  }

  %binds {
    three-scroll(&self,
      drive: $drive,
      from: $from,
      to: $to,
      ease: $ease
    ) -> {
      $progress
    }
  }
}
