/// Mobile Swipe Gesture
///
/// Re-exports and extends the core swipe gesture for mobile use.
/// The core @swipe is available from stdlib/macros/drag.st
///
/// @example
/// .carousel-item {
///   @swipe {
///     on-left: nextSlide()
///     on-right: prevSlide()
///   }
/// }

// Mobile-specific swipe variant with visual feedback
%macro mobile-swipe {

  %form {
    @mobile-swipe(
      direction: $direction:("left" | "right" | "up" | "down" | "horizontal" | "vertical") = "horizontal",
      threshold: $threshold:number = 50
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
      $velocityY
    }
  }

  %derives {
    // Calculate progress based on direction
    $progress: $direction == "left" ? min(1, -$deltaX / 100)
             : $direction == "right" ? min(1, $deltaX / 100)
             : $direction == "up" ? min(1, -$deltaY / 100)
             : $direction == "down" ? min(1, $deltaY / 100)
             : $direction == "horizontal" ? min(1, abs($deltaX) / 100)
             : min(1, abs($deltaY) / 100)
  }

  %states {
    idle {
      touch-action: none
    }

    swiping when $active {
      user-select: none
    }
  }

  %when $active {
    %applies {
      $styles
    }
  }
}
