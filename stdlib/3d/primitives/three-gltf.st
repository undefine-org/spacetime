/// Three GLTF Primitive — load and place a real 3D model.
///
/// Loads an uncompressed .gltf/.glb model with the vendored GLTFLoader and adds it
/// to the stage's scene, auto-centered and scaled to a target size so any model
/// frames sensibly without hand-tuning. Optional auto-spin, same as @object.
///
/// NOTE: DRACO-compressed meshes are not supported in this build — DRACOLoader's
/// source uses `import.meta.url`, which breaks the classic-script runtime (see
/// three-entry.ts). Export models without Draco compression (the common path).
///
/// FEAT-153/BUG-159: `src` accepts EITHER a literal path OR a `$signal`/
/// `$signal.field` reference (a plain `string`-typed param carries the raw
/// captured text either way — see three-stage.st's `resolveReactive`, the ONE
/// shared mechanism every 3D primitive reuses for this). When `src` is a
/// signal, the model HOT-SWAPS (old mesh disposed, new one loaded + centered)
/// every time the referenced signal changes — no full page/region remount
/// needed to change which model a `@gltf` shows.
///
/// @param &el         - the canvas hosting the @stage
/// @param src         - URL/path to the .glb/.gltf, OR a `$signal`/`$signal.field` reference
/// @param size        - target bounding size to fit the model into (0 = no rescale); default 2
/// @param posX/Y/Z    - position offset applied after centering
/// @param spinY       - auto-rotation speed (radians/second)
///
/// @yields $loaded    - true once the CURRENT model is in the scene (re-fires on hot-swap)
/// @yields $progress  - load progress 0..1 (when the server reports lengthComputable)
/// @yields $error     - error message string, or empty

%primitive three-gltf(
  &el,
  src: string = "",
  size: number = 2,
  posX: number = 0,
  posY: number = 0,
  posZ: number = 0,
  spinY: number = 0
) {
  %emit js {
    const canvas = %&el;
    const ctx = canvas && canvas._stThree;
    if (!ctx) { console.warn('[Spacetime] three-gltf: no @stage on this canvas.'); return; }
    const { THREE, scene } = ctx;
    const A = THREE.addons;
    const rawSrc = %src;

    const loader = new A.GLTFLoader();

    let root = null;
    let spinUnregister = null;
    let loadToken = 0; // guards a stale in-flight load from landing after a newer one starts
    const spin = %spinY;
    const targetSize = %size;

    const disposeCurrent = () => {
      if (spinUnregister) { spinUnregister(); spinUnregister = null; }
      if (root) {
        scene.remove(root);
        if (canvas._stObjects) {
          const i = canvas._stObjects.indexOf(root);
          if (i >= 0) canvas._stObjects.splice(i, 1);
        }
        if (canvas._stObject === root) canvas._stObject = null;
        root = null;
      }
    };

    const load = (url) => {
      if (!url) { console.warn('[Spacetime] three-gltf: no src given.'); return; }
      const myToken = ++loadToken;

      loader.load(
        url,
        (gltf) => {
          if (myToken !== loadToken) return; // a newer load superseded this one
          disposeCurrent();
          root = gltf.scene;

          // Center + scale-to-fit so any model frames nicely.
          if (targetSize > 0) {
            const box = new THREE.Box3().setFromObject(root);
            const sphere = new THREE.Sphere();
            box.getBoundingSphere(sphere);
            const center = sphere.center;
            root.position.sub(center);
            if (sphere.radius > 0) {
              const k = targetSize / (sphere.radius * 2);
              root.scale.setScalar(k);
              root.position.multiplyScalar(k);
            }
          }
          root.position.add(new THREE.Vector3(%posX, %posY, %posZ));

          scene.add(root);
          canvas._stObjects = canvas._stObjects || [];
          canvas._stObjects.push(root);
          canvas._stObject = root;

          if (spin !== 0) {
            spinUnregister = ctx.registerFrame((t, dt) => { root.rotation.y += spin * dt; });
          }
          %yield true -> $loaded;
          %yield 1 -> $progress;
        },
        (xhr) => {
          if (myToken !== loadToken) return;
          if (xhr && xhr.lengthComputable && xhr.total > 0) {
            %yield xhr.loaded / xhr.total -> $progress;
          }
        },
        (err) => {
          if (myToken !== loadToken) return;
          console.error('[Spacetime] three-gltf: load failed', url, err);
          %yield String((err && err.message) || err) -> $error;
        }
      );
    };

    // FEAT-153: resolveReactive calls load(url) once immediately with whatever
    // `rawSrc` resolves to NOW; if rawSrc is a `$signal` reference, it ALSO
    // re-invokes load(...) with the NEW value every time that signal changes —
    // one shared mechanism (three-stage.st), not a hand-rolled watch here.
    const unwatch = ctx.resolveReactive(rawSrc, load);

    %cleanup {
      unwatch();
      disposeCurrent();
    }
  }

  %exports {
    $loaded: bool
    $progress: number
    $error: string
  }
}
