/// Tab Bar Navigation Macro
///
/// Creates a tab bar navigation component with configurable positioning.
/// Supports top and bottom positioning for iOS and Android patterns.
///
/// @example basic bottom tabs
/// .tab-bar {
///   @tabs {
///     background: #fff
///     border-top: 1px solid #eee
///   }
/// }
///
/// @example top tabs (Android-style)
/// .tab-bar {
///   @tabs(position: "top") {
///     background: #6200ee
///     color: #fff
///   }
/// }

%macro tabs {

  %form {
    @tabs(
      position: $position:("top" | "bottom") = "bottom"
    ) {
      $styles:properties
    }
  }

  %derives {
    $isTop: $position == "top"
    $isBottom: $position == "bottom"
  }

  %states {
    idle {
      display: flex
      flex-direction: row
      justify-content: space-around
      align-items: center
    }

    positioned-top when $isTop {
      position: fixed
      top: 0
      left: 0
      right: 0
      border-bottom: 1px solid #e0e0e0
    }

    positioned-bottom when $isBottom {
      position: fixed
      bottom: 0
      left: 0
      right: 0
      border-top: 1px solid #e0e0e0
    }
  }

  %when $isTop {
    %applies {
      $styles
    }
  }

  %when $isBottom {
    %applies {
      $styles
    }
  }
}
