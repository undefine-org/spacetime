/// Magnetic Hover Primitive
///
/// Proximity-based magnetic pull that draws an element toward the cursor
/// when the mouse enters a configurable radius. Uses lerp interpolation
/// for smooth GPU-accelerated movement via translate3d.
///
/// @param &el - The element to apply magnetic effect to
/// @param strength - Pull strength (0-1, how much element moves toward cursor)
/// @param radius - Activation radius from element center (px)
/// @param lerp - Interpolation smoothness (0-1)
/// @param axis - Movement axis: "both", "x", or "y"
/// @param trigger - What triggers the effect: "self" or "parent"
/// @param tolerance - Minimum distance ratio to start effect (0-1)
///
/// @yields $magnetX - Current X translation offset
/// @yields $magnetY - Current Y translation offset
/// @yields $distance - Distance from mouse to element center
/// @yields $isActive - Whether magnetic pull is currently active

%primitive magnetic-effect(
  &el,
  strength: number = 0.3,
  radius: number = 100,
  lerp: number = 0.1,
  axis: ("both" | "x" | "y") = "both",
  trigger: ("self" | "parent") = "self",
  tolerance: number = 0.5
) {
  %emit js {
    const triggerEl = %trigger === 'parent' ? el.parentElement : el;

    const pointerFineMql = window.matchMedia('(pointer: fine)');
    const hoverHoverMql = window.matchMedia('(hover: hover)');
    const pointerCoarseMql = window.matchMedia('(pointer: coarse)');
    const hoverNoneMql = window.matchMedia('(hover: none)');
    const reducedMotionMql = window.matchMedia('(prefers-reduced-motion: reduce)');

    const emitIdleValues = () => {
      %yield 0 -> $magnetX;
      %yield 0 -> $magnetY;
      %yield 0 -> $distance;
      %yield false -> $isActive;
    };

    const resetTransform = () => {
      el.style.transform = '';
    };

    const hasTouchCapability = () => {
      const nav = typeof navigator !== 'undefined' ? navigator : null;
      const maxTouchPoints = nav && typeof nav.maxTouchPoints === 'number' ? nav.maxTouchPoints : 0;
      const msMaxTouchPoints = nav && typeof nav.msMaxTouchPoints === 'number' ? nav.msMaxTouchPoints : 0;
      return maxTouchPoints > 0 || msMaxTouchPoints > 0 || 'ontouchstart' in window;
    };

    const resolveMode = () => {
      const isPrimaryDesktop =
        pointerFineMql.matches &&
        hoverHoverMql.matches &&
        !reducedMotionMql.matches;
      if (isPrimaryDesktop) return 'desktop';

      const isTouchFallback =
        pointerCoarseMql.matches ||
        hoverNoneMql.matches ||
        hasTouchCapability() ||
        reducedMotionMql.matches;
      return isTouchFallback ? 'touch' : 'desktop';
    };

    const addMqlListener = (mql, listener) => {
      if (mql.addEventListener) {
        mql.addEventListener('change', listener);
      } else {
        mql.addListener(listener);
      }
    };

    const removeMqlListener = (mql, listener) => {
      if (mql.removeEventListener) {
        mql.removeEventListener('change', listener);
      } else {
        mql.removeListener(listener);
      }
    };

    const startTouchMode = () => {
      resetTransform();
      emitIdleValues();
      return () => {
        resetTransform();
      };
    };

    const startDesktopMode = () => {
      let mouseX = 0, mouseY = 0;
      let currentX = 0, currentY = 0;
      let targetX = 0, targetY = 0;
      let isActive = false;
      let animId = null;

      const animate = () => {
        // Lerp toward target
        currentX += (targetX - currentX) * %lerp;
        currentY += (targetY - currentY) * %lerp;

        // Snap to zero if very close
        if (Math.abs(currentX) < 0.1 && Math.abs(currentY) < 0.1 && !isActive) {
          currentX = 0;
          currentY = 0;
          resetTransform();
          %yield 0 -> $magnetX;
          %yield 0 -> $magnetY;
          animId = null;
          return; // Stop loop when idle
        }

        el.style.transform = 'translate3d(' + currentX + 'px, ' + currentY + 'px, 0)';

        %yield currentX -> $magnetX;
        %yield currentY -> $magnetY;

        animId = requestAnimationFrame(animate);
      };

      const startLoop = () => {
        if (!animId) {
          animId = requestAnimationFrame(animate);
        }
      };

      const onPointerMove = (e) => {
        mouseX = e.clientX;
        mouseY = e.clientY;

        const rect = (triggerEl || el).getBoundingClientRect();
        const centerX = rect.left + rect.width / 2;
        const centerY = rect.top + rect.height / 2;

        const dx = mouseX - centerX;
        const dy = mouseY - centerY;
        const distance = Math.sqrt(dx * dx + dy * dy);

        %yield distance -> $distance;

        const radiusPx = %radius;
        const toleranceDistance = radiusPx * %tolerance;

        if (distance < radiusPx && distance > toleranceDistance) {
          // Within radius — apply magnetic pull
          const factor = %strength * (1 - distance / radiusPx);

          switch (%axis) {
            case 'x':
              targetX = dx * factor;
              targetY = 0;
              break;
            case 'y':
              targetX = 0;
              targetY = dy * factor;
              break;
            default:
              targetX = dx * factor;
              targetY = dy * factor;
          }

          if (!isActive) {
            isActive = true;
            %yield true -> $isActive;
          }
          startLoop();
        } else if (distance <= toleranceDistance) {
          // Too close — max pull
          const factor = %strength;
          switch (%axis) {
            case 'x':
              targetX = dx * factor;
              targetY = 0;
              break;
            case 'y':
              targetX = 0;
              targetY = dy * factor;
              break;
            default:
              targetX = dx * factor;
              targetY = dy * factor;
          }
          if (!isActive) {
            isActive = true;
            %yield true -> $isActive;
          }
          startLoop();
        } else {
          // Outside radius — return to origin
          targetX = 0;
          targetY = 0;
          if (isActive) {
            isActive = false;
            %yield false -> $isActive;
          }
          startLoop(); // Keep animating return
        }
      };

      window.addEventListener('pointermove', onPointerMove, { passive: true });
      emitIdleValues();

      return () => {
        if (animId) cancelAnimationFrame(animId);
        window.removeEventListener('pointermove', onPointerMove);
        targetX = 0;
        targetY = 0;
        currentX = 0;
        currentY = 0;
        isActive = false;
        resetTransform();
        emitIdleValues();
      };
    };

    let mode = null;
    let cleanupMode = () => {};

    const applyMode = () => {
      const nextMode = resolveMode();
      if (nextMode === mode) return;

      cleanupMode();
      mode = nextMode;
      cleanupMode = mode === 'desktop' ? startDesktopMode() : startTouchMode();
    };

    const onModalityChange = () => {
      applyMode();
    };

    addMqlListener(pointerFineMql, onModalityChange);
    addMqlListener(hoverHoverMql, onModalityChange);
    addMqlListener(pointerCoarseMql, onModalityChange);
    addMqlListener(hoverNoneMql, onModalityChange);
    addMqlListener(reducedMotionMql, onModalityChange);

    applyMode();

    %cleanup {
      cleanupMode();
      removeMqlListener(pointerFineMql, onModalityChange);
      removeMqlListener(hoverHoverMql, onModalityChange);
      removeMqlListener(pointerCoarseMql, onModalityChange);
      removeMqlListener(hoverNoneMql, onModalityChange);
      removeMqlListener(reducedMotionMql, onModalityChange);
      resetTransform();
    }
  }

  %exports {
    $magnetX: number    // current X translation offset (px)
    $magnetY: number    // current Y translation offset (px)
    $distance: number   // distance from mouse to element center (px)
    $isActive: bool     // whether magnetic pull is currently active
  }
}
