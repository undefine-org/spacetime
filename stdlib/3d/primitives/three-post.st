/// Three Post-Processing Primitive — bloom + chromatic aberration.
///
/// Builds an EffectComposer (RenderPass → UnrealBloomPass → optional RGB-shift →
/// OutputPass) and swaps the stage's render call to draw through it via
/// `ctx.setRender`. Resizes with the canvas through `ctx.onResize`. This is the
/// pass stack behind the glowing, cinematic look of award-winning 3D sites.
///
/// Both effects are individually gated: bloomStrength <= 0 omits bloom,
/// rgbShift <= 0 omits the chromatic-aberration pass — so the composer only does
/// the work the author asked for.
///
/// @param &el           - the canvas hosting the @stage
/// @param bloomStrength - UnrealBloom strength (0 = no bloom); default 0.6
/// @param bloomRadius   - bloom radius; default 0.4
/// @param bloomThreshold- luminance threshold above which pixels bloom; default 0.85
/// @param rgbShift      - chromatic-aberration amount (0 = none); default 0
///
/// @yields $ready       - true once the composer is driving the stage

%primitive three-post(
  &el,
  bloomStrength: number = 0.6,
  bloomRadius: number = 0.4,
  bloomThreshold: number = 0.85,
  rgbShift: number = 0
) {
  %emit js {
    const canvas = %&el;
    const ctx = canvas && canvas._stThree;
    if (!ctx) { console.warn('[Spacetime] three-post: no @stage on this canvas.'); return; }
    const { THREE, scene, camera, renderer } = ctx;
    const A = THREE.addons;

    const sz = ctx.size();
    const composer = new A.EffectComposer(renderer);
    composer.setSize(sz.width, sz.height);
    composer.setPixelRatio(renderer.getPixelRatio());

    composer.addPass(new A.RenderPass(scene, camera));

    if (%bloomStrength > 0) {
      const bloom = new A.UnrealBloomPass(new THREE.Vector2(sz.width, sz.height), %bloomStrength, %bloomRadius, %bloomThreshold);
      composer.addPass(bloom);
    }

    if (%rgbShift > 0) {
      const shift = new A.ShaderPass(A.RGBShiftShader);
      shift.uniforms['amount'].value = %rgbShift;
      composer.addPass(shift);
    }

    composer.addPass(new A.OutputPass());

    ctx.setRender(function() { composer.render(); });
    ctx.onResize(function(w, h, dpr) {
      composer.setSize(w, h);
      composer.setPixelRatio(dpr);
    });

    %yield true -> $ready;

    %cleanup {
      ctx.setRender(function() { renderer.render(scene, camera); });
      composer.dispose();
    }
  }

  %exports {
    $ready: bool
  }
}
