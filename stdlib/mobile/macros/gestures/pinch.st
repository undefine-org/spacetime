/// Pinch Gesture Macro
///
/// Tracks two-finger pinch gestures for zoom and scale.
/// Uses the gesture primitive for basic touch tracking.
///
/// @example
/// .image {
///   @pinch {
///     scale: $scale
///   }
/// }

%macro pinch {

  %form {
    @pinch(
      minScale: $minScale:number = 0.5,
      maxScale: $maxScale:number = 4
    ) {
      $styles:properties
    }
  }

  %binds {
    gesture(&self) -> {
      $active,
      $deltaX,
      $deltaY,
      $distance
    }
  }

  %derives {
    // Scale based on distance change (simplified single-touch fallback)
    $scale: clamp($minScale, 1 + $distance / 200, $maxScale)
  }

  %states {
    idle {
      touch-action: none
    }

    pinching when $active {
      user-select: none
    }
  }

  %when $active {
    %applies {
      $styles
    }
  }
}
