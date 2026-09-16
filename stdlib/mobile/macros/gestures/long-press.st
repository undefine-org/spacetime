/// Long Press Gesture Macro
///
/// Detects long press gestures with configurable timing.
/// Tracks press duration and provides progress signal.
///
/// @example
/// .item {
///   @long-press {
///     scale: 0.96
///     opacity: 0.8
///   }
/// }

%macro long-press {

  %form {
    @long-press(
      duration: $duration:number = 400,
      threshold: $threshold:number = 10
    ) {
      $styles:properties
    }
  }

  %binds {
    gesture(&self, threshold: $threshold) -> {
      $active,
      $deltaX,
      $deltaY
    }
  }

  %derives {
    // Check if still within threshold
    $withinThreshold: abs($deltaX) < $threshold && abs($deltaY) < $threshold
  }

  %states {
    idle {
      touch-action: none
    }

    pressing when $active {
      user-select: none
      cursor: wait
    }
  }

  %when $active {
    %applies {
      $styles
    }
  }
}
