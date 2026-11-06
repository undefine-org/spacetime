/// Three Fog Primitive — distance fog (PLAN-152).
///
/// A PART in raea's vocabulary (`Fog { color, near, far }`): far things fade
/// into `color` — match the stage's `background` so the horizon dissolves. A
/// named fog answers to a score: `&haze { near: 4 -> 12; far: 30 -> 60; }`.
///
/// @param &el    - the canvas hosting the @stage
/// @param color  - the fog's colour
/// @param near   - where it begins (world units)
/// @param far    - where it is total

%primitive three-fog(
  &el,
  name: string = "",
  color: color = black,
  near: number = 8,
  far: number = 40
) {
  %emit js {
    const canvas = %&el;
    const ctx = canvas && canvas._stThree;
    if (!ctx) { console.warn('[Spacetime] three-fog: no @stage on this canvas.'); return; }
    ctx.part('Fog', { color: ctx.hex(%color.rgba), near: %near, far: %far }, { name: String(%name || '').replace(/^&/, '') });
    %yield true -> $added;
  }

  %exports {
    $added: bool
  }
}
