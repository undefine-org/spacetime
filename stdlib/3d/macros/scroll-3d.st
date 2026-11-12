/// Scroll-3D Macro — drive the stage from page scroll.
///
/// `@scroll-3d(drive: …)` maps page-scroll progress onto a stage target with
/// eased smoothing. Place inside `@stage`.

/// What page scroll moves.
%capture_type scroll_drive {
    /// the `@particles` field's shape morph (0 → to)
    "morph"
    /// the camera's Z position (from → to): fly toward or through the scene
  | "dollyZ"
    /// the most recent `@object`'s Y rotation (0 → to)
  | "spinY"
    /// the camera's Y position (from → to)
  | "camY"
}

%macro scroll-3d {

  %scope selector

  %form {
    @scroll-3d(
      /// what page scroll moves
      drive: $drive:scroll_drive = morph,
      /// value at scroll 0 (dollyZ/camY)
      from: $from:number = 0,
      /// value at scroll 1
      to: $to:number = 1,
      /// per-frame smoothing 0..1
      ease: $ease:number = 0.08
    )
  }

  %binds {
    three-scroll(&self, ...) -> {
      $progress
    }
  }

  %example "scroll morphs the particle field" {
    @stage { @particles(count: 30000) @scroll-3d(drive: morph, to: 3) }
  }

  %example "scroll flies the camera through a tunnel" {
    @stage(camZ: 0) { @scroll-3d(drive: dollyZ, from: 0, to: -40) }
  }
}
