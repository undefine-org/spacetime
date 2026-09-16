/// Smooth Scroll Primitive
///
/// Lenis-style smooth scroll engine that overrides native scroll
/// with rAF-based interpolation for a buttery-smooth experience.
/// Intercepts wheel, keyboard, and anchor navigation.
///
/// @param &el - The scrollable element (or window/document)
/// @param lerp - Interpolation factor (0-1, lower = smoother)
/// @param duration - Scroll duration in seconds
/// @param smoothWheel - Whether to smooth mouse wheel scrolling
/// @param smoothTouch - Whether to smooth touch scrolling (false preserves native)
/// @param infinite - Enable infinite scroll wrapping
/// @param orientation - Scroll axis: "vertical" or "horizontal"
///
/// @yields $scrollY - Current interpolated scroll position
/// @yields $velocity - Scroll velocity in px/s
/// @yields $progress - Normalized scroll progress (0-1)
/// @yields $isScrolling - Whether currently animating
/// @yields $direction - Scroll direction: "up", "down", or "none"

%primitive smooth-scroll(
  &el,
  lerp: number = 0.1,
  duration: number = 1.2,
  smoothWheel: bool = true,
  smoothTouch: bool = false,
  infinite: bool = false,
  orientation: ("vertical" | "horizontal") = "vertical"
) {
  %emit js {
    const isWindow = el === window || el === document || el === document.documentElement || el === document.body;
    const target = isWindow ? document.documentElement : el;
    const scrollTarget = isWindow ? window : el;

    // Respect prefers-reduced-motion
    const prefersReduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    if (prefersReduced) {
      // Expose default signals without intercepting scroll
      %yield 0 -> $scrollY;
      %yield 0 -> $velocity;
      %yield 0 -> $progress;
      %yield false -> $isScrolling;
      %yield 'none' -> $direction;

      const passiveUpdate = () => {
        const y = isWindow ? window.scrollY : el.scrollTop;
        const max = isWindow
          ? document.documentElement.scrollHeight - window.innerHeight
          : el.scrollHeight - el.clientHeight;
        %yield y -> $scrollY;
        %yield max > 0 ? y / max : 0 -> $progress;
      };
      scrollTarget.addEventListener('scroll', passiveUpdate, { passive: true });
      %cleanup {
        scrollTarget.removeEventListener('scroll', passiveUpdate);
      }
    } else {

      const isVertical = %orientation === 'vertical';
      let currentScroll = isVertical
        ? (isWindow ? window.scrollY : el.scrollTop)
        : (isWindow ? window.scrollX : el.scrollLeft);
      let targetScroll = currentScroll;
      let velocity = 0;
      let lastTime = performance.now();
      let isScrolling = false;
      let animId = null;
      let lastDirection = 'none';

      const getMax = () => {
        if (isVertical) {
          return isWindow
            ? document.documentElement.scrollHeight - window.innerHeight
            : el.scrollHeight - el.clientHeight;
        }
        return isWindow
          ? document.documentElement.scrollWidth - window.innerWidth
          : el.scrollWidth - el.clientWidth;
      };

      const clamp = (val, min, max) => Math.min(Math.max(val, min), max);

      const animate = () => {
        const now = performance.now();
        const dt = (now - lastTime) / 1000;
        lastTime = now;

        const diff = targetScroll - currentScroll;

        // Lerp interpolation
        currentScroll += diff * %lerp;

        // Snap if close enough
        if (Math.abs(diff) < 0.5) {
          currentScroll = targetScroll;
        }

        // Calculate velocity
        velocity = dt > 0 ? diff * %lerp / dt : 0;

        // Determine direction
        if (diff > 0.5) {
          lastDirection = isVertical ? 'down' : 'right';
        } else if (diff < -0.5) {
          lastDirection = isVertical ? 'up' : 'left';
        }

        // Apply scroll
        if (isVertical) {
          scrollTarget.scrollTo({ top: currentScroll, behavior: 'instant' });
        } else {
          scrollTarget.scrollTo({ left: currentScroll, behavior: 'instant' });
        }

        // Yield signals
        const max = getMax();
        %yield currentScroll -> $scrollY;
        %yield velocity -> $velocity;
        %yield max > 0 ? currentScroll / max : 0 -> $progress;
        %yield lastDirection -> $direction;

        // Continue or stop
        if (Math.abs(diff) > 0.5) {
          isScrolling = true;
          %yield true -> $isScrolling;
          animId = requestAnimationFrame(animate);
        } else {
          isScrolling = false;
          %yield false -> $isScrolling;
          %yield 'none' -> $direction;
          animId = null;
        }
      };

      const startAnimation = () => {
        if (!animId) {
          lastTime = performance.now();
          animId = requestAnimationFrame(animate);
        }
      };

      const scrollTo = (value) => {
        const max = getMax();
        if (%infinite) {
          targetScroll = value;
        } else {
          targetScroll = clamp(value, 0, max);
        }
        startAnimation();
      };

      // Wheel handler
      const onWheel = (e) => {
        if (!%smoothWheel) return;
        e.preventDefault();
        const delta = isVertical ? e.deltaY : e.deltaX;
        scrollTo(targetScroll + delta);
      };

      // Keyboard handler
      const onKeyDown = (e) => {
        const step = 100;
        const pageStep = isWindow ? window.innerHeight * 0.9 : el.clientHeight * 0.9;
        const max = getMax();

        switch (e.key) {
          case 'ArrowDown':
            if (isVertical) { e.preventDefault(); scrollTo(targetScroll + step); }
            break;
          case 'ArrowUp':
            if (isVertical) { e.preventDefault(); scrollTo(targetScroll - step); }
            break;
          case 'ArrowRight':
            if (!isVertical) { e.preventDefault(); scrollTo(targetScroll + step); }
            break;
          case 'ArrowLeft':
            if (!isVertical) { e.preventDefault(); scrollTo(targetScroll - step); }
            break;
          case 'PageDown':
            e.preventDefault(); scrollTo(targetScroll + pageStep);
            break;
          case 'PageUp':
            e.preventDefault(); scrollTo(targetScroll - pageStep);
            break;
          case 'Home':
            e.preventDefault(); scrollTo(0);
            break;
          case 'End':
            e.preventDefault(); scrollTo(max);
            break;
          case ' ':
            if (!e.target.matches('input, textarea, select, [contenteditable]')) {
              e.preventDefault();
              scrollTo(targetScroll + (e.shiftKey ? -pageStep : pageStep));
            }
            break;
        }
      };

      // Touch passthrough (no smoothing if smoothTouch is false)
      const onTouchStart = () => {
        if (!%smoothTouch) {
          if (animId) {
            cancelAnimationFrame(animId);
            animId = null;
          }
          // Sync target with actual scroll position on touch
          currentScroll = isVertical
            ? (isWindow ? window.scrollY : el.scrollTop)
            : (isWindow ? window.scrollX : el.scrollLeft);
          targetScroll = currentScroll;
        }
      };

      const onTouchEnd = () => {
        if (!%smoothTouch) {
          currentScroll = isVertical
            ? (isWindow ? window.scrollY : el.scrollTop)
            : (isWindow ? window.scrollX : el.scrollLeft);
          targetScroll = currentScroll;
        }
      };

      // Anchor click handler
      const onAnchorClick = (e) => {
        const anchor = e.target.closest('a[href^="#"]');
        if (!anchor) return;
        const id = anchor.getAttribute('href').slice(1);
        const targetEl = document.getElementById(id);
        if (!targetEl) return;
        e.preventDefault();
        const rect = targetEl.getBoundingClientRect();
        const offset = isVertical
          ? rect.top + (isWindow ? window.scrollY : el.scrollTop)
          : rect.left + (isWindow ? window.scrollX : el.scrollLeft);
        scrollTo(offset);
      };

      // Attach listeners
      const wheelEl = isWindow ? document : el;
      if (%smoothWheel) {
        wheelEl.addEventListener('wheel', onWheel, { passive: false });
      }
      document.addEventListener('keydown', onKeyDown);
      scrollTarget.addEventListener('touchstart', onTouchStart, { passive: true });
      scrollTarget.addEventListener('touchend', onTouchEnd, { passive: true });
      document.addEventListener('click', onAnchorClick);

      // Initial yield
      const max = getMax();
      %yield currentScroll -> $scrollY;
      %yield 0 -> $velocity;
      %yield max > 0 ? currentScroll / max : 0 -> $progress;
      %yield false -> $isScrolling;
      %yield 'none' -> $direction;

      %cleanup {
        if (animId) cancelAnimationFrame(animId);
        if (%smoothWheel) {
          wheelEl.removeEventListener('wheel', onWheel);
        }
        document.removeEventListener('keydown', onKeyDown);
        scrollTarget.removeEventListener('touchstart', onTouchStart);
        scrollTarget.removeEventListener('touchend', onTouchEnd);
        document.removeEventListener('click', onAnchorClick);
      }
    }
  }

  %exports {
    $scrollY: number      // current interpolated scroll position (px)
    $velocity: number     // scroll velocity (px/s)
    $progress: number     // normalized scroll progress (0-1)
    $isScrolling: bool    // whether currently animating
    $direction: string    // "up", "down", "left", "right", or "none"
  }
}
