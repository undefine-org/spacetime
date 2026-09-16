/// Parallax Macro — pointer-driven camera sway.
///
/// `@parallax(…)` makes the stage camera follow the mouse with eased depth. Place
/// inside `@stage`. Skips itself when `@orbit` is present (orbit owns the camera).
///
/// @example
/// @stage {
///   @parallax(strength: 0.18)
///   @object icosahedron(material: glass)
/// }
///
/// @param strength  - max camera offset at screen edge (default 0.15)
/// @param ease      - per-frame smoothing 0..1 (default 0.06)

%macro parallax {

  %scope selector

  %form {
    @parallax(
      strength: $strength:number = 0.15,
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
}
