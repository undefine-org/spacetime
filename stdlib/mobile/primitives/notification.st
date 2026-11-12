/// Notification Primitive
///
/// Wraps the push notification API for mobile and web.
/// Requests permission on bind and provides signals for notification state.

%primitive notification(
  title: string,
  body: string,
  icon: string = ""
) {
  %emit js {
    // Import the SpacetimeMobile API
    const mobile = globalThis.__SpacetimeMobile || {
      notify: async () => { throw new Error('Notifications not supported'); },
      notifyPermission: async () => 'default'
    };

    // Initialize state
    %yield false -> $pending;
    %yield false -> $sent;
    %yield null -> $error;
    %yield 'default' -> $permission;

    // Request permission on bind
    const checkPermission = async () => {
      try {
        const permission = await mobile.notifyPermission();
        %yield permission -> $permission;
        return permission;
      } catch (e) {
        %yield e -> $error;
        return 'denied';
      }
    };

    // Send notification function
    const send = async () => {
      %yield true -> $pending;
      %yield false -> $sent;
      %yield null -> $error;

      try {
        // Check permission first
        const permission = await checkPermission();
        if (permission !== 'granted') {
          throw new Error('Notification permission not granted');
        }

        // Build notification options
        const options = {
          title: %title,
          body: %body
        };

        // Add icon if provided
        const iconValue = %icon;
        if (iconValue && iconValue.length > 0) {
          options.icon = iconValue;
        }

        await mobile.notify(options);

        %yield false -> $pending;
        %yield true -> $sent;
      } catch (e) {
        %yield false -> $pending;
        %yield false -> $sent;
        %yield e -> $error;
      }
    };

    // Expose the send function
    %yield send -> $send;

    // Check permission on initialization
    checkPermission();
  }

  %exports {
    /// Permission state ('granted', 'denied', 'default')
    $permission: string    // 'granted', 'denied', or 'default'
    /// True after notification was successfully sent
    $sent: bool            // notification was sent successfully
    /// True while sending notification
    $pending: bool         // sending in progress
    /// Error if notification failed
    $error: Error?         // error if failed
    /// Function to trigger sending the notification
    $send: fn              // trigger sending the notification
  }
}


/// Notification Macro
///
/// Declarative wrapper for the notification primitive.
/// Allows binding notifications to element events.

%macro notification-trigger {

  %form {
    @notification(
      /// Notification title (required)
      title: $title:string,
      /// Notification body text (required)
      body: $body:string,
      /// Optional icon URL for the notification
      icon: $icon:string = ""
    ) {
      $styles:properties
    }
  }

  %binds {
    notification(...) -> {
      $permission,
      $sent,
      $pending,
      $error,
      $send
    }
  }

  %derives {
    $isGranted: $permission == "granted"
    $isDenied: $permission == "denied"
    $hasError: $error != null
  }

  %states {
    idle {
      cursor: pointer
    }

    denied when $isDenied {
      opacity: 0.5
      pointer-events: none
    }

    sending when $pending {
      opacity: 0.7
      pointer-events: none
    }
  }

  %when $isGranted {
    %applies {
      $styles
    }
  }

  %example "@notification-trigger" {
    .notify-button {
      @notification(title: "Hello", body: "World") {
        on-click: $send()
      }
    }
  }
}
