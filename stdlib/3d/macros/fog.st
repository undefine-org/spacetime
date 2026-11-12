/// Fog Macro — distance fog in the stage (PLAN-152).

%macro fog {
  %scope selector

  %capture_type fog_name { $name:element }

  %form {
    @fog $named:fog_name? (
      /// the fog's colour (match the stage background)
      color: $color:color = black,
      /// where it begins (world units)
      near: $near:number = 8,
      /// where it is total
      far: $far:number = 40
    ) {
      $children*
    }
  }

  %binds {
    three-fog(&self, name: $named, ...) -> {
      $added
    }
  }

  %template {
    $children
  }

  %example "a dusk haze that matches the sky" {
    canvas.forum {
      @stage(background: "#2a1d24") {
        @fog &haze (color: #2a1d24, near: 10, far: 45)
      }
    }
  }
}
