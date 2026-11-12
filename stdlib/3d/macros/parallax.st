/// Parallax Macro — pointer-driven camera sway.
///
/// `@parallax(…)` makes the stage camera follow the mouse with eased depth. Place
/// inside `@stage`. Skips itself when `@orbit` is present (orbit owns the camera).

%macro parallax {

  %scope selector

  %form {
    @parallax(
      /// max camera offset at screen edge
      strength: $strength:number = 0.15,
      /// per-frame smoothing 0..1
      ease: $ease:number = 0.06
    )
  }

  %binds {
    three-parallax(&self,
      strength: $strength,
      ease: $ease
    ) -> {
      $active
    }
  }

  %example "@parallax" {
    @stage {
      @parallax(strength: 0.18)
      @object icosahedron(material: glass)
    }
  }
}
