/// Three Environment Primitive — image-based lighting + reflections.
///
/// Generates a pre-filtered environment map from the vendored RoomEnvironment
/// (a procedural studio room) using PMREMGenerator, and assigns it as the scene's
/// `environment` so all PBR materials pick up realistic reflections — the single
/// biggest contributor to the "studio glass" look. Optionally also sets it as the
/// visible `background`.

%primitive three-env(
  /// the canvas hosting the @stage
  &el,
  preset: string = "studio",
  intensity: number = 1.0,
  background: bool = false
) {
  %emit js {
    const canvas = %&el;
    const ctx = canvas && canvas._stThree;
    if (!ctx) { console.warn('[Spacetime] three-env: no @stage on this canvas.'); return; }
    const { THREE, scene, renderer } = ctx;
    const A = THREE.addons;

    const pmrem = new THREE.PMREMGenerator(renderer);
    let envRT = null;
    try {
      const roomScene = new A.RoomEnvironment();
      envRT = pmrem.fromScene(roomScene, 0.04);
      scene.environment = envRT.texture;
      if (THREE.Scene.prototype.hasOwnProperty('environmentIntensity') || 'environmentIntensity' in scene) {
        scene.environmentIntensity = %intensity;
      }
      if (%background) scene.background = envRT.texture;
      if (roomScene.dispose) roomScene.dispose();
    } catch (e) {
      console.error('[Spacetime] three-env: failed to build environment map.', e);
    } finally {
      pmrem.dispose();
    }

    %yield true -> $ready;

    %cleanup {
      if (scene.environment === (envRT && envRT.texture)) scene.environment = null;
      if (scene.background === (envRT && envRT.texture)) scene.background = null;
      if (envRT) envRT.dispose();
    }
  }

  %exports {
    /// true once the env map is applied
    $ready: bool
  }
}
