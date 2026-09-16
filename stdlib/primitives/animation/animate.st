/// Core Animation Primitives
///
/// These primitives provide the fundamental building blocks for animation.
/// The animate primitive uses `%if` to choose between CSS and JS implementations
/// based on whether the animation can be handled by CSS (linear easing with
/// numeric values) or requires JavaScript (custom easing, color interpolation).


/// Animate Property Primitive
///
/// Animates a CSS property based on a driver signal.
/// Chooses CSS or JS implementation based on easing and value types.
///
/// @param &el - Element to animate
/// @param driver - Signal driving the animation (0-1 progress)
/// @param property - CSS property name to animate
/// @param from - Starting value
/// @param to - Ending value
/// @param easing - Easing function name (default: "linear")
///
/// Linear easing with numeric values uses pure CSS calc().
/// Non-linear easing or color values use JavaScript.

%primitive animate(
  &el,
  driver: signal,
  property: string,
  from: any,
  to: any,
  easing: string = "linear"
) {
  %if $easing == "linear" {
    %emit css {
      %&el {
        %property: calc(%from + var(--st-%driver) * (%to - %from));
      }
    }
  } %else {
    %emit js {
      const ease = ST.ease['%easing'] || (t => t);
      ST.watch(%&el, %driver, p => {
        const t = ease(p);
        const value = ST.lerp(%from, %to, t);
        %&el.style['%property'] = value;
      });
    }
  }
}


/// Animate Transform Primitive
///
/// Animates transform properties with proper composition.
///
/// @param &el - Element to animate
/// @param driver - Signal driving the animation
/// @param transform - Transform function (translateX, rotate, scale, etc.)
/// @param from - Starting value
/// @param to - Ending value
/// @param easing - Easing function name
///
/// Note: Multiple transforms on the same element are composed.

%primitive animate-transform(
  &el,
  driver: signal,
  transform: string,
  from: any,
  to: any,
  easing: string = "linear"
) {
  %if $easing == "linear" {
    %emit css {
      %&el {
        %transform: calc(%from + var(--st-%driver) * (%to - %from));
      }
    }
  } %else {
    %emit js {
      const ease = ST.ease['%easing'] || (t => t);
      ST.watch(%&el, %driver, p => {
        const t = ease(p);
        const value = ST.lerp(%from, %to, t);
        ST.setTransform(%&el, '%transform', value);
      });
    }
  }
}


/// Animate Color Primitive
///
/// Animates color values with proper color space interpolation.
/// Always uses JavaScript due to color interpolation complexity.
///
/// @param &el - Element to animate
/// @param driver - Signal driving the animation
/// @param property - CSS property (color, background-color, etc.)
/// @param from - Starting color
/// @param to - Ending color
/// @param colorSpace - Color space for interpolation (default: "oklch")

%primitive animate-color(
  &el,
  driver: signal,
  property: string,
  from: string,
  to: string,
  colorSpace: string = "oklch"
) {
  %emit js {
    const fromColor = ST.parseColor('%from');
    const toColor = ST.parseColor('%to');
    const space = '%colorSpace';

    ST.watch(%&el, %driver, p => {
      // Interpolation is dispatched by COLOR SPACE, matching the two runtime
      // entry points that actually exist (ST.interpolateColor = OKLCH,
      // ST.interpolateColorRgb = sRGB). The previous `ST.lerpColor(from, to, p,
      // space)` named a function the runtime never defined, so every tick threw
      // a TypeError and the animation silently did nothing (BUG-202).
      //
      // This is the same dispatch the keyframe path already performs
      // (stdlib/primitives/animation/interpolate.st, apply-animations.st) — one
      // rule for "how do two colours blend", not a per-primitive spelling.
      if (!fromColor || !toColor) return;
      const color = space === 'oklch'
        ? ST.interpolateColor(fromColor, toColor, p)
        : ST.interpolateColorRgb(fromColor, toColor, p);
      if (color) %&el.style['%property'] = color;
    });
  }
}


/// Bind Signal to CSS Variable
///
/// Exposes a signal as a CSS custom property for use in calc() expressions.
/// This is the bridge between JS signals and CSS animations.
///
/// @param &el - Element to bind
/// @param signal - Signal name to expose
///
/// Creates: --st-{signal} on the element

%primitive bind-css-var(
  &el,
  signal: string
) {
  %emit js {
    // Initial value
    const initial = ST.get(%&el, '%signal');
    if (initial !== undefined) {
      %&el.style.setProperty('--st-%signal', initial);
    }

    // Watch for updates
    ST.watch(%&el, '%signal', value => {
      %&el.style.setProperty('--st-%signal', value);
    });
  }
}
