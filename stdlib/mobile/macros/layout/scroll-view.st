/// Scroll View Macro
///
/// Creates a scroll container with mobile optimizations including
/// momentum scrolling and optional snap behavior.
///
/// @example vertical scrolling
/// .content-list {
///   @scroll-view(direction: "y", momentum: true) {
///     scrolling {
///       pointer-events: none
///     }
///   }
/// }
///
/// @example horizontal carousel with snap
/// .carousel {
///   @scroll-view(direction: "x", momentum: true, snap: true) {
///     scrolling {
///       pointer-events: none
///     }
///   }
/// }
///
/// @example bidirectional scroll
/// .canvas {
///   @scroll-view(direction: "both") {
///     scrolling {
///       cursor: grabbing
///     }
///   }
/// }

%macro scroll-view {

  %form {
    @scroll-view(
      direction: $direction:("x" | "y" | "both") = "y",
      momentum: $momentum:bool = true,
      snap: $snap:bool = false
    ) {
      $styles:properties
    }
  }

  %binds {
    pointer(&self, events: ["enter", "leave"]) -> {
      $isHovered
    }
  }

  %derives {
    // Direction flags
    $isHorizontal: $direction == "x"
    $isVertical: $direction == "y"
    $isBoth: $direction == "both"

    // Export scroll position signals
    $scrollX: 0
    $scrollY: 0

    // Scroll state
    $isScrolling: false
  }

  %states {
    idle {
      position: relative
      -webkit-overflow-scrolling: touch
    }

    scrolling when $isScrolling {
      user-select: none
    }

    horizontal when $isHorizontal {
      overflow-x: auto
      overflow-y: hidden
      scroll-snap-type: x mandatory
    }

    vertical when $isVertical {
      overflow-x: hidden
      overflow-y: auto
      scroll-snap-type: y mandatory
    }

    bidirectional when $isBoth {
      overflow: auto
    }

    momentum when $momentum {
      -webkit-overflow-scrolling: touch
      overscroll-behavior: contain
    }

    snap-enabled when $snap {
      scroll-snap-type: both mandatory
      scroll-behavior: smooth
    }
  }

  %when $isScrolling {
    %applies {
      $styles
    }
  }
}
