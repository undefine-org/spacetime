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

  // PLAN-152: a model MAY be named (`@gltf &robot (src: …, clip: "Idle")`) so a
  // score can drive it: `&robot { clip: "Walking" -> "Wave" at 70% --steps(1); posZ: -8 -> 0; }`.
  %capture_type model_name { $name:element }

  %form {
    @gltf $named:model_name? (
      src: $src:expr = "",
      clip: $clip:string = "",
      size: $size:number = 2,
      scale: $scale:number = 0,
      rotateY: $rotateY:number = 0,
      posX: $posX:number = 0,
      posY: $posY:number = 0,
      posZ: $posZ:number = 0,
      spinY: $spinY:number = 0
    ) {
      $children*
    }
  }

  %binds {
    three-gltf(&self,
      name: $named,
      src: $src,
      clip: $clip,
      size: $size,
      scale: $scale,
      rotateY: $rotateY,
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

  %template {
    $children
  }
}
