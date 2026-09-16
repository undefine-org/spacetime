/// Bottom Sheet Macro
///
/// Creates a draggable bottom sheet with detent positions.
/// Supports drag-to-dismiss and snap-to-detent behavior.
///
/// @example basic
/// .sheet-container {
///   @sheet {
///     transform: translateY($offset)
///     background: white
///     border-radius: 16px 16px 0 0
///   }
/// }
///
/// @example with custom height
/// .tall-sheet {
///   @sheet(height: 600) {
///     transform: translateY($offset)
///     background: #f5f5f5
///   }
/// }

%macro sheet {

  %form {
    @sheet(
      height: $height:number = 400,
      dismissThreshold: $dismissThreshold:number = 100
    ) {
      $styles:properties
    }
  }

  %binds {
    gesture(&self, threshold: 10) -> {
      $active,
      $deltaY,
      $velocityY,
      $finalVelocityY
    }
  }

  %derives {
    // Calculate sheet offset (positive = dragged down)
    $offset: $deltaY > 0 ? $deltaY : 0

    // Determine if should dismiss (dragged past threshold)
    $shouldDismiss: $offset > $dismissThreshold

    // Calculate opacity for backdrop fade
    $backdropOpacity: 1 - ($offset / $height)
  }

  %states {
    idle {
      touch-action: none
      will-change: transform
      transition: transform 0.3s ease-out
    }

    dragging when $active {
      user-select: none
      cursor: grabbing
      transition: none
    }
  }

  %when $active {
    %applies {
      $styles
    }
  }
}
