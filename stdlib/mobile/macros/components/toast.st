/// Toast Notification Macro
///
/// Creates toast/snackbar notifications with auto-dismiss functionality.
/// Supports top and bottom positioning with customizable duration.
///
/// @example basic bottom toast
/// .notification {
///   @toast {
///     showing {
///       transform: translateY(0)
///       opacity: 1
///     }
///     hidden {
///       transform: translateY(100%)
///       opacity: 0
///     }
///   }
/// }
///
/// @example top toast with custom duration
/// .alert {
///   @toast(duration: 4000, position: "top") {
///     showing {
///       transform: translateY(0)
///       opacity: 1
///     }
///     hidden {
///       transform: translateY(-100%)
///       opacity: 0
///     }
///   }
/// }

%macro toast {

  %form {
    @toast(
      duration: $duration:number = 3000,
      position: $position:("top" | "bottom") = "bottom"
    ) {
      $styles:properties
    }
  }

  %binds {
    pointer(&self, events: ["enter", "leave"]) -> {
      $isHovered
    }
  }

  %derives {
    // Position flags for styling
    $isTop: $position == "top"
    $isBottom: $position == "bottom"

    // Export visible signal for external control
    $visible: false
  }

  %states {
    hidden {
      opacity: 0
      visibility: hidden
      pointer-events: none
      transition: opacity 0.3s ease-out, transform 0.3s ease-out, visibility 0.3s
    }

    showing {
      opacity: 1
      visibility: visible
      pointer-events: auto
      transition: opacity 0.3s ease-in, transform 0.3s ease-in
    }

    dismissing {
      opacity: 0
      pointer-events: none
      transition: opacity 0.2s ease-out, transform 0.2s ease-out
    }

    position-top when $isTop {
      position: fixed
      top: env(safe-area-inset-top, 16px)
      left: 50%
      transform: translateX(-50%)
    }

    position-bottom when $isBottom {
      position: fixed
      bottom: env(safe-area-inset-bottom, 16px)
      left: 50%
      transform: translateX(-50%)
    }
  }

  %when $visible {
    %applies {
      $styles
    }
  }
}
