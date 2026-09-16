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
  flatShading: bool = false
) {
  %emit js {
    const canvas = %&el;
    const ctx = canvas && canvas._stThree;
    if (!ctx) {
      console.warn('[Spacetime] three-object: no @stage on this canvas.');
      return;
    }
    const { THREE, scene } = ctx;

    const s = %size;
    const d = Math.max(0, Math.round(%detail));
    const geomFor = (name) => {
      switch (name) {
        case 'sphere':    return new THREE.SphereGeometry(s, 32 + d * 16, 16 + d * 8);
        case 'box':       return new THREE.BoxGeometry(s, s, s, 1 + d, 1 + d, 1 + d);
        case 'torus':     return new THREE.TorusGeometry(s, s * 0.35, 24 + d * 8, 64 + d * 16);
        case 'torusknot': return new THREE.TorusKnotGeometry(s, s * 0.3, 128 + d * 32, 24 + d * 4);
        case 'cone':      return new THREE.ConeGeometry(s, s * 2, 32 + d * 8);
        case 'cylinder':  return new THREE.CylinderGeometry(s, s, s * 2, 32 + d * 8);
        case 'plane':     return new THREE.PlaneGeometry(s * 2, s * 2, 1 + d, 1 + d);
        case 'icosahedron':
        default:          return new THREE.IcosahedronGeometry(s, d);
      }
    };
    const geometry = geomFor(%shape);

    const cRGBA = %color.rgba;
    const eRGBA = %emissive.rgba;
    const col = new THREE.Color().setRGB(cRGBA[0], cRGBA[1], cRGBA[2]);
    const emis = new THREE.Color().setRGB(eRGBA[0], eRGBA[1], eRGBA[2]);
    let mat;
    switch (%material) {
      case 'basic':
        mat = new THREE.MeshBasicMaterial({ color: col });
        break;
      case 'physical':
      case 'glass':
        mat = new THREE.MeshPhysicalMaterial({
          color: col,
          roughness: %roughness,
          metalness: %metalness,
          transmission: %transmission,
          ior: %ior,
          thickness: %thickness,
          clearcoat: %clearcoat,
          clearcoatRoughness: %roughness,
          iridescence: %iridescence,
          iridescenceIOR: %iridescenceIOR,
          iridescenceThicknessRange: [120, 520],
          emissive: emis,
          emissiveIntensity: %emissiveIntensity,
          transparent: %transmission > 0,
          flatShading: %flatShading,
        });
        break;
      case 'standard':
      default:
        mat = new THREE.MeshStandardMaterial({
          color: col,
          roughness: %roughness,
          metalness: %metalness,
          emissive: emis,
          emissiveIntensity: %emissiveIntensity,
          flatShading: %flatShading,
        });
        break;
    }

    const mesh = new THREE.Mesh(geometry, mat);
    mesh.position.set(%posX, %posY, %posZ);
    scene.add(mesh);

    // Register for sibling directives.
    canvas._stObjects = canvas._stObjects || [];
    canvas._stObjects.push(mesh);
    canvas._stObject = mesh;

    // PLAN-150 W4: a NAMED object is addressable from `@on &.clip { &name {
    // … } }`. The @on engine (apply-animations) only ever writes to an
    // element's `.style`, so we register a CHANNEL PROXY under the name: a
    // stand-in "element" whose `.style.setProperty(prop, v)` records the
    // keyframed object property into a channel bag. This frame loop reads the
    // bag and applies each channel to the real mesh — so the ordinary DOM
    // keyframe engine drives a 3D object with NO new interpolation path (the
    // same move @post and camera made). Grid-specific channels (reveal,
    // displace, fog) are handled by three-grid; the generic object exposes the
    // transform/material channels every mesh has.
    const objName = String(%name || '').replace(/^&/, '');
    const channels = {};
    let proxy = null;
    if (objName) {
      proxy = {
        _stChannels: channels,
        style: {
          setProperty: (prop, v) => { channels[prop] = v; },
          removeProperty: (prop) => { delete channels[prop]; },
          getPropertyValue: (prop) => channels[prop] != null ? String(channels[prop]) : '',
        },
      };
      // Mirror `.style[camelProp] = v` assignments too (apply-animations sets
      // both the custom prop AND the inline style for immediate effect).
      proxy.style = new Proxy(proxy.style, {
        set: (o, k, v) => { channels[k] = v; return true; },
        get: (o, k) => o[k],
      });
      canvas._stStageObjects = canvas._stStageObjects || {};
      canvas._stStageObjects['&' + objName] = proxy;
      canvas._stStageObjects[objName] = proxy;
    }

    const num = (v, d) => { const n = parseFloat(v); return isNaN(n) ? d : n; };
    const baseScale = mesh.scale.x;

    // Declarative auto-spin + channel application.
    const sx = %spinX, sy = %spinY, sz = %spinZ;
    let unregister = null;
    if (sx !== 0 || sy !== 0 || sz !== 0 || proxy) {
      unregister = ctx.registerFrame((t, dt) => {
        mesh.rotation.x += sx * dt;
        mesh.rotation.y += sy * dt;
        mesh.rotation.z += sz * dt;
        if (proxy) {
          // The @on engine writes TRANSFORM-family properties (scale, rotate,
          // translate…) into `proxy._stTransforms` (composed for CSS), and
          // everything else into the channel bag. Read from BOTH so a keyframe
          // on `scale`/`rotate`/`posY` reaches the mesh regardless of which
          // path apply-animations chose for it.
          const tf = proxy._stTransforms || {};
          const ch = (k) => (k in channels) ? channels[k] : tf[k];
          const has = (k) => (k in channels) || (k in tf);

          // Generic mesh channels, applied each frame from the keyframe bag.
          if (has('opacity')) {
            mesh.material.transparent = true;
            mesh.material.opacity = num(ch('opacity'), 1);
          }
          if (has('emissive')) mesh.material.emissiveIntensity = num(ch('emissive'), 1);
          if (has('metalness') && mesh.material.metalness != null) mesh.material.metalness = num(ch('metalness'), mesh.material.metalness);
          if (has('roughness') && mesh.material.roughness != null) mesh.material.roughness = num(ch('roughness'), mesh.material.roughness);
          if (has('scale'))  { const s = num(ch('scale'), 1) * baseScale; mesh.scale.set(s, s, s); }
          if (has('rotateX')) mesh.rotation.x = num(ch('rotateX'), 0) * Math.PI / 180;
          if (has('rotateY')) mesh.rotation.y = num(ch('rotateY'), 0) * Math.PI / 180;
          if (has('rotateZ')) mesh.rotation.z = num(ch('rotateZ'), 0) * Math.PI / 180;
          if (has('spin'))   mesh.rotation.y = num(ch('spin'), 0);
          if (has('posY'))   mesh.position.y = num(ch('posY'), mesh.position.y);
          if (has('posX'))   mesh.position.x = num(ch('posX'), mesh.position.x);
          if (has('posZ'))   mesh.position.z = num(ch('posZ'), mesh.position.z);
          if (has('morph'))  canvas._stParticleMorph = num(ch('morph'), 0);
        }
      });
    }

    %yield true -> $added;

    %cleanup {
      if (unregister) unregister();
      scene.remove(mesh);
      geometry.dispose();
      mat.dispose();
    }
  }

  %exports {
    $added: bool
  }
}
