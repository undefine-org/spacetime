/// Orbit Macro — drag-to-rotate camera.
///
/// `@orbit(…)` attaches inertial OrbitControls to the stage camera. Place it
/// inside `@stage` (it controls the camera, not a specific object).
///
/// @example auto-spin that yields to the user's drag
/// @stage {
///   @orbit(damping: 0.08, autoRotate: 0.4)
///   @object torusknot
/// }
///
/// @param damping     - inertia (0 disables); default 0.08
/// @param autoRotate  - idle rotation speed (0 disables); default 0
/// @param enableZoom  - scroll-to-zoom (default false)
/// @param enablePan   - right-drag pan (default false)
/// @param minDistance/maxDistance - zoom clamps
/// @param minPolar/maxPolar       - vertical orbit clamps (degrees)

%macro orbit {

  %scope selector

  %form {
    @orbit(
      damping: $damping:number = 0.08,
      autoRotate: $autoRotate:number = 0,
      enableZoom: $enableZoom:bool = false,
      enablePan: $enablePan:bool = false,
      minDistance: $minDistance:number = 0,
      maxDistance: $maxDistance:number = 1000,
      minPolar: $minPolar:number = 0,
      maxPolar: $maxPolar:number = 180
    )
  }

  %binds {
    three-orbit(&self,
      damping: $damping,
      autoRotate: $autoRotate,
      enableZoom: $enableZoom,
      enablePan: $enablePan,
      minDistance: $minDistance,
      maxDistance: $maxDistance,
      minPolar: $minPolar,
      maxPolar: $maxPolar
    ) -> {
      $ready
    }
  }
}
