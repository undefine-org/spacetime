/// Mobile Button Macro
///
/// Creates a mobile-optimized button with haptic feedback.
/// Supports press states and loading indicator.
///
/// @example basic
/// .submit-btn {
///   @mobile-button(haptic: "medium") {
///     pressing { transform: scale(0.97); }
///   }
/// }
///
/// @example with loading state
/// .async-btn {
///   @mobile-button(haptic: "light", loading: true) {
///     pressing { transform: scale(0.98); opacity: 0.9; }
///     loading { opacity: 0.7; pointer-events: none; }
///   }
/// }

%macro mobile-button {

  %form {
    @mobile-button(
      haptic: $haptic:string = "light",
      loading: $loading:bool = false
    ) {
      $styles:properties
    }
  }

  %binds {
    gesture(&self, threshold: 0) -> {
      $active,
      $deltaX,
      $deltaY
    }
    haptic(&self, style: $haptic) -> { $triggered }
  }

  %derives {
    // Pressing state - active and within small movement threshold
    $pressing: $active && abs($deltaX) < 10 && abs($deltaY) < 10

    // Disabled when loading
    $disabled: $loading
  }

  %states {
    idle {
      touch-action: manipulation
      user-select: none
      -webkit-tap-highlight-color: transparent
      cursor: pointer
    }

    pressing when $pressing {
      user-select: none
    }

    loading when $loading {
      cursor: wait
      pointer-events: none
    }

    disabled when $disabled {
      cursor: not-allowed
      pointer-events: none
    }
  }

  %when $pressing {
    %applies {
      $styles
    }
  }
}
