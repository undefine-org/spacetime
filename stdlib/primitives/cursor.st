/// Custom Cursor Primitive
///
/// Creates a branded custom cursor that replaces the native browser cursor.
/// Features smooth lerp-based following, hover state scaling, click compression,
/// and mix-blend-mode effects.
///
/// @param &el - The element scope (cursor hides native cursor on this element)
/// @param size - Cursor dot diameter
/// @param color - Cursor fill color
/// @param blend - CSS mix-blend-mode
/// @param lerp - Follow smoothness (0-1, lower = smoother)
/// @param borderWidth - Border width
/// @param borderColor - Border color
/// @param scale - Base scale
/// @param hoverScale - Scale when hovering interactive elements
/// @param clickScale - Scale on mousedown
/// @param hoverElements - CSS selector for interactive elements
///
/// @yields $cursorX - Current cursor X position
/// @yields $cursorY - Current cursor Y position
/// @yields $isHovering - Whether cursor is over an interactive element
/// @yields $isClicking - Whether mouse button is pressed

%primitive cursor-engine(
  &el,
  size: string = "24px",
  color: string = "#ff6b47",
  blend: string = "difference",
  lerp: number = 0.15,
  borderWidth: string = "0px",
  borderColor: string = "currentColor",
  borderRadius: string = "50%",
  scale: number = 1,
  hoverScale: number = 1.5,
  clickScale: number = 0.9,
  hoverElements: string = "a, button, [data-cursor]"
) {
  %emit js {
    const scopeEl = %&el;

    const pointerFineMql = window.matchMedia('(pointer: fine)');
    const hoverHoverMql = window.matchMedia('(hover: hover)');
    const pointerCoarseMql = window.matchMedia('(pointer: coarse)');
    const hoverNoneMql = window.matchMedia('(hover: none)');
    const reducedMotionMql = window.matchMedia('(prefers-reduced-motion: reduce)');

    const emitIdleValues = () => {
      %yield 0 -> $cursorX;
      %yield 0 -> $cursorY;
      %yield false -> $isHovering;
      %yield false -> $isClicking;
    };

    const hasTouchCapability = () => {
      const nav = typeof navigator !== 'undefined' ? navigator : null;
      const maxTouchPoints = nav && typeof nav.maxTouchPoints === 'number' ? nav.maxTouchPoints : 0;
      const msMaxTouchPoints = nav && typeof nav.msMaxTouchPoints === 'number' ? nav.msMaxTouchPoints : 0;
      return maxTouchPoints > 0 || msMaxTouchPoints > 0 || 'ontouchstart' in window;
    };

    const resolveMode = () => {
      const isPrimaryDesktop = pointerFineMql.matches && hoverHoverMql.matches;
      if (isPrimaryDesktop) return 'desktop';

      const isTouchFallback =
        pointerCoarseMql.matches ||
        hoverNoneMql.matches ||
        hasTouchCapability();
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
      emitIdleValues();

      const pulseDurationMs = 260;
      const pulseCleanupDelayMs = 280;
      const pulseNodes = new Set();
      const pulseTimeoutIds = new Set();

      const onPointerDown = (e) => {
        if (reducedMotionMql.matches) return;
        if (!e.isPrimary || e.pointerType !== 'touch') return;

        const pulse = document.createElement('div');
        pulse.className = 'st-cursor-pulse';
        pulse.style.cssText = [
          'position: fixed',
          'left: ' + e.clientX + 'px',
          'top: ' + e.clientY + 'px',
          'width: calc(' + %size + ' * 1.6)',
          'height: calc(' + %size + ' * 1.6)',
          'border-radius: ' + %borderRadius,
          'border: 2px solid ' + %color,
          'background: transparent',
          'pointer-events: none',
          'z-index: 999999',
          'opacity: 0.42',
          'transform: translate3d(-50%, -50%, 0) scale(0.72)',
          'transition: transform ' + pulseDurationMs + 'ms ease-out, opacity ' + pulseDurationMs + 'ms ease-out',
          'will-change: transform, opacity'
        ].join(';');

        pulseNodes.add(pulse);
        document.body.appendChild(pulse);

        requestAnimationFrame(() => {
          if (!pulseNodes.has(pulse)) return;
          pulse.style.transform = 'translate3d(-50%, -50%, 0) scale(1.75)';
          pulse.style.opacity = '0';
        });

        const timeoutId = window.setTimeout(() => {
          pulseTimeoutIds.delete(timeoutId);
          if (pulseNodes.delete(pulse)) {
            pulse.remove();
          }
        }, pulseCleanupDelayMs);

        pulseTimeoutIds.add(timeoutId);
      };

      window.addEventListener('pointerdown', onPointerDown, { passive: true });

      return () => {
        window.removeEventListener('pointerdown', onPointerDown);

        pulseTimeoutIds.forEach((timeoutId) => {
          clearTimeout(timeoutId);
        });
        pulseTimeoutIds.clear();

        pulseNodes.forEach((pulseNode) => {
          pulseNode.remove();
        });
        pulseNodes.clear();
      };
    };

    const startDesktopMode = () => {
      const prefersReduced = reducedMotionMql.matches;
      const useLerp = prefersReduced ? 1 : %lerp;

      // Create cursor element
      const cursor = document.createElement('div');
      cursor.className = 'st-cursor';
      cursor.style.cssText = [
        'position: fixed',
        'top: 0',
        'left: 0',
        'width: ' + %size,
        'height: ' + %size,
        'background: ' + %color,
        'border-radius: ' + %borderRadius,
        'pointer-events: none',
        'z-index: 999999',
        'mix-blend-mode: ' + %blend,
        'transform: translate3d(-50%, -50%, 0) scale(' + %scale + ')',
        'transition: width 0.2s, height 0.2s, background 0.2s',
        'will-change: transform',
        'border: ' + %borderWidth + ' solid ' + %borderColor
      ].join(';');
      document.body.appendChild(cursor);

      // Hide native cursor on scope element
      scopeEl.style.cursor = 'none';

      // Mark scope and inject global cursor-hide rule for all descendants
      scopeEl.setAttribute('data-st-cursor', '');
      const cursorStyle = document.createElement('style');
      cursorStyle.setAttribute('data-st-cursor-style', '');
      cursorStyle.textContent = [
        '[data-st-cursor] *,',
        '[data-st-cursor] a,',
        '[data-st-cursor] button,',
        '[data-st-cursor] input,',
        '[data-st-cursor] textarea,',
        '[data-st-cursor] select,',
        '[data-st-cursor] [role="button"],',
        '[data-st-cursor] label {',
        '  cursor: none !important;',
        '}'
      ].join('\n');
      document.head.appendChild(cursorStyle);

      let mouseX = 0, mouseY = 0;
      let curX = 0, curY = 0;
      let currentScale = %scale;
      let targetScale = %scale;
      let isHovering = false;
      let isClicking = false;
      let animId = null;

      const animate = () => {
        // Lerp position
        curX += (mouseX - curX) * useLerp;
        curY += (mouseY - curY) * useLerp;

        // Lerp scale
        currentScale += (targetScale - currentScale) * 0.15;

        cursor.style.transform =
          'translate3d(' + curX + 'px, ' + curY + 'px, 0) scale(' + currentScale + ')';

        %yield curX -> $cursorX;
        %yield curY -> $cursorY;

        animId = requestAnimationFrame(animate);
      };

      const onPointerMove = (e) => {
        mouseX = e.clientX;
        mouseY = e.clientY;

        // Check if hovering an interactive element
        const hoveredEl = document.elementFromPoint(e.clientX, e.clientY);
        const wasHovering = isHovering;
        isHovering = hoveredEl && hoveredEl.closest(%hoverElements) !== null;

        if (isHovering !== wasHovering) {
          targetScale = isHovering ? %hoverScale : %scale;
          %yield isHovering -> $isHovering;
        }
      };

      const onPointerDown = () => {
        isClicking = true;
        targetScale = %clickScale;
        %yield true -> $isClicking;
      };

      const onPointerUp = () => {
        isClicking = false;
        targetScale = isHovering ? %hoverScale : %scale;
        %yield false -> $isClicking;
      };

      // Attach listeners
      window.addEventListener('pointermove', onPointerMove, { passive: true });
      window.addEventListener('pointerdown', onPointerDown);
      window.addEventListener('pointerup', onPointerUp);

      // Initial signal values
      emitIdleValues();

      // Start animation loop
      animId = requestAnimationFrame(animate);

      return () => {
        if (animId) cancelAnimationFrame(animId);
        window.removeEventListener('pointermove', onPointerMove);
        window.removeEventListener('pointerdown', onPointerDown);
        window.removeEventListener('pointerup', onPointerUp);
        cursor.remove();
        scopeEl.style.cursor = '';
        const injectedStyle = document.querySelector('style[data-st-cursor-style]');
        if (injectedStyle) injectedStyle.remove();
        scopeEl.removeAttribute('data-st-cursor');
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

    applyMode();

    %cleanup {
      cleanupMode();
      removeMqlListener(pointerFineMql, onModalityChange);
      removeMqlListener(hoverHoverMql, onModalityChange);
      removeMqlListener(pointerCoarseMql, onModalityChange);
      removeMqlListener(hoverNoneMql, onModalityChange);
    }
  }

  %exports {
    $cursorX: number    // current cursor X position (px)
    $cursorY: number    // current cursor Y position (px)
    $isHovering: bool   // cursor is over an interactive element
    $isClicking: bool   // mouse button is pressed
  }
}
