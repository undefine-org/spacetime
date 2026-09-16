/// Pan Gesture Macro
///
/// Enables free-form dragging in any direction with velocity tracking.
/// Wraps the gesture primitive for mobile touch interactions.
///
/// @example basic
/// .draggable {
///   @pan {
///     translate-x: $deltaX
///     translate-y: $deltaY
///   }
/// }
///
/// @example constrained to axis
/// .slider {
///   @pan(axis: "x") {
///     translate-x: $deltaX
///   }
/// }

%macro pan {

  %form {
    @pan(
      axis: $axis:("x" | "y" | "both") = "both",
      threshold: $threshold:number = 0
    ) {
      $styles:properties
    }
  }

  %binds {
    gesture(&self, threshold: $threshold) -> {
      $active,
      $deltaX,
      $deltaY,
      $velocityX,
      $velocityY,
      $finalVelocityX,
      $finalVelocityY
    }
  }

  %derives {
    // Axis constraints
    $panX: $axis != "y" ? $deltaX : 0
    $panY: $axis != "x" ? $deltaY : 0
  }

  %states {
    idle {
      cursor: grab
      touch-action: none
    }

    panning when $active {
      cursor: grabbing
      user-select: none
    }
  }

  %when $active {
    %applies {
      $styles
    }
  }
}
