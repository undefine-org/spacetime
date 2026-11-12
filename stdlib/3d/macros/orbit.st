/// Orbit Macro — drag-to-rotate camera.
///
/// `@orbit(…)` attaches inertial OrbitControls to the stage camera. Place it
/// inside `@stage` (it controls the camera, not a specific object).

%macro orbit {

  %scope selector

  %form {
    @orbit(
      /// inertia (0 disables)
      damping: $damping:number = 0.08,
      /// idle rotation speed (0 disables)
      autoRotate: $autoRotate:number = 0,
      /// scroll-to-zoom
      enableZoom: $enableZoom:bool = false,
      /// right-drag pan
      enablePan: $enablePan:bool = false,
      /// zoom clamps
      minDistance: $minDistance:number = 0,
      /// zoom clamps
      maxDistance: $maxDistance:number = 1000,
      /// vertical orbit clamps (degrees)
      minPolar: $minPolar:number = 0,
      /// vertical orbit clamps (degrees)
      maxPolar: $maxPolar:number = 180
    )
  }

  %binds {
    three-orbit(&self, ...) -> {
      $ready
    }
  }

  %example "auto-spin that yields to the user's drag" {
    @stage {
      @orbit(damping: 0.08, autoRotate: 0.4)
      @object torusknot
    }
  }
}
