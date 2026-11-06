/// Fog Macro — distance fog in the stage (PLAN-152).
///
/// @example a dusk haze that matches the sky
/// canvas.forum {
///   @stage(background: "#2a1d24") {
///     @fog &haze (color: #2a1d24, near: 10, far: 45)
///   }
/// }
///
/// @param color - the fog's colour (match the stage background)
/// @param near  - where it begins (world units)
/// @param far   - where it is total

%macro fog {
  %scope selector

  %capture_type fog_name { $name:element }

  %form {
    @fog $named:fog_name? (
      color: $color:color = black,
      near: $near:number = 8,
      far: $far:number = 40
    ) {
      $children*
    }
  }

  %binds {
    three-fog(&self, name: $named, color: $color, near: $near, far: $far) -> {
      $added
    }
  }

  %template {
    $children
  }
}
