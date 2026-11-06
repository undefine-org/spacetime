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
  penumbra: number = 0.3,
  name: string = "",
  castShadow: bool = false
) {
  %emit js {
    const canvas = %&el;
    const ctx = canvas && canvas._stThree;
    if (!ctx) { console.warn('[Spacetime] three-light: no @stage on this canvas.'); return; }
    // PLAN-152: a light is a PART in raea's vocabulary; a named light answers to
    // `&name { intensity: …; color: …; posX: …; }` in an `@on` body.
    const KIND = { ambient: 'AmbientLight', directional: 'DirectionalLight', point: 'PointLight', spot: 'SpotLight' };
    const type = %type;
    const kind = KIND[type] || 'DirectionalLight';
    const props = { intensity: %intensity, color: ctx.hex(%color.rgba) };
    if (kind !== 'AmbientLight') props.position = [%posX, %posY, %posZ];
    if (kind === 'DirectionalLight' || kind === 'SpotLight') props.target = [%targetX, %targetY, %targetZ];
    if (kind === 'PointLight' || kind === 'SpotLight') props.distance = %distance;
    if (kind === 'SpotLight') { props.angle = %angle * Math.PI / 180; props.penumbra = %penumbra; }
    if (ctx.shadows && (kind === 'DirectionalLight' || kind === 'SpotLight' || %castShadow)) props.castShadow = true;
    ctx.part(kind, props, { name: String(%name || '').replace(/^&/, '') });

    %yield true -> $added;
  }

  %exports {
    $added: bool
  }
}
