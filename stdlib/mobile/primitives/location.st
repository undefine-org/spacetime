/// Location Primitive
///
/// Requests the device's current geolocation. Uses the native location API
/// when running in a Tauri mobile app, or the Web Geolocation API as fallback.

%primitive location(
  /// Element to bind location data to
  &el,
  highAccuracy: bool = true,
  timeout: number = 10000
) {
  %emit js {
    // Import the SpacetimeMobile API
    const mobile = globalThis.__SpacetimeMobile || {
      location: (opts) => {
        return new Promise((resolve, reject) => {
          if (typeof navigator !== 'undefined' && navigator.geolocation) {
            navigator.geolocation.getCurrentPosition(
              (pos) => resolve({
                latitude: pos.coords.latitude,
                longitude: pos.coords.longitude,
                accuracy: pos.coords.accuracy
              }),
              (err) => reject(new Error(err.message)),
              {
                enableHighAccuracy: opts.enableHighAccuracy,
                timeout: opts.timeout
              }
            );
          } else {
            reject(new Error('Geolocation not available'));
          }
        });
      }
    };

    // Initialize signals with default values
    %yield 0 -> $latitude;
    %yield 0 -> $longitude;
    %yield 0 -> $accuracy;
    %yield true -> $loading;
    %yield "" -> $error;

    // Request location
    mobile.location({
      enableHighAccuracy: highAccuracy,
      timeout: timeout
    }).then((pos) => {
      %yield pos.latitude -> $latitude;
      %yield pos.longitude -> $longitude;
      %yield pos.accuracy -> $accuracy;
      %yield false -> $loading;
      %yield "" -> $error;
    }).catch((err) => {
      %yield false -> $loading;
      %yield err.message -> $error;
    });
  }

  %exports {
    /// Current latitude coordinate
    $latitude: number
    /// Current longitude coordinate
    $longitude: number
    /// Accuracy of the position in meters
    $accuracy: number
    /// Whether the location request is in progress
    $loading: bool
    /// Error message if request failed, empty string otherwise
    $error: string
  }
}


/// Location Macro
///
/// Declarative macro for requesting location on element mount.
///
/// .map-container {
///   @location(highAccuracy: true, timeout: 15000)
/// }

%macro location-request {

  %form {
    @location(
      /// Whether to use high accuracy mode (GPS)
      highAccuracy: $highAccuracy:bool = true,
      /// Request timeout in milliseconds
      timeout: $timeout:number = 10000
    )
  }

  %binds {
    location(&self, highAccuracy: $highAccuracy, timeout: $timeout) -> {
      $latitude,
      $longitude,
      $accuracy,
      $loading,
      $error
    }
  }

  %example "@location-request" {
    .location-display {
      @location(highAccuracy: true)
    }
  }
}
