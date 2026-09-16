/// Drawer Navigation Macro
///
/// Provides slide-out drawer navigation from left or right.
/// Supports swipe-to-open gestures and backdrop overlay.
///
/// @example basic
/// .drawer-menu {
///   @drawer {
///     transform: translateX($translateX)
///   }
/// }
///
/// @example right-side drawer
/// .settings-drawer {
///   @drawer(side: "right", width: 320) {
///     transform: translateX($translateX)
///   }
/// }

%macro drawer {

  %form {
    @drawer(
      side: $side:("left" | "right") = "left",
      width: $width:number = 280,
      threshold: $threshold:number = 20
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
    // Calculate translate based on side
    $translateX: $side == "left" ? $deltaX : -$deltaX

    // Calculate progress (0 = closed, 1 = open)
    $progress: min(1, max(0, abs($deltaX) / $width))

    // Backdrop opacity based on progress
    $backdropOpacity: $progress * 0.5

    // Determine if drawer should snap open based on velocity or position
    $shouldOpen: abs($velocityX) > 300 ? $velocityX > 0 : $progress > 0.5
  }

  %states {
    idle {
      touch-action: pan-x
      will-change: transform
    }

    dragging when $active {
      user-select: none
    }
  }

  %when $active {
    %applies {
      $styles
    }
  }
}
