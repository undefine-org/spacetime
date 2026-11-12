/// Post Macro — cinematic post-processing for the stage.
///
/// `@post(…)` routes the stage through an EffectComposer with bloom and optional
/// chromatic aberration. Place inside `@stage` (it wraps the whole render).

%macro post {

  %scope selector

  %form {
    @post(
      /// bloom strength, 0 disables
      bloomStrength: $bloomStrength:number = 0.6,
      bloomRadius: $bloomRadius:number = 0.4,
      /// bloom luminance threshold
      bloomThreshold: $bloomThreshold:number = 0.85,
      /// chromatic aberration amount, 0 disables
      rgbShift: $rgbShift:number = 0
    )
  }

  %binds {
    three-post(&self, ...) -> {
      $ready
    }
  }

  %example "glowing bloom" {
    @stage {
      @object icosahedron(emissive: #4060ff, emissiveIntensity: 2)
      @post(bloomStrength: 0.8, bloomThreshold: 0.6)
    }
  }

  %example "bloom + chromatic aberration (neon tunnel look)" {
    @post(bloomStrength: 0.7, rgbShift: 0.0025)
  }
}
