/// Haptic Feedback Primitive
///
/// Triggers haptic feedback on the device. Uses the native haptic API
/// when running in a Tauri mobile app, or the Web Vibration API as fallback.
///
/// @param &el - Element to bind haptic feedback to
/// @param style - Haptic style: 'light', 'medium', 'heavy', 'success', 'warning', 'error', 'selection'
///
/// @yields $triggered - Whether the haptic feedback was triggered

%primitive haptic(&el, style: string = "medium") {
  %emit js {
    // Import the SpacetimeMobile API
    const mobile = globalThis.__SpacetimeMobile || {
      haptic: (s) => {
        // Web Vibration API fallback
        if (typeof navigator !== 'undefined' && navigator.vibrate) {
          const patterns = {
            light: [10],
            medium: [20],
            heavy: [40],
            success: [10, 30, 10],
            warning: [20, 10, 20],
            error: [50, 30, 50],
            selection: [5]
          };
          navigator.vibrate(patterns[s] || patterns.medium);
        }
      }
    };

    // Trigger haptic feedback immediately on bind
    mobile.haptic(style);

    // Yield triggered signal
    %yield true -> $triggered;
  }

  %exports {
    $triggered: bool
  }
}


/// Haptic Feedback Macro
///
/// Declarative macro for triggering haptic feedback on element mount.
///
/// @example
/// .button {
///   @haptic("light")
/// }
///
/// .submit-button {
///   @haptic("success")
/// }

%macro haptic-feedback {

  %form {
    @haptic($style:string = "medium")
  }

  %binds {
    haptic(&self, style: $style) -> { $triggered }
  }
}
