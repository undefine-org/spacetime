/// Gesture macros: `@swipe` + `@resize`.
///
/// NOTE: `@drag` MOVED to the `stdlib/dnd` module (PLAN-058) — drag-and-drop is
/// one concern (gesture + drop zones + lifecycle), so it lives together there.
/// A page that uses `@drag` (plain or with drop zones) imports `stdlib/dnd`.
/// `@swipe` and `@resize` are distinct gestures and stay here; they bind the
/// same shared `gesture` primitive (stdlib/primitives/gesture.st), which is
/// unchanged and still shared by drag/swipe/resize/mobile.


/// Swipe Macro
///
/// Detects swipe gestures and triggers actions.
///
/// @example
/// .carousel-item {
///   @swipe {
///     on-left: nextSlide()
///     on-right: prevSlide()
///   }
/// }

%macro swipe {
  %form {
    @swipe(
      threshold: $threshold:number = 50,
      velocity: $velocityThreshold:number = 300
    ) {
      on-left: $onLeft:expr?
      on-right: $onRight:expr?
      on-up: $onUp:expr?
      on-down: $onDown:expr?
    }
  }

  %binds {
    gesture(&self) -> {
      $active,
      $deltaX,
      $deltaY,
      $finalVelocityX,
      $finalVelocityY
    }
  }

  %on $active -> false {
    // Determine swipe direction
    %derives {
      $absX: abs($deltaX)
      $absY: abs($deltaY)
      $isHorizontal: $absX > $absY
      $meetsThreshold: max($absX, $absY) >= $threshold
      $meetsVelocity: max(abs($finalVelocityX), abs($finalVelocityY)) >= $velocityThreshold
    }

    %if $meetsThreshold || $meetsVelocity {
      %if $isHorizontal {
        %if $deltaX < 0 { $onLeft?.() }
        %if $deltaX > 0 { $onRight?.() }
      } %else {
        %if $deltaY < 0 { $onUp?.() }
        %if $deltaY > 0 { $onDown?.() }
      }
    }
  }
}


/// Resize Handle Macro
///
/// Makes an element resizable via a drag handle.
///
/// @example
/// .resizable-panel {
///   @resize(direction: "horizontal", min: 200px, max: 600px)
/// }

%macro resize-handle {
  %form {
    @resize(
      direction: $direction:("horizontal" | "vertical" | "both") = "both",
      min: $min:length = 0,
      max: $max:length = none,
      handle: $handle:selector = ".resize-handle"
    )
  }

  %binds {
    gesture(&self >> $handle) -> { $active, $deltaX, $deltaY }
    resize(&self) -> { $width, $height }
  }

  %derives {
    $newWidth: $direction != "vertical"
      ? clamp($min, $width + $deltaX, $max ?? 9999)
      : $width

    $newHeight: $direction != "horizontal"
      ? clamp($min, $height + $deltaY, $max ?? 9999)
      : $height
  }

  %animates {
    width: $newWidth
    height: $newHeight
  }

  %states {
    resizing when $active {
      cursor: $direction == "horizontal" ? ew-resize
            : $direction == "vertical" ? ns-resize
            : nwse-resize
    }
  }
}
