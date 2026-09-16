/// Gesture Primitive
///
/// Tracks drag gestures with pointer capture.
/// Provides delta movement, velocity, and gesture state.
///
/// @param &el - The element to make draggable
/// @param capture - Use pointer capture for reliable tracking
/// @param threshold - Minimum movement to start drag (prevents accidental drags)
///
/// @yields $active - Currently in drag state
/// @yields $deltaX - Total X movement from start
/// @yields $deltaY - Total Y movement from start
/// @yields $velocityX - Current X velocity (px/s)
/// @yields $velocityY - Current Y velocity (px/s)
/// @yields $startX - Starting X position
/// @yields $startY - Starting Y position

%primitive gesture(
  &el,
  capture: bool = true,
  threshold: number = 0
) {
  %emit js {
    // el is provided by the wrapper (forEach callback parameter)
    let state = 'idle'; // idle, pending, active
    let pointerId = null;
    let startX = 0, startY = 0;
    let lastX = 0, lastY = 0;
    let lastTime = 0;
    let velocityX = 0, velocityY = 0;

    // Velocity smoothing (exponential moving average)
    const velocitySmoothing = 0.8;

    const onPointerDown = (e) => {
      if (state !== 'idle') return;

      pointerId = e.pointerId;
      startX = e.clientX;
      startY = e.clientY;
      lastX = startX;
      lastY = startY;
      lastTime = performance.now();
      velocityX = 0;
      velocityY = 0;

      if (%threshold > 0) {
        state = 'pending';
      } else {
        state = 'active';
        %yield true -> $active;
      }

      if (%capture) {
        el.setPointerCapture(e.pointerId);
      }

      %yield startX -> $startX;
      %yield startY -> $startY;
      %yield 0 -> $deltaX;
      %yield 0 -> $deltaY;
      %yield 0 -> $velocityX;
      %yield 0 -> $velocityY;
      // Reset the drop edge so a fresh gesture can fire on-drop again. A drop is
      // a $dropped false->true PULSE emitted only on a genuine release (pointerup
      // while active), never on cancel — so an interrupted drag fires no drop.
      %yield false -> $dropped;

      e.preventDefault();
    };

    const onPointerMove = (e) => {
      if (state === 'idle' || e.pointerId !== pointerId) return;

      const now = performance.now();
      const dt = (now - lastTime) / 1000;

      const currentX = e.clientX;
      const currentY = e.clientY;
      const deltaX = currentX - startX;
      const deltaY = currentY - startY;

      // Check threshold for pending state
      if (state === 'pending') {
        const distance = Math.sqrt(deltaX * deltaX + deltaY * deltaY);
        if (distance >= %threshold) {
          state = 'active';
          %yield true -> $active;
        } else {
          return;
        }
      }

      // Calculate instantaneous velocity
      if (dt > 0) {
        const instantVx = (currentX - lastX) / dt;
        const instantVy = (currentY - lastY) / dt;

        // Smooth velocity
        velocityX = velocityX * velocitySmoothing + instantVx * (1 - velocitySmoothing);
        velocityY = velocityY * velocitySmoothing + instantVy * (1 - velocitySmoothing);
      }

      %yield deltaX -> $deltaX;
      %yield deltaY -> $deltaY;
      %yield velocityX -> $velocityX;
      %yield velocityY -> $velocityY;

      // Distance and angle from start
      %yield Math.sqrt(deltaX * deltaX + deltaY * deltaY) -> $distance;
      %yield Math.atan2(deltaY, deltaX) -> $angle;

      lastX = currentX;
      lastY = currentY;
      lastTime = now;
    };

    const onPointerUp = (e) => {
      if (e.pointerId !== pointerId) return;

      const wasActive = state === 'active';
      state = 'idle';
      pointerId = null;

      if (%capture) {
        el.releasePointerCapture(e.pointerId);
      }

      // Release point (viewport coords). Yielded BEFORE $active -> false so a
      // drop handler watching that transition (e.g. @drag's on-drop) sees the
      // coordinates already set when it runs. Used by ST.dropTarget to hit-test
      // the element under the release point (PLAN-055).
      %yield e.clientX -> $dropX;
      %yield e.clientY -> $dropY;

      // Final velocity for momentum/flick gestures
      if (wasActive) {
        %yield velocityX -> $finalVelocityX;
        %yield velocityY -> $finalVelocityY;
      }

      // Set inactive before the drop edge so all release data is in place.
      %yield false -> $active;

      // Signal a genuine DROP: a false->true pulse on $dropped, emitted only
      // when the gesture was actually active (a real drag that was released),
      // AFTER $dropX/$dropY are set. Drop handlers watch THIS edge, not
      // `$active -> false`, so a cancelled/interrupted drag never fires on-drop.
      if (wasActive) {
        %yield true -> $dropped;
      }
    };

    const onPointerCancel = (e) => {
      if (e.pointerId !== pointerId) return;

      state = 'idle';
      pointerId = null;

      %yield false -> $active;
      %yield 0 -> $deltaX;
      %yield 0 -> $deltaY;
    };

    el.addEventListener('pointerdown', onPointerDown);
    el.addEventListener('pointermove', onPointerMove);
    el.addEventListener('pointerup', onPointerUp);
    el.addEventListener('pointercancel', onPointerCancel);

    // Prevent default drag behavior
    el.style.touchAction = 'none';

    %cleanup {
      el.removeEventListener('pointerdown', onPointerDown);
      el.removeEventListener('pointermove', onPointerMove);
      el.removeEventListener('pointerup', onPointerUp);
      el.removeEventListener('pointercancel', onPointerCancel);
    }
  }

  %exports {
    $active: bool          // currently dragging
    $deltaX: number        // total X movement from start
    $deltaY: number        // total Y movement from start
    $velocityX: number     // current X velocity (px/s)
    $velocityY: number     // current Y velocity (px/s)
    $startX: number        // gesture start X
    $startY: number        // gesture start Y
    $distance: number      // distance from start
    $angle: number         // angle from start (radians)
    $finalVelocityX: number // velocity at release
    $finalVelocityY: number // velocity at release
    $dropX: number          // viewport X at release (for drop hit-testing)
    $dropY: number          // viewport Y at release (for drop hit-testing)
    $dropped: bool          // false->true PULSE on a genuine release (not cancel)
  }
}
