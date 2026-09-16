/// `@post { … }` — a cinematic "lens" over any element (PLAN-150 W2).
///
/// A film has a lens: bloom (glow around bright things), film grain, an edge
/// vignette. On a `@stage` canvas these are real render passes; on ordinary
/// DOM they are compositor layers. THE SAME directive drives both — this is the
/// DOM half; the `@stage` half lives in `stdlib/3d`.
///
/// The block's keys set the INITIAL look:
///
///   .stage {
///     @post { bloom: 0.45; grain: 0.035; vignette: 0.55; }
///   }
///
/// Animating the look is animating the custom properties it reads, with the
/// ordinary keyframe engine (so positioned stops etc. all apply):
///
///   .stage {
///     @post { bloom: 0.45; }
///     @on &.clip { --st-post-bloom: 0.45 -> 1.3 at 61% -> 0.45; }
///   }
///
/// Keys → properties: `bloom` → `--st-post-bloom`, `grain` → `--st-post-grain`,
/// `vignette` → `--st-post-vignette`. Grain flicker follows
/// `--st-post-grain-seed` (drive it with `random()`/`--noise` once W6 lands).

%macro post-dom {

  %scope selector

  %form {
    @post {
      $styles:properties
    }
  }

  %binds {
    post-dom(&self, styles: $styles) -> { $ready }
  }
}
