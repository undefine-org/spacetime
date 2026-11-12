/// Env Macro — image-based lighting + reflections for the stage.
///
/// `@env(…)` builds a studio environment map (vendored RoomEnvironment) and
/// assigns it as the scene environment, so every PBR / glass material reflects a
/// realistic surrounding. The biggest single upgrade to material realism.

%macro env {

  %scope selector

  %form {
    @env(
      /// studio
      preset: $preset:string = "studio",
      /// environment lighting intensity
      intensity: $intensity:number = 1.0,
      /// also paint the env as the canvas background
      background: $background:bool = false
    )
  }

  %binds {
    three-env(&self, ...) -> {
      $ready
    }
  }

  %example "@env" {
    @stage {
      @env(preset: studio, intensity: 1.1)
      @object icosahedron(material: glass, transmission: 1)
    }
  }
}
