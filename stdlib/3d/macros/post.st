/// Post Macro — cinematic post-processing for the stage.
///
/// `@post(…)` routes the stage through an EffectComposer with bloom and optional
/// chromatic aberration. Place inside `@stage` (it wraps the whole render).
///
/// @example glowing bloom
/// @stage {
///   @object icosahedron(emissive: #4060ff, emissiveIntensity: 2)
///   @post(bloomStrength: 0.8, bloomThreshold: 0.6)
/// }
///
/// @example bloom + chromatic aberration (neon tunnel look)
/// @post(bloomStrength: 0.7, rgbShift: 0.0025)
///
/// @param bloomStrength  - bloom strength, 0 disables (default 0.6)
/// @param bloomRadius    - bloom radius (default 0.4)
/// @param bloomThreshold - bloom luminance threshold (default 0.85)
/// @param rgbShift       - chromatic aberration amount, 0 disables (default 0)

%macro post {

  %scope selector

  %form {
    @post(
      bloomStrength: $bloomStrength:number = 0.6,
      bloomRadius: $bloomRadius:number = 0.4,
      bloomThreshold: $bloomThreshold:number = 0.85,
      rgbShift: $rgbShift:number = 0
    )
  }

  %binds {
    three-post(&self,
      bloomStrength: $bloomStrength,
      bloomRadius: $bloomRadius,
      bloomThreshold: $bloomThreshold,
      rgbShift: $rgbShift
    ) -> {
      $ready
    }
  }
}
