/// Apply Animations Primitive
///
/// Batch-applies keyframe animations driven by a signal.
/// Handles nested selectors, stagger, ranges, and easing.
/// This primitive processes the $body:keyframes structure captured from timeline macros.
///
/// @param &el - Root element (scope for nested selectors)
/// @param driver - Signal driving the animation (0-1 progress from scroll-driver, time-driver, etc.)
/// @param animations - Captured keyframes structure from $body:keyframes
/// @param easing - Default easing function name
/// @param stagger - Delay between elements (ms)
/// @param staggerFrom - Stagger origin ("first", "last", "center")
/// @param colorSpace - Color interpolation space ("rgb" or "oklch", default "rgb")
///
/// The animations parameter expects a structure like:
/// {
///   properties: [{ property: "opacity", keyframes: [{ at: 0, value: 0 }, { at: 1, value: 1 }] }],
///   staticProperties: [{ property: "transform-origin", value: "left" }],
///   scopes: [{
///     selector: ".child",
///     properties: [...],
///     staticProperties: [...],
///     easing: "ease-out-quad",
///     range: { start: 0, end: 0.5 },
///     stagger: { delay: 0.05, from: "first", grid: null }
///   }]
/// }

%primitive apply-animations(
  &el,
  driver: signal,
  animations: any,
  easing: string = "linear",
  stagger: number = 0,
  staggerFrom: string = "first",
  interactive: bool = false,
  colorSpace: string = "rgb"
) {
  // NOTE: CSS variables are set on elements via JS (e.g., --st-{driver}-opacity)
  // These can be used in custom CSS for additional styling. The JS also sets
  // inline styles directly for immediate visual effect.

  %emit js {
    (function() {
      const root = %&el;
      const anims = %animations;
      const defaultEasing = %easing;
      const defaultStaggerDelay = %stagger;
      const defaultStaggerOrigin = %staggerFrom;
      const colorSpace = '%colorSpace';

      // Early exit if no animations data
      if (!anims) return;

      // PLAN-150 W4: a `&name` scope selector may address a STAGE OBJECT (a
      // 3D mesh/particle field registered by the stage under that name) rather
      // than a DOM element. Stage objects register a channel proxy on the
      // canvas as `_stStageObjects['&name']`; return it so property writes flow
      // to the mesh. Returns null for ordinary DOM selectors.
      const resolveStageObject = (rootEl, sel) => {
        if (!sel || sel[0] !== '&' || !rootEl) return null;
        // The stage registry lives on the <canvas> that hosts the @stage. The
        // @on consumer may be that canvas itself, or — because `@on` bound
        // directly on a <canvas> does not receive a score driver — an ancestor
        // element whose subtree contains the canvas. Look on the root first,
        // then descend into a hosted canvas.
        let reg = rootEl._stStageObjects;
        if (reg && reg[sel]) return reg[sel];
        const canvases = rootEl.querySelectorAll ? rootEl.querySelectorAll('canvas') : [];
        for (let i = 0; i < canvases.length; i++) {
          reg = canvases[i]._stStageObjects;
          if (reg && reg[sel]) return reg[sel];
        }
        return null;
      };

      // Easing functions
      const easings = {
        'linear': t => t,
        'ease': t => t < 0.5 ? 2 * t * t : -1 + (4 - 2 * t) * t,
        'ease-in': t => t * t,
        'ease-out': t => t * (2 - t),
        'ease-in-out': t => t < 0.5 ? 2 * t * t : -1 + (4 - 2 * t) * t,
        'ease-in-quad': t => t * t,
        'ease-out-quad': t => t * (2 - t),
        'ease-in-out-quad': t => t < 0.5 ? 2 * t * t : -1 + (4 - 2 * t) * t,
        'ease-in-cubic': t => t * t * t,
        'ease-out-cubic': t => (--t) * t * t + 1,
        'ease-in-out-cubic': t => t < 0.5 ? 4 * t * t * t : (t - 1) * (2 * t - 2) * (2 * t - 2) + 1,
        'ease-in-expo': t => t === 0 ? 0 : Math.pow(2, 10 * (t - 1)),
        'ease-out-expo': t => t === 1 ? 1 : 1 - Math.pow(2, -10 * t),
        'ease-in-out-expo': t => {
          if (t === 0 || t === 1) return t;
          return t < 0.5
            ? Math.pow(2, 20 * t - 10) / 2
            : (2 - Math.pow(2, -20 * t + 10)) / 2;
        }
      };

      // Get easing function by name (strip & prefix if present)
      const getEase = (name) => {
        if (!name) return easings[defaultEasing] || easings.linear;
        const cleanName = name.replace(/^&/, '').replace(/_/g, '-');
        // PLAN-150 W1: `steps(N)` is a parameterized built-in (a stepped
        // curve of N jumps) with no registry entry — parse it here, the same
        // way ST.getEasing does, so a per-step or body `--steps(N)` easing
        // resolves on this hot path too.
        const stepsM = cleanName.match(/^steps\((\d+)\)$/);
        if (stepsM) {
          const n = Math.max(1, parseInt(stepsM[1], 10));
          return (t) => t <= 0 ? 0 : (t >= 1 ? 1 : Math.floor(t * n) / n);
        }
        // The baked subset is only the fast path. The GLOBAL registry
        // (ST.easings) holds everything else: the full stdlib curve library
        // AND custom `@form easing` declarations, which the compiler emits
        // as ST.registerEasing calls (BUG-297) — without this hop a declared
        // curve silently ran linear. The lookup avoids ST.getEasing's own
        // linear fallback so the scope default still applies on a true miss.
        const local = easings[cleanName];
        if (local) return local;
        const camel = cleanName.replace(/-([a-z])/g, function(_, c) { return c.toUpperCase(); });
        const global = (typeof ST !== 'undefined' && ST.easings)
          ? (ST.easings[cleanName] || ST.easings[camel])
          : null;
        return global || easings[defaultEasing] || easings.linear;
      };

      // Calculate stagger offset
      const calcStagger = (index, total, delay, from, grid) => {
        if (delay <= 0 || total <= 1) return 0;

        let position;
        if (grid && grid.length === 2) {
          const [cols, rows] = grid;
          const x = index % cols;
          const y = Math.floor(index / cols);
          const centerX = (cols - 1) / 2;
          const centerY = (rows - 1) / 2;
          const dist = Math.sqrt(Math.pow(x - centerX, 2) + Math.pow(y - centerY, 2));
          const maxDist = Math.sqrt(Math.pow(centerX, 2) + Math.pow(centerY, 2));
          position = maxDist > 0 ? dist / maxDist : 0;
        } else {
          switch (from) {
            case 'last':
              position = (total - 1 - index) / (total - 1);
              break;
            case 'center':
              position = Math.abs(index - (total - 1) / 2) / ((total - 1) / 2 || 1);
              break;
            case 'random':
              position = Math.random();
              break;
            case 'first':
            default:
              position = index / (total - 1);
          }
        }

        // Convert position to normalized offset (0-1 range for progress adjustment)
        return position * delay / 1000;
      };
      // Color parsing and interpolation delegate to the runtime OKLCH-first parser (ST.parseColor / ST.interpolateColor).
      // Resolve var() references in keyframe values using element's computed style
      const resolveVarRefs = (properties, el) => {
        if (!el || !properties) return properties;
        let hasVars = false;
        for (let i = 0; i < properties.length; i++) {
          const kfs = properties[i].keyframes;
          if (kfs) for (let j = 0; j < kfs.length; j++) {
            if (typeof kfs[j].value === 'string' && kfs[j].value.startsWith('var(')) {
              hasVars = true; break;
            }
          }
          if (hasVars) break;
        }
        if (!hasVars) return properties;

        const cs = getComputedStyle(el);
        return properties.map(p => ({
          ...p,
          keyframes: p.keyframes ? p.keyframes.map(kf => {
            if (typeof kf.value === 'string' && kf.value.startsWith('var(')) {
              const inner = kf.value.slice(4, -1).trim();
              const commaIdx = inner.indexOf(',');
              const propName = commaIdx >= 0 ? inner.slice(0, commaIdx).trim() : inner;
              const fallback = commaIdx >= 0 ? inner.slice(commaIdx + 1).trim() : null;
              const resolved = cs.getPropertyValue(propName).trim();
              return { ...kf, value: resolved || fallback || kf.value };
            }
            return kf;
          }) : p.keyframes
        }));
      };

      // Parse numeric value with unit
      const parseNumeric = (v) => {
        if (typeof v === 'number') return { num: v, unit: '' };
        if (typeof v !== 'string') return null;
        const match = v.match(/^(-?[\d.]+)(.*)$/);
        return match ? { num: parseFloat(match[1]), unit: match[2] } : null;
      };

      // Interpolate keyframes at given progress
      const interpolateKeyframes = (keyframes, progress, propEasing, space = 'rgb') => {
        if (!keyframes || keyframes.length === 0) return null;
        if (keyframes.length === 1) return keyframes[0].value;

        // Find surrounding keyframes
        let prev = keyframes[0];
        let next = keyframes[keyframes.length - 1];

        for (let i = 0; i < keyframes.length - 1; i++) {
          if (progress >= keyframes[i].at && progress <= keyframes[i + 1].at) {
            prev = keyframes[i];
            next = keyframes[i + 1];
            break;
          }
        }

        if (progress <= prev.at) return prev.value;
        if (progress >= next.at) return next.value;

        // Calculate local progress between keyframes
        const localProgress = (progress - prev.at) / (next.at - prev.at);
        const ease = getEase(prev.easing || propEasing);
        const easedProgress = ease(localProgress);

        // Try color interpolation
        const c1 = ST.parseColor(prev.value);
        const c2 = ST.parseColor(next.value);
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
          const unit = n2.unit || n1.unit;
          return unit ? `${val}${unit}` : val;
        }

        // Handle function values (blur, etc.)
        if (prev.value && typeof prev.value === 'object' && prev.value.fn) {
          const fn = prev.value.fn;
          const args1 = prev.value.args || [];
          const args2 = (next.value && next.value.args) || [];
          const interpolatedArgs = args1.map((a, i) => {
            const b = args2[i] !== undefined ? args2[i] : a;
            if (typeof a === 'number' && typeof b === 'number') {
              return a + (b - a) * easedProgress;
            }
            return easedProgress < 0.5 ? a : b;
          });
          return `${fn}(${interpolatedArgs.join(', ')})`;
        }

        // Snap at 50% for non-interpolatable values
        return easedProgress < 0.5 ? prev.value : next.value;
      };

      // Generate a unique scope ID from selector for CSS variable naming
      const getScopeId = (selector) => {
        if (!selector || selector === '&self' || selector === '&') return 'self';
        // Extract meaningful part from selector (e.g., ".ik-hero__line--h1" -> "line-h1")
        return selector
          .replace(/^[.#\[\]>+~\s]+/, '')  // Remove leading selectors
          .replace(/[.#\[\]=">~+\s]/g, '-') // Replace special chars with dash
          .replace(/--+/g, '-')             // Collapse multiple dashes
          .replace(/^-|-$/g, '')            // Trim dashes
          .toLowerCase()
          .slice(0, 30);                    // Limit length
      };

      // Apply a value to an element's style via CSS variables
      // driverName + scopeId create unique namespace (e.g., --st-h-lines-line-h1-opacity)
      const applyStyle = (el, property, value, driverName, scopeId) => {
        if (value === null || value === undefined) return;

        // Normalize property name (kebab-case to camelCase)
        const normProp = property.replace(/-([a-z])/g, (_, c) => c.toUpperCase());
        const kebabProp = property.replace(/([A-Z])/g, '-$1').toLowerCase();

        // Build CSS variable prefix: --st-{driver}-{scope}
        const varPrefix = scopeId && scopeId !== 'self'
          ? `--st-${driverName}-${scopeId}`
          : `--st-${driverName}`;

        // Handle CSS custom properties
        if (property.startsWith('--')) {
          el.style.setProperty(property, value);
          return;
        }

        // Handle transform properties - compose into single CSS variable
        const transformProps = ['translateX', 'translateY', 'translateZ', 'translate',
          'rotate', 'rotateX', 'rotateY', 'rotateZ',
          'scale', 'scaleX', 'scaleY', 'scaleZ',
          'skew', 'skewX', 'skewY'];

        if (transformProps.includes(normProp) || transformProps.includes(property)) {
          const current = el._stTransforms || {};
          current[normProp] = value;
          el._stTransforms = current;

          // Transform FUNCTIONS take comma-separated args
          // (`translate(10px, 20px)`); keyframe values arrive SPACE-separated
          // (the individual-property spelling, `translate: 10px 20px`).
          // Composed verbatim, `translate(-25% 0%)` is invalid CSS the
          // browser discards WHOLESALE — the element holds its first frame
          // forever with a green build. Split top-level spaces (paren-aware:
          // calc()/var() keep theirs) and join with commas.
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

          const transformStr = Object.entries(current)
            .map(([k, v]) => {
              if (k.startsWith('translate')) {
                return `${k}(${typeof v === 'number' ? v + 'px' : fnArgs(v)})`;
              }
              if (k.startsWith('rotate') || k.startsWith('skew')) {
                return `${k}(${typeof v === 'number' ? v + 'deg' : fnArgs(v)})`;
              }
              return `${k}(${fnArgs(v)})`;
            })
            .join(' ');

          // Set CSS variable for transform (allows CSS to use it with defaults)
          if (driverName) {
            el.style.setProperty(`${varPrefix}-transform`, transformStr);
          }
          el.style.transform = transformStr;
          return;
        }

        // Handle filter properties - compose into single CSS variable
        const filterProps = ['blur', 'brightness', 'contrast', 'grayscale',
          'saturate', 'sepia', 'hueRotate', 'invert', 'opacity-filter', 'dropShadow'];

        if (filterProps.includes(normProp)) {
          const current = el._stFilters || {};
          current[normProp] = value;
          el._stFilters = current;

          const filterStr = Object.entries(current)
            .map(([k, v]) => {
              const fn = k === 'hueRotate' ? 'hue-rotate' : k.replace(/[A-Z]/g, m => '-' + m.toLowerCase());
              return `${fn}(${v})`;
            })
            .join(' ');

          // Set CSS variable for filter
          if (driverName) {
            el.style.setProperty(`${varPrefix}-filter`, filterStr || 'none');
          }
          el.style.filter = filterStr || 'none';
          return;
        }

        // Standard property - set both CSS variable and inline style
        const cssVal = typeof value === 'number'
          ? (normProp === 'opacity' || normProp === 'zIndex' ? value : value + 'px')
          : value;

        // Set CSS variable for this property (allows CSS fallback approach)
        if (driverName) {
          el.style.setProperty(`${varPrefix}-${kebabProp}`, cssVal);
        }
        el.style[normProp] = cssVal;
      };

      // Apply initial state synchronously (progress=0) to prevent flickering
      // This runs BEFORE ST.watch() is set up, ensuring elements start in correct state
      // Uses first-writer-wins: if another animation already claimed this property, skip.
      const applyInitialState = (elements, properties, scopeEasing, driverName, selector, space) => {
        if (!properties || properties.length === 0) return;
        const scopeId = getScopeId(selector);
        elements.forEach(el => {
          if (!el._stAnimOwner) el._stAnimOwner = {};
          properties.forEach(prop => {
            const normProp = prop.property.replace(/-([a-z])/g, (_, c) => c.toUpperCase());
            // First-writer wins: skip if another animation already claimed this property
            if (el._stAnimOwner[normProp] && el._stAnimOwner[normProp] !== driverName) return;
            el._stAnimOwner[normProp] = driverName;
            const value = interpolateKeyframes(prop.keyframes, 0, prop.easing || scopeEasing, space);
            applyStyle(el, prop.property, value, driverName, scopeId);
          });
        });
      };

      // Process animations for a scope (root or nested selector)
      const processScope = (scope, elements, driverSignal, selector) => {
        const scopeEasing = scope.easing || defaultEasing;
        const scopeStagger = scope.stagger || {
          delay: defaultStaggerDelay,
          from: defaultStaggerOrigin,
          grid: null
        };
        const scopeRange = scope.range;
        const scopeId = getScopeId(selector);

        // Apply static properties once. EXCEPT PLAN-150 W6 `--noise-1d(…)`:
        // a value like `--noise-1d(8px, 4)` is not static — it is a smooth,
        // deterministic function of clip PROGRESS (shake/drift/flicker). Split
        // those out and drive them on the same driver watch as keyframes, so
        // motion written as noise rides the one engine (no new interpolation
        // path). A plain static value is applied once as before.
        const noiseStatics = [];
        if (scope.staticProperties) {
          elements.forEach(el => {
            scope.staticProperties.forEach(sp => {
              if (typeof sp.value === 'string' && sp.value.indexOf('--noise-1d(') !== -1) {
                noiseStatics.push({ el: el, property: sp.property, spec: sp.value });
              } else {
                applyStyle(el, sp.property, sp.value, driverSignal, scopeId);
              }
            });
          });
        }
        if (noiseStatics.length > 0 && typeof ST !== 'undefined' && ST.watchScoped) {
          // Parse `--noise-1d(amp, hz[, seed])` and evaluate against progress.
          // Deterministic layered sines (matches ST.stNoise's shape): smooth,
          // periodic, seedable. `amp` keeps its unit (px/deg/…); hz is cycles
          // over the clip window; seed shifts the phase for independent streams.
          const parseNoise = (spec) => {
            const m = spec.match(/--noise-1d\(([^)]*)\)/);
            const args = m ? m[1].split(',').map(a => a.trim()) : [];
            const ampRaw = args[0] || '6px';
            const unitM = String(ampRaw).match(/[a-z%]+$/i);
            return {
              amp: parseFloat(ampRaw) || 0,
              unit: unitM ? unitM[0] : '',
              hz: parseFloat(args[1]) || 4,
              seed: parseFloat((args[2] || '').replace(/^seed:/, '')) || 0,
            };
          };
          const noiseAt = (n, p) => {
            const x = p * n.hz * 6.28318 + n.seed * 1.618;
            const v = Math.sin(x) * 0.62 + Math.sin(x * 2.3 + 1.7) * 0.26 + Math.sin(x * 4.1 + 0.5) * 0.12;
            return v * n.amp;
          };
          noiseStatics.forEach(ns => { ns.parsed = parseNoise(ns.spec); });
          ST.watchScoped(root, driverSignal, (rawProgress) => {
            const p = typeof rawProgress === 'number' ? rawProgress : 0;
            noiseStatics.forEach(ns => {
              const val = noiseAt(ns.parsed, p).toFixed(3) + ns.parsed.unit;
              applyStyle(ns.el, ns.property, val, driverSignal, scopeId);
            });
          });
        }

        // If no animated properties, nothing more to do
        if (!scope.properties || scope.properties.length === 0) return;

        // Resolve var() keyframe values, re-resolve on scheme change
        let activeProperties = resolveVarRefs(scope.properties, elements[0]);
        if (activeProperties !== scope.properties) {
          try {
            const reResolve = () => {
              requestAnimationFrame(() => {
                activeProperties = resolveVarRefs(scope.properties, elements[0]);
              });
            };
            const mql = window.matchMedia('(prefers-color-scheme: dark)');
            mql.addEventListener ? mql.addEventListener('change', reResolve) : mql.addListener(reResolve);
            window.addEventListener('st:scheme-change', reResolve);
          } catch(e) { /* headless/SSR safety */ }
        }

        // Watch the driver signal and update animations
        if (typeof ST !== 'undefined' && ST.watch) {
          // Detect if driver provides {x, y} object progress (e.g. @mouse axis:"both")
          const isXYDriver = rawP => rawP !== null && typeof rawP === 'object' && 'x' in rawP && 'y' in rawP;

          // For {x, y} progress, resolve which axis a property should use:
          //   rotate-x, translate-y, etc. — property suffix determines axis
          //   Properties ending in -x/X use progress.x, -y/Y use progress.y
          const axisForProp = (name) => {
            if (name.endsWith('-x') || name.endsWith('X')) return 'x';
            if (name.endsWith('-y') || name.endsWith('Y')) return 'y';
            return 'x'; // default to x-axis for unlabeled properties
          };

          // Clamp + stagger + range for a scalar progress value
          const adjustProgress = (scalar, staggerOffset, scopeRange) => {
            let p = scalar;
            if (staggerOffset > 0) {
              p = Math.max(0, Math.min(1, (p - staggerOffset) / (1 - staggerOffset)));
            }
            if (scopeRange && scopeRange.start !== undefined && scopeRange.end !== undefined) {
              if (p < scopeRange.start) p = 0;
              else if (p > scopeRange.end) p = 1;
              else p = (p - scopeRange.start) / (scopeRange.end - scopeRange.start);
            }
            return p;
          };

          // SCOPED watch (W5a/D1, FUP-094 doctrine): the driver signal is not
          // always owned by `root`. A `@score` publishes each clip's window on
          // the SCORE's element, and `@on &.clip` consumes it on the CLIP's
          // element — an ancestor relationship. `ST.resolve` already walks up
          // the parent chain, so the initial read worked while the reactive
          // watch never fired: the element sat at its start value forever, with
          // a green build and a correct-looking bundle.
          //
          // `watchScoped` mirrors `resolve` — that mirroring IS the FUP-094
          // rule (stdlib/enum/dispatch.st:192 states it canonically): a read
          // path and its watch path that disagree produce exactly this class of
          // silent staleness. Own-element signals are unaffected; the walk
          // starts at `root`.
          ST.watchScoped(root, driverSignal, (rawProgress) => {
            if (%interactive && rawProgress === 0) return;
            const xyMode = isXYDriver(rawProgress);

            elements.forEach((el, i, arr) => {
              // Calculate stagger offset
              const staggerOffset = calcStagger(
                i, arr.length,
                scopeStagger.delay || 0,
                scopeStagger.from || 'first',
                scopeStagger.grid
              );

              if (xyMode) {
                // Per-property axis resolution for {x, y} drivers
                activeProperties.forEach(prop => {
                  const axis = axisForProp(prop.property);
                  const progress = adjustProgress(rawProgress[axis], staggerOffset, scopeRange);
                  const normProp = prop.property.replace(/-([a-z])/g, (_, c) => c.toUpperCase());
                  if (!el._stAnimOwner) el._stAnimOwner = {};
                  const owner = el._stAnimOwner[normProp];
                  const isActive = progress > 0 && progress < 1;
                  if (owner && owner !== driverSignal && !isActive) return;
                  if (isActive || !owner) el._stAnimOwner[normProp] = driverSignal;
                  const value = interpolateKeyframes(prop.keyframes, progress, prop.easing || scopeEasing, colorSpace);
                  applyStyle(el, prop.property, value, driverSignal, scopeId);
                });
              } else {
                // Scalar progress path (scroll, time, single-axis mouse)
                let progress = rawProgress;
                if (staggerOffset > 0) {
                  progress = Math.max(0, Math.min(1, (rawProgress - staggerOffset) / (1 - staggerOffset)));
                }

                if (scopeRange && scopeRange.start !== undefined && scopeRange.end !== undefined) {
                  if (progress < scopeRange.start) {
                    progress = 0;
                  } else if (progress > scopeRange.end) {
                    progress = 1;
                  } else {
                    progress = (progress - scopeRange.start) / (scopeRange.end - scopeRange.start);
                  }
                }

                activeProperties.forEach(prop => {
                  const normProp = prop.property.replace(/-([a-z])/g, (_, c) => c.toUpperCase());
                  if (!el._stAnimOwner) el._stAnimOwner = {};
                  const owner = el._stAnimOwner[normProp];
                  const isActive = progress > 0 && progress < 1;
                  if (owner && owner !== driverSignal && !isActive) return;
                  if (isActive || !owner) el._stAnimOwner[normProp] = driverSignal;
                  const value = interpolateKeyframes(prop.keyframes, progress, prop.easing || scopeEasing, colorSpace);
                  applyStyle(el, prop.property, value, driverSignal, scopeId);
                });
              }
            });
          });
        } else {
          // Fallback: use a simple polling mechanism if ST.watch not available
          console.warn('[apply-animations] ST.watch not available, animations may not work correctly');
        }
      };

      // Bind driver signal to CSS variable for CSS-path animations
      if (typeof ST !== 'undefined' && ST.watch) {
        ST.watch(root, %driver, p => {
          if (p !== null && typeof p === 'object' && 'x' in p && 'y' in p) {
            root.style.setProperty('--st-' + %driver + '-x', p.x);
            root.style.setProperty('--st-' + %driver + '-y', p.y);
          } else {
            root.style.setProperty('--st-' + %driver, p);
          }
        });
      }

      if (!%interactive) {
      // Apply initial state to root element BEFORE setting up watchers (prevents flicker)
      if (anims.properties && anims.properties.length > 0) {
        const resolvedRoot = resolveVarRefs(anims.properties, root);
        applyInitialState([root], resolvedRoot, defaultEasing, %driver, null, colorSpace);
      }

      // Apply initial state to nested scopes BEFORE setting up watchers
      if (anims.scopes && anims.scopes.length > 0) {
        anims.scopes.forEach(scope => {
          const selector = scope.selector;
          let elements;
          if (selector === '&self' || selector === '&') {
            elements = [root];
          } else if (resolveStageObject(root, selector)) {
            elements = [resolveStageObject(root, selector)];
          } else {
            elements = Array.from(root.querySelectorAll(selector));
          }
          if (elements.length > 0 && scope.properties) {
            const resolved = resolveVarRefs(scope.properties, elements[0]);
            applyInitialState(elements, resolved, scope.easing || defaultEasing, %driver, selector, colorSpace);
          }
        });
      }
      }

      // Process root element animations (properties without nested selector)
      if (anims.properties && anims.properties.length > 0) {
        processScope({
          properties: anims.properties,
          staticProperties: anims.staticProperties,
          easing: defaultEasing
        }, [root], %driver, null);
      } else if (anims.staticProperties && anims.staticProperties.length > 0) {
        // Static-only scope. Route through processScope so PLAN-150 W6
        // `--noise-1d(…)` values are driven against progress (a noise motion
        // form has ONLY static properties); plain statics are still applied once
        // inside processScope exactly as before.
        processScope({
          properties: [],
          staticProperties: anims.staticProperties,
          easing: defaultEasing
        }, [root], %driver, null);
      }

      // Process nested scope animations
      if (anims.scopes && anims.scopes.length > 0) {
        anims.scopes.forEach(scope => {
          const selector = scope.selector;
          let elements;

          if (selector === '&self' || selector === '&') {
            elements = [root];
          } else if (resolveStageObject(root, selector)) {
            // PLAN-150 W4: `&name` inside @on resolves to a STAGE OBJECT (a
            // channel proxy) when the root canvas hosts one under that name,
            // so a keyframe on `morph`/`reveal`/`scale` drives the 3D mesh
            // through the same engine that drives DOM. Only falls here when the
            // selector is a bare `&name` the stage registered.
            elements = [resolveStageObject(root, selector)];
          } else {
            elements = Array.from(root.querySelectorAll(selector));
          }

          if (elements.length > 0) {
            processScope(scope, elements, %driver, selector);
          }
        });
      }
    })();
  }

  %exports {
    // No signal exports - this primitive only produces side effects
  }

  %cleanup {
    // Cleanup handled by ST.watch internally
  }
}
