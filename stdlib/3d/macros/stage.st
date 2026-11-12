/// Stage Macro — the declarative entry point for 3D.
///
/// `@stage { … }` on a `<canvas>` sets up the three.js renderer, scene, camera,
/// and render loop, then hosts child directives (`@object`, `@light`, `@env`,
/// `@orbit`, `@parallax`, `@post`, `@gltf`, `@particles`). Authors never touch
/// three.js — the engine lives behind this surface.

/// How the stage maps bright light to the screen.
%capture_type stage_tone {
    /// a filmic curve: highlights roll off softly
    "aces"
    /// no curve: values clip at white
  | "linear"
  | "reinhard"
    /// no tone mapping at all
  | "none"
}

%macro stage {

  // GH-12 (PLAN-137 W6): a stage ONLY renders into a <canvas>. Declaring the
  // required element as `%scope element(canvas)` makes the compiler refuse a
  // `@stage` bound to a non-canvas selector (div.hero -> E0963) and warn when
  // the element is unknowable (.hero -> W0963) — previously it compiled clean
  // and failed only at runtime (three.js `canvas.getContext` throw on a div).
  %scope selector | element(canvas)

  %form {
    @stage(
      /// the camera's vertical field of view
      fov: $fov:degrees = 50,
      /// the nearest and farthest the camera sees
      near: $near:meters = 0.1,
      far: $far:meters = 100,
      /// initial camera position
      camX: $camX:number = 0,
      camY: $camY:number = 0,
      camZ: $camZ:number = 5,
      /// tone-mapping exposure
      exposure: $exposure:number = 1.0,
      /// how bright light is mapped to the screen
      tone: $tone:stage_tone = aces,
      /// transparent background
      alpha: $alpha:bool = true,
      /// "" (off) | "soft" | "hard": shadow maps for every light that casts and every part
      shadows: $shadows:string = "",
      /// the 3D's clear colour (hex); "" keeps the canvas transparent
      background: $background:string = "",
      /// glow around bright and emissive parts (0 none, 1 natural, 2 strong); raea's `Bloom`
      bloom: $bloom:number = 0
    ) {
      $children*
    }
  }

  // `pos: 0 3 -7` is `posX: 0, posY: 3, posZ: -7` (one value sets all three);
  // the later of a shorthand and a longhand wins, as in CSS
  %shorthand cam { camX camY camZ }

  %binds {
    three-stage(&self, ...) -> {
      $ready,
      $frame,
      $fps
    }
  }

  %template {
    $children
  }

  %example "a lit, orbitable object" {
    @import "stdlib/3d"
    canvas.hero {
      @stage(fov: 50, camZ: 5, tone: aces) {
        @light (type: directional, intensity: 2)
        @object (shape: torusknot, size: 1, material: standard, color: #c0c5ff, roughness: 0.2, metalness: 0.9)
        @orbit
      }
    }
    <canvas class="hero"></canvas>
  }
}
