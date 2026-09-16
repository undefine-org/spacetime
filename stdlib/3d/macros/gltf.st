/// GLTF Macro — load a real 3D model into the stage.
///
/// `@gltf(src: …)` loads a .glb/.gltf model, auto-centers and scales it to frame
/// nicely, and adds it to the parent `@stage`'s scene. Pair with `@light` / `@env`
/// for studio lighting and `@orbit` for inspection.
///
/// @example a studio model viewer
/// @stage(camZ: 4) {
///   @env(preset: studio)
///   @light directional(intensity: 2)
///   @gltf(src: "/models/helmet.glb", size: 2.4, spinY: 0.2)
///   @orbit(damping: 0.08)
/// }
///
/// @param src        - URL/path to the model (.glb/.gltf), OR a `$signal`/`$signal.field`
///                      reference (FEAT-153/BUG-159) — when reactive, the model HOT-SWAPS
///                      (old mesh disposed, new one loaded + centered) every time the
///                      referenced signal changes. See three-gltf.st's `resolveReactive`.
/// @param size       - target fit size (0 = keep native scale); default 2
/// @param posX/Y/Z   - position offset
/// @param spinY      - auto-rotation speed (radians/second)
///
/// NOTE: models must be exported WITHOUT Draco compression (the loader's Draco
/// path is excluded from this build — see three-gltf.st).

%macro gltf {

  %scope selector

  %form {
    @gltf(
      src: $src:expr = "",
      size: $size:number = 2,
      posX: $posX:number = 0,
      posY: $posY:number = 0,
      posZ: $posZ:number = 0,
      spinY: $spinY:number = 0
    )
  }

  %binds {
    three-gltf(&self,
      src: $src,
      size: $size,
      posX: $posX,
      posY: $posY,
      posZ: $posZ,
      spinY: $spinY
    ) -> {
      $loaded,
      $progress,
      $error
    }
  }
}
