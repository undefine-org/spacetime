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
/// Linear easing with numeric values uses pure CSS calc().
/// Non-linear easing or color values use JavaScript.

%primitive animate(
  /// Element to animate
  &el,
  /// Signal driving the animation (0-1 progress)
  driver: signal,
  /// CSS property name to animate
  property: string,
  /// Starting value
  from: any,
  /// Ending value
  to: any,
  /// Easing function name
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
/// Note: Multiple transforms on the same element are composed.

%primitive animate-transform(
  /// Element to animate
  &el,
  /// Signal driving the animation
  driver: signal,
  /// Transform function (translateX, rotate, scale, etc.)
  transform: string,
  /// Starting value
  from: any,
  /// Ending value
  to: any,
  /// Easing function name
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

%primitive animate-color(
  /// Element to animate
  &el,
  /// Signal driving the animation
  driver: signal,
  /// CSS property (color, background-color, etc.)
  property: string,
  /// Starting color
  from: string,
  /// Ending color
  to: string,
  /// Color space for interpolation
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
/// Creates: --st-{signal} on the element

%primitive bind-css-var(
  /// Element to bind
  &el,
  /// Signal name to expose
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
