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

/// PLAN-152: the model is a PART (raea's `Model`). A named model answers to a
/// score: `&robot { clip: "Walking" -> "Death" at 60% --steps(1); posZ: -6 -> 0; }`.
/// `clip` names the glTF animation clip playing; the clip runs from the moment
/// it began, at `rate`. The pose is a SEEK into the clip (not a step), so a
/// virtual-clock render is deterministic. `scale` is the model's world scale
/// (both engines); `size` auto-fits instead, in a browser only.
///
%primitive three-gltf(
  &el,
  name: string = "",
  src: string = "",
  clip: string = "",
  size: number = 2,
  scale: number = 0,
  posX: number = 0,
  posY: number = 0,
  posZ: number = 0,
  rotateY: number = 0,
  spinY: number = 0,
  bind: string = ""
) {
  %emit js {
    const canvas = %&el;
    const ctx = canvas && canvas._stThree;
    if (!ctx) { console.warn('[Spacetime] three-gltf: no @stage on this canvas.'); return; }
    const raw = %src;
    const src = (typeof raw === 'string' && raw.charAt(0) === '$' && typeof ST !== 'undefined' && ST.resolvePath) ? ST.resolvePath(canvas, raw.slice(1)) : raw;
    if (!src) { console.warn('[Spacetime] three-gltf: no src given.'); return; }
    let k = %scale;
    if (!(k > 0) && ctx.headless) {
      console.warn('[Spacetime] @gltf ' + src + ': give `scale:` — `size:` auto-fit needs the loaded mesh, which only a browser has; drawing at scale 1');
      k = 1;
    }
    // mutable rest pose: a hot edit (PLAN-155) can move it
    const rest = { spin: %spinY, ry: %rotateY * Math.PI / 180 };
    const spin = rest.spin;
    const ry = rest.ry;
    const pt = ctx.part('Model', {
      src, animation: %clip || undefined, time: 0,
      position: [%posX, %posY, %posZ], rotation: [0, ry, 0], scale: k > 0 ? k : 1,
      castShadow: !!ctx.shadows,
    }, { name: String(%name || '').replace(/^&/, ''), bind: %bind,
         update: spin ? (p, t) => { p.props.rotation = [0, rest.ry + rest.spin * t, 0]; } : null });

    // PLAN-155 hot bind: the same model, a new pose / scale / starting clip
    // (a new file, or a spin started or stopped, is a reload).
    ST.hot(%bind, (a) => {
      if (a.src !== raw || a.name !== %name || !!a.spinY !== !!spin) return false;
      if (!(a.scale > 0) !== !(k > 0)) return false;
      rest.ry = a.rotateY * Math.PI / 180;
      rest.spin = a.spinY;
      pt.props.position = [a.posX, a.posY, a.posZ];
      pt.props.rotation = [0, rest.ry, 0];
      if (a.scale > 0) pt.props.scale = a.scale;
      if (!pt.channels || pt.channels.clip == null) pt.props.animation = a.clip || undefined;
      ctx.touch(pt);
      return true;
    });
    // `size` (browser only): scale the loaded model to fit
    if (!(k > 0) && pt.obj && ctx.THREE) {
      const THREE = ctx.THREE, target = %size;
      const fit = () => {
        const box = new THREE.Box3().setFromObject(pt.obj);
        if (box.isEmpty()) return false;
        const sph = new THREE.Sphere(); box.getBoundingSphere(sph);
        if (sph.radius > 0) pt.props.scale = target / (sph.radius * 2);
        return true;
      };
      const stop = ctx.registerFrame(() => { if (fit()) stop(); });
    }
    %yield true -> $loaded;
    %yield 1 -> $progress;
  }

  %exports {
    $loaded: bool
    $progress: number
    $error: string
  }
}
