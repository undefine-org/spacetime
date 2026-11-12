/// Scroll Observer Primitive
///
/// Tracks scroll position with optional progress normalization.
/// Works with both window/document and scrollable elements.

%primitive scroll(
  /// The scrollable element (or window/document)
  &el,
  /// Primary axis: "x", "y", or "both"
  axis: ("x" | "y" | "both") = "y",
  /// Throttle updates (ms), 0 for every frame
  throttle: number = 0
) {
  %emit js {
    const isWindow = el === window || el === document || el === document.documentElement;
    const target = isWindow ? window : el;

    let lastX = 0, lastY = 0;
    let lastTime = performance.now();
    let throttleTimer = null;
    let velocityX = 0, velocityY = 0;

    const getScrollInfo = () => {
      let scrollX, scrollY, maxX, maxY;

      if (isWindow) {
        scrollX = window.scrollX || window.pageXOffset;
        scrollY = window.scrollY || window.pageYOffset;
        maxX = document.documentElement.scrollWidth - window.innerWidth;
        maxY = document.documentElement.scrollHeight - window.innerHeight;
      } else {
        scrollX = el.scrollLeft;
        scrollY = el.scrollTop;
        maxX = el.scrollWidth - el.clientWidth;
        maxY = el.scrollHeight - el.clientHeight;
      }

      return { scrollX, scrollY, maxX, maxY };
    };

    const update = () => {
      const now = performance.now();
      const dt = (now - lastTime) / 1000; // seconds
      const { scrollX, scrollY, maxX, maxY } = getScrollInfo();

      // Calculate velocity
      if (dt > 0) {
        velocityX = (scrollX - lastX) / dt;
        velocityY = (scrollY - lastY) / dt;
      }

      // Determine direction
      let direction = 'none';
      const dx = scrollX - lastX;
      const dy = scrollY - lastY;

      if (Math.abs(dy) > Math.abs(dx)) {
        direction = dy > 0 ? 'down' : dy < 0 ? 'up' : 'none';
      } else if (dx !== 0) {
        direction = dx > 0 ? 'right' : 'left';
      }

      // Yield all values
      %yield scrollX -> $x;
      %yield scrollY -> $y;

      const progressX = maxX > 0 ? scrollX / maxX : 0;
      const progressY = maxY > 0 ? scrollY / maxY : 0;

      %yield progressX -> $progressX;
      %yield progressY -> $progressY;

      // Primary axis progress
      switch (%axis) {
        case "x":
          %yield progressX -> $progress;
          %yield velocityX -> $velocity;
          break;
        case "y":
          %yield progressY -> $progress;
          %yield velocityY -> $velocity;
          break;
        default:
          %yield Math.max(progressX, progressY) -> $progress;
          %yield Math.max(Math.abs(velocityX), Math.abs(velocityY)) -> $velocity;
      }

      %yield direction -> $direction;
      %yield maxX -> $maxX;
      %yield maxY -> $maxY;

      lastX = scrollX;
      lastY = scrollY;
      lastTime = now;
    };

    const handleScroll = () => {
      if (%throttle > 0) {
        if (!throttleTimer) {
          throttleTimer = setTimeout(() => {
            throttleTimer = null;
            update();
          }, %throttle);
        }
      } else {
        update();
      }
    };

    target.addEventListener('scroll', handleScroll, { passive: true });

    // Initial read
    update();

    %cleanup {
      target.removeEventListener('scroll', handleScroll);
      if (throttleTimer) clearTimeout(throttleTimer);
    }
  }

  %exports {
    /// Horizontal scroll position
    $x: number           // horizontal scroll position (px)
    /// Vertical scroll position
    $y: number           // vertical scroll position (px)
    /// Normalized horizontal progress (0-1)
    $progressX: number   // horizontal progress (0-1)
    /// Normalized vertical progress (0-1)
    $progressY: number   // vertical progress (0-1)
    /// Progress on primary axis (0-1)
    $progress: number    // progress on primary axis (0-1)
    /// Scroll velocity in px/s
    $velocity: number    // scroll velocity (px/s)
    /// Scroll direction: "up", "down", "left", "right", or "none"
    $direction: string   // "up", "down", "left", "right", "none"
    $maxX: number        // maximum horizontal scroll
    $maxY: number        // maximum vertical scroll
  }
}
