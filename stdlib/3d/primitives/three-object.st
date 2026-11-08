/// Three Object Primitive — procedural mesh factory.
///
/// Builds a three.js geometry + material and adds the resulting Mesh to the
/// parent stage's scene (read from `el._stThree`, set by three-stage). Supports
/// the procedural shapes most 3D heroes use, and the three material families
/// that cover the award-site look: `standard` (PBR), `physical` / `glass`
/// (transmission + clearcoat), and `basic` (unlit).
///
/// A per-frame spin can be applied declaratively via `spinX/Y/Z` (radians/sec).
/// The created object is stored on the canvas as `el._stObjects[…]` and the most
/// recent one as `el._stObject` so sibling child directives (`@orbit` acts on the
/// camera; transforms act on the object) can find it.
///
/// @param &el         - the canvas hosting the @stage (children share its scope)
/// @param shape       - "icosahedron"|"sphere"|"box"|"torus"|"torusknot"|"cone"|"cylinder"|"plane"
/// @param size        - overall scale (radius / half-extent), default 1
/// @param detail      - subdivision level for polyhedra (higher = smoother), default 3
/// @param material    - "standard"|"physical"|"glass"|"basic", default "standard"
/// @param color       - base color
/// @param roughness   - PBR roughness 0..1
/// @param metalness   - PBR metalness 0..1
/// @param transmission- glass transmission 0..1 (physical/glass only)
/// @param ior         - index of refraction (physical/glass)
/// @param thickness   - refraction thickness (physical/glass)
/// @param clearcoat   - clearcoat layer 0..1 (physical/glass)
/// @param iridescence    - thin-film iridescence 0..1 (soap-bubble/oil-slick sheen, physical/glass)
/// @param iridescenceIOR - iridescence film IOR 1.0..2.333 (shifts the colour band)
/// @param emissive    - emissive color
/// @param emissiveIntensity - emissive strength
/// @param spinX/Y/Z   - auto-rotation speed (radians/second)
/// @param posX/Y/Z    - position offset
/// @param flatShading - render flat per-face normals (crisp faceted/low-poly look)
///
/// @yields $added     - true once the mesh is in the scene

%primitive three-object(
  &el,
  name: string = "",
  shape: string = "icosahedron",
  size: number = 1,
  detail: number = 3,
  material: string = "standard",
  color: color = white,
  roughness: number = 0.3,
  metalness: number = 0.6,
  transmission: number = 0,
  ior: number = 1.5,
  thickness: number = 1.0,
  clearcoat: number = 0,
  iridescence: number = 0,
  iridescenceIOR: number = 1.3,
  emissive: color = black,
  emissiveIntensity: number = 1.0,
  spinX: number = 0,
  spinY: number = 0,
  spinZ: number = 0,
  posX: number = 0,
  posY: number = 0,
  posZ: number = 0,
  rotateX: number = 0,
  rotateY: number = 0,
  rotateZ: number = 0,
  scaleX: number = 1,
  scaleY: number = 1,
  scaleZ: number = 1,
  flatShading: bool = false,
  bind: string = ""
) {
  %emit js {
    const canvas = %&el;
    const ctx = canvas && canvas._stThree;
    if (!ctx) {
      console.warn('[Spacetime] three-object: no @stage on this canvas.');
      return;
    }
    // PLAN-152: the mesh is a PART (raea's `Mesh` + material); the stage's
    // realizer builds the three.js mesh, raea draws it from the same props.
    const s = %size;
    const SHAPES = {
      box:       { shape: `box(${s}, ${s}, ${s})`, geom: { type: 'box', args: [s, s, s] } },
      sphere:    { shape: `sphere(${s})`, geom: { type: 'sphere', args: [s] } },
      // raea has no icosahedron or knot: it draws the nearest shape it has; a browser draws the real one
      icosahedron: { shape: `sphere(${s})`, geom: { type: 'icosahedron', args: [s, Math.max(0, Math.round(%detail))] } },
      plane:     { shape: `box(${s * 2}, ${s * 2}, 0)`, geom: { type: 'plane', args: [s * 2, s * 2] } },
      cylinder:  { shape: `rotate(cone(${s}, ${s}, ${s * 2}), -90, 0, 0)`, geom: { type: 'cylinder', args: [s, s, s * 2] } },
      cone:      { shape: `rotate(cone(${s}, 0, ${s * 2}), -90, 0, 0)`, geom: { type: 'cone', args: [s, s * 2] } },
      torus:     { shape: `torus(${s}, ${s * 0.35})`, geom: { type: 'torus', args: [s, s * 0.35] } },
      torusknot: { shape: `torus(${s}, ${s * 0.3})`, geom: { type: 'torusknot', args: [s, s * 0.3] } },
      coin:      { shape: `rotate(cone(${s}, ${s}, ${s * 0.12}), -90, 0, 0)`, geom: { type: 'cylinder', args: [s, s, s * 0.12] } },
    };
    const g = SHAPES[%shape] || SHAPES.sphere;
    const sx = %spinX, sy = %spinY, sz = %spinZ;
    const kind = %material === 'basic' ? 'MeshBasicMaterial' : (%material === 'physical' || %material === 'glass') ? 'MeshPhysicalMaterial' : 'MeshStandardMaterial';
    const mat = { color: ctx.hex(%color.rgba), roughness: %roughness, metalness: %metalness, emissive: ctx.hex(%emissive.rgba), emissiveIntensity: %emissiveIntensity };
    if (kind === 'MeshPhysicalMaterial') Object.assign(mat, { transmission: %transmission, ior: %ior, thickness: %thickness, clearcoat: %clearcoat });
    const g0 = SHAPES[%shape];
    if (!g0) console.warn('[Spacetime] @object: unknown shape `' + %shape + '`, drawing a sphere');
    // the rest pose (its rotation is what a spin turns from): mutable, so a
    // hot edit (PLAN-155) can move it
    const rad = (d) => d * Math.PI / 180;
    const rest = { r: [rad(%rotateX), rad(%rotateY), rad(%rotateZ)], spin: [sx, sy, sz] };
    const pt = ctx.part('Mesh', {
      position: [%posX, %posY, %posZ],
      rotation: rest.r.slice(),
      scale: (%scaleX === 1 && %scaleY === 1 && %scaleZ === 1) ? 1 : [%scaleX, %scaleY, %scaleZ],
    }, { name: String(%name || '').replace(/^&/, ''), shape: g.shape, geom: g.geom, material: mat, materialKind: kind,
         update: (sx || sy || sz) ? (p, t) => { p.props.rotation = rest.r.map((r, i) => r + rest.spin[i] * t); } : null });

    // PLAN-155 hot bind: a new rest pose or material is the same part; a new
    // shape, size, material kind or a spin started/stopped is not (reload).
    ST.hot(%bind, (a) => {
      if (a.shape !== %shape || a.size !== %size || a.detail !== %detail || a.material !== %material || a.name !== %name) return false;
      if (!!(a.spinX || a.spinY || a.spinZ) !== !!(sx || sy || sz)) return false;
      rest.r = [rad(a.rotateX), rad(a.rotateY), rad(a.rotateZ)];
      rest.spin = [a.spinX, a.spinY, a.spinZ];
      pt.props.position = [a.posX, a.posY, a.posZ];
      pt.props.rotation = rest.r.slice();
      pt.props.scale = (a.scaleX === 1 && a.scaleY === 1 && a.scaleZ === 1) ? 1 : [a.scaleX, a.scaleY, a.scaleZ];
      Object.assign(pt.mat, { color: ctx.hex(a.color.rgba), roughness: a.roughness, metalness: a.metalness, emissive: ctx.hex(a.emissive.rgba), emissiveIntensity: a.emissiveIntensity });
      if (kind === 'MeshPhysicalMaterial') Object.assign(pt.mat, { transmission: a.transmission, ior: a.ior, thickness: a.thickness, clearcoat: a.clearcoat });
      ctx.touch(pt);
      return true;
    });

    // Register for sibling directives (legacy: @orbit / @parallax read these).
    canvas._stObjects = canvas._stObjects || [];
    if (pt.obj) { canvas._stObjects.push(pt.obj); canvas._stObject = pt.obj; }

    %yield true -> $added;
  }

  %exports {
    $added: bool
  }
}
