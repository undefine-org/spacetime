/// Safe Area Macro
///
/// Applies safe area padding using CSS env() for mobile devices.
/// Handles notches, home indicators, and other system UI intrusions.

%macro safe-area {

  %form {
    @safe-area(
      /// Apply safe area padding to top
      top: $top:bool = true,
      /// Apply safe area padding to bottom
      bottom: $bottom:bool = true,
      /// Apply safe area padding to left
      left: $left:bool = true,
      /// Apply safe area padding to right
      right: $right:bool = true
    )
  }

  %states {
    base {
      // Expose safe area values as CSS custom properties for runtime access
      --st-safe-area-top: env(safe-area-inset-top, 0px)
      --st-safe-area-bottom: env(safe-area-inset-bottom, 0px)
      --st-safe-area-left: env(safe-area-inset-left, 0px)
      --st-safe-area-right: env(safe-area-inset-right, 0px)
    }
  }

  // Conditionally apply padding based on parameters
  %if $top {
    %states {
      base {
        padding-top: env(safe-area-inset-top, 0px)
      }
    }
  }

  %if $bottom {
    %states {
      base {
        padding-bottom: env(safe-area-inset-bottom, 0px)
      }
    }
  }

  %if $left {
    %states {
      base {
        padding-left: env(safe-area-inset-left, 0px)
      }
    }
  }

  %if $right {
    %states {
      base {
        padding-right: env(safe-area-inset-right, 0px)
      }
    }
  }

  %example "basic" {
    .app-container {
      @safe-area(top: true, bottom: true)
    }
  }

  %example "only bottom for tab bar" {
    .tab-bar {
      @safe-area(top: false, bottom: true, left: false, right: false)
    }
  }

  %example "custom properties access" {
    .custom-layout {
      @safe-area(top: true, bottom: true)
      // Access via CSS custom properties:
      // var(--st-safe-area-top)
      // var(--st-safe-area-bottom)
      // var(--st-safe-area-left)
      // var(--st-safe-area-right)
    }
  }
}


/// Safe Area Inset Macro
///
/// Adds safe area insets to existing padding values.
/// Use when you already have padding and want to add safe areas on top.

%macro safe-area-inset {

  %form {
    @safe-area-inset(
      top: $top:bool = false,
      bottom: $bottom:bool = false,
      left: $left:bool = false,
      right: $right:bool = false
    )
  }

  %states {
    base {
      // Expose safe area values as CSS custom properties
      --st-safe-area-top: env(safe-area-inset-top, 0px)
      --st-safe-area-bottom: env(safe-area-inset-bottom, 0px)
      --st-safe-area-left: env(safe-area-inset-left, 0px)
      --st-safe-area-right: env(safe-area-inset-right, 0px)
    }
  }

  %example "@safe-area-inset" {
    .header {
      padding: 16px
      @safe-area-inset(top: true)
      // Results in padding-top: calc(16px + env(safe-area-inset-top))
    }
  }
}


/// Safe Area Container Macro
///
/// Creates a container that handles all safe areas automatically.
/// Useful for full-screen app containers.

%macro safe-area-container {

  %form {
    @safe-area-container
  }

  %states {
    base {
      // Full viewport with safe area padding
      min-height: 100vh
      min-height: 100dvh
      box-sizing: border-box

      // All safe area paddings
      padding-top: env(safe-area-inset-top, 0px)
      padding-bottom: env(safe-area-inset-bottom, 0px)
      padding-left: env(safe-area-inset-left, 0px)
      padding-right: env(safe-area-inset-right, 0px)

      // Expose as custom properties
      --st-safe-area-top: env(safe-area-inset-top, 0px)
      --st-safe-area-bottom: env(safe-area-inset-bottom, 0px)
      --st-safe-area-left: env(safe-area-inset-left, 0px)
      --st-safe-area-right: env(safe-area-inset-right, 0px)
    }
  }

  %example "@safe-area-container" {
    .app {
      @safe-area-container
    }
  }
}
