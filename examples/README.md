# Spacetime Examples

This directory contains example `.st` files demonstrating various Spacetime features.

## Examples

### [hero-section.st](./hero-section.st)
Scroll-driven animations, parallax effects, and visibility triggers.
- `@fade-in` for entrance animations
- `@scroll` for scroll-linked timelines
- `@parallax` for background effects
- `@after` for sequenced animations
- `@loop` for continuous animations

### [product-grid.st](./product-grid.st)
Data binding, iteration, and interactive cards.
- `@type` for type definitions
- `@data` for remote data sources
- `@computed` for derived data
- `@each` for data iteration with enter/exit animations
- Hover effects and loading states

### [drag-and-drop.st](./drag-and-drop.st)
Gesture handling and complex interactions.
- `@drag` for draggable elements
- `@swipe` for swipe gestures
- Custom state styling during drag
- Slider and sortable list patterns
- Tinder-style card swiping

### [modal-dialog.st](./modal-dialog.st)
State machines, focus management, and accessibility.
- `@state_machine` for lifecycle management
- `@transition` for state changes
- `@mutate` for DOM side effects
- Focus trapping and body scroll lock
- Async transitions with loading states
- Toast notifications

### [responsive-layout.st](./responsive-layout.st)
Media queries, dark mode, and accessibility.
- `@dark` / `@light` for color scheme
- `@breakpoint` for responsive design
- `@media` for custom queries
- `@reduced-motion` for accessibility
- `@portrait` / `@landscape` for orientation
- `@print` for print styles

## Key Patterns

### Visibility Animation
```st
.element {
  @fade-in(duration: 800ms, threshold: 0.2)
}
```

### Hover Effect
```st
.card {
  @on hover lift(300ms) {
    translate-y: 0 -> -8px
    box-shadow: none -> 0 8px 24px rgba(0,0,0,0.15)
  }
}
```

### Scroll Timeline
```st
.section {
  @scroll reveal {
    opacity: 0 -> 1
    range: 0.1 to 0.5
  }
}
```

### Data Binding
```st
@data items: Item[] { src: "/api/items" }

.list {
  @each($items) {
    > .item {
      [slot="name"]: $.name
    }
  }
}
```

### Reactive State
```st
$modalOpen bool: false;

.modal {
  .is-open: $modalOpen;
  opacity: 0;
}
.modal.is-open { opacity: 1; }

.modal-trigger { @on &.click { $modalOpen <- true; } }
.modal-close   { @on &.click { $modalOpen <- false; } }
```

### Responsive Design
```st
.grid {
  @breakpoint(sm) { grid-template-columns: 1fr }
  @breakpoint(md) { grid-template-columns: repeat(2, 1fr) }
  @breakpoint(lg) { grid-template-columns: repeat(3, 1fr) }
}
```

## Note

These examples use the end-user syntax (`@`, `$`, `&`). The `%` macro syntax is only used by library authors in the [stdlib](../stdlib/).
