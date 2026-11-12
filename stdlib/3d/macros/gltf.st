/// GLTF Macro — load a real 3D model into the stage.
///
/// `@gltf(src: …)` loads a .glb/.gltf model, auto-centers and scales it to frame
/// nicely, and adds it to the parent `@stage`'s scene. Pair with `@light` / `@env`
/// for studio lighting and `@orbit` for inspection.
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
      /// URL/path to the model (.glb/.gltf), OR a `$signal`/`$signal.field`
      /// reference (FEAT-153/BUG-159) — when reactive, the model HOT-SWAPS
      /// (old mesh disposed, new one loaded + centered) every time the
      /// referenced signal changes. See three-gltf.st's `resolveReactive`.
      src: $src:expr = "",
      /// the animation playing: one of the model file's clips
      clip: $clip:model_clip = "",
      /// target fit size (0 = keep native scale)
      size: $size:number = 2,
      scale: $scale:number = 0,
      rotateY: $rotateY:number = 0,
      /// position offset
      posX: $posX:number = 0,
      posY: $posY:number = 0,
      posZ: $posZ:number = 0,
      /// auto-rotation speed (radians/second)
      spinY: $spinY:number = 0
    ) {
      $children*
    }
  }

  // `pos: 0 3 -7` is `posX: 0, posY: 3, posZ: -7` (one value sets all three);
  // the later of a shorthand and a longhand wins, as in CSS
  %shorthand pos { posX posY posZ }

  %binds {
    three-gltf(&self, name: $named, ...) -> {
      $loaded,
      $progress,
      $error
    }
  }

  %registers part($named) {
    kind: gltf
  }

  %template {
    $children
  }

  %example "a studio model viewer" {
    @import "stdlib/3d"
    canvas.viewer {
      @stage(camZ: 4) {
        @env(preset: studio)
        @light (type: directional, intensity: 2)
        @gltf (src: "models/helmet.glb", size: 2.4, spinY: 0.2)
        @orbit(damping: 0.08)
      }
    }
    <canvas class="viewer"></canvas>
  }

  %example "a named model the score can direct" {
    @import "stdlib/3d"
    canvas.world {
      @stage(camZ: 6) {
        @light (type: directional, intensity: 2)
        @gltf &robot (src: "models/robot.glb", clip: "Idle", scale: 0.42)
      }
    }
    <canvas class="world"></canvas>
  }
}
