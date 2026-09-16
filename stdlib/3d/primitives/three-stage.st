/// Three Stage Primitive — the engine boundary.
///
/// Owns the entire three.js lifecycle for a `<canvas>`: WebGLRenderer, Scene,
/// PerspectiveCamera, the requestAnimationFrame loop, resize + device-pixel-ratio
/// handling, WebGL context-loss recovery, and tone-mapping. Everything else in
/// the module (objects, lights, controls, post-processing) is a CHILD that reads
/// the shared handle this primitive stores on the canvas element:
///
///   el._stThree = {
///     THREE, scene, camera, renderer, clock,
///     registerFrame(cb)  // cb(t, dt) called every frame, returns an unsubscribe
///     setRender(fn)      // override the draw call (post-processing swaps it)
///     size()             // -> { width, height, dpr }
///   }
///
/// This is the ONE place three.js is touched directly per page; children compose
/// against the handle, never re-create a renderer. The vendored `THREE` global is
/// emitted into the page only because this primitive references it (lean law).
///
/// @param &el       - the <canvas> to render into
/// @param fov       - perspective vertical field of view (degrees)
/// @param near      - camera near plane
/// @param far       - camera far plane
/// @param camX/Y/Z  - initial camera position
/// @param exposure  - tone-mapping exposure
/// @param tone      - tone-mapping: "aces" | "linear" | "reinhard" | "none"
/// @param alpha     - transparent canvas background
///
/// @yields $ready   - true once the renderer + loop are live
/// @yields $frame   - frame counter (reactive)
/// @yields $fps     - frames per second (updated ~1Hz)

%primitive three-stage(
  &el,
  fov: number = 50,
  near: number = 0.1,
  far: number = 100,
  camX: number = 0,
  camY: number = 0,
  camZ: number = 5,
  exposure: number = 1.0,
  tone: string = "aces",
  alpha: bool = true
) {
  %emit js {
    const canvas = %&el;
    if (!canvas) {
      console.warn('[Spacetime] three-stage: target element not found.');
      return;
    }
    // GH-12 (PLAN-137 W6): the element constraint is a COMPILE-time error
    // (E0963) / warning (W0963) via `%scope element(canvas)` on the stage macro.
    // This runtime guard is the demoted safety net for the warn-not-error
    // UNKNOWABLE case (`.hero` — the element may be a runtime-inserted canvas):
    // it names the actual tag instead of letting three.js throw a cryptic
    // `canvas.getContext` error inside the WebGLRenderer constructor below.
    if (canvas.tagName !== 'CANVAS') {
      console.warn('[Spacetime] three-stage: target element is <' +
        canvas.tagName.toLowerCase() + '>, but a <canvas> is required.');
      return;
    }
    if (typeof THREE === 'undefined') {
      console.warn('[Spacetime] three-stage: three.js engine not loaded.');
      return;
    }

    // --- Renderer -----------------------------------------------------------
    let renderer;
    try {
      renderer = new THREE.WebGLRenderer({
        canvas,
        antialias: true,
        alpha: %alpha,
        powerPreference: 'high-performance',
      });
    } catch (e) {
      console.error('[Spacetime] three-stage: WebGL unavailable.', e);
      return;
    }
    renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));

    const toneMap = {
      aces: THREE.ACESFilmicToneMapping,
      linear: THREE.LinearToneMapping,
      reinhard: THREE.ReinhardToneMapping,
      none: THREE.NoToneMapping,
    };
    renderer.toneMapping = toneMap[%tone] ?? THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = %exposure;

    // --- Scene + camera -----------------------------------------------------
    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(%fov, 1, %near, %far);
    camera.position.set(%camX, %camY, %camZ);
    camera.lookAt(0, 0, 0);

    const clock = new THREE.Clock();

    // --- Sizing (DPR-aware, ResizeObserver-driven) --------------------------
    const sizeState = { width: 1, height: 1, dpr: renderer.getPixelRatio() };
    const resize = () => {
      const rect = canvas.getBoundingClientRect();
      const w = Math.max(1, Math.floor(rect.width));
      const h = Math.max(1, Math.floor(rect.height));
      if (w === sizeState.width && h === sizeState.height) return;
      sizeState.width = w;
      sizeState.height = h;
      renderer.setSize(w, h, false);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
      // notify any size listeners (e.g. post-processing composer)
      for (const cb of sizeListeners) cb(w, h, sizeState.dpr);
    };
    const sizeListeners = [];
    const resizeObs = new ResizeObserver(resize);
    resizeObs.observe(canvas);

    // --- Frame loop ---------------------------------------------------------
    const frameCbs = [];
    const registerFrame = (cb) => {
      frameCbs.push(cb);
      return () => {
        const i = frameCbs.indexOf(cb);
        if (i >= 0) frameCbs.splice(i, 1);
      };
    };

    // Default render = straight scene draw. Post-processing overrides via setRender.
    let renderFn = () => renderer.render(scene, camera);
    const setRender = (fn) => { renderFn = fn; };

    const sizeFn = () => ({ ...sizeState });

    // FEAT-149 / PLAN-062 Wave 1: the camera's true visible half-width/height
    // at a given world-space plane `z=<arg>` (the plane parallel to XY at that
    // Z coordinate). This is the exact frustum trig zellijist-2 (and any
    // exploded-view / scatter scene) used to hand-derive per-project to keep
    // procedurally-placed objects within frame — computed ONCE here, correctly,
    // from the camera's OWN live fov/aspect/position AND its ACTUAL view
    // direction (reviewer-caught bug: a first draft assumed a pure -Z look
    // direction and used `camera.position.z - z` for depth, which is WRONG for
    // any elevated/offset camera that still `lookAt(0,0,0)`s — confirmed a real
    // case in this repo: `demos/3d/genesis/index.st` uses `camY: 2.0` and
    // `everswap-clone` uses `camY: 18/40`, both tilted, not looking down -Z).
    //
    // Fix: derive the camera's REAL forward unit vector via
    // `camera.getWorldDirection()`, then compute DEPTH as the signed distance
    // from the camera to the target Z-plane ALONG that forward vector (not the
    // raw Z-axis delta) — this degrades gracefully (returns 0-size bounds, not
    // NaN/negative) only in the degenerate case where the view direction is
    // parallel to the target plane (forward.z ≈ 0), which no @stage scene in
    // this repo currently produces (audited: every camY/camX offset still
    // `lookAt(0,0,0)`, so forward.z is always meaningfully non-zero toward the
    // origin plane).
    const boundsFn = (z) => {
        const targetZ = typeof z === 'number' ? z : 0;
        const forward = new THREE.Vector3();
        camera.getWorldDirection(forward); // unit vector, camera's REAL look direction
        const dz = forward.z;
        let depth;
        if (Math.abs(dz) < 1e-6) {
            depth = 0; // view direction parallel to the target plane: no meaningful depth
        } else {
            // Distance along the view ray from the camera to the target Z-plane:
            // camera.position.z + t*dz = targetZ  =>  t = (targetZ - camera.position.z) / dz
            depth = Math.max(0, (targetZ - camera.position.z) / dz);
        }
        const vFovRad = (camera.fov * Math.PI) / 180;
        const halfH = Math.tan(vFovRad / 2) * depth;
        const halfW = halfH * camera.aspect;
        return { halfW, halfH };
    };

    // FEAT-153: reactive typed-param support, ONE mechanism every child primitive
    // opts into instead of each hand-rolling its own $-detection + watch. A typed
    // param declared `string` (not `expr`) arrives at the primitive as the RAW
    // captured text -- a literal ("/models/a.glb") OR a signal reference
    // ("$modelPath", "$modelPath.field") -- exactly like derived-signal.st's
    // `expr: string` param already does its OWN one-off regex rewrite for. This
    // generalizes that SAME pattern once, here, so every 3D primitive can reuse it
    // instead of re-deriving it:
    //   resolveReactive(rawValue, onChange) -> unsubscribe
    // Calls onChange(resolvedValue) once immediately. If rawValue is a `$name` /
    // `$name.field` reference, ALSO watches it in scope (ST.watchScoped, which
    // already fires immediately + on every subsequent change) and re-calls
    // onChange on each update; returns an unsubscribe fn. A plain literal calls
    // onChange once and returns a no-op unsubscribe (never reactive — nothing to
    // watch), matching every existing static-param primitive's behavior exactly.
    const resolveReactive = (rawValue, onChange) => {
      if (typeof rawValue !== 'string' || rawValue.charAt(0) !== '$') {
        onChange(rawValue);
        return () => {};
      }
      const path = rawValue.slice(1);
      const head = path.split('.')[0];
      const resolve = () => (typeof ST !== 'undefined' && ST.resolvePath) ? ST.resolvePath(canvas, path) : undefined;
      onChange(resolve());
      if (typeof ST !== 'undefined' && ST.watchScoped) {
        return ST.watchScoped(canvas, head, () => onChange(resolve()));
      }
      return () => {};
    };

    // FEAT-152: shared pointer-pick support. ONE Raycaster + ONE pointerdown/
    // pointermove listener per @stage (not per-object -- avoids N raycasters
    // firing on every event), against whichever meshes have registered a click/
    // hover handler via `onPick`. Each registered mesh is matched by object
    // IDENTITY (not a rebuilt intersection list per call) so @object/@gltf
    // primitives opt in without knowing about each other.
    const pickHandlers = new Map(); // THREE.Object3D -> { click, hover }
    const raycaster = new THREE.Raycaster();
    const pointer = new THREE.Vector2();
    let hoveredMesh = null;

    const setPointerFromEvent = (e) => {
      const rect = canvas.getBoundingClientRect();
      pointer.x = ((e.clientX - rect.left) / rect.width) * 2 - 1;
      pointer.y = -((e.clientY - rect.top) / rect.height) * 2 + 1;
    };

    // Walk up from the intersected object to find the REGISTERED root (a
    // primitive may register a group/root object whose CHILDREN are what the
    // raycaster actually hits, e.g. a loaded glTF scene graph).
    const findRegisteredAncestor = (obj) => {
      let node = obj;
      while (node) {
        if (pickHandlers.has(node)) return node;
        node = node.parent;
      }
      return null;
    };

    const intersectRegistered = () => {
      if (pickHandlers.size === 0) return null;
      raycaster.setFromCamera(pointer, camera);
      const hits = raycaster.intersectObjects(scene.children, true);
      for (const hit of hits) {
        const owner = findRegisteredAncestor(hit.object);
        if (owner) return owner;
      }
      return null;
    };

    const onPointerDown = (e) => {
      if (pickHandlers.size === 0) return;
      setPointerFromEvent(e);
      const owner = intersectRegistered();
      if (owner) {
        const h = pickHandlers.get(owner);
        if (h && h.click) h.click();
      }
    };
    const onPointerMove = (e) => {
      if (pickHandlers.size === 0) return;
      setPointerFromEvent(e);
      const owner = intersectRegistered();
      if (owner !== hoveredMesh) {
        if (hoveredMesh) {
          const prev = pickHandlers.get(hoveredMesh);
          if (prev && prev.hoverEnd) prev.hoverEnd();
        }
        hoveredMesh = owner;
        if (owner) {
          const next = pickHandlers.get(owner);
          if (next && next.hoverStart) next.hoverStart();
        }
      }
    };
    canvas.addEventListener('pointerdown', onPointerDown);
    canvas.addEventListener('pointermove', onPointerMove);

    // registerPick(object3D, { click?, hoverStart?, hoverEnd? }) -> unregister
    const registerPick = (obj, handlers) => {
      if (!obj) return () => {};
      pickHandlers.set(obj, handlers);
      return () => {
        pickHandlers.delete(obj);
        if (hoveredMesh === obj) hoveredMesh = null;
      };
    };

    // Shared handle every child primitive composes against.
    canvas._stThree = {
      THREE, scene, camera, renderer, clock,
      registerFrame, setRender,
      onResize: (cb) => { sizeListeners.push(cb); },
      size: sizeFn,
      bounds: boundsFn,
      resolveReactive,
      registerPick,
    };

    // --- FPS + tick ---------------------------------------------------------
    let frameCount = 0;
    let fpsFrames = 0;
    let fpsLast = performance.now();
    let rafId = null;
    let revealed = false;

    // Avoid first-frame flash: keep canvas hidden until something renders.
    canvas.style.opacity = '0';

    const tick = () => {
      const dt = clock.getDelta();
      const t = clock.elapsedTime;
      frameCount++;

      for (const cb of frameCbs) cb(t, dt);
      renderFn();

      if (!revealed) {
        revealed = true;
        canvas.style.transition = 'opacity 0.5s ease';
        canvas.style.opacity = '1';
      }

      %yield frameCount -> $frame;
      fpsFrames++;
      const now = performance.now();
      if (now - fpsLast >= 1000) {
        %yield Math.round((fpsFrames * 1000) / (now - fpsLast)) -> $fps;
        fpsFrames = 0;
        fpsLast = now;
      }
      rafId = requestAnimationFrame(tick);
    };

    // Pause when tab hidden (battery + correctness).
    const onVisibility = () => {
      if (document.hidden) {
        if (rafId) { cancelAnimationFrame(rafId); rafId = null; }
      } else if (!rafId) {
        clock.getDelta(); // discard the gap so dt doesn't spike
        rafId = requestAnimationFrame(tick);
      }
    };
    document.addEventListener('visibilitychange', onVisibility);

    // Context-loss recovery.
    const onLost = (e) => { e.preventDefault(); if (rafId) { cancelAnimationFrame(rafId); rafId = null; } };
    const onRestored = () => { if (!rafId && !document.hidden) rafId = requestAnimationFrame(tick); };
    canvas.addEventListener('webglcontextlost', onLost);
    canvas.addEventListener('webglcontextrestored', onRestored);

    // Go.
    resize();
    %yield true -> $ready;
    rafId = requestAnimationFrame(tick);

    %cleanup {
      if (rafId) cancelAnimationFrame(rafId);
      resizeObs.disconnect();
      document.removeEventListener('visibilitychange', onVisibility);
      canvas.removeEventListener('webglcontextlost', onLost);
      canvas.removeEventListener('webglcontextrestored', onRestored);
      canvas.removeEventListener('pointerdown', onPointerDown);
      canvas.removeEventListener('pointermove', onPointerMove);
      if (renderer && renderer.dispose) renderer.dispose();
      canvas._stThree = null;
    }
  }

  %exports {
    $ready: bool
    $frame: number
    $fps: number
  }
}
