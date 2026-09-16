/// Notification Primitive
///
/// Wraps the push notification API for mobile and web.
/// Requests permission on bind and provides signals for notification state.
///
/// @param title - Notification title (required)
/// @param body - Notification body text (required)
/// @param icon - Optional icon URL for the notification
///
/// @yields $permission - Permission state ('granted', 'denied', 'default')
/// @yields $sent - True after notification was successfully sent
/// @yields $pending - True while sending notification
/// @yields $error - Error if notification failed
/// @yields $send - Function to trigger sending the notification

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
    $permission: string    // 'granted', 'denied', or 'default'
    $sent: bool            // notification was sent successfully
    $pending: bool         // sending in progress
    $error: Error?         // error if failed
    $send: fn              // trigger sending the notification
  }
}


/// Notification Macro
///
/// Declarative wrapper for the notification primitive.
/// Allows binding notifications to element events.
///
/// @example
/// .notify-button {
///   @notification(title: "Hello", body: "World") {
///     on-click: $send()
///   }
/// }

%macro notification-trigger {

  %form {
    @notification(
      title: $title:string,
      body: $body:string,
      icon: $icon:string = ""
    ) {
      $styles:properties
    }
  }

  %binds {
    notification(
      title: $title,
      body: $body,
      icon: $icon
    ) -> {
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
}
