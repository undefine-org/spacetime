/// `@drag` — make an element draggable, with an optional declarative drop
/// LIFECYCLE (PLAN-058). This is the SOLE `@drag` macro: it lives in the dnd
/// module because drag-and-drop is one concern. A page that drags ANYTHING
/// imports `stdlib/dnd`. (`@swipe` / `@resize` are distinct gestures and stay in
/// stdlib/macros/drag.st.)
///
/// @example plain (no drop zones)
/// .slider-handle {
///   @drag(axis: "x") {
///     dragging { scale: 1.1 }
///     on-drop: handleRelease($deltaX)        // zero-zone short form (the `$` is the element)
///   }
/// }
///
/// @example with drop zones (the declarative lifecycle)
/// .card {
///   @drag(group: "cards", payload: $c) {
///     dragging { scale: 1.04 }               // visual state while active
///     on-enter $zone { ... }                  // pointer entered a compatible zone
///     on-leave $zone { ... }                  // pointer left it
///     on-drop  $zone { $move({ card: $payload.id, to: $zone.col }); } // over an ACCEPTING zone
///     on-cancel { ... }                       // released over nothing / a rejecting zone
///   }
/// }
///
/// `group` pairs with `@drop-zone(group:)`. `payload: $c` names the dragged datum
/// (`$payload` in the clauses). `on-drop $zone` fires only over an ACCEPTING zone;
/// otherwise `on-cancel` runs (declarative spring-back, no manual JS). `$zone`
/// reads the zone's dataset (`$zone.col` ← `data-col`). `axis` constrains motion;
/// `bounds: &el` clamps to an element (PLAN-057). The zero-zone `on-drop: <expr>`
/// short form stays for trivial drags.

%macro drag {
  %form {
    @drag(
      group: $group:string = "",
      payload: $payload:expr? = none,
      axis: $axis:("x" | "y" | "both") = "both",
      bounds: $bounds:element? = none,
      threshold: $threshold:number = 0,
      momentum: $momentum:bool = false
    ) {
      $customStates:states?
      ( $lifecycle:drag_lifecycle )?
      on-drop: $onDrop:expr?
    }
  }

  %binds {
    // The dnd runtime prelude (zone registry) — bound so it always ships when a
    // page drags. A macro CAN bind multiple primitives; a primitive cannot %binds.
    dnd-runtime() -> { }

    // The generic gesture: visual drag (delta/velocity/active/drop pulse).
    gesture(&self, threshold: $threshold) -> {
      $active,
      $deltaX,
      $deltaY,
      $velocityX,
      $velocityY,
      $finalVelocityX,
      $finalVelocityY,
      $dropped
    }

    // The drop lifecycle: hit-tests zones, fires on-enter/leave/drop/cancel with
    // $zone/$payload bound. Inert when no zones are registered (hitTest → null),
    // so a plain `@drag(axis:…)` page pays only the (idempotent) prelude.
    drag-zones(&self, group: $group, payload: $payload, arms: $lifecycle) -> {
      $overZone
    }
  }

  %derives {
    // Axis constraints.
    $rawX: $axis != "y" ? $deltaX : 0
    $rawY: $axis != "x" ? $deltaY : 0

    // Bounds constraints (PLAN-057): clamp to the bounds element when provided.
    $x: $bounds != none
      ? clamp($rawX, 0, &$bounds.rect.width - &self.rect.width)
      : $rawX

    $y: $bounds != none
      ? clamp($rawY, 0, &$bounds.rect.height - &self.rect.height)
      : $rawY
  }

  %states {
    idle {
      cursor: grab
      user-select: auto
    }

    dragging when $active {
      cursor: grabbing
      user-select: none
    }

    // Merge the caller's custom states (e.g. `dragging { scale: 1.04 }`).
    $customStates?
  }

  %animates {
    translate-x: $x
    translate-y: $y
  }

  %on $dropped -> true {
    // The zero-zone short form: run the caller's `on-drop: <expr>` on a GENUINE
    // release (the `$dropped` false->true pulse, never on cancel). Inside the
    // action `$` is the dragged element. When the lifecycle `on-drop $zone { }`
    // arm is used instead, this is empty (drag-zones owns the zone-aware drop).
    $onDrop
  }
}
