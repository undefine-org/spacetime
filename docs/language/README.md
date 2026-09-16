# Spacetime Language Reference

Spacetime is a declarative DSL for web animations, interactions, and reactive behaviors. This documentation covers the language design, including the macro system that powers the standard library.

## Quick Start

```st
// Fade in when element enters viewport
.hero {
  @fade-in(duration: 800ms)
}

// Hover animation
.card {
  @on &.hover lift(300ms) {
    translate-y: 0 -> -8px
  }
}

// Data binding
@data products: Product[] { src: "/api/products" }

.grid {
  @each($products) {
    > .item {
      [slot="name"]: $.name
    }
  }
}
```

## Documentation

### Core Concepts

| Document | Description |
|----------|-------------|
| [Symbol System](./symbol-system.md) | The four symbols: `$`, `&`, `@`, `%` |
| [Block Types](./block-types.md) | Keyframes, properties, templates, fields, states |
| [Data & Rendering](./data-and-rendering.md) | `@data` input, `@each`, inline HTML, SSG |
| [Reactive Output](./reactive-output.md) | `@host`, `@data signal`/`stream`, `receive`, `@handle` |
| [Module System](./module-system.md) | `@use`, namespaces, aliasing, `only`/`hiding`, `MODULE.st` |
| [Literate Spacetime](./literate.md) | `.st.md` — a Markdown document whose ```st fences ARE the program |
| [Film](./film.st.md) | **Golden reference** for the film surface: positioned stops, `@post`, camera forms, score-driven 3D, `@scatter`, audio on the score, transitions, shots. Written affirmatively; `— status —` blocks say what ships. Build order: `!tasks/plans/PLAN-150` |

### Macro System (for library authors)

| Document | Description |
|----------|-------------|
| [Primitives](./primitives.md) | `%primitive` — wrapping browser APIs |
| [Macros](./macros.md) | `%macro` — declarative compositions |
| [Form Syntax](./form-syntax.md) | `%form` — pattern matching for syntax |
| [Syntax Migrations](./migrations.md) | `%migration` — old syntax → new syntax as stdlib data; dated waves, `@version`, dev-dock pill |
| [Capture Types](./capture-types.md) | `%capture_type` — custom pattern types |
| [Parser Extension](./parser-extension.md) | Grammar specification for `%` constructs |

## Symbol Reference

### For End Users

| Symbol | Domain | Example |
|--------|--------|---------|
| `$` | Data bindings | `$products`, `$.name`, `$_index` |
| `&` | Element refs | `&hero`, `&container.rect.width` |
| `@` | Directives | `@fade-in`, `@scroll`, `@each` |

### For Library Authors

| Symbol | Domain | Example |
|--------|--------|---------|
| `%` | Macros | `%primitive`, `%macro`, `%binds` |

## Directive Categories

### Animation

| Directive | Description |
|-----------|-------------|
| `@fade-in` | Visibility-triggered fade |
| `@scroll` | Scroll-driven timeline |
| `@parallax` | Parallax scrolling |
| `@loop` | Looping animation |
| `@on &.hover/click/visible` | Event-triggered |

### Data

| Directive | Description |
|-----------|-------------|
| `@type` | Type definition (product **or** tagged sum `A(T) \| B(U)`) |
| `@data` | Data source (input) / `@data signal`/`stream` (reactive output) |
| `@host` | Transport+config binding for reactive output |
| `@handle` | Consume a signal's replies in a scope |
| `@computed` | Derived data |
| `@each` | Data iteration |

### Interaction

| Directive | Description |
|-----------|-------------|
| `@drag` | Draggable element |
| `@swipe` | Swipe gestures |

> State is handled via reactive `$`-signals and reactive classes (for example, `.is-loading: $status == "loading";`), not by an explicit state-machine directive.

### Responsive

| Directive | Description |
|-----------|-------------|
| `@media` | Media query |
| `@dark` / `@light` | Color scheme |
| `@breakpoint` | Named breakpoints |
| `@reduced-motion` | Accessibility |

## Architecture

```
User Code (.st)        Stdlib (.st)           Runtime (JS)
─────────────────      ─────────────          ────────────
                       %primitive
@fade-in(800ms)  ───>    ↓
                       %macro fade-in  ───>   IntersectionObserver
                         %binds                Web Animations API
                         %animates             Reactive bindings
```

**Key insight:** Users write `@`, library authors write `%`. The `%` layer compiles away — it's pure declarations that the compiler interprets.

## See Also

- [stdlib/README.md](../../stdlib/README.md) — Standard library reference
- [examples/](../../examples/) — Complete usage examples
