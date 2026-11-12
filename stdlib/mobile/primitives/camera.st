/// Camera Primitive
///
/// Wraps the camera/barcode scanning API for mobile devices.
/// Provides signals for scan state and results. Does NOT auto-trigger -
/// use $scan() to start scanning.

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
    /// The scanned value (barcode content)
    $scannedValue: string?   // scanned barcode value
    /// The format of the scanned code
    $scannedFormat: string?  // format of scanned code
    /// True while scanning is in progress
    $scanning: bool          // scan in progress
    /// Error if scan failed
    $error: Error?           // error if scan failed
    /// Function to trigger a scan
    $scan: fn                // trigger a scan
  }
}


/// Camera Scan Macro
///
/// Declarative wrapper for the camera primitive.
/// Allows binding camera scans to element events.

%macro camera-scan-trigger {

  %form {
    @camera-scan(
      /// Array of barcode formats to scan for (e.g., "QR_CODE", "EAN_13")
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

  %example "@camera-scan-trigger" {
    .scan-button {
      @camera-scan(formats: ["QR_CODE", "EAN_13"]) {
        background: #007aff
      }
    }
  }
}
