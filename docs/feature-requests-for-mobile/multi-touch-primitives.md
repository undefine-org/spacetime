# Feature Request: Multi-Touch Primitives

## Problem Statement

The current `gesture` primitive in `stdlib/primitives/gesture.st` only tracks a single pointer. Mobile applications require multi-touch gestures:
- **Pinch-to-zoom**: Two fingers moving apart/together
- **Rotate**: Two fingers rotating around a center point
- **Multi-finger swipe**: Two or three finger swipes for navigation
- **Simultaneous gestures**: Pinch while panning

### Current Limitation

The existing gesture primitive uses `pointerId` to track a single pointer:

```javascript
// From stdlib/primitives/gesture.st
let pointerId = null;

const onPointerDown = (e) => {
  if (state !== 'idle') return;  // Only one pointer at a time
  pointerId = e.pointerId;
  // ...
};
```

This prevents multi-touch gestures because:
1. Only one pointer is tracked
2. Second touch is ignored when first is active
3. No way to calculate distance/angle between touches

### Desired Syntax (Does Not Compile)

```spacetime
%primitive pinch-gesture(&el) {
  // Need to track multiple pointers
  %emit js {
    const pointers = new Map();  // Map<pointerId, { x, y }>

    const onPointerDown = (e) => {
      pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });

      if (pointers.size === 2) {
        const [p1, p2] = Array.from(pointers.values());
        const initialDistance = Math.hypot(p2.x - p1.x, p2.y - p1.y);
        const initialAngle = Math.atan2(p2.y - p1.y, p2.x - p1.x);
        // ...
      }
    };
  }

  %exports {
    $scale: number      // Current scale factor
    $rotation: number   // Current rotation in radians
    $centerX: number    // Center point X
    $centerY: number    // Center point Y
  }
}
```

## Proposed Solution

### 1. Multi-Pointer Tracking Primitive

Create a low-level primitive that tracks all active pointers:

```spacetime
%primitive pointer-tracker(&el, maxPointers: number = 10) {
  %emit js {
    const pointers = new Map();

    const onPointerDown = (e) => {
      if (pointers.size >= %maxPointers) return;

      pointers.set(e.pointerId, {
        id: e.pointerId,
        x: e.clientX,
        y: e.clientY,
        startX: e.clientX,
        startY: e.clientY,
        pressure: e.pressure,
        type: e.pointerType  // "touch", "pen", "mouse"
      });

      el.setPointerCapture(e.pointerId);
      %yield Array.from(pointers.values()) -> $pointers;
      %yield pointers.size -> $count;
    };

    const onPointerMove = (e) => {
      if (!pointers.has(e.pointerId)) return;

      pointers.set(e.pointerId, {
        ...pointers.get(e.pointerId),
        x: e.clientX,
        y: e.clientY,
        pressure: e.pressure
      });

      %yield Array.from(pointers.values()) -> $pointers;
    };

    const onPointerUp = (e) => {
      if (!pointers.has(e.pointerId)) return;

      el.releasePointerCapture(e.pointerId);
      pointers.delete(e.pointerId);

      %yield Array.from(pointers.values()) -> $pointers;
      %yield pointers.size -> $count;
    };

    el.addEventListener('pointerdown', onPointerDown);
    el.addEventListener('pointermove', onPointerMove);
    el.addEventListener('pointerup', onPointerUp);
    el.addEventListener('pointercancel', onPointerUp);

    el.style.touchAction = 'none';

    %cleanup {
      for (const id of pointers.keys()) {
        el.releasePointerCapture(id);
      }
      el.removeEventListener('pointerdown', onPointerDown);
      el.removeEventListener('pointermove', onPointerMove);
      el.removeEventListener('pointerup', onPointerUp);
      el.removeEventListener('pointercancel', onPointerUp);
    }
  }

  %exports {
    $pointers: object[]   // Array of pointer objects
    $count: number        // Number of active pointers
  }
}
```

### 2. Pinch Gesture Primitive

Build on top of pointer-tracker:

```spacetime
%primitive pinch-gesture(
  &el,
  minPointers: number = 2,
  threshold: number = 10
) {
  %emit js {
    const pointers = new Map();
    let initialDistance = null;
    let initialAngle = null;
    let initialCenterX = null;
    let initialCenterY = null;
    let active = false;

    const calculateGeometry = () => {
      if (pointers.size < %minPointers) return null;

      const pts = Array.from(pointers.values());
      const [p1, p2] = pts;

      const centerX = (p1.x + p2.x) / 2;
      const centerY = (p1.y + p2.y) / 2;
      const distance = Math.hypot(p2.x - p1.x, p2.y - p1.y);
      const angle = Math.atan2(p2.y - p1.y, p2.x - p1.x);

      return { centerX, centerY, distance, angle };
    };

    const onPointerDown = (e) => {
      pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
      el.setPointerCapture(e.pointerId);

      if (pointers.size === %minPointers) {
        const geo = calculateGeometry();
        initialDistance = geo.distance;
        initialAngle = geo.angle;
        initialCenterX = geo.centerX;
        initialCenterY = geo.centerY;

        %yield 1 -> $scale;
        %yield 0 -> $rotation;
        %yield geo.centerX -> $centerX;
        %yield geo.centerY -> $centerY;
      }

      e.preventDefault();
    };

    const onPointerMove = (e) => {
      if (!pointers.has(e.pointerId)) return;

      pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });

      if (pointers.size >= %minPointers && initialDistance !== null) {
        const geo = calculateGeometry();
        const scale = geo.distance / initialDistance;
        const rotation = geo.angle - initialAngle;

        // Check threshold
        if (!active) {
          const scaleDelta = Math.abs(scale - 1) * initialDistance;
          const rotationDelta = Math.abs(rotation) * initialDistance;
          if (scaleDelta > %threshold || rotationDelta > %threshold) {
            active = true;
            %yield true -> $active;
          }
        }

        if (active) {
          %yield scale -> $scale;
          %yield rotation -> $rotation;
          %yield geo.centerX -> $centerX;
          %yield geo.centerY -> $centerY;
          %yield geo.centerX - initialCenterX -> $deltaX;
          %yield geo.centerY - initialCenterY -> $deltaY;
        }
      }
    };

    const onPointerUp = (e) => {
      if (!pointers.has(e.pointerId)) return;

      el.releasePointerCapture(e.pointerId);
      pointers.delete(e.pointerId);

      if (pointers.size < %minPointers && active) {
        active = false;
        initialDistance = null;
        initialAngle = null;
        %yield false -> $active;
        %yield 1 -> $finalScale;
        %yield 0 -> $finalRotation;
      }
    };

    el.addEventListener('pointerdown', onPointerDown);
    el.addEventListener('pointermove', onPointerMove);
    el.addEventListener('pointerup', onPointerUp);
    el.addEventListener('pointercancel', onPointerUp);

    el.style.touchAction = 'none';

    %cleanup {
      for (const id of pointers.keys()) {
        el.releasePointerCapture(id);
      }
      el.removeEventListener('pointerdown', onPointerDown);
      el.removeEventListener('pointermove', onPointerMove);
      el.removeEventListener('pointerup', onPointerUp);
      el.removeEventListener('pointercancel', onPointerUp);
    }
  }

  %exports {
    $active: bool
    $scale: number           // Scale factor (1.0 = no change)
    $rotation: number        // Rotation in radians
    $centerX: number         // Center point X
    $centerY: number         // Center point Y
    $deltaX: number          // Pan X from start
    $deltaY: number          // Pan Y from start
    $finalScale: number      // Scale at gesture end
    $finalRotation: number   // Rotation at gesture end
  }
}
```

### 3. Rotation Gesture Primitive

```spacetime
%primitive rotation-gesture(
  &el,
  threshold: number = 0.1   // radians
) {
  %emit js {
    const pointers = new Map();
    let initialAngle = null;
    let active = false;
    let totalRotation = 0;

    const calculateAngle = () => {
      if (pointers.size < 2) return null;
      const [p1, p2] = Array.from(pointers.values());
      return Math.atan2(p2.y - p1.y, p2.x - p1.x);
    };

    const onPointerDown = (e) => {
      pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
      el.setPointerCapture(e.pointerId);

      if (pointers.size === 2) {
        initialAngle = calculateAngle();
        totalRotation = 0;
      }

      e.preventDefault();
    };

    const onPointerMove = (e) => {
      if (!pointers.has(e.pointerId)) return;

      const prevAngle = calculateAngle();
      pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
      const currentAngle = calculateAngle();

      if (pointers.size === 2 && prevAngle !== null && currentAngle !== null) {
        let delta = currentAngle - prevAngle;

        // Handle angle wrap-around
        if (delta > Math.PI) delta -= 2 * Math.PI;
        if (delta < -Math.PI) delta += 2 * Math.PI;

        totalRotation += delta;

        if (!active && Math.abs(totalRotation) > %threshold) {
          active = true;
          %yield true -> $active;
        }

        if (active) {
          %yield totalRotation -> $rotation;
          %yield totalRotation * (180 / Math.PI) -> $rotationDegrees;
          %yield delta -> $deltaRotation;
        }
      }
    };

    const onPointerUp = (e) => {
      if (!pointers.has(e.pointerId)) return;

      el.releasePointerCapture(e.pointerId);
      pointers.delete(e.pointerId);

      if (pointers.size < 2 && active) {
        active = false;
        %yield false -> $active;
        %yield totalRotation -> $finalRotation;
      }
    };

    el.addEventListener('pointerdown', onPointerDown);
    el.addEventListener('pointermove', onPointerMove);
    el.addEventListener('pointerup', onPointerUp);
    el.addEventListener('pointercancel', onPointerUp);

    el.style.touchAction = 'none';

    %cleanup {
      for (const id of pointers.keys()) {
        el.releasePointerCapture(id);
      }
      el.removeEventListener('pointerdown', onPointerDown);
      el.removeEventListener('pointermove', onPointerMove);
      el.removeEventListener('pointerup', onPointerUp);
      el.removeEventListener('pointercancel', onPointerUp);
    }
  }

  %exports {
    $active: bool
    $rotation: number          // Total rotation in radians
    $rotationDegrees: number   // Total rotation in degrees
    $deltaRotation: number     // Rotation delta since last move
    $finalRotation: number     // Rotation at gesture end
  }
}
```

### 4. Unified Multi-Touch Gesture Primitive

Combine all multi-touch gestures into one primitive:

```spacetime
%primitive multi-touch-gesture(
  &el,
  pinchEnabled: bool = true,
  rotateEnabled: bool = true,
  panEnabled: bool = true,
  scaleThreshold: number = 0.05,
  rotationThreshold: number = 0.1,
  panThreshold: number = 10
) {
  %emit js {
    const pointers = new Map();
    let initialState = null;
    let active = { pinch: false, rotate: false, pan: false };

    const getState = () => {
      if (pointers.size < 2) return null;
      const [p1, p2] = Array.from(pointers.values());
      return {
        centerX: (p1.x + p2.x) / 2,
        centerY: (p1.y + p2.y) / 2,
        distance: Math.hypot(p2.x - p1.x, p2.y - p1.y),
        angle: Math.atan2(p2.y - p1.y, p2.x - p1.x)
      };
    };

    const onPointerDown = (e) => {
      pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
      el.setPointerCapture(e.pointerId);

      if (pointers.size === 2) {
        initialState = getState();
        %yield true -> $twoFingerActive;
        %yield 1 -> $scale;
        %yield 0 -> $rotation;
        %yield initialState.centerX -> $centerX;
        %yield initialState.centerY -> $centerY;
      }

      %yield pointers.size -> $fingerCount;
      e.preventDefault();
    };

    const onPointerMove = (e) => {
      if (!pointers.has(e.pointerId)) return;

      pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });

      if (pointers.size >= 2 && initialState) {
        const current = getState();

        // Calculate transforms
        const scale = current.distance / initialState.distance;
        let rotation = current.angle - initialState.angle;
        if (rotation > Math.PI) rotation -= 2 * Math.PI;
        if (rotation < -Math.PI) rotation += 2 * Math.PI;

        const deltaX = current.centerX - initialState.centerX;
        const deltaY = current.centerY - initialState.centerY;

        // Check thresholds and activate
        if (%pinchEnabled && !active.pinch && Math.abs(scale - 1) > %scaleThreshold) {
          active.pinch = true;
          %yield true -> $pinchActive;
        }

        if (%rotateEnabled && !active.rotate && Math.abs(rotation) > %rotationThreshold) {
          active.rotate = true;
          %yield true -> $rotateActive;
        }

        if (%panEnabled && !active.pan && Math.hypot(deltaX, deltaY) > %panThreshold) {
          active.pan = true;
          %yield true -> $panActive;
        }

        // Update values
        if (active.pinch) %yield scale -> $scale;
        if (active.rotate) {
          %yield rotation -> $rotation;
          %yield rotation * (180 / Math.PI) -> $rotationDegrees;
        }
        if (active.pan) {
          %yield deltaX -> $deltaX;
          %yield deltaY -> $deltaY;
        }

        %yield current.centerX -> $centerX;
        %yield current.centerY -> $centerY;
      }
    };

    const onPointerUp = (e) => {
      if (!pointers.has(e.pointerId)) return;

      el.releasePointerCapture(e.pointerId);
      pointers.delete(e.pointerId);

      %yield pointers.size -> $fingerCount;

      if (pointers.size < 2) {
        // Gesture ended
        if (active.pinch || active.rotate || active.pan) {
          %yield false -> $twoFingerActive;
          %yield false -> $pinchActive;
          %yield false -> $rotateActive;
          %yield false -> $panActive;

          // Final values for momentum
          const finalState = getState() || initialState;
          if (active.pinch) %yield finalState.distance / initialState.distance -> $finalScale;
          if (active.rotate) %yield finalState.angle - initialState.angle -> $finalRotation;
        }

        active = { pinch: false, rotate: false, pan: false };
        initialState = null;
      }
    };

    el.addEventListener('pointerdown', onPointerDown);
    el.addEventListener('pointermove', onPointerMove);
    el.addEventListener('pointerup', onPointerUp);
    el.addEventListener('pointercancel', onPointerUp);

    el.style.touchAction = 'none';

    %cleanup {
      for (const id of pointers.keys()) {
        el.releasePointerCapture(id);
      }
      el.removeEventListener('pointerdown', onPointerDown);
      el.removeEventListener('pointermove', onPointerMove);
      el.removeEventListener('pointerup', onPointerUp);
      el.removeEventListener('pointercancel', onPointerUp);
    }
  }

  %exports {
    $fingerCount: number
    $twoFingerActive: bool

    // Pinch
    $pinchActive: bool
    $scale: number
    $finalScale: number

    // Rotation
    $rotateActive: bool
    $rotation: number
    $rotationDegrees: number
    $finalRotation: number

    // Pan
    $panActive: bool
    $deltaX: number
    $deltaY: number
    $centerX: number
    $centerY: number
  }
}
```

## Macro Examples

### Pinch-to-Zoom Macro

```spacetime
%macro pinch-zoom {
  %creates @pinch-zoom

  %form {
    @pinch-zoom(
      minScale: $minScale:number = 0.5,
      maxScale: $maxScale:number = 3,
      momentum: $momentum:bool = true
    ) {
      $customStates:states?
    }
  }

  %binds {
    pinch-gesture(&self) -> {
      $active,
      $scale,
      $centerX,
      $centerY,
      $finalScale
    }
  }

  %derives {
    // Clamp scale to min/max
    $clampedScale: Math.max($minScale, Math.min($maxScale, $scale))

    // Calculate transform origin based on pinch center
    $originX: $centerX
    $originY: $centerY
  }

  %states {
    idle { }
    pinching when $active {
      transform-origin: calc($originX * 1px) calc($originY * 1px);
    }
    $customStates?
  }

  %animates {
    scale: $clampedScale
  }
}
```

### Pan-Rotate-Zoom (Combined)

```spacetime
%macro pan-rotate-zoom {
  %creates @pan-rotate-zoom

  %form {
    @pan-rotate-zoom(
      minScale: $minScale:number = 0.1,
      maxScale: $maxScale:number = 10
    )
  }

  %binds {
    multi-touch-gesture(&self) -> {
      $fingerCount,
      $twoFingerActive,
      $scale,
      $rotation,
      $deltaX,
      $deltaY,
      $centerX,
      $centerY
    }

    gesture(&self) -> {
      $active as $panActive,
      $deltaX as $panDeltaX,
      $deltaY as $panDeltaY
    }
  }

  %derives {
    // Combine single and multi-touch pan
    $totalDeltaX: $twoFingerActive ? $deltaX : $panDeltaX
    $totalDeltaY: $twoFingerActive ? $deltaY : $panDeltaY

    // Clamp scale
    $clampedScale: Math.max($minScale, Math.min($maxScale, $scale))

    // Active if any gesture is active
    $isActive: $twoFingerActive || $panActive
  }

  %states {
    idle { cursor: grab; }
    active when $isActive { cursor: grabbing; }
  }

  %animates {
    translate-x: $totalDeltaX
    translate-y: $totalDeltaY
    scale: $clampedScale
    rotate: $rotation
  }
}
```

## Usage Examples

### Basic Pinch-to-Zoom

```spacetime
.zoomable-image {
  @pinch-zoom(minScale: 0.5, maxScale: 4)
}
```

### Image Viewer

```spacetime
.image-viewer {
  @pan-rotate-zoom(minScale: 0.1, maxScale: 10)
}

.image-viewer > img {
  transform: translate(var(--st-delta-x), var(--st-delta-y))
             scale(var(--st-scale))
             rotate(var(--st-rotation));
}
```

### Custom Multi-Touch Behavior

```spacetime
.canvas-container {
  @on gesture-start {
    // Save state for undo
    saveCanvasState();
  }

  @on gesture-end {
    // Apply final transform
    applyTransform($finalScale, $finalRotation);
  }
}
```

## Implementation Notes

### Touch Action CSS

All multi-touch primitives should set `touch-action: none` to prevent browser default behaviors (scrolling, zooming). This is already done in the primitive definitions.

### Pointer Capture

Using `setPointerCapture` ensures the element receives all pointer events even if the pointer moves outside the element bounds. This is crucial for reliable gesture tracking.

### Coordinate Systems

- Client coordinates (`e.clientX`, `e.clientY`) are relative to the viewport
- Consider adding support for local coordinates (relative to element) in the future

### Performance

- Pointer tracking uses Maps for O(1) lookup
- Calculations are done on every move event (60fps)
- Consider debouncing `%yield` for non-critical signals

## Testing Checklist

- [ ] Two-finger pinch detection
- [ ] Two-finger rotation detection
- [ ] Combined pinch and rotation
- [ ] Three+ finger gesture handling
- [ ] Pointer capture across element boundaries
- [ ] Cleanup releases all pointer captures
- [ ] Works with touch, pen, and mouse
- [ ] Momentum animation on release
- [ ] Scale/rotation constraints (min/max)
- [ ] Transform origin follows gesture center

## Files to Create/Modify

| File | Changes |
|------|---------|
| `stdlib/primitives/pointer-tracker.st` | New file - base pointer tracking |
| `stdlib/primitives/pinch-gesture.st` | New file - pinch detection |
| `stdlib/primitives/rotation-gesture.st` | New file - rotation detection |
| `stdlib/primitives/multi-touch-gesture.st` | New file - unified multi-touch |
| `stdlib/macros/pinch-zoom.st` | New file - pinch-zoom macro |
| `stdlib/macros/pan-rotate-zoom.st` | New file - combined gesture macro |
