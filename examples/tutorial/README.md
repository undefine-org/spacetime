# Learn Spacetime - Interactive Tutorial

An interactive tutorial that teaches Spacetime by example. **The tutorial itself is built with Spacetime** - each section demonstrates the concept it teaches.

## Quick Start

```bash
# From the spacetime-testing directory
spacetime serve examples/tutorial
```

Then open http://localhost:3000 in your browser.

## What You'll Learn

| Section | Concept | Technique |
|---------|---------|-----------|
| 1. Welcome | Page load animations | `@load` |
| 2. Your First Animation | Scroll-driven reveals | `@scroll` |
| 3. Hover Effects | Interactive feedback | `@on &.hover` |
| 4. State Machines | UI state management | `@state_machine` |
| 5. Data Binding | Reactive signals | `$signals`, `@bind` |
| 6. Composition | Combining techniques | All together |

## The Meta-Learning Approach

This tutorial is unique: **every section uses the exact technique it teaches**.

- The welcome section fades in using `@load`
- The scroll section reveals content using `@scroll`
- The hover section has interactive cards using `@on &.hover`
- And so on...

You learn by experiencing the animation, then seeing the code that creates it.

## File Structure

```
examples/tutorial/
  index.html      # Tutorial webpage with all sections
  tutorial.st     # Spacetime styles and animations
  README.md       # This file
```

## Key Spacetime Concepts

### @load - Page Load Animations

```spacetime
.hero {
    @load entrance(800ms) {
        opacity: 0 -> 1;
        translateY: -20px -> 0;
    }
}
```

### @scroll - Scroll-Driven Animations

```spacetime
.card {
    @scroll reveal(600ms, threshold: 0.2) {
        opacity: 0 -> 1;
        translateY: 40px -> 0;
        stagger: 0.1 first;
    }
}
```

### @on hover - Hover Interactions

```spacetime
.button {
    @on hover lift(300ms, easing: spring(300, 20)) {
        translateY: 0 -> -4px;
        box-shadow: none -> 0 8px 24px rgba(0,0,0,0.15);
    }
}
```

### Reactive State Management

```spacetime
$on bool: false;

.toggle {
    .is-on: $on;
    background: gray;
    @on &.click { $on <- !$on; }
}
.toggle.is-on { background: green; }
```

### $signals - Reactive Data Binding

```spacetime
body {
    $count number: 0;
}

.display {
    @bind { text: $count; }
}

.increment {
    @on &.click { $count <- $count + 1; }
}
```

## Tips for Learning

1. **Read the code comments** in `tutorial.st` - they explain each technique
2. **Experiment** by modifying values and seeing what changes
3. **Inspect elements** in DevTools to see the generated CSS
4. **Build something** - the best way to learn is by doing

## Next Steps

After completing this tutorial:

1. Browse other examples in `examples/`
2. Check out the stdlib macros in `stdlib/`
3. Build your own project with Spacetime
