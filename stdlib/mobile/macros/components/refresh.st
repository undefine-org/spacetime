/// Pull to Refresh Macro
///
/// Creates a pull-to-refresh gesture for mobile scrollable containers.
/// Tracks pull distance and provides progress signals for custom indicators.
///
/// @example basic
/// .feed {
///   @pull-refresh(threshold: 80) {
///     pulling { transform: translateY($progress * 80); }
///     refreshing { opacity: 0.7; }
///   }
/// }
///
/// @example with resistance
/// .list-container {
///   @pull-refresh(threshold: 100, resistance: 0.4) {
///     pulling { transform: translateY($progress * 100 * 0.5); }
///   }
/// }

%macro pull-refresh {

  %form {
    @pull-refresh(
      threshold: $threshold:number = 80,
      resistance: $resistance:number = 0.5
    ) {
      $styles:properties
    }
  }

  %binds {
    gesture(&self, threshold: 0) -> {
      $active,
      $deltaY,
      $velocityY
    }
  }

  %derives {
    // Apply resistance to pull distance (only track downward pulls)
    $pullDistance: $deltaY > 0 ? $deltaY * $resistance : 0

    // Progress from 0 to 1 based on threshold
    $progress: min(1, $pullDistance / $threshold)

    // Whether pull has exceeded threshold
    $overThreshold: $pullDistance >= $threshold

    // Signal for pulling state
    $pulling: $active && $deltaY > 0

    // Signal for refreshing state (set externally via callback)
    $refreshing: false
  }

  %states {
    idle {
      touch-action: pan-y
      will-change: transform
      transition: transform 0.3s ease-out
    }

    pulling when $pulling {
      user-select: none
      transition: none
    }

    refreshing when $refreshing {
      user-select: none
      pointer-events: none
    }
  }

  %when $active {
    %applies {
      $styles
    }
  }
}
