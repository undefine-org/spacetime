/// Stage Macro — the declarative entry point for 3D.
///
/// `@stage { … }` on a `<canvas>` sets up the three.js renderer, scene, camera,
/// and render loop, then hosts child directives (`@object`, `@light`, `@env`,
/// `@orbit`, `@parallax`, `@post`, `@gltf`, `@particles`). Authors never touch
/// three.js — the engine lives behind this surface.
///
/// @example a lit, orbitable object
/// canvas.hero {
///   @stage(fov: 50, camZ: 5, tone: aces) {
///     @light directional(intensity: 2)
///     @object torusknot(size: 1) {
///       material: standard(color: #c0c5ff, roughness: 0.2, metalness: 0.9)
///       @orbit
///     }
///   }
/// }
///
/// @param fov       - camera vertical field of view in degrees (default 50)
/// @param near/far  - camera clip planes
/// @param camX/Y/Z  - initial camera position (default 0,0,5)
/// @param exposure  - tone-mapping exposure (default 1.0)
/// @param tone      - "aces" | "linear" | "reinhard" | "none" (default aces)
/// @param alpha     - transparent background (default true)

%macro stage {

  // GH-12 (PLAN-137 W6): a stage ONLY renders into a <canvas>. Declaring the
  // required element as `%scope element(canvas)` makes the compiler refuse a
  // `@stage` bound to a non-canvas selector (div.hero -> E0963) and warn when
  // the element is unknowable (.hero -> W0963) — previously it compiled clean
  // and failed only at runtime (three.js `canvas.getContext` throw on a div).
  %scope selector | element(canvas)

  %form {
    @stage(
      fov: $fov:number = 50,
      near: $near:number = 0.1,
      far: $far:number = 100,
      camX: $camX:number = 0,
      camY: $camY:number = 0,
      camZ: $camZ:number = 5,
      exposure: $exposure:number = 1.0,
      tone: $tone:string = "aces",
      alpha: $alpha:bool = true
    ) {
      $children*
    }
  }

  %binds {
    three-stage(&self,
      fov: $fov,
      near: $near,
      far: $far,
      camX: $camX,
      camY: $camY,
      camZ: $camZ,
      exposure: $exposure,
      tone: $tone,
      alpha: $alpha
    ) -> {
      $ready,
      $frame,
      $fps
    }
  }

  %template {
    $children
  }
}
