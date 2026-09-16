/// Biometric Authentication Primitive
///
/// Provides biometric authentication (Face ID, Touch ID, fingerprint) on native platforms.
/// In browser mode, returns an error since biometric auth requires native runtime.
///
/// @param &el - Element to bind authentication state to
/// @param message - Prompt message to display (default: "Authenticate")
///
/// @yields $authenticated - True if authentication succeeded
/// @yields $pending - True while authentication is in progress
/// @yields $error - Error message if authentication failed
/// @yields $trigger - Event name to dispatch to trigger authentication

%primitive biometric(&el, message: string = "Authenticate") {
  %emit js {
    // Import the SpacetimeMobile API
    const mobile = globalThis.__SpacetimeMobile || {
      biometric: () => Promise.reject(new Error('Biometric authentication requires native runtime'))
    };

    // Initialize signals
    %yield false -> $authenticated;
    %yield false -> $pending;
    %yield null -> $error;

    // Trigger event name for this instance
    const triggerEvent = 'biometric.trigger';
    %yield triggerEvent -> $trigger;

    // Authentication handler
    const authenticate = async () => {
      // Reset state
      %yield false -> $authenticated;
      %yield true -> $pending;
      %yield null -> $error;

      try {
        const result = await mobile.biometric(%message);

        if (result.success) {
          %yield true -> $authenticated;
          %yield false -> $pending;
        } else {
          %yield false -> $authenticated;
          %yield false -> $pending;
          %yield result.error || 'Authentication failed' -> $error;
        }
      } catch (e) {
        %yield false -> $authenticated;
        %yield false -> $pending;
        %yield e.message || 'Authentication failed' -> $error;
      }
    };

    // Listen for trigger event
    const triggerHandler = () => {
      authenticate();
    };

    %&el.addEventListener(triggerEvent, triggerHandler);

    %cleanup {
      %&el.removeEventListener(triggerEvent, triggerHandler);
    }
  }

  %exports {
    $authenticated: bool
    $pending: bool
    $error: string?
    $trigger: string
  }
}


/// Biometric macro wrapper
///
/// Convenience macro for biometric authentication with state-machine integration.
/// Binds the biometric primitive and exposes signals for use in other macros.
///
/// @example
/// .auth-form {
///   @biometric("Confirm your identity")
/// }

%macro biometric-auth {

  %form {
    @biometric($message:string = "Authenticate")
  }

  %binds {
    biometric(&self, message: $message) -> { $authenticated, $pending, $error, $trigger }
  }
}
