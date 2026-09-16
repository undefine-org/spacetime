/// Pointer Observer Primitive
///
/// Tracks pointer (mouse/touch/pen) position and state.
/// Provides both viewport-relative and element-relative coordinates.
///
/// @param &el - The element to track pointer over
/// @param events - Which pointer events to track: "move", "enter", "leave", "down", "up"
/// @param normalize - Normalize coordinates to 0-1 within element bounds
///
/// @yields $x - Viewport X coordinate
/// @yields $y - Viewport Y coordinate
/// @yields $localX - Element-relative X coordinate
/// @yields $localY - Element-relative Y coordinate
/// @yields $normX - Normalized X (0-1) within element
/// @yields $normY - Normalized Y (0-1) within element
/// @yields $pressure - Pen pressure (0-1)
/// @yields $pointerType - "mouse", "pen", or "touch"
/// @yields $isOver - Whether pointer is currently over element

%primitive pointer(
  &el,
  events: string[] = ["move"],
  normalize: bool = true
) {
  %emit js {
    const target = %&el;
    let isOver = false;

    const updatePosition = (e) => {
      const rect = target.getBoundingClientRect();

      // Viewport coordinates
      %yield e.clientX -> $x;
      %yield e.clientY -> $y;

      // Element-relative coordinates
      const localX = e.clientX - rect.left;
      const localY = e.clientY - rect.top;
      %yield localX -> $localX;
      %yield localY -> $localY;

      // Store on element for cross-scope access (used by glow-render, etc.)
      target._stMouseX = localX;
      target._stMouseY = localY;

      // Mouse coordinate aliases for convenience
      %yield localX -> $mouseX;
      %yield localY -> $mouseY;

      // Normalized coordinates (0-1)
      if (%normalize && rect.width > 0 && rect.height > 0) {
        %yield localX / rect.width -> $normX;
        %yield localY / rect.height -> $normY;

        // Centered normalized (-0.5 to 0.5, useful for effects)
        %yield (localX / rect.width) - 0.5 -> $centerX;
        %yield (localY / rect.height) - 0.5 -> $centerY;
      }

      // Pressure and type
      %yield e.pressure ?? 0 -> $pressure;
      %yield e.pointerType ?? 'mouse' -> $pointerType;
      %yield e.isPrimary ?? true -> $isPrimary;

      // Button state
      %yield e.buttons -> $buttons;
      %yield (e.buttons & 1) !== 0 -> $isPressed;
    };

    const handleEnter = (e) => {
      isOver = true;
      %yield true -> $isOver;
      updatePosition(e);
    };

    const handleLeave = (e) => {
      isOver = false;
      %yield false -> $isOver;
    };

    const handleMove = (e) => {
      updatePosition(e);
    };

    const handleDown = (e) => {
      %yield true -> $isDown;
      updatePosition(e);
    };

    const handleUp = (e) => {
      %yield false -> $isDown;
      updatePosition(e);
    };

    // Set up requested event listeners
    const eventMap = {
      'move': ['pointermove', handleMove],
      'enter': ['pointerenter', handleEnter],
      'leave': ['pointerleave', handleLeave],
      'down': ['pointerdown', handleDown],
      'up': ['pointerup', handleUp]
    };

    // Parse events - handle a real array, a JSON-array string (how array-typed
    // macro params arrive through %emit marshalling), or a comma-separated string.
    const rawEvents = %events;
    const eventList = Array.isArray(rawEvents)
      ? rawEvents
      : (() => {
          const s = String(rawEvents).trim();
          if (s.startsWith('[')) {
            try { const parsed = JSON.parse(s); if (Array.isArray(parsed)) return parsed; } catch (e) {}
          }
          return s.split(/,\s*/).map(s => s.trim()).filter(Boolean);
        })();

    const listeners = [];
    for (const eventType of eventList) {
      const mapping = eventMap[eventType];
      if (mapping) {
        const [eventName, handler] = mapping;
        target.addEventListener(eventName, handler, { passive: true });
        listeners.push([eventName, handler]);
      }
    }

    // Always track enter/leave if move is requested
    if (eventList.includes('move')) {
      if (!eventList.includes('enter')) {
        target.addEventListener('pointerenter', handleEnter, { passive: true });
        listeners.push(['pointerenter', handleEnter]);
      }
      if (!eventList.includes('leave')) {
        target.addEventListener('pointerleave', handleLeave, { passive: true });
        listeners.push(['pointerleave', handleLeave]);
      }
    }

    %cleanup {
      for (const [eventName, handler] of listeners) {
        target.removeEventListener(eventName, handler);
      }
    }
  }

  %exports {
    $x: number           // viewport X
    $y: number           // viewport Y
    $localX: number      // element-relative X
    $localY: number      // element-relative Y
    $mouseX: number      // alias for localX
    $mouseY: number      // alias for localY
    $normX: number       // normalized X (0-1)
    $normY: number       // normalized Y (0-1)
    $centerX: number     // centered normalized X (-0.5 to 0.5)
    $centerY: number     // centered normalized Y (-0.5 to 0.5)
    $pressure: number    // pen pressure (0-1)
    $pointerType: string // "mouse", "pen", "touch"
    $isPrimary: bool     // is primary pointer
    $isOver: bool        // pointer is over element
    $isDown: bool        // pointer button is down
    $isPressed: bool     // primary button pressed
    $buttons: number     // button bitmask
  }
}
