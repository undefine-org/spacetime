/// Three Orbit Primitive — drag-to-rotate camera controls.
///
/// Attaches the vendored OrbitControls to the stage camera + canvas, with inertial
/// damping and optional auto-rotation. The per-frame `controls.update()` is driven
/// by the stage's frame loop (registerFrame), so it stays in lockstep with render.

%primitive three-orbit(
  /// the canvas hosting the @stage
  &el,
  damping: number = 0.08,
  autoRotate: number = 0,
  enableZoom: bool = false,
  enablePan: bool = false,
  minDistance: number = 0,
  maxDistance: number = 1000,
  minPolar: number = 0,
  maxPolar: number = 180
) {
  %emit js {
    const canvas = %&el;
    const ctx = canvas && canvas._stThree;
    if (!ctx) { console.warn('[Spacetime] three-orbit: no @stage on this canvas.'); return; }
    const { THREE, camera, renderer } = ctx;
    const A = THREE.addons;

    const controls = new A.OrbitControls(camera, renderer.domElement);
    controls.enableDamping = %damping > 0;
    controls.dampingFactor = %damping > 0 ? %damping : 0.05;
    controls.enableZoom = %enableZoom;
    controls.enablePan = %enablePan;
    if (%autoRotate > 0) {
      controls.autoRotate = true;
      controls.autoRotateSpeed = %autoRotate * 10;
    }
    if (%minDistance > 0) controls.minDistance = %minDistance;
    controls.maxDistance = %maxDistance;
    controls.minPolarAngle = (%minPolar * Math.PI) / 180;
    controls.maxPolarAngle = (%maxPolar * Math.PI) / 180;

    const unregister = ctx.registerFrame(() => controls.update());
    canvas._stOrbit = controls;

    %yield true -> $ready;

    %cleanup {
      if (unregister) unregister();
      controls.dispose();
      canvas._stOrbit = null;
    }
  }

  %exports {
    /// true once controls are attached
    $ready: bool
  }
}
