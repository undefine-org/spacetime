/// Three Light Primitive — adds a light to the stage's scene.
///
/// Reads the shared `el._stThree` handle and adds one three.js light. Covers the
/// four light types that compose the studio-lighting look: ambient (fill),
/// directional (key/sun), point (local), spot (focused). Positions/targets are
/// flat numeric params.
///
/// @param &el        - the canvas hosting the @stage
/// @param type       - "ambient"|"directional"|"point"|"spot"
/// @param color      - light color
/// @param intensity  - light intensity
/// @param posX/Y/Z   - position (directional uses this as its direction-from-origin)
/// @param targetX/Y/Z- aim point (directional / spot)
/// @param distance   - point/spot falloff distance (0 = infinite)
/// @param decay      - point/spot physical decay
/// @param angle      - spot cone half-angle (degrees)
/// @param penumbra   - spot soft-edge 0..1
///
/// @yields $added    - true once added

%primitive three-light(
  &el,
  type: string = "directional",
  color: color = white,
  intensity: number = 1.0,
  posX: number = 3,
  posY: number = 5,
  posZ: number = 4,
  targetX: number = 0,
  targetY: number = 0,
  targetZ: number = 0,
  distance: number = 0,
  decay: number = 2,
  angle: number = 30,
  penumbra: number = 0.3
) {
  %emit js {
    const canvas = %&el;
    const ctx = canvas && canvas._stThree;
    if (!ctx) { console.warn('[Spacetime] three-light: no @stage on this canvas.'); return; }
    const { THREE, scene } = ctx;

    const rgba = %color.rgba;
    const col = new THREE.Color().setRGB(rgba[0], rgba[1], rgba[2]);
    const I = %intensity;

    let light;
    switch (%type) {
      case 'ambient':
        light = new THREE.AmbientLight(col, I);
        break;
      case 'point':
        light = new THREE.PointLight(col, I, %distance, %decay);
        light.position.set(%posX, %posY, %posZ);
        break;
      case 'spot':
        light = new THREE.SpotLight(col, I, %distance, (%angle * Math.PI) / 180, %penumbra, %decay);
        light.position.set(%posX, %posY, %posZ);
        light.target.position.set(%targetX, %targetY, %targetZ);
        scene.add(light.target);
        break;
      case 'directional':
      default:
        light = new THREE.DirectionalLight(col, I);
        light.position.set(%posX, %posY, %posZ);
        light.target.position.set(%targetX, %targetY, %targetZ);
        scene.add(light.target);
        break;
    }
    scene.add(light);

    %yield true -> $added;

    %cleanup {
      scene.remove(light);
      if (light.target && light.target.parent) scene.remove(light.target);
      if (light.dispose) light.dispose();
    }
  }

  %exports {
    $added: bool
  }
}
