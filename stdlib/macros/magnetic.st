/// Magnetic Hover Macro
///
/// Applies proximity-based magnetic pull to an element.
/// When the mouse enters the configured radius, the element
/// is smoothly drawn toward the cursor position.

%macro magnetic-macro {

  %form {
    @magnetic(
      /// Pull strength (0-1, how much element moves toward cursor)
      strength: $strength:number = 0.3,
      /// Activation radius from element center (px)
      radius: $radius:number = 100,
      /// Interpolation smoothness (0-1)
      lerp: $lerp:number = 0.1,
      /// Movement axis: "both", "x", or "y"
      axis: $axis:string = "both",
      /// What triggers the effect: "self" or "parent"
      trigger: $trigger:string = "self",
      /// Minimum distance ratio to start effect (0-1)
      tolerance: $tolerance:number = 0.5
    )
  }

  %binds {
    magnetic-effect(&self, ...) -> { $magnetX, $magnetY, $distance, $isActive }
  }

  %example "button" {
    .cta-button {
      @magnetic(strength: 0.3, radius: 100px)
    }
  }

  %example "horizontal only" {
    .nav-link {
      @magnetic(strength: 0.2, axis: "x", radius: 80px)
    }
  }
}
