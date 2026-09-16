/// Stack Navigation Macro
///
/// Provides push/pop navigation between screens.
/// Manages a stack of views with animated transitions.
///
/// @example basic
/// .app-container {
///   @stack {
///     transform: translateX($offsetX)
///   }
/// }
///
/// @example with custom transition
/// .screen {
///   @stack(direction: "horizontal") {
///     transform: translateX($offsetX)
///     opacity: $visible ? 1 : 0
///   }
/// }

%macro stack {

  %form {
    @stack(
      direction: $direction:("horizontal" | "vertical") = "horizontal",
      duration: $duration:number = 300
    ) {
      $styles:properties
    }
  }

  %binds {
    gesture(&self, threshold: 20) -> {
      $active,
      $deltaX,
      $deltaY,
      $velocityX,
      $velocityY
    }
  }

  %derives {
    // Calculate offset based on direction
    $offsetX: $direction == "horizontal" ? $deltaX : 0
    $offsetY: $direction == "vertical" ? $deltaY : 0

    // Track visibility state
    $visible: true

    // Edge swipe detection for back gesture
    $isBackSwipe: $direction == "horizontal" ? $deltaX > 50 : $deltaY > 50
  }

  %states {
    idle {
      touch-action: pan-x
      will-change: transform
    }

    navigating when $active {
      user-select: none
    }
  }

  %when $active {
    %applies {
      $styles
    }
  }
}
