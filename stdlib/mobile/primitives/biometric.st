/// Biometric Authentication Primitive
///
/// Provides biometric authentication (Face ID, Touch ID, fingerprint) on native platforms.
/// In browser mode, returns an error since biometric auth requires native runtime.

%primitive biometric(
  /// Element to bind authentication state to
  &el,
  /// Prompt message to display
  message: string = "Authenticate"
) {
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
    /// True if authentication succeeded
    $authenticated: bool
    /// True while authentication is in progress
    $pending: bool
    /// Error message if authentication failed
    $error: string?
    /// Event name to dispatch to trigger authentication
    $trigger: string
  }
}


/// Biometric macro wrapper
///
/// Convenience macro for biometric authentication with state-machine integration.
/// Binds the biometric primitive and exposes signals for use in other macros.

%macro biometric-auth {

  %form {
    @biometric($message:string = "Authenticate")
  }

  %binds {
    biometric(&self, message: $message) -> { $authenticated, $pending, $error, $trigger }
  }

  %example "@biometric-auth" {
    .auth-form {
      @biometric("Confirm your identity")
    }
  }
}
