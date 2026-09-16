/// Env Macro — image-based lighting + reflections for the stage.
///
/// `@env(…)` builds a studio environment map (vendored RoomEnvironment) and
/// assigns it as the scene environment, so every PBR / glass material reflects a
/// realistic surrounding. The biggest single upgrade to material realism.
///
/// @example
/// @stage {
///   @env(preset: studio, intensity: 1.1)
///   @object icosahedron(material: glass, transmission: 1)
/// }
///
/// @param preset      - studio (default)
/// @param intensity   - environment lighting intensity (default 1)
/// @param background  - also paint the env as the canvas background (default false)

%macro env {

  %scope selector

  %form {
    @env(
      preset: $preset:string = "studio",
      intensity: $intensity:number = 1.0,
      background: $background:bool = false
    )
  }

  %binds {
    three-env(&self,
      preset: $preset,
      intensity: $intensity,
      background: $background
    ) -> {
      $ready
    }
  }
}
