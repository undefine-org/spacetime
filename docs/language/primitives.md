# Spacetime Primitives

Primitives are the foundation layer of Spacetime's macro system. They wrap browser APIs and expose reactive bindings that macros can compose.

## Overview

**Primitives are the only place JavaScript lives.** Everything else in Spacetime is declarative composition of primitives.

```
┌─────────────────────────────────────────┐
│  %macro (declarative composition)       │
│  No JS here — just wiring declarations  │
└──────────────────────┬──────────────────┘
                       │ uses
                       ▼
┌─────────────────────────────────────────┐
│  %primitive (JS wrapper)                │
│  The ONLY place JS exists               │
│  IntersectionObserver, scroll, etc.     │
└─────────────────────────────────────────┘
```

---

## Primitive Syntax

```st
%primitive name(&element, param: type = default, ...) {
  %emit js {
    // JavaScript code that creates observers, listeners, etc.
    // Use %param to splice compile-time values
    // Use %&element to reference the element
    // Use %yield to expose reactive values
  }

  %cleanup {
    // Cleanup code (remove listeners, disconnect observers)
  }

  %exports {
    // Declare the reactive bindings this primitive provides
    $binding: type
  }
}
```

---

## Primitive Constructs

### `%emit js { }`

Emits JavaScript code. Compile-time values are spliced in.

```st
%emit js {
  const observer = new IntersectionObserver(callback, {
    threshold: %threshold,  // splice param value
    root: %root             // splice param value
  });
  observer.observe(%&el);   // splice element reference
}
```

### Selector-init local names

For primitives on the selector-init path (class, attribute, or tag selectors), `%&el` expands to the selector-init callback's injected `el` parameter. Do not declare `const el`, `let el`, or `var el` inside `%emit js`: `const el = %&el` self-references the new binding and fails at runtime. Use a distinct local such as `const node = %&el`; reads such as `el.addEventListener(...)` remain valid.

### `%yield value -> $binding`

Exposes a value as a reactive binding. When the yielded value changes, all dependents update.

```st
%emit js {
  const handler = (entries) => {
    %yield entries[0].isIntersecting -> $visible;
    %yield entries[0].intersectionRatio -> $ratio;
  };
}
```

### `%cleanup { }`

Specifies cleanup code to run when the primitive is disposed.

```st
%cleanup {
  observer.disconnect();
  element.removeEventListener('scroll', handler);
}
```

### `%exports { }`

Declares the primitive's interface — what bindings it provides and their types.

```st
%exports {
  $visible: bool
  $ratio: number
  $rect: DOMRect
}
```

---

## Core Primitives

### intersection

Wraps `IntersectionObserver` for visibility detection.

```st
%primitive intersection(&el, threshold: number = 0.5, once: bool = false, root: element? = none, margin: string = "0px") {
  %emit js {
    const obs = new IntersectionObserver((entries) => {
      for (const entry of entries) {
        %yield entry.isIntersecting -> $visible;
        %yield entry.intersectionRatio -> $ratio;
        %yield entry.boundingClientRect -> $rect;

        if (%once && entry.isIntersecting) {
          obs.disconnect();
        }
      }
    }, {
      threshold: %threshold,
      root: %root,
      rootMargin: %margin
    });

    obs.observe(%&el);

    %cleanup { obs.disconnect(); }
  }

  %exports {
    $visible: bool      // true when element is visible
    $ratio: number      // 0-1 intersection ratio
    $rect: DOMRect      // bounding client rect
  }
}
```

**Usage in macros:**
```st
%binds {
  intersection(&self, threshold: 0.2, once: true) -> { $visible, $ratio }
}
```

---

### resize

Wraps `ResizeObserver` for element size changes.

```st
%primitive resize(&el) {
  %emit js {
    const obs = new ResizeObserver((entries) => {
      const entry = entries[0];
      const rect = entry.contentRect;

      %yield rect.width -> $width;
      %yield rect.height -> $height;
      %yield rect -> $rect;
      %yield entry.borderBoxSize[0] -> $borderBox;
    });

    obs.observe(%&el);

    %cleanup { obs.disconnect(); }
  }

  %exports {
    $width: number
    $height: number
    $rect: DOMRectReadOnly
    $borderBox: ResizeObserverSize
  }
}
```

---

### mutation

Wraps `MutationObserver` for DOM changes.

```st
%primitive mutation(&el, children: bool = true, attributes: bool = false, characterData: bool = false, subtree: bool = false) {
  %emit js {
    const obs = new MutationObserver((mutations) => {
      const added = [];
      const removed = [];
      const changed = [];

      for (const m of mutations) {
        if (m.type === 'childList') {
          added.push(...m.addedNodes);
          removed.push(...m.removedNodes);
        } else {
          changed.push(m);
        }
      }

      %yield added -> $added;
      %yield removed -> $removed;
      %yield changed -> $changed;
      %yield mutations -> $mutations;
    });

    obs.observe(%&el, {
      childList: %children,
      attributes: %attributes,
      characterData: %characterData,
      subtree: %subtree
    });

    %cleanup { obs.disconnect(); }
  }

  %exports {
    $added: Node[]
    $removed: Node[]
    $changed: MutationRecord[]
    $mutations: MutationRecord[]
  }
}
```

---

### scroll

Tracks scroll position with optional progress normalization.

```st
%primitive scroll(&el, axis: "x" | "y" | "both" = "y") {
  %emit js {
    const isWindow = %&el === window || %&el === document.documentElement;

    const getScrollInfo = () => {
      let scrollX, scrollY, maxX, maxY;

      if (isWindow) {
        scrollX = window.scrollX;
        scrollY = window.scrollY;
        maxX = document.documentElement.scrollWidth - window.innerWidth;
        maxY = document.documentElement.scrollHeight - window.innerHeight;
      } else {
        scrollX = %&el.scrollLeft;
        scrollY = %&el.scrollTop;
        maxX = %&el.scrollWidth - %&el.clientWidth;
        maxY = %&el.scrollHeight - %&el.clientHeight;
      }

      return { scrollX, scrollY, maxX, maxY };
    };

    const update = () => {
      const { scrollX, scrollY, maxX, maxY } = getScrollInfo();

      %yield scrollX -> $x;
      %yield scrollY -> $y;
      %yield maxX > 0 ? scrollX / maxX : 0 -> $progressX;
      %yield maxY > 0 ? scrollY / maxY : 0 -> $progressY;

      // Combined progress based on axis
      if (%axis === "x") {
        %yield maxX > 0 ? scrollX / maxX : 0 -> $progress;
      } else if (%axis === "y") {
        %yield maxY > 0 ? scrollY / maxY : 0 -> $progress;
      } else {
        %yield Math.max($progressX, $progressY) -> $progress;
      }
    };

    const target = isWindow ? window : %&el;
    target.addEventListener('scroll', update, { passive: true });
    update(); // Initial read

    %cleanup {
      target.removeEventListener('scroll', update);
    }
  }

  %exports {
    $x: number          // scroll left position
    $y: number          // scroll top position
    $progressX: number  // 0-1 horizontal progress
    $progressY: number  // 0-1 vertical progress
    $progress: number   // 0-1 progress on primary axis
  }
}
```

---

### pointer

Tracks pointer position and state.

```st
%primitive pointer(&el, events: string[] = ["move"]) {
  %emit js {
    const handler = (e) => {
      const rect = %&el.getBoundingClientRect();

      %yield e.clientX -> $x;
      %yield e.clientY -> $y;
      %yield e.clientX - rect.left -> $localX;
      %yield e.clientY - rect.top -> $localY;
      %yield (e.clientX - rect.left) / rect.width -> $normX;
      %yield (e.clientY - rect.top) / rect.height -> $normY;
      %yield e.pressure -> $pressure;
      %yield e.pointerType -> $pointerType;
    };

    const eventNames = %events.map(e => 'pointer' + e);
    for (const evt of eventNames) {
      %&el.addEventListener(evt, handler, { passive: true });
    }

    %cleanup {
      for (const evt of eventNames) {
        %&el.removeEventListener(evt, handler);
      }
    }
  }

  %exports {
    $x: number           // viewport X
    $y: number           // viewport Y
    $localX: number      // element-relative X
    $localY: number      // element-relative Y
    $normX: number       // normalized 0-1 X within element
    $normY: number       // normalized 0-1 Y within element
    $pressure: number    // pen pressure
    $pointerType: string // mouse, pen, touch
  }
}
```

---

### gesture

Tracks drag gestures with start/move/end states.

```st
%primitive gesture(&el, capture: bool = true) {
  %emit js {
    let state = 'idle';
    let startX = 0, startY = 0;
    let pointerId = null;

    const onDown = (e) => {
      state = 'active';
      startX = e.clientX;
      startY = e.clientY;
      pointerId = e.pointerId;

      if (%capture) {
        %&el.setPointerCapture(e.pointerId);
      }

      %yield true -> $active;
      %yield 0 -> $deltaX;
      %yield 0 -> $deltaY;
      %yield 0 -> $velocityX;
      %yield 0 -> $velocityY;
    };

    let lastX = 0, lastY = 0, lastTime = 0;

    const onMove = (e) => {
      if (state !== 'active') return;

      const dx = e.clientX - startX;
      const dy = e.clientY - startY;
      const now = performance.now();
      const dt = now - lastTime;

      %yield dx -> $deltaX;
      %yield dy -> $deltaY;

      if (dt > 0) {
        %yield (e.clientX - lastX) / dt * 1000 -> $velocityX;
        %yield (e.clientY - lastY) / dt * 1000 -> $velocityY;
      }

      lastX = e.clientX;
      lastY = e.clientY;
      lastTime = now;
    };

    const onUp = (e) => {
      if (e.pointerId !== pointerId) return;

      state = 'idle';
      %yield false -> $active;

      if (%capture) {
        %&el.releasePointerCapture(e.pointerId);
      }
    };

    %&el.addEventListener('pointerdown', onDown);
    %&el.addEventListener('pointermove', onMove);
    %&el.addEventListener('pointerup', onUp);
    %&el.addEventListener('pointercancel', onUp);

    %cleanup {
      %&el.removeEventListener('pointerdown', onDown);
      %&el.removeEventListener('pointermove', onMove);
      %&el.removeEventListener('pointerup', onUp);
      %&el.removeEventListener('pointercancel', onUp);
    }
  }

  %exports {
    $active: bool       // currently dragging
    $deltaX: number     // total X movement from start
    $deltaY: number     // total Y movement from start
    $velocityX: number  // X velocity in px/s
    $velocityY: number  // Y velocity in px/s
  }
}
```

---

### media

Wraps `matchMedia` for responsive queries.

```st
%primitive media(query: string) {
  %emit js {
    const mql = window.matchMedia(%query);

    const update = () => {
      %yield mql.matches -> $matches;
    };

    mql.addEventListener('change', update);
    update();

    %cleanup {
      mql.removeEventListener('change', update);
    }
  }

  %exports {
    $matches: bool
  }
}
```

**Usage:**
```st
%binds {
  media("(prefers-color-scheme: dark)") -> { $matches as $darkMode }
  media("(min-width: 768px)") -> { $matches as $isDesktop }
}
```

---

### tick

Wraps `requestAnimationFrame` for animation loops.

```st
%primitive tick(fps: number? = none) {
  %emit js {
    let running = true;
    let lastTime = performance.now();
    let frameCount = 0;
    const interval = %fps ? 1000 / %fps : 0;
    let accumulated = 0;

    const loop = (now) => {
      if (!running) return;

      const dt = now - lastTime;

      if (%fps) {
        accumulated += dt;
        if (accumulated >= interval) {
          accumulated -= interval;
          frameCount++;
          %yield now -> $t;
          %yield interval -> $dt;
          %yield frameCount -> $frame;
        }
      } else {
        frameCount++;
        %yield now -> $t;
        %yield dt -> $dt;
        %yield frameCount -> $frame;
      }

      lastTime = now;
      requestAnimationFrame(loop);
    };

    requestAnimationFrame(loop);

    %cleanup {
      running = false;
    }
  }

  %exports {
    $t: number      // current timestamp
    $dt: number     // delta time since last frame
    $frame: number  // frame count
  }
}
```

---

### fetch

Wraps the Fetch API for data loading.

```st
%primitive fetch(url: string, options: object = {}, cache: duration = 0, refetch: trigger? = none) {
  %emit js {
    let cached = null;
    let cacheTime = 0;
    const cacheDuration = %cache;

    const doFetch = async () => {
      const now = Date.now();

      // Check cache
      if (cached && cacheDuration > 0 && (now - cacheTime) < cacheDuration) {
        return;
      }

      %yield true -> $loading;
      %yield null -> $error;

      try {
        const response = await fetch(%url, %options);

        if (!response.ok) {
          throw new Error(`HTTP ${response.status}`);
        }

        const data = await response.json();
        cached = data;
        cacheTime = now;

        %yield data -> $data;
        %yield false -> $loading;
      } catch (e) {
        %yield e -> $error;
        %yield false -> $loading;
      }
    };

    doFetch();

    // If refetch trigger provided, set up listener
    if (%refetch) {
      %refetch.addEventListener('trigger', doFetch);
      %cleanup {
        %refetch.removeEventListener('trigger', doFetch);
      }
    }
  }

  %exports {
    $data: any        // the fetched data
    $loading: bool    // loading state
    $error: Error?    // error if failed
  }
}
```

---

## Testing Primitives

Primitives should be tested by mocking browser APIs:

```ts
// intersection.test.ts
describe('intersection primitive', () => {
  let mockObserver: jest.Mock;
  let observerCallback: Function;

  beforeEach(() => {
    mockObserver = jest.fn((cb) => {
      observerCallback = cb;
      return {
        observe: jest.fn(),
        disconnect: jest.fn()
      };
    });
    global.IntersectionObserver = mockObserver;
  });

  it('yields $visible when element becomes visible', () => {
    const bindings = createPrimitive('intersection', { el: div, threshold: 0.5 });

    // Simulate intersection
    observerCallback([{ isIntersecting: true, intersectionRatio: 0.8 }]);

    expect(bindings.$visible).toBe(true);
    expect(bindings.$ratio).toBe(0.8);
  });

  it('disconnects on cleanup', () => {
    const { cleanup, observer } = createPrimitive('intersection', { el: div });

    cleanup();

    expect(observer.disconnect).toHaveBeenCalled();
  });
});
```

---

## Best Practices

### 1. Keep primitives focused
Each primitive should wrap exactly one browser API. Don't combine multiple APIs in one primitive.

### 2. Always provide cleanup
Every `addEventListener`, `observe()`, or resource allocation must have corresponding cleanup.

### 3. Type your exports
Use `%exports` to declare the exact types of all bindings. This enables type checking in macros.

### 4. Use passive listeners
For scroll and pointer events, use `{ passive: true }` for better performance.

### 5. Handle edge cases
Check for null elements, window vs element contexts, and other boundary conditions.

---

## Primitive vs Macro

| Aspect | Primitive | Macro |
|--------|-----------|-------|
| Contains JS | Yes | No |
| Wraps browser API | Yes | No |
| Composable | Used by macros | Composes primitives |
| User-visible | Never | Via `@` directive |
| Tested with | API mocks | Declaration verification |
