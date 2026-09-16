# Spacetime Standard Library

The stdlib provides the core primitives and macros that power Spacetime's declarative animation and interaction system.

## Architecture

```
stdlib/
├── primitives/       # Low-level JS wrappers (only place JS exists)
│   ├── intersection.st   # IntersectionObserver
│   ├── resize.st         # ResizeObserver
│   ├── mutation.st       # MutationObserver
│   ├── scroll.st         # Scroll tracking
│   ├── pointer.st        # Pointer events
│   ├── gesture.st        # Drag gestures
│   ├── media.st          # Media queries
│   ├── tick.st           # Animation frames
│   └── fetch.st          # Data fetching
│
├── macros/           # Declarative compositions (no JS)
│   ├── fade-in.st        # Visibility animations
│   ├── on-event.st       # Event-triggered animations
│   ├── scroll-timeline.st # Scroll-driven animations
│   ├── drag.st           # Drag interactions
│   ├── each.st           # Data iteration
│   ├── type-data.st      # Type & data definitions
│   ├── websocket.st      # @socket (typed-union WebSocket)
│   ├── loop.st           # Looping animations
│   ├── responsive.st     # Media query responses
│   └── presets.st        # Reusable presets
│
└── index.st          # Main export file
```

## Symbol System

| Symbol | Domain | Phase | Description |
|--------|--------|-------|-------------|
| `$` | Data | Runtime | Reactive values |
| `&` | Elements | Runtime | DOM references |
| `@` | Directives | Runtime | User-facing behaviors |
| `%` | Macros | Compile | Library authors only |

**Users write `@`, `$`, `&`. Library authors write `%`.**

## Primitives

Primitives wrap browser APIs and expose reactive bindings:

```st
%primitive intersection(&el, threshold: 0.5) {
  %emit js { new IntersectionObserver(...) }
  %exports { $visible: bool, $ratio: number }
}
```

| Primitive | Wraps | Yields |
|-----------|-------|--------|
| `intersection` | IntersectionObserver | `$visible`, `$ratio`, `$rect` |
| `resize` | ResizeObserver | `$width`, `$height`, `$aspectRatio` |
| `mutation` | MutationObserver | `$added`, `$removed`, `$changed` |
| `scroll` | scroll events | `$x`, `$y`, `$progress`, `$velocity` |
| `pointer` | pointer events | `$x`, `$y`, `$normX`, `$normY`, `$isOver` |
| `gesture` | pointer capture | `$active`, `$deltaX`, `$deltaY`, `$velocity` |
| `media` | matchMedia | `$matches` |
| `tick` | rAF | `$t`, `$dt`, `$frame` |
| `fetch` | fetch API | `$data`, `$loading`, `$error` |

## Macros

Macros compose primitives declaratively:

```st
%macro fade-in {
  %form { @fade-in($duration:time = 600ms) }
  %binds { intersection(&self, once: true) -> { $visible } }
  %when $visible { %animates { opacity: 0 -> 1 } }
}
```

### Visibility & Scroll

| Macro | Creates | Description |
|-------|---------|-------------|
| `fade-in` | `@fade-in` | Fade in on visibility |
| `scroll-timeline` | `@scroll` | Scroll-driven animations |
| `parallax` | `@parallax` | Parallax scrolling |

### Interaction

| Macro | Creates | Description |
|-------|---------|-------------|
| `on-hover` | `@on &.hover` | Hover animations |
| `on-click` | `@on &.click` | Click animations |
| `on-visible` | `@on &.visible` | Visibility-triggered |
| `drag` | `@drag` | Draggable elements |
| `swipe` | `@swipe` | Swipe gestures |

### Data & State

| Macro | Creates | Description |
|-------|---------|-------------|
| `each` | `@each` | Data iteration |
| `type` | `@type` | Type definitions |
| `data` | `@data` | Data sources |
| `state` (enum/state.st) | `@state`, `@state-match` | State→CSS reflection (string gate + typed-union variants) |

### Animation

| Macro | Creates | Description |
|-------|---------|-------------|
| `loop` | `@loop` | Looping animations |
| `sequence` | `@sequence` | Sequential animations |
| `mouse` | `@mouse` | Mouse-driven animations |

### Responsive

| Macro | Creates | Description |
|-------|---------|-------------|
| `media-query` | `@media` | Media query responses |
| `dark-mode` | `@dark` | Dark mode styles |
| `reduced-motion` | `@reduced-motion` | Accessibility |

## Usage Examples

### Fade In on Scroll

```st
.hero {
  @fade-in(duration: 800ms, threshold: 0.2)
}
```

### Hover Animation

```st
.card {
  @on hover lift(300ms) {
    translate-y: 0 -> -8px
    box-shadow: none -> 0 4px 20px rgba(0,0,0,0.15)
  }
}
```

### Scroll Timeline

```st
.parallax-bg {
  @scroll bg-reveal {
    opacity: 0 -> 1
    translate-y: 50px -> 0
    range: 0.1 to 0.5
  }
}
```

### Draggable Element

```st
.sortable-item {
  @drag(axis: "y") {
    dragging { scale: 1.02; z-index: 100 }
    on-drop: handleReorder($deltaY)
  }
}
```

### Data Binding

```st
@type Product {
  id: string
  name: string
  price: number
}

@data products: Product[] {
  src: "/api/products"
  cache: 5m
}

.product-grid {
  @each($products) {
    > product-card {
      [slot="name"]: $.name
      [slot="price"]: $.price | currency("$")
    }

    :entering { opacity: 0 -> 1 }
    :exiting { opacity: 1 -> 0 }
  }
}
```

### Reactive State

```st
$modalOpen bool: false;

.modal {
  .is-open: $modalOpen;
  opacity: 0; pointer-events: none;
}
.modal.is-open { opacity: 1; pointer-events: auto; }

.modal-trigger { @on &.click { $modalOpen <- true; } }
.modal-close   { @on &.click { $modalOpen <- false; } }
```

## Extending the Stdlib

Create custom primitives:

```st
%primitive myApi(&el, option: string) {
  %emit js {
    // Your JavaScript here
    %yield someValue -> $binding;
  }
  %exports { $binding: type }
}
```

Create custom macros:

```st
%macro my-animation {
  %form { @my-animation($duration:time) }
  %binds { /* primitives */ }
  %animates { /* properties */ }
}
```

## Testing

Primitives are tested by mocking browser APIs.
Macros are tested by verifying their declaration graphs.

See `stdlib/primitives/__tests__/` and `stdlib/macros/__tests__/`.
