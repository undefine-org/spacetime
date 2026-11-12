/// Share Primitive
///
/// Provides native share functionality (share sheet on mobile, Web Share API on browser).
/// Allows sharing content with other apps like social media, messaging, or email.

%primitive share(
  /// Element to bind share state to
  &el,
  title: string = "",
  text: string = "",
  url: string = ""
) {
  %emit js {
    // Import the SpacetimeMobile API
    const mobile = globalThis.__SpacetimeMobile || {
      share: (opts) => {
        if (typeof navigator !== 'undefined' && navigator.share) {
          return navigator.share(opts);
        }
        return Promise.reject(new Error('Share requires native runtime or Web Share API'));
      }
    };

    // Initialize signals
    %yield false -> $shared;
    %yield false -> $pending;
    %yield null -> $error;

    // Trigger event name for this instance
    const triggerEvent = 'share.trigger';
    %yield triggerEvent -> $trigger;

    // Share handler
    const doShare = async () => {
      // Reset state
      %yield false -> $shared;
      %yield true -> $pending;
      %yield null -> $error;

      // Build share options, filtering out empty values
      const options = {};
      if (%title) options.title = %title;
      if (%text) options.text = %text;
      if (%url) options.url = %url;

      try {
        await mobile.share(options);
        %yield true -> $shared;
        %yield false -> $pending;
      } catch (e) {
        // User cancellation is not an error in Web Share API
        if (e.name === 'AbortError') {
          %yield false -> $shared;
          %yield false -> $pending;
        } else {
          %yield false -> $shared;
          %yield false -> $pending;
          %yield e.message || 'Share failed' -> $error;
        }
      }
    };

    // Listen for trigger event
    const triggerHandler = () => {
      doShare();
    };

    %&el.addEventListener(triggerEvent, triggerHandler);

    %cleanup {
      %&el.removeEventListener(triggerEvent, triggerHandler);
    }
  }

  %exports {
    /// True if share completed successfully
    $shared: bool
    /// True while share sheet is open
    $pending: bool
    /// Error message if share failed
    $error: string?
    /// Event name to dispatch to trigger share
    $trigger: string
  }
}


/// Share macro wrapper
///
/// Convenience macro for native share with state integration.
/// Binds the share primitive and exposes signals for use in other macros.

%macro share-native {

  %form {
    @share(
      /// Title of shared content (optional)
      title: $title:string = "",
      /// Text content to share (optional)
      text: $text:string = "",
      /// URL to share (optional)
      url: $url:string = ""
    )
  }

  %binds {
    share(&self, ...) -> { $shared, $pending, $error, $trigger }
  }

  %example "@share-native" {
    .share-button {
      @share(title: "Check this out", url: "https://example.com")
    }
  }
}
