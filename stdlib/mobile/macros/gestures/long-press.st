/// Long Press Gesture Macro
///
/// Detects long press gestures with configurable timing.
/// Tracks press duration and provides progress signal.

%macro long-press {

  %form {
    @long-press(
      duration: $duration:number = 400,
      /// Minimum movement to start drag (prevents accidental drags)
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

  %example "@long-press" {
    .item {
      @long-press {
        scale: 0.96
        opacity: 0.8
      }
    }
  }
}
