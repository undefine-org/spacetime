/// Value Interpolation Primitive
///
/// Provides keyframe interpolation and property application for animations.
/// Handles numeric values, colors, transforms, and filters.
///
/// @param &el - Element to animate
/// @param keyframes - Array of keyframe objects { at, value, easing? }
/// @param property - CSS property to animate
/// @param colorSpace - Color interpolation space (rgb, hsl, oklch)
///
/// @yields $setProgress - Function to update animation progress (0-1)

%primitive animate-property(
  &el,
  keyframes: array,
  property: string,
  colorSpace: string = "rgb"
) %uses easing {

  %emit js {
    const parseColor = ST.parseColor;
    const getEasing = window.__stGetEasing;

    const kfs = %keyframes;
    const prop = '%property';
    const space = '%colorSpace';

    // Transform and filter property lists
    const transformProps = ['translateX', 'translateY', 'translateZ', 'rotate', 'rotateX', 'rotateY', 'rotateZ', 'scale', 'scaleX', 'scaleY', 'skewX', 'skewY'];
    const filterProps = ['blur', 'brightness', 'contrast', 'grayscale', 'saturate', 'sepia', 'hueRotate', 'invert', 'opacityFilter', 'dropShadow'];

    // Normalize property name
    const normProp = prop.replace(/-([a-z])/g, (_, c) => c.toUpperCase());

    // Parse numeric value with unit
    const parseNumeric = (v) => {
      if (typeof v === 'number') return { num: v, unit: '' };
      const match = String(v).match(/^(-?\d*\.?\d+)(.*)$/);
      return match ? { num: parseFloat(match[1]), unit: match[2] } : null;
    };

    // Interpolate between keyframes
    const interpolate = (progress, easing) => {
      if (kfs.length === 0) return null;
      if (kfs.length === 1) return kfs[0].value;

      let prev = kfs[0];
      let next = kfs[kfs.length - 1];

      for (let i = 0; i < kfs.length - 1; i++) {
        if (progress >= kfs[i].at && progress <= kfs[i + 1].at) {
          prev = kfs[i];
          next = kfs[i + 1];
          break;
        }
      }

      if (progress <= prev.at) return prev.value;
      if (progress >= next.at) return next.value;

      const localProgress = (progress - prev.at) / (next.at - prev.at);
      const easeFn = getEasing(prev.easing || easing);
      const easedProgress = easeFn(localProgress);

      // Try color interpolation
      const c1 = parseColor(prev.value);
      const c2 = parseColor(next.value);
      if (c1 && c2) {
        if (space === 'oklch') {
          return ST.interpolateColor(c1, c2, easedProgress);
        }
        return ST.interpolateColorRgb(c1, c2, easedProgress);
      }

      // Try numeric interpolation
      const n1 = parseNumeric(prev.value);
      const n2 = parseNumeric(next.value);
      if (n1 && n2) {
        const val = n1.num + (n2.num - n1.num) * easedProgress;
        return n2.unit || n1.unit ? val + (n2.unit || n1.unit) : val;
      }

      // Snap at 50%
      return easedProgress < 0.5 ? prev.value : next.value;
    };

    // Apply value to element
    const applyValue = (value) => {
      if (value === null) return;

      if (normProp.startsWith('--')) {
        %&el.style.setProperty(normProp, value);
      } else if (transformProps.includes(normProp)) {
        const current = %&el._stTransforms || {};
        current[normProp] = value;
        %&el._stTransforms = current;
        // See apply-animations.st: transform FUNCTIONS take comma-separated
        // args, keyframe values arrive space-separated — `translate(a b)`
        // composed verbatim is invalid CSS the browser discards wholesale.
        const fnArgs = (v) => {
          if (typeof v !== 'string' || !v.includes(' ')) return v;
          const parts = [];
          let depth = 0;
          let cur = '';
          for (const ch of v) {
            if (ch === '(') depth++;
            if (ch === ')') depth--;
            if (/\s/.test(ch) && depth === 0) {
              if (cur) { parts.push(cur); cur = ''; }
            } else {
              cur += ch;
            }
          }
          if (cur) parts.push(cur);
          return parts.join(', ');
        };
        %&el.style.transform = Object.entries(current)
          .map(([k, v]) => {
            if (k.startsWith('translate')) return `${k}(${typeof v === 'number' ? v + 'px' : fnArgs(v)})`;
            if (k.startsWith('rotate') || k.startsWith('skew')) return `${k}(${typeof v === 'number' ? v + 'deg' : fnArgs(v)})`;
            return `${k}(${fnArgs(v)})`;
          })
          .join(' ');
      } else if (filterProps.includes(normProp)) {
        const current = %&el._stFilters || {};
        current[normProp] = value;
        %&el._stFilters = current;
        const parts = [];
        for (const fp of filterProps) {
          if (current[fp] !== undefined) {
            const fn = fp === 'opacityFilter' ? 'opacity' : fp.replace(/[A-Z]/g, m => '-' + m.toLowerCase());
            parts.push(`${fn}(${current[fp]})`);
          }
        }
        %&el.style.filter = parts.length > 0 ? parts.join(' ') : 'none';
      } else {
        const cssVal = typeof value === 'number' ? (normProp === 'opacity' ? value : value + 'px') : value;
        %&el.style[normProp] = cssVal;
      }
    };

    // Set progress function
    const setProgress = (progress, easing = 'linear') => {
      const value = interpolate(progress, easing);
      applyValue(value);
    };

    // Apply initial state
    setProgress(0);

    %yield setProgress -> $setProgress;

    %cleanup {
      // Reset transforms/filters on cleanup if desired
    }
  }

  %exports {
    $setProgress: fn(progress: number, property: string?) ~> void
  }
}


/// Stagger Calculation Primitive
///
/// Calculates stagger delays for multi-element animations.
///
/// @param index - Element index in the group
/// @param total - Total number of elements
/// @param delay - Base delay between elements (ms)
/// @param from - Stagger origin: "first", "last", "center", "random", or index number
/// @param grid - Optional grid dimensions [cols, rows] for 2D stagger
///
/// @yields $offset - Calculated stagger offset in ms

%primitive stagger(
  index: number,
  total: number,
  delay: number,
  from: string = "first",
  grid: array?
) {
  %emit js {
    const idx = %index;
    const tot = %total;
    const baseDelay = %delay;
    const origin = '%from';
    const gridDims = %grid;

    let position;

    if (gridDims && gridDims.length === 2) {
      const [cols, rows] = gridDims;
      const x = idx % cols;
      const y = Math.floor(idx / cols);
      const centerX = (cols - 1) / 2;
      const centerY = (rows - 1) / 2;
      const dist = Math.sqrt(Math.pow(x - centerX, 2) + Math.pow(y - centerY, 2));
      const maxDist = Math.sqrt(Math.pow(centerX, 2) + Math.pow(centerY, 2));
      position = maxDist > 0 ? dist / maxDist : 0;
    } else {
      switch (origin) {
        case 'first':
          position = idx / (tot - 1 || 1);
          break;
        case 'last':
          position = (tot - 1 - idx) / (tot - 1 || 1);
          break;
        case 'center':
          position = Math.abs(idx - (tot - 1) / 2) / ((tot - 1) / 2 || 1);
          break;
        case 'random':
          position = Math.random();
          break;
        default:
          // Numeric origin
          const fromIdx = parseInt(origin, 10);
          if (!isNaN(fromIdx)) {
            position = Math.abs(idx - fromIdx) / (tot - 1 || 1);
          } else {
            position = idx / (tot - 1 || 1);
          }
      }
    }

    const offset = position * baseDelay;
    %yield offset -> $offset;
  }

  %exports {
    $offset: number
  }
}
