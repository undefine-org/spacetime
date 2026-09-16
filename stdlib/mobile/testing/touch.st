/// Touch Event Simulation Helpers for Mobile Testing
///
/// Provides declarative touch simulation definitions using CSS variables
/// for test coordination. These helpers define touch states that test
/// frameworks can use to simulate gestures.
///
/// @example simulating a tap
/// .test-button {
///   @touch-tap(x: 50, y: 50)
/// }
///
/// @example simulating a swipe
/// .test-carousel {
///   @touch-swipe(
///     start-x: 300,
///     start-y: 200,
///     end-x: 50,
///     end-y: 200,
///     duration: 300
///   )
/// }


/// Touch Point Type
///
/// Represents a single touch point with coordinates and metadata.
///
/// @property x - X coordinate relative to element
/// @property y - Y coordinate relative to element
/// @property identifier - Touch identifier for multi-touch
/// @property force - Touch pressure (0.0 to 1.0)

@type TouchPoint {
  x: number;
  y: number;
  identifier?: number;
  force?: number;
}


/// Touch Tap Simulation
///
/// Defines CSS variables for a tap gesture at specific coordinates.
/// Test frameworks read these variables to dispatch touch events.
///
/// @param x - X coordinate for tap
/// @param y - Y coordinate for tap
/// @param duration - Tap duration in milliseconds
///
/// @example
/// .tap-target {
///   @touch-tap(x: 100, y: 100, duration: 100)
/// }

%macro touch-tap {

  %form {
    @touch-tap(
      x: $x:number = 0,
      y: $y:number = 0,
      duration: $duration:number = 100
    )
  }

  %states {
    base {
      --st-touch-type: "tap"
      --st-touch-x: $x
      --st-touch-y: $y
      --st-touch-duration: $duration
      --st-touch-identifier: 0
    }
  }
}


/// Touch Swipe Simulation
///
/// Defines CSS variables for a swipe gesture between two points.
///
/// @param start-x - Starting X coordinate
/// @param start-y - Starting Y coordinate
/// @param end-x - Ending X coordinate
/// @param end-y - Ending Y coordinate
/// @param duration - Swipe duration in milliseconds
///
/// @example left swipe
/// .swipeable {
///   @touch-swipe(
///     start-x: 300,
///     start-y: 200,
///     end-x: 50,
///     end-y: 200,
///     duration: 250
///   )
/// }
///
/// @example up swipe
/// .scrollable {
///   @touch-swipe(
///     start-x: 200,
///     start-y: 500,
///     end-x: 200,
///     end-y: 100,
///     duration: 200
///   )
/// }

%macro touch-swipe {

  %form {
    @touch-swipe(
      start-x: $startX:number = 0,
      start-y: $startY:number = 0,
      end-x: $endX:number = 0,
      end-y: $endY:number = 0,
      duration: $duration:number = 250
    )
  }

  %states {
    base {
      --st-touch-type: "swipe"
      --st-touch-start-x: $startX
      --st-touch-start-y: $startY
      --st-touch-end-x: $endX
      --st-touch-end-y: $endY
      --st-touch-duration: $duration
      --st-touch-identifier: 0
    }
  }
}


/// Touch Pinch Simulation
///
/// Defines CSS variables for a two-finger pinch gesture.
/// Used for testing zoom interactions.
///
/// @param center-x - Center X coordinate of pinch
/// @param center-y - Center Y coordinate of pinch
/// @param start-distance - Initial distance between fingers
/// @param end-distance - Final distance between fingers
/// @param duration - Pinch duration in milliseconds
///
/// @example pinch to zoom out
/// .zoomable {
///   @touch-pinch(
///     center-x: 200,
///     center-y: 300,
///     start-distance: 200,
///     end-distance: 50,
///     duration: 300
///   )
/// }
///
/// @example pinch to zoom in
/// .zoomable {
///   @touch-pinch(
///     center-x: 200,
///     center-y: 300,
///     start-distance: 50,
///     end-distance: 200,
///     duration: 300
///   )
/// }

%macro touch-pinch {

  %form {
    @touch-pinch(
      center-x: $centerX:number = 0,
      center-y: $centerY:number = 0,
      start-distance: $startDistance:number = 100,
      end-distance: $endDistance:number = 50,
      duration: $duration:number = 300
    )
  }

  %states {
    base {
      --st-touch-type: "pinch"
      --st-touch-center-x: $centerX
      --st-touch-center-y: $centerY
      --st-touch-start-distance: $startDistance
      --st-touch-end-distance: $endDistance
      --st-touch-duration: $duration
      --st-touch-scale: calc($endDistance / $startDistance)
    }
  }
}


/// Touch Long Press Simulation
///
/// Defines CSS variables for a long press gesture.
///
/// @param x - X coordinate for press
/// @param y - Y coordinate for press
/// @param duration - Hold duration in milliseconds
///
/// @example
/// .context-menu-trigger {
///   @touch-long-press(x: 100, y: 100, duration: 500)
/// }

%macro touch-long-press {

  %form {
    @touch-long-press(
      x: $x:number = 0,
      y: $y:number = 0,
      duration: $duration:number = 500
    )
  }

  %states {
    base {
      --st-touch-type: "long-press"
      --st-touch-x: $x
      --st-touch-y: $y
      --st-touch-duration: $duration
      --st-touch-identifier: 0
    }
  }
}


/// Touch Drag Simulation
///
/// Defines CSS variables for a drag gesture with multiple waypoints.
/// Uses start/end points; for complex paths use touch-path.
///
/// @param start-x - Starting X coordinate
/// @param start-y - Starting Y coordinate
/// @param end-x - Ending X coordinate
/// @param end-y - Ending Y coordinate
/// @param duration - Drag duration in milliseconds
///
/// @example
/// .draggable-item {
///   @touch-drag(
///     start-x: 50,
///     start-y: 100,
///     end-x: 250,
///     end-y: 100,
///     duration: 400
///   )
/// }

%macro touch-drag {

  %form {
    @touch-drag(
      start-x: $startX:number = 0,
      start-y: $startY:number = 0,
      end-x: $endX:number = 0,
      end-y: $endY:number = 0,
      duration: $duration:number = 400
    )
  }

  %states {
    base {
      --st-touch-type: "drag"
      --st-touch-start-x: $startX
      --st-touch-start-y: $startY
      --st-touch-end-x: $endX
      --st-touch-end-y: $endY
      --st-touch-duration: $duration
      --st-touch-delta-x: calc($endX - $startX)
      --st-touch-delta-y: calc($endY - $startY)
      --st-touch-identifier: 0
    }
  }
}


/// Touch State Container
///
/// Provides CSS variables for tracking current touch state.
/// Useful for visual feedback during test execution.
///
/// @example
/// .test-overlay {
///   @touch-state
/// }

%macro touch-state {

  %form {
    @touch-state
  }

  %states {
    base {
      --st-touch-active: 0
      --st-touch-count: 0
      --st-touch-x: 0
      --st-touch-y: 0
      --st-touch-identifier: 0
    }
  }
}


/// Touch Gesture Presets
///
/// Common gesture configurations as a convenience wrapper.
///
/// @param gesture - Preset gesture name
///
/// Supported presets:
/// - swipe-left: Left swipe from center-right to center-left
/// - swipe-right: Right swipe from center-left to center-right
/// - swipe-up: Up swipe from bottom-center to top-center
/// - swipe-down: Down swipe from top-center to bottom-center
/// - pinch-in: Pinch to zoom out
/// - pinch-out: Pinch to zoom in
///
/// @example
/// .card {
///   @touch-preset("swipe-left")
/// }

%macro touch-preset {

  %form {
    @touch-preset($gesture:string = "swipe-left")
  }

  %states {
    base {
      --st-touch-preset: "%$gesture"
    }
  }

  // Swipe left preset
  %if $gesture == "swipe-left" {
    %states {
      base {
        --st-touch-type: "swipe"
        --st-touch-start-x: 300
        --st-touch-start-y: 200
        --st-touch-end-x: 50
        --st-touch-end-y: 200
        --st-touch-duration: 250
      }
    }
  }

  // Swipe right preset
  %if $gesture == "swipe-right" {
    %states {
      base {
        --st-touch-type: "swipe"
        --st-touch-start-x: 50
        --st-touch-start-y: 200
        --st-touch-end-x: 300
        --st-touch-end-y: 200
        --st-touch-duration: 250
      }
    }
  }

  // Swipe up preset
  %if $gesture == "swipe-up" {
    %states {
      base {
        --st-touch-type: "swipe"
        --st-touch-start-x: 200
        --st-touch-start-y: 400
        --st-touch-end-x: 200
        --st-touch-end-y: 100
        --st-touch-duration: 250
      }
    }
  }

  // Swipe down preset
  %if $gesture == "swipe-down" {
    %states {
      base {
        --st-touch-type: "swipe"
        --st-touch-start-x: 200
        --st-touch-start-y: 100
        --st-touch-end-x: 200
        --st-touch-end-y: 400
        --st-touch-duration: 250
      }
    }
  }

  // Pinch in preset (zoom out)
  %if $gesture == "pinch-in" {
    %states {
      base {
        --st-touch-type: "pinch"
        --st-touch-center-x: 200
        --st-touch-center-y: 300
        --st-touch-start-distance: 200
        --st-touch-end-distance: 50
        --st-touch-duration: 300
        --st-touch-scale: 0.25
      }
    }
  }

  // Pinch out preset (zoom in)
  %if $gesture == "pinch-out" {
    %states {
      base {
        --st-touch-type: "pinch"
        --st-touch-center-x: 200
        --st-touch-center-y: 300
        --st-touch-start-distance: 50
        --st-touch-end-distance: 200
        --st-touch-duration: 300
        --st-touch-scale: 4
      }
    }
  }
}
