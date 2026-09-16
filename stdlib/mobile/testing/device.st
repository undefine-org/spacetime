/// Device Profile Definitions for Mobile Testing
///
/// Provides device profiles for testing with different screen sizes
/// and capabilities. Each profile includes viewport dimensions,
/// pixel ratio, safe areas, and platform information.
///
/// @example basic usage
/// .test-container {
///   @device("iPhone 15 Pro")
/// }
///
/// @example with custom device
/// .test-container {
///   @device-custom(
///     width: 375,
///     height: 812,
///     pixel-ratio: 3,
///     safe-top: 44,
///     safe-bottom: 34
///   )
/// }


/// Device Profile Macro
///
/// Applies device dimensions and safe areas as CSS variables
/// for testing with specific device configurations.
///
/// Supported devices:
/// - iPhone SE
/// - iPhone 15 Pro
/// - iPhone 15 Pro Max
/// - Pixel 8
/// - Galaxy S24
///
/// @param name - Device name to emulate

%macro device-profile {

  %form {
    @device($name:string = "iPhone 15 Pro")
  }

  // Device configurations are applied via CSS custom properties
  // Test frameworks can read these to configure viewport and behavior

  %states {
    base {
      // Default to iPhone 15 Pro dimensions
      --st-device-name: "%$name"
      --st-viewport-width: 393px
      --st-viewport-height: 852px
      --st-device-pixel-ratio: 3
      --st-safe-area-top: 59px
      --st-safe-area-bottom: 34px
      --st-safe-area-left: 0px
      --st-safe-area-right: 0px
      --st-platform: ios
    }
  }

  // iPhone SE profile
  %if $name == "iPhone SE" {
    %states {
      base {
        --st-viewport-width: 375px
        --st-viewport-height: 667px
        --st-device-pixel-ratio: 2
        --st-safe-area-top: 20px
        --st-safe-area-bottom: 0px
        --st-safe-area-left: 0px
        --st-safe-area-right: 0px
        --st-platform: ios
      }
    }
  }

  // iPhone 15 Pro profile (default)
  %if $name == "iPhone 15 Pro" {
    %states {
      base {
        --st-viewport-width: 393px
        --st-viewport-height: 852px
        --st-device-pixel-ratio: 3
        --st-safe-area-top: 59px
        --st-safe-area-bottom: 34px
        --st-safe-area-left: 0px
        --st-safe-area-right: 0px
        --st-platform: ios
      }
    }
  }

  // iPhone 15 Pro Max profile
  %if $name == "iPhone 15 Pro Max" {
    %states {
      base {
        --st-viewport-width: 430px
        --st-viewport-height: 932px
        --st-device-pixel-ratio: 3
        --st-safe-area-top: 59px
        --st-safe-area-bottom: 34px
        --st-safe-area-left: 0px
        --st-safe-area-right: 0px
        --st-platform: ios
      }
    }
  }

  // Pixel 8 profile
  %if $name == "Pixel 8" {
    %states {
      base {
        --st-viewport-width: 412px
        --st-viewport-height: 915px
        --st-device-pixel-ratio: 2.625
        --st-safe-area-top: 24px
        --st-safe-area-bottom: 0px
        --st-safe-area-left: 0px
        --st-safe-area-right: 0px
        --st-platform: android
      }
    }
  }

  // Galaxy S24 profile
  %if $name == "Galaxy S24" {
    %states {
      base {
        --st-viewport-width: 360px
        --st-viewport-height: 780px
        --st-device-pixel-ratio: 3
        --st-safe-area-top: 25px
        --st-safe-area-bottom: 0px
        --st-safe-area-left: 0px
        --st-safe-area-right: 0px
        --st-platform: android
      }
    }
  }
}


/// Custom Device Profile Macro
///
/// Define a custom device profile with specific dimensions.
///
/// @param width - Viewport width in pixels
/// @param height - Viewport height in pixels
/// @param pixel-ratio - Device pixel ratio
/// @param safe-top - Top safe area inset
/// @param safe-bottom - Bottom safe area inset
/// @param safe-left - Left safe area inset
/// @param safe-right - Right safe area inset
/// @param platform - Platform identifier (ios or android)
///
/// @example
/// .custom-device {
///   @device-custom(
///     width: 400,
///     height: 800,
///     pixel-ratio: 2,
///     safe-top: 40,
///     safe-bottom: 20,
///     platform: "android"
///   )
/// }

%macro device-custom {

  %form {
    @device-custom(
      width: $width:number = 393,
      height: $height:number = 852,
      pixel-ratio: $pixelRatio:number = 3,
      safe-top: $safeTop:number = 59,
      safe-bottom: $safeBottom:number = 34,
      safe-left: $safeLeft:number = 0,
      safe-right: $safeRight:number = 0,
      platform: $platform:string = "ios"
    )
  }

  %states {
    base {
      --st-device-name: "custom"
      --st-viewport-width: calc($width * 1px)
      --st-viewport-height: calc($height * 1px)
      --st-device-pixel-ratio: $pixelRatio
      --st-safe-area-top: calc($safeTop * 1px)
      --st-safe-area-bottom: calc($safeBottom * 1px)
      --st-safe-area-left: calc($safeLeft * 1px)
      --st-safe-area-right: calc($safeRight * 1px)
      --st-platform: "%$platform"
    }
  }
}


/// Device Frame Macro
///
/// Applies the device profile and constrains the element
/// to match the device viewport dimensions.
///
/// @param name - Device name to emulate
///
/// @example
/// .app-preview {
///   @device-frame("iPhone 15 Pro")
/// }

%macro device-frame {

  %form {
    @device-frame($name:string = "iPhone 15 Pro")
  }

  %states {
    base {
      // Apply device profile variables
      --st-device-name: "%$name"

      // Constrain to viewport dimensions
      width: var(--st-viewport-width)
      height: var(--st-viewport-height)
      max-width: var(--st-viewport-width)
      max-height: var(--st-viewport-height)
      overflow: hidden
      position: relative

      // Visual frame styling
      border-radius: 40px
      box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.25)
      background: #000
    }
  }

  // Apply device-specific dimensions
  %if $name == "iPhone SE" {
    %states {
      base {
        --st-viewport-width: 375px
        --st-viewport-height: 667px
        --st-device-pixel-ratio: 2
        --st-safe-area-top: 20px
        --st-safe-area-bottom: 0px
        --st-platform: ios
        border-radius: 0
      }
    }
  }

  %if $name == "iPhone 15 Pro" {
    %states {
      base {
        --st-viewport-width: 393px
        --st-viewport-height: 852px
        --st-device-pixel-ratio: 3
        --st-safe-area-top: 59px
        --st-safe-area-bottom: 34px
        --st-platform: ios
      }
    }
  }

  %if $name == "iPhone 15 Pro Max" {
    %states {
      base {
        --st-viewport-width: 430px
        --st-viewport-height: 932px
        --st-device-pixel-ratio: 3
        --st-safe-area-top: 59px
        --st-safe-area-bottom: 34px
        --st-platform: ios
      }
    }
  }

  %if $name == "Pixel 8" {
    %states {
      base {
        --st-viewport-width: 412px
        --st-viewport-height: 915px
        --st-device-pixel-ratio: 2.625
        --st-safe-area-top: 24px
        --st-safe-area-bottom: 0px
        --st-platform: android
        border-radius: 24px
      }
    }
  }

  %if $name == "Galaxy S24" {
    %states {
      base {
        --st-viewport-width: 360px
        --st-viewport-height: 780px
        --st-device-pixel-ratio: 3
        --st-safe-area-top: 25px
        --st-safe-area-bottom: 0px
        --st-platform: android
        border-radius: 28px
      }
    }
  }
}
