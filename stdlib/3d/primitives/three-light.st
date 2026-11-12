/// Three Light Primitive — adds a light to the stage's scene.
///
/// Reads the shared `el._stThree` handle and adds one three.js light. Covers the
/// four light types that compose the studio-lighting look: ambient (fill),
/// directional (key/sun), point (local), spot (focused). Positions/targets are
/// flat numeric params.

%primitive three-light(
  /// the canvas hosting the @stage
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
  castShadow: bool = false,
  bind: string = ""
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
    const pt = ctx.part(kind, props, { name: String(%name || '').replace(/^&/, ''), bind: %bind });

    // PLAN-155 hot bind: the same light, new numbers (a new type is not).
    ST.hot(%bind, (a) => {
      if (a.type !== type || a.name !== %name || !!a.castShadow !== !!%castShadow) return false;
      pt.props.intensity = a.intensity;
      pt.props.color = ctx.hex(a.color.rgba);
      if (kind !== 'AmbientLight') pt.props.position = [a.posX, a.posY, a.posZ];
      if (kind === 'DirectionalLight' || kind === 'SpotLight') pt.props.target = [a.targetX, a.targetY, a.targetZ];
      if (kind === 'PointLight' || kind === 'SpotLight') pt.props.distance = a.distance;
      if (kind === 'SpotLight') { pt.props.angle = a.angle * Math.PI / 180; pt.props.penumbra = a.penumbra; }
      ctx.touch(pt);
      return true;
    });

    %yield true -> $added;
  }

  %exports {
    /// true once added
    $added: bool
  }
}
