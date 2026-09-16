/// Camera Primitive
///
/// Wraps the camera/barcode scanning API for mobile devices.
/// Provides signals for scan state and results. Does NOT auto-trigger -
/// use $scan() to start scanning.
///
/// @param formats - Array of barcode formats to scan for (e.g., "QR_CODE", "EAN_13")
///
/// @yields $scannedValue - The scanned value (barcode content)
/// @yields $scannedFormat - The format of the scanned code
/// @yields $scanning - True while scanning is in progress
/// @yields $error - Error if scan failed
/// @yields $scan - Function to trigger a scan

%primitive camera(
  formats: array = []
) {
  %emit js {
    // Import the SpacetimeMobile API
    const mobile = globalThis.__SpacetimeMobile || {
      camera: {
        scan: async () => { throw new Error('Camera not supported'); }
      }
    };

    // Initialize state
    %yield false -> $scanning;
    %yield null -> $scannedValue;
    %yield null -> $scannedFormat;
    %yield null -> $error;

    // Scan function
    const scan = async () => {
      %yield true -> $scanning;
      %yield null -> $scannedValue;
      %yield null -> $scannedFormat;
      %yield null -> $error;

      try {
        // Build scan options
        const options = {};
        const formatsValue = %formats;
        if (formatsValue && formatsValue.length > 0) {
          options.formats = formatsValue;
        }

        const result = await mobile.camera.scan(options);

        %yield false -> $scanning;
        %yield result.value -> $scannedValue;
        %yield result.format -> $scannedFormat;

        return result;
      } catch (e) {
        %yield false -> $scanning;
        %yield null -> $scannedValue;
        %yield null -> $scannedFormat;
        %yield e -> $error;

        throw e;
      }
    };

    // Expose the scan function
    %yield scan -> $scan;
  }

  %exports {
    $scannedValue: string?   // scanned barcode value
    $scannedFormat: string?  // format of scanned code
    $scanning: bool          // scan in progress
    $error: Error?           // error if scan failed
    $scan: fn                // trigger a scan
  }
}


/// Camera Scan Macro
///
/// Declarative wrapper for the camera primitive.
/// Allows binding camera scans to element events.
///
/// @example
/// .scan-button {
///   @camera-scan(formats: ["QR_CODE", "EAN_13"]) {
///     background: #007aff
///   }
/// }

%macro camera-scan-trigger {

  %form {
    @camera-scan(
      formats: $formats:array = []
    ) {
      $styles:properties
    }
  }

  %binds {
    camera(
      formats: $formats
    ) -> {
      $scannedValue,
      $scannedFormat,
      $scanning,
      $error,
      $scan
    }
  }

  %derives {
    $hasResult: $scannedValue != null
    $hasError: $error != null
  }

  %states {
    idle {
      cursor: pointer
    }

    scanning when $scanning {
      opacity: 0.7
      pointer-events: none
    }
  }

  %when $hasResult {
    %applies {
      $styles
    }
  }
}
