/// Three Particles Primitive — a GPU particle field that morphs between shapes.
///
/// Builds a single Points cloud of `count` particles and precomputes several
/// target shapes (sphere, torus-knot, galaxy spiral, wave grid). A `morph` value
/// in [0, shapes-1] interpolates positions between adjacent shapes every frame —
/// the classic particle-hero transition. Additive blending makes overlaps glow
/// (pair with @post bloom).
///
/// The morph value is read live from `canvas._stParticleMorph` so other directives
/// (e.g. a scroll binding) can drive it: set `el._stParticleMorph = n`. It also
/// auto-rotates slowly for life.
///
/// @param &el        - the canvas hosting the @stage
/// @param count      - particle count (default 24000)
/// @param size       - point size in world units (default 0.02)
/// @param radius     - overall cloud radius (default 2)
/// @param color      - particle color (default white)
/// @param morph      - initial morph position 0..(shapes-1) (default 0)
/// @param spin       - auto-rotation speed (radians/second, default 0.05)
///
/// @yields $ready    - true once the cloud is in the scene
/// @yields $shapes   - number of morph targets available

%primitive three-particles(
  &el,
  count: number = 24000,
  size: number = 0.02,
  radius: number = 2,
  color: color = white,
  morph: number = 0,
  spin: number = 0.05
) {
  %emit js {
    const canvas = %&el;
    const ctx = canvas && canvas._stThree;
    if (!ctx) { console.warn('[Spacetime] three-particles: no @stage on this canvas.'); return; }
    const { THREE, scene } = ctx;

    const N = Math.max(1, Math.round(%count));
    const R = %radius;

    // --- Precompute morph-target shapes ------------------------------------
    const sphere = new Float32Array(N * 3);
    const knot = new Float32Array(N * 3);
    const galaxy = new Float32Array(N * 3);
    const grid = new Float32Array(N * 3);

    const gridN = Math.ceil(Math.sqrt(N));
    for (let i = 0; i < N; i++) {
      const i3 = i * 3;
      // Sphere (even distribution via golden spiral).
      const t = i / N;
      const phi = Math.acos(1 - 2 * t);
      const theta = Math.PI * (1 + Math.sqrt(5)) * i;
      sphere[i3]     = R * Math.sin(phi) * Math.cos(theta);
      sphere[i3 + 1] = R * Math.cos(phi);
      sphere[i3 + 2] = R * Math.sin(phi) * Math.sin(theta);
      // Torus knot.
      const u = t * Math.PI * 2 * 3;
      const kr = R * 0.6 * (2 + Math.cos(3 * u));
      knot[i3]     = (kr * Math.cos(2 * u)) / 3;
      knot[i3 + 1] = (kr * Math.sin(2 * u)) / 3;
      knot[i3 + 2] = (R * 0.6 * Math.sin(3 * u));
      // Galaxy spiral.
      const arm = (i % 3) * ((Math.PI * 2) / 3);
      const gr = Math.sqrt(t) * R * 1.4;
      const ga = gr * 1.6 + arm;
      galaxy[i3]     = Math.cos(ga) * gr;
      galaxy[i3 + 1] = (Math.random() - 0.5) * R * 0.12;
      galaxy[i3 + 2] = Math.sin(ga) * gr;
      // Wave grid.
      const gx = i % gridN, gz = Math.floor(i / gridN);
      const px = (gx / gridN - 0.5) * R * 2.6;
      const pz = (gz / gridN - 0.5) * R * 2.6;
      grid[i3]     = px;
      grid[i3 + 1] = Math.sin(px * 1.2) * Math.cos(pz * 1.2) * R * 0.4;
      grid[i3 + 2] = pz;
    }
    const shapes = [sphere, knot, galaxy, grid];

    // --- Points cloud ------------------------------------------------------
    const positions = new Float32Array(N * 3);
    positions.set(sphere);
    const geo = new THREE.BufferGeometry();
    geo.setAttribute('position', new THREE.BufferAttribute(positions, 3));

    const rgba = %color.rgba;
    const col = new THREE.Color().setRGB(rgba[0], rgba[1], rgba[2]);
    const mat = new THREE.PointsMaterial({
      size: %size,
      color: col,
      transparent: true,
      opacity: 0.9,
      depthWrite: false,
      blending: THREE.AdditiveBlending,
      sizeAttenuation: true,
    });
    const points = new THREE.Points(geo, mat);
    scene.add(points);

    canvas._stParticleMorph = %morph;
    const spin = %spin;

    const posAttr = geo.attributes.position;
    const arr = posAttr.array;
    const unregister = ctx.registerFrame((t, dt) => {
      // Interpolate between adjacent shapes by the live morph value.
      let m = canvas._stParticleMorph || 0;
      m = Math.max(0, Math.min(shapes.length - 1, m));
      const lo = Math.floor(m);
      const hi = Math.min(shapes.length - 1, lo + 1);
      const f = m - lo;
      const a = shapes[lo], b = shapes[hi];
      for (let k = 0; k < arr.length; k++) {
        arr[k] = a[k] + (b[k] - a[k]) * f;
      }
      posAttr.needsUpdate = true;
      points.rotation.y += spin * dt;
    });

    %yield true -> $ready;
    %yield shapes.length -> $shapes;

    %cleanup {
      if (unregister) unregister();
      scene.remove(points);
      geo.dispose();
      mat.dispose();
    }
  }

  %exports {
    $ready: bool
    $shapes: number
  }
}
