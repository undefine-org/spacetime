/// Light Macro — a light source in the stage.
///
/// `@light <type>(…)` adds a three.js light to the parent `@stage`'s scene. Type
/// is a leading keyword; parameters are flat and typed.
///
/// @example a sun-like key light plus soft ambient fill
/// @stage {
///   @light ambient(intensity: 0.4)
///   @light directional(intensity: 2.2, posX: 4, posY: 6, posZ: 3)
/// }
///
/// @example a focused spot
/// @light spot(intensity: 60, posY: 8, angle: 24, penumbra: 0.4, distance: 30)
///
/// @param type        - ambient|directional|point|spot (leading keyword)
/// @param color       - light color (default white)
/// @param intensity   - intensity (default 1)
/// @param posX/Y/Z    - position / direction
/// @param targetX/Y/Z - aim point (directional/spot)
/// @param distance/decay - point/spot falloff
/// @param angle/penumbra - spot cone

%macro light {

  // GH-12 note: @light deliberately does NOT take `element(canvas)`. It is a
  // SCENE CHILD (composes against the stage handle), not a canvas-bounded DOM
  // element — the @stage macro alone owns the canvas element constraint.
  %scope selector

  %form {
    @light(
      type: $type:string = "directional",
      color: $color:color = white,
      intensity: $intensity:number = 1.0,
      posX: $posX:number = 3,
      posY: $posY:number = 5,
      posZ: $posZ:number = 4,
      targetX: $targetX:number = 0,
      targetY: $targetY:number = 0,
      targetZ: $targetZ:number = 0,
      distance: $distance:number = 0,
      decay: $decay:number = 2,
      angle: $angle:number = 30,
      penumbra: $penumbra:number = 0.3
    )
  }

  %binds {
    three-light(&self,
      type: $type,
      color: $color,
      intensity: $intensity,
      posX: $posX,
      posY: $posY,
      posZ: $posZ,
      targetX: $targetX,
      targetY: $targetY,
      targetZ: $targetZ,
      distance: $distance,
      decay: $decay,
      angle: $angle,
      penumbra: $penumbra
    ) -> {
      $added
    }
  }
}
