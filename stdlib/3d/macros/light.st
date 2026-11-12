/// A light on the stage. Name it (`@light &sun (…)`) so a score can animate it:
/// `@on &.clip { &sun { intensity: 0.5 -> 3; } }`.

/// How a light shines.
%capture_type light_kind {
    /// light from everywhere at once, without shadows: a fill
    "ambient"
    /// parallel rays from far away, like the sun: a key light
  | "directional"
    /// light from one point, in every direction, fading with distance: a lamp
  | "point"
    /// a cone from one point toward its target: a stage spot
  | "spot"
}

%macro light {

  // GH-12 note: @light deliberately does NOT take `element(canvas)`. It is a
  // SCENE CHILD (composes against the stage handle), not a canvas-bounded DOM
  // element — the @stage macro alone owns the canvas element constraint.
  %scope selector

  %capture_type stage_light_name { $name:element }

  %form {
    @light $named:stage_light_name? (
      /// how it shines
      type: $type:light_kind = directional,
      color: $color:color = white,
      intensity: $intensity:intensity = 1.0,
      /// where it is; for a directional light, the direction it shines from
      posX: $posX:number = 3,
      posY: $posY:number = 5,
      posZ: $posZ:number = 4,
      /// where a directional or spot light points
      targetX: $targetX:number = 0,
      targetY: $targetY:number = 0,
      targetZ: $targetZ:number = 0,
      /// how far a point or spot light reaches; 0 reaches forever
      distance: $distance:meters = 0,
      /// how fast a point or spot light fades with distance (2 is physical)
      decay: $decay:number = 2,
      /// the spot's cone, from its axis to its edge
      angle: $angle:degrees = 30,
      /// how soft the spot's edge is
      penumbra: $penumbra:unit = 0.3,
      /// cast shadows (directional and spot lights always do when the stage has shadows)
      castShadow: $castShadow:bool = false
    ) {
      $children*
    }
  }

  // `pos: 0 3 -7` is `posX: 0, posY: 3, posZ: -7` (one value sets all three);
  // the later of a shorthand and a longhand wins, as in CSS
  %shorthand pos { posX posY posZ }
  %shorthand target { targetX targetY targetZ }

  %binds {
    three-light(&self, name: $named, ...) -> {
      $added
    }
  }

  %registers part($named) {
    kind: light
  }

  %template {
    $children
  }

  %example "a sun-like key light plus a soft fill" {
    @import "stdlib/3d"
    canvas.world {
      @stage {
        @light (type: ambient, intensity: 0.4)
        @light &sun (type: directional, intensity: 2.2, posX: 4, posY: 6, posZ: 3)
        @object (shape: box)
      }
    }
    <canvas class="world"></canvas>
  }

  %example "a warm lamp the score can switch on" {
    @import "stdlib/3d"
    canvas.world {
      @stage {
        @light &lamp (type: point, color: #ffcf8a, intensity: 0, pos: -2.6 2.3 -4, distance: 9)
        @object (shape: sphere, size: 0.3, pos: -2.6 2.3 -4)
      }
    }
    <canvas class="world"></canvas>
  }
}
