/// Modal Overlay Macro
///
/// Creates a modal overlay with fullscreen or card presentation.
/// Supports state-based visibility and backdrop interactions.
///
/// @example card modal
/// .modal-container {
///   @modal {
///     background: white
///     border-radius: 12px
///   }
/// }
///
/// @example fullscreen modal
/// .fullscreen-modal {
///   @modal(type: "fullscreen") {
///     background: white
///   }
/// }

%macro modal {

  %form {
    @modal(
      type: $type:("fullscreen" | "card") = "card"
    ) {
      $styles:properties
    }
  }

  %binds {
    pointer(&self, events: ["enter", "leave"]) -> {
      $isOver
    }
  }

  %derives {
    // Card modals get centered positioning with padding
    $isCard: $type == "card"

    // Fullscreen modals fill the viewport
    $isFullscreen: $type == "fullscreen"
  }

  %states {
    hidden {
      opacity: 0
      visibility: hidden
      pointer-events: none
    }

    visible {
      opacity: 1
      visibility: visible
      pointer-events: auto
    }

    card-style when $isCard {
      position: fixed
      top: 50%
      left: 50%
      transform: translate(-50%, -50%)
      max-width: 90vw
      max-height: 90vh
      border-radius: 12px
      box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.25)
    }

    fullscreen-style when $isFullscreen {
      position: fixed
      top: 0
      left: 0
      right: 0
      bottom: 0
      width: 100vw
      height: 100vh
    }
  }

  %when $isOver {
    %applies {
      $styles
    }
  }
}
