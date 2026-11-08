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
/// PLAN-152: the SCENE is data. Every stage child (`@object`, `@light`, `@gltf`,
/// `@fog`, `@grid`, the camera) is a PART: a kind and its props in raea's 3D
/// vocabulary (the TSX `node3d` markup: `PerspectiveCamera`, `DirectionalLight`,
/// `Mesh` + `MeshStandardMaterial`, `Model`, `Fog`, …), mirrored as a hidden
/// `<i data-3d data-props>` element inside the canvas. A score animates a named
/// part's CHANNELS (`&sun { intensity: 0 -> 3; }`); each frame the channels are
/// folded into the part's props and the part is realized:
///
///   - in a browser, by the REALIZE table below (one three.js builder + updater
///     per kind — data, not a switch in each primitive);
///   - under `spacetime render --engine raea` (`ST._stage3d.renderer === "none"`),
///     by raea itself, which reads the `data-3d` markup every frame and draws it
///     with Bevy under the page. No WebGL is touched.
///
/// The frame loop ticks through `ST._raf`, so under a virtual clock (`render`,
/// `@clock`) the 3D moves on the same instants as the DOM.
///
/// @param &el       - the <canvas> to render into
/// @param fov       - perspective vertical field of view (degrees)
/// @param near      - camera near plane
/// @param far       - camera far plane
/// @param camX/Y/Z  - initial camera position
/// @param exposure  - tone-mapping exposure
/// @param tone      - tone-mapping: "aces" | "linear" | "reinhard" | "none"
/// @param alpha     - transparent canvas background
/// @param shadows   - "" (off) | "soft" | "hard": shadow maps for every light that casts and every part
/// @param background - the 3D's clear colour (hex); "" keeps the canvas transparent
/// @param bloom     - glow around bright and emissive parts (0 none, 1 natural, 2 strong); raea's `Bloom`
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
  alpha: bool = true,
  shadows: string = "",
  background: string = "",
  bloom: number = 0
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
    // PLAN-152: under raea the page has no WebGL; the scene's parts are drawn by
    // raea from their `data-3d` markup. Nothing below that needs a renderer runs.
    const headless3d = typeof ST !== 'undefined' && ST._stage3d && ST._stage3d.renderer === 'none';
    if (typeof THREE === 'undefined' && !headless3d) {
      console.warn('[Spacetime] three-stage: three.js engine not loaded.');
      return;
    }

    // --- Renderer -----------------------------------------------------------
    let renderer = null;
    if (!headless3d) try {
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
    const shadowMode = String(%shadows || '');
    if (renderer) {
      renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
      const toneMap = {
        aces: THREE.ACESFilmicToneMapping,
        linear: THREE.LinearToneMapping,
        reinhard: THREE.ReinhardToneMapping,
        none: THREE.NoToneMapping,
      };
      renderer.toneMapping = toneMap[%tone] ?? THREE.ACESFilmicToneMapping;
      renderer.toneMappingExposure = %exposure;
      if (shadowMode) {
        renderer.shadowMap.enabled = true;
        renderer.shadowMap.type = shadowMode === 'hard' ? THREE.BasicShadowMap : THREE.PCFSoftShadowMap;
      }
    }

    // --- Scene + camera -----------------------------------------------------
    const scene = renderer ? new THREE.Scene() : null;
    const camera = renderer ? new THREE.PerspectiveCamera(%fov, 1, %near, %far) : null;
    if (camera) {
      camera.position.set(%camX, %camY, %camZ);
      camera.lookAt(0, 0, 0);
    }
    const bg = String(%background || '');
    if (scene && bg) scene.background = new THREE.Color(bg);
    const EXPOSURE_GAIN = String(%tone) === 'none' ? 1 : %exposure / (String(%tone) === 'aces' ? 0.6 : 1);
    // A colour the world shows as it is — the fog, a tone-mapped clear colour.
    // three.js exposes everything it tone-maps (ACES after a 1/0.6 gain), while
    // raea exposes only its lights; so such a colour enters three.js divided by
    // that gain, and the two engines paint the same fog and the same sky.
    const shown = (c) => new THREE.Color(c).multiplyScalar(1 / EXPOSURE_GAIN);

    // --- Sizing (DPR-aware, ResizeObserver-driven) --------------------------
    const sizeState = { width: 1, height: 1, dpr: renderer ? renderer.getPixelRatio() : 1 };
    const resize = () => {
      const rect = canvas.getBoundingClientRect();
      const w = Math.max(1, Math.floor(rect.width));
      const h = Math.max(1, Math.floor(rect.height));
      if (w === sizeState.width && h === sizeState.height) return;
      sizeState.width = w;
      sizeState.height = h;
      if (renderer) {
        renderer.setSize(w, h, false);
        camera.aspect = w / h;
        camera.updateProjectionMatrix();
      }
      // notify any size listeners (e.g. post-processing composer)
      for (const cb of sizeListeners) cb(w, h, sizeState.dpr);
    };
    const sizeListeners = [];
    const resizeObs = typeof ResizeObserver !== 'undefined' ? new ResizeObserver(resize) : null;
    if (resizeObs) resizeObs.observe(canvas);

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
    let renderFn = () => { if (renderer) renderer.render(scene, camera); };
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
        if (!camera) return { halfW: 0, halfH: 0 };
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
    const raycaster = renderer ? new THREE.Raycaster() : null;
    const pointer = renderer ? new THREE.Vector2() : null;
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
      if (pickHandlers.size === 0 || !raycaster) return null;
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

    // --- PLAN-152: the scene as data -----------------------------------------
    //
    // A PART is one node of the scene: `kind` and `props` in raea's 3D
    // vocabulary, optional `shape` (raea's geometry string, `box(1, 1, 1)`),
    // `kids` (a Mesh's material), and a `geom` the three.js realizer reads. Its
    // markup lives in a hidden world element right AFTER the canvas, which is
    // what raea reads each frame. (Not inside it: a canvas re-rasterizes when
    // its subtree changes, so parts rewritten every tick inside one cost a
    // fresh picture per tick.)
    const world = document.createElement('div');
    world.setAttribute('data-st-world', '');
    world.hidden = true;
    world.style.display = 'none';
    // (a canvas not yet in the document, a test fixture's, keeps it inside)
    if (canvas.parentNode) canvas.parentNode.insertBefore(world, canvas.nextSibling);
    else canvas.appendChild(world);
    const parts = [];
    // Under raea the world is drawn UNDER the whole page (a full-frame,
    // back-most stage), so the backgrounds a browser paints behind the canvas
    // must not cover it there. The stage background is the clear colour.
    if (headless3d) {
      let up = canvas.parentElement;
      while (up) {
        if (up.style) up.style.setProperty('background', 'transparent');
        up = up.parentElement;
      }
    }
    const num = (v, d) => { const n = parseFloat(v); return isNaN(n) ? d : n; };
    const hex = (v) => {
      const h2 = (x) => Math.round(Math.max(0, Math.min(1, x)) * 255).toString(16).padStart(2, '0');
      if (Array.isArray(v)) return '#' + h2(v[0]) + h2(v[1]) + h2(v[2]);
      if (typeof v !== 'string') return v;
      const t = v.trim();
      if (t[0] === '#') return t.length === 4 ? '#' + t[1] + t[1] + t[2] + t[2] + t[3] + t[3] : t.slice(0, 7);
      const c = (typeof ST !== 'undefined' && ST.parseColor) ? ST.parseColor(t) : null;
      if (!c) return t;
      const h = (x) => Math.round(Math.max(0, Math.min(1, x > 1 ? x / 255 : x)) * 255).toString(16).padStart(2, '0');
      return '#' + h(c.r) + h(c.g) + h(c.b);
    };
    const deg = (v) => num(v, 0) * Math.PI / 180;

    // How a score channel lands in a part's props: data, one entry per channel.
    // `k` is the channel's value; each writer mutates the props (or the material).
    const vec = (key, i, f) => (p, k) => { const a = (p[key] || [0, 0, 0]).slice(); a[i] = f ? f(k) : num(k, 0); p[key] = a; };
    const CHANNELS = {
      posX: vec('position', 0), posY: vec('position', 1), posZ: vec('position', 2),
      // the camera's aim and lens (`&camera { lookY: 1 -> 2; fov: 40 -> 28; }`)
      lookX: vec('lookAt', 0), lookY: vec('lookAt', 1), lookZ: vec('lookAt', 2),
      fov: (p, k) => { p.fov = num(k, p.fov); },
      rotateX: vec('rotation', 0, deg), rotateY: vec('rotation', 1, deg), rotateZ: vec('rotation', 2, deg),
      spin: vec('rotation', 1),
      scale: (p, k, part) => { const b = part.baseScale; p.scale = Array.isArray(b) ? b.map((x) => x * num(k, 1)) : num(k, 1) * (b || 1); },
      intensity: (p, k) => { p.intensity = num(k, p.intensity); },
      color: (p, k, part) => { if (part.mat) part.mat.color = hex(k); else p.color = hex(k); },
      opacity: (p, k, part) => { if (part.mat) { part.mat.opacity = num(k, 1); part.mat.transparent = true; } },
      emissive: (p, k, part) => { if (part.mat) part.mat.emissiveIntensity = num(k, 1); },
      metalness: (p, k, part) => { if (part.mat) part.mat.metalness = num(k, 0); },
      roughness: (p, k, part) => { if (part.mat) part.mat.roughness = num(k, 0.5); },
      clip: (p, k, part) => {
        const name = String(k).replace(/^["']|["']$/g, '');
        // a clip plays from the frame the score switched to it
        if (name !== p.animation) { p.animation = name; part.clipFrom = part.since.clip != null ? part.since.clip : sceneTime; }
      },
      // its exact age in the score (apply-animations reads it off the keyframe
      // table and the window): a seek or a live edit lands mid-clip, not at 0
      clipAge: (p, k, part) => { const a = num(k, NaN); if (!isNaN(a)) part.clipFrom = sceneTime - a / 1000; },
      rate: (p, k, part) => { part.rate = num(k, 1); },
      near: (p, k) => { p.near = num(k, p.near); },
      far: (p, k) => { p.far = num(k, p.far); },
      pulse: (p, k, part) => { part.pulse = num(k, 0); },
      phase: (p, k, part) => { part.phase = num(k, 0); },
    };

    let sceneTime = 0;
    // part(kind, props, {name, shape, geom, material, parent}) -> the part
    const part = (kind, props, opts) => {
      const o = opts || {};
      const pt = { kind, props: Object.assign({}, props), shape: o.shape, geom: o.geom, mat: o.material ? Object.assign({}, o.material) : null,
                   matKind: o.materialKind || 'MeshStandardMaterial', name: o.name || '', parent: o.parent || null, kids: [],
                   baseScale: props.scale != null ? props.scale : 1, el: document.createElement('i'), obj: null,
                   clipFrom: 0, since: {}, rate: 1, pulse: 0, phase: 0, update: o.update || null, text: '' };
      if (pt.name) pt.props.name = pt.name;
      pt.el.setAttribute('data-3d', kind);
      if (pt.shape) pt.el.setAttribute('data-shape', pt.shape);
      if (pt.mat) { pt.matEl = document.createElement('i'); pt.matEl.setAttribute('data-3d', pt.matKind); pt.el.appendChild(pt.matEl); }
      (pt.parent ? pt.parent.el : world).appendChild(pt.el);
      if (pt.parent) pt.parent.kids.push(pt);
      parts.push(pt);
      if (pt.name) nameable(pt.name, pt);
      if (renderer && REALIZE[kind]) pt.obj = REALIZE[kind].make(pt);
      publish(pt);
      return pt;
    };

    // A named part answers to `&name` in an `@on` body: the score's keyframe
    // engine writes channels into a proxy (apply-animations' stage-object
    // rail); the frame loop folds them into the part's props.
    const nameable = (name, pt) => {
      const channels = {};
      const proxy = { _stChannels: channels, _stPart: pt };
      // a write lands at once (a reader after the score sees the score's
      // frame); the draw waits for the frame to settle
      const write = (k, v) => { if (channels[k] !== v) { channels[k] = v; pt.since[k] = sceneTime; fold(pt); publish(pt); requestSettle(); } };
      proxy.style = new Proxy({
        setProperty: write,
        removeProperty: (k) => { delete channels[k]; fold(pt); publish(pt); requestSettle(); },
        getPropertyValue: (k) => channels[k] != null ? String(channels[k]) : '',
      }, { set: (o, k, v) => { write(k, v); return true; }, get: (o, k) => o[k] });
      pt.channels = channels;
      pt.proxy = proxy;
      canvas._stStageObjects = canvas._stStageObjects || {};
      canvas._stStageObjects['&' + name] = proxy;
      canvas._stStageObjects[name] = proxy;
    };

    // Units. A stage's numbers are three.js's (the websites are tuned in
    // them); raea reads its own (stdlib/film/vendor/raea core/scene/src/props.rs:
    // a directional light of 1 is 8000 lux, a point light of 1 is 900,000 lm,
    // an ambient of 1 is 400 cd/m², seen at EV100 9.7). Against a directional
    // light — the shared yardstick — a three.js point light of I candela
    // matches raea's I / 8.95, and three.js ambient (irradiance, divided by π
    // like any diffuse light) matches raea's 8000/π/400 = 6.37 times it. Both
    // tone-map with the same fitted ACES, three.js's after a 1/0.6 gain, so
    // exposure E is raea's log2(E · 1.2 · 2^9.7 / (0.6 · 8000)) = log2(E) - 2.26
    // stops. raea leaves emissive unexposed (Bevy's emissive_exposure_weight
    // 0) where three.js exposes it, so raea's is three.js's times that gain.
    const RAEA_UNITS = { PointLight: 1 / 8.95, SpotLight: 1 / 8.95, AmbientLight: 8000 / Math.PI / 400 };
    const RAEA_EV = Math.log2(1.2 * Math.pow(2, 9.7) / (0.6 * 8000));
    const raeaProps = (pt) => {
      const k = RAEA_UNITS[pt.kind];
      return k && pt.props.intensity != null ? Object.assign({}, pt.props, { intensity: pt.props.intensity * k }) : pt.props;
    };
    const raeaMat = (m) => m.emissiveIntensity != null ? Object.assign({}, m, { emissiveIntensity: m.emissiveIntensity * EXPOSURE_GAIN }) : m;

    // The part's props as raea reads them (data-props), and to three.js.
    const publish = (pt) => {
      const text = JSON.stringify(raeaProps(pt));
      if (text !== pt.text) { pt.text = text; pt.el.setAttribute('data-props', text); }
      if (pt.matEl) {
        const mt = JSON.stringify(raeaMat(pt.mat));
        if (mt !== pt.matText) { pt.matText = mt; pt.matEl.setAttribute('data-props', mt); }
      }
      if (pt.obj && REALIZE[pt.kind].update) REALIZE[pt.kind].update(pt);
    };

    const fold = (pt) => {
      if (pt.channels) {
        const tf = pt.proxy._stTransforms || {};
        for (const k of new Set([...Object.keys(pt.channels), ...Object.keys(tf)])) {
          const w = CHANNELS[k];
          if (w) w(pt.props, k in pt.channels ? pt.channels[k] : tf[k], pt);
        }
      }
      // a model's clip plays from when it began (its own seconds, at its rate)
      if (pt.kind === 'Model') pt.props.time = Math.max(0, (sceneTime - pt.clipFrom) * pt.rate);
      if (pt.update) pt.update(pt, sceneTime);
    };

    // --- three.js realizers: one builder + updater per kind (data) ---------
    const place = (o, p) => {
      if (p.position) o.position.set(p.position[0], p.position[1], p.position[2]);
      if (p.rotation) o.rotation.set(p.rotation[0], p.rotation[1], p.rotation[2]);
      if (p.scale != null) { if (Array.isArray(p.scale)) o.scale.set(p.scale[0], p.scale[1], p.scale[2]); else o.scale.setScalar(p.scale); }
    };
    const shadowed = (o, p) => { if (shadowMode) { o.castShadow = p.castShadow !== false; o.receiveShadow = p.receiveShadow !== false; } };
    const light = (make) => ({
      make: (pt) => { const l = make(pt.props); if (l.target) scene.add(l.target); scene.add(l); if (shadowMode && pt.props.castShadow && l.shadow) { l.castShadow = true; l.shadow.mapSize.set(2048, 2048); if (l.shadow.camera.isOrthographicCamera) Object.assign(l.shadow.camera, { left: -12, right: 12, top: 12, bottom: -12, far: 60 }); } return l; },
      update: (pt) => { const l = pt.obj, p = pt.props; l.intensity = p.intensity; if (p.color) l.color.set(p.color); if (p.position) l.position.set(...p.position); if (l.target && p.target) l.target.position.set(...p.target); },
    });
    const GEOM = {
      box: (a) => new THREE.BoxGeometry(a[0], a[1], a[2]),
      sphere: (a) => new THREE.SphereGeometry(a[0], 48, 32),
      plane: (a) => new THREE.PlaneGeometry(a[0], a[1]),
      cylinder: (a) => new THREE.CylinderGeometry(a[0], a[1], a[2], 48),
      cone: (a) => new THREE.ConeGeometry(a[0], a[1], 48),
      torus: (a) => new THREE.TorusGeometry(a[0], a[1], 32, 96),
      icosahedron: (a) => new THREE.IcosahedronGeometry(a[0], a[1] || 0),
      torusknot: (a) => new THREE.TorusKnotGeometry(a[0], a[1], 160, 24),
    };
    const material = (m) => {
      const M = m.kind === 'MeshPhysicalMaterial' ? THREE.MeshPhysicalMaterial : m.kind === 'MeshBasicMaterial' ? THREE.MeshBasicMaterial : THREE.MeshStandardMaterial;
      return new M({});
    };
    const syncMaterial = (mm, m) => {
      if (m.color) mm.color.set(m.color);
      for (const k of ['roughness', 'metalness', 'opacity', 'transmission', 'ior', 'thickness', 'clearcoat', 'emissiveIntensity']) if (m[k] != null && k in mm) mm[k] = m[k];
      if (m.emissive && mm.emissive) mm.emissive.set(m.emissive);
      mm.transparent = !!m.transparent || (m.opacity != null && m.opacity < 1);
    };
    const REALIZE = {
      PerspectiveCamera: {
        make: () => camera,
        update: (pt) => {
          const p = pt.props;
          camera.position.set(...p.position);
          camera.up.set(...(p.up || [0, 1, 0]));
          camera.lookAt(...p.lookAt);
          if (camera.fov !== p.fov) { camera.fov = p.fov; camera.updateProjectionMatrix(); }
        },
      },
      AmbientLight: light((p) => new THREE.AmbientLight(p.color || '#ffffff', p.intensity)),
      DirectionalLight: light((p) => new THREE.DirectionalLight(p.color || '#ffffff', p.intensity)),
      PointLight: light((p) => new THREE.PointLight(p.color || '#ffffff', p.intensity, p.distance || 0, 2)),
      SpotLight: light((p) => new THREE.SpotLight(p.color || '#ffffff', p.intensity, p.distance || 0, p.angle || Math.PI / 6, p.penumbra || 0.3, 2)),
      // bloom: three's composer (a render pass, the bloom, the output transform) in place of the plain draw
      Bloom: {
        make: (pt) => {
          const A = THREE.addons, sz = sizeFn();
          const composer = new A.EffectComposer(renderer);
          composer.addPass(new A.RenderPass(scene, camera));
          // the glow at half resolution: it is soft anyway, and a software GL (Chromium's SwiftShader) pays per pixel.
          // three.js ADDS what passes a threshold (before exposure); raea's Bevy bloom blends without one. The
          // strength and threshold are measured against raea on demos/film-disco (an emissive floor, point lights)
          const pass = new A.UnrealBloomPass(new THREE.Vector2(sz.width / 2, sz.height / 2), pt.props.intensity * 0.15, 0.5, 4.0);
          composer.addPass(pass);
          composer.addPass(new A.OutputPass());
          composer.setSize(sz.width, sz.height);
          sizeListeners.push((w, h) => { composer.setSize(w, h); pass.setSize(w / 2, h / 2); });
          setRender(() => composer.render());
          // the output pass tone-maps the clear colour too
          if (bg) scene.background = shown(bg);
          return pass;
        },
        update: (pt) => { pt.obj.strength = pt.props.intensity * 0.15; },
      },
      Fog: { make: (pt) => { scene.fog = new THREE.Fog(shown(pt.props.color), pt.props.near, pt.props.far); return scene.fog; },
             update: (pt) => { scene.fog.color.copy(shown(pt.props.color)); scene.fog.near = pt.props.near; scene.fog.far = pt.props.far; } },
      Group: { make: (pt) => { const g = new THREE.Group(); (pt.parent && pt.parent.obj ? pt.parent.obj : scene).add(g); return g; }, update: (pt) => place(pt.obj, pt.props) },
      Mesh: {
        make: (pt) => {
          const g = pt.geom ? GEOM[pt.geom.type](pt.geom.args) : new THREE.BoxGeometry(1, 1, 1);
          const mesh = new THREE.Mesh(g, material({ kind: pt.matKind }));
          shadowed(mesh, pt.props);
          (pt.parent && pt.parent.obj ? pt.parent.obj : scene).add(mesh);
          return mesh;
        },
        update: (pt) => { place(pt.obj, pt.props); if (pt.mat) syncMaterial(pt.obj.material, pt.mat); },
      },
      Model: {
        make: (pt) => {
          const holder = new THREE.Group();
          scene.add(holder);
          // a render waits for this (ST._loading): frame 0 must have its cast
          ST._loading = (ST._loading || 0) + 1;
          const done = () => { ST._loading = Math.max(0, (ST._loading || 1) - 1); };
          new THREE.addons.GLTFLoader().load(pt.props.src, (gltf) => {
            done();
            holder.add(gltf.scene);
            gltf.scene.traverse((o) => { if (o.isMesh) shadowed(o, pt.props); });
            pt.mixer = new THREE.AnimationMixer(gltf.scene);
            pt.clips = gltf.animations;
            publish(pt);
          }, undefined, (e) => { done(); console.error('[Spacetime] @gltf: load failed', pt.props.src, e); });
          return holder;
        },
        update: (pt) => {
          place(pt.obj, pt.props);
          if (!pt.mixer) return;
          const p = pt.props;
          if (p.animation !== pt.playing) {
            pt.mixer.stopAllAction();
            const clip = (pt.clips || []).find((c) => c.name === p.animation);
            pt.action = clip ? pt.mixer.clipAction(clip).play() : null;
            pt.playing = p.animation;
          }
          // seek, don't step: the pose is a function of the clip's own time
          if (pt.action) { const d = pt.action.getClip().duration || 1; pt.action.time = p.time % d; pt.mixer.update(0); }
        },
      },
    };

    // --- the camera: a part like any other, its rest pose the stage's params,
    // its axes the score's camera tracks (`@on &.clip --crane;` writes
    // --cam-x/y/z, --cam-tx/ty/tz, --cam-fov, --cam-roll on the consumer) or a
    // `&camera { posZ: … }` block. Axes a score doesn't drive keep the rest.
    const cam = part('PerspectiveCamera', { position: [%camX, %camY, %camZ], lookAt: [0, 0, 0], fov: %fov, near: %near, far: %far, ...(bg ? { background: hex(bg) } : {}) }, { name: 'camera' });
    const rest = { position: cam.props.position.slice(), lookAt: [0, 0, 0], fov: %fov };
    // the lens: bloom (raea draws it; a browser's bloom is `@post`)
    if (%bloom > 0) part('Bloom', { intensity: %bloom }, {});
    // the exposure: the same picture on raea (stops; see RAEA_UNITS)
    part('ToneMapping', { mode: String(%tone) === 'linear' ? 'none' : String(%tone), exposure: Math.log2(%exposure) + RAEA_EV }, {});
    const track = (name) => {
      for (let n = canvas; n && n.style; n = n.parentElement) {
        const v = n.style.getPropertyValue && n.style.getPropertyValue(name);
        if (v !== '' && v != null) { n.style.setProperty('--cam-rig-off', 'none'); return num(v, NaN); }
      }
      return NaN;
    };
    const camFold = () => {
      const p = cam.props;
      const axes = [['--cam-x', 'position', 0], ['--cam-y', 'position', 1], ['--cam-z', 'position', 2],
                    ['--cam-tx', 'lookAt', 0], ['--cam-ty', 'lookAt', 1], ['--cam-tz', 'lookAt', 2]];
      for (const [t, key, i] of axes) { const v = track(t); if (!isNaN(v)) { p[key] = p[key].slice(); p[key][i] = v; } }
      const fv = track('--cam-fov'); if (!isNaN(fv)) p.fov = fv;
      const rl = track('--cam-roll'); if (!isNaN(rl)) { const r = rl * Math.PI / 180; p.up = [Math.sin(r), Math.cos(r), 0]; }
    };

    // Shared handle every child primitive composes against.
    canvas._stThree = {
      THREE: typeof THREE !== 'undefined' ? THREE : null, scene, camera, renderer,
      registerFrame, setRender,
      part, headless: headless3d, shadows: shadowMode, hex, num,
      // a part whose rest props changed (a hot edit, PLAN-155): fold its score
      // channels over them, publish, draw
      touch: (pt) => { fold(pt); publish(pt); requestSettle(); },
      onResize: (cb) => { sizeListeners.push(cb); },
      size: sizeFn,
      bounds: boundsFn,
      resolveReactive,
      registerPick,
    };

    // --- The frame loop: one tick per runtime frame, through ST._raf --------
    //
    // Under a virtual clock (`spacetime render`, `@clock`) ST._raf's ticks carry
    // the virtual time, so the 3D moves on the same instants as the DOM. In a
    // plain browser ST._raf runs on requestAnimationFrame as before.
    let frameCount = 0;
    let fpsFrames = 0;
    let fpsLast = performance.now();
    let lastT = null;
    let drawQueued = false;
    let revealed = false;
    if (renderer) canvas.style.opacity = '0';

    // Settle (fold, publish, draw) once per frame, after the score has spoken.
    // A signal's changes reach its dependents in microtask batches, several
    // hops deep (score -> clip window -> @on body), so the channels a tick
    // sees are the previous frame's (a virtual clock settles each tick's batch
    // before the next, `ST._settleSignals`). So a channel write folds its part
    // at once, and a tick AND every write ask for one draw when the frame has
    // settled: an advance() of ~800 ticks draws once (a bloom composer drawn
    // every tick on a software GL cost minutes).
    const requestSettle = () => {
      if (!drawQueued) { drawQueued = true; queueMicrotask(settle); }
    };
    const settle = () => {
      drawQueued = false;
      camFold();
      for (const pt of parts) fold(pt);
      for (const pt of parts) publish(pt);
      renderFn();
    };
    const tick = (ts) => {
      const t = (typeof ts === 'number' ? ts : performance.now()) / 1000;
      const dt = lastT === null ? 0 : Math.max(0, t - lastT);
      lastT = t;
      sceneTime = t;
      frameCount++;
      for (const pt of parts) fold(pt);
      for (const cb of frameCbs) cb(t, dt);
      requestSettle();
      if (renderer && !revealed) {
        revealed = true;
        // a film being rendered shows its first frame at once (a CSS fade runs on the wall clock)
        if (!(typeof ST !== 'undefined' && ST._clock && ST._clock.virtual)) canvas.style.transition = 'opacity 0.5s ease';
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
    };
    // a stage's loop is not a score's driver: a renderer waiting for the
    // drivers to mount must not take this one for them (it would start the
    // clock before the score's driver exists, and the film would run late)
    tick._stStage = true;
    const raf = (typeof ST !== 'undefined' && ST._raf) ? ST._raf : null;
    let stopLoop = null;
    let rafId = null;
    const start = () => {
      if (raf) stopLoop = raf.add(tick);
      else { const loop = (ts) => { tick(ts); rafId = requestAnimationFrame(loop); }; rafId = requestAnimationFrame(loop); }
    };

    // Context-loss recovery (the loop keeps ticking; drawing resumes on restore).
    const onLost = (e) => { e.preventDefault(); };
    canvas.addEventListener('webglcontextlost', onLost);

    // Go.
    resize();
    %yield true -> $ready;
    start();

    %cleanup {
      if (stopLoop) stopLoop();
      if (rafId) cancelAnimationFrame(rafId);
      if (resizeObs) resizeObs.disconnect();
      canvas.removeEventListener('webglcontextlost', onLost);
      world.remove();
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
