# Spacetime

If CSS describes space, Spacetime adds the time dimensions to its declarative abilities.
Dimensions, plural: Spacetime allows you to superpose timelines and describe their interactions.
With its pattern system akin to Rust procedural macros, Spacetime lets you define complex behaviours from simple primitives.
Spacetime encourages good practices; with it, you can design fully dynamic, interactive websites with semantic HTML.


**A declarative animation and data binding system for the web.**

Spacetime lets you build animated, data-driven websites using a CSS-like syntax. Define animations, state machines, and data bindings in `.st` files, then compile them to optimized JavaScript and CSS.

---

## Features

### Animation System

- **Declarative Animations** - CSS-like syntax for complex animations
- **Scroll Triggers** - Animate elements as they scroll into view
- **State Machines** - Model UI states and transitions
- **Interaction Triggers** - Hover, click, focus, and more
- **Timeline Sequences** - Orchestrate multi-step animations
- **Easing Functions** - Built-in and custom easing curves
- **Stagger Effects** - Animate lists with automatic delays

### Data Binding System

- **Type-Safe Data** - Define schemas with compile-time validation
- **Multiple Sources** - Load from JSON, localStorage, or inline
- **Template Iteration** - Generate HTML from data automatically
- **Filters** - Transform data inline (currency, dates, etc.)
- **Computed Data** - Derive new data from existing sources
- **Cross-References** - Link related data with helper functions
- **State Integration** - Loading, empty, and error states built-in

---

## Quick Start

### Installation

```bash
# Install Spacetime
cargo install spacetime-cli

# Or build from source
git clone https://github.com/your-repo/spacetime.git
cd spacetime
cargo build --release
```

### Create a project

From the Spacetime repo root:

```bash
# Scaffolds projects/<name>/ when projects/ exists, otherwise ./<name>/.
cargo run -- init my-site

# Live dev server
cargo run -- serve projects/my-site/

# Build to projects/my-site/dist/ (gitignored)
cargo run -- build projects/my-site/
```

`projects/` is its own git repo, separate from the compiler. Build outputs
(`**/dist`) are gitignored.

### Your First Animation

Create a file `styles.st`:

```css
.hero {
    @scroll reveal(&fade-up) {
        opacity: 0 -> 1;
        translate-y: 40px -> 0;
        duration: 800ms;
        easing: &ease-out;
    }
}
```

Compile it:

```bash
spacetime compile styles.st
```

Include the generated files in your HTML:

```html
<link rel="stylesheet" href="styles.css">
<script src="styles.js"></script>

<div class="hero">
    <h1>Welcome to Spacetime</h1>
</div>
```

### Your First Data Binding

Define your data structure in `styles.st`:

```css
@type Product {
    id: string;
    name: string;
    price: number;
    imageUrl: url;
}

@data products: Product[] {
    src: "/data/products.json";
}

.product-list {
    @each(products) {
        template: "product-card";

        [slot="image"] {
            src: $.imageUrl;
            alt: $.name;
        }
        [slot="name"]: $.name;
        [slot="price"]: $.price | currency("$");
    }
}
```

Create your template in HTML:

```html
<template id="product-card">
    <div class="product-card">
        <img slot="image" src="" alt="">
        <h3 slot="name"></h3>
        <span slot="price" class="price"></span>
    </div>
</template>

<div class="product-list"></div>
```

Create `/data/products.json`:

```json
[
    {
        "id": "1",
        "name": "Ceramic Mug",
        "price": 24.99,
        "imageUrl": "/images/mug.jpg"
    }
]
```

Compile and run!

---

## Examples

### Scroll-Triggered Animation

```css
.cards-grid > .card {
    @scroll reveal(&quick-reveal) {
        opacity: 0 -> 1;
        translate-y: 60px -> 0;
        stagger: 0.1 first;
        easing: &ease-out-expo;
    }
}
```

### Hover Effect

```css
.button {
    background: #007bff;
    scale: 1;
    transition: background 150ms, scale 150ms;
}
.button:hover {
    background: #0056b3;
    scale: 1.05;
}
```

### Shopping Cart with localStorage

```css
@type CartItem {
    productId: string;
    quantity: number;
}

@data cart: CartItem[] {
    src: localStorage("cart");
    default: [];
}

@computed cartTotal: number {
    from: cart;
    reduce: (sum, item) => sum + priceFor(item.productId) * item.quantity;
    initial: 0;
}

.cart-total {
    @bind {
        text: cartTotal | currency("$");
    }
}
```

### Complex Timeline

```css
.hero {
    @timeline hero-entrance {
        @step(at: 0ms) {
            .hero__title {
                opacity: 0 -> 1;
                translate-y: -40px -> 0;
            }
        }

        @step(at: 300ms) {
            .hero__subtitle {
                opacity: 0 -> 1;
                translate-y: 20px -> 0;
            }
        }

        @step(at: 600ms) {
            .hero__cta {
                opacity: 0 -> 1;
                scale: 0.9 -> 1;
            }
        }
    }

    @load trigger(&hero-entrance);
}
```

---

## Documentation

### Getting Started

- **[Data Binding Guide](./docs/DATA_BINDING_GUIDE.md)** - Learn data binding step-by-step
- **[Migration Guide](./docs/MIGRATION.md)** - Migrate existing sites to data binding

### Reference

- **[API Reference](./docs/API_REFERENCE.md)** - Complete reference for all directives
- **[Data System Spec](./docs/DATA_SYSTEM.md)** - Technical specification
- **[Examples](./docs/DATA_SYSTEM_EXAMPLES.md)** - Real-world examples

### Runtime

- **[Runtime API](./public/runtime/README.md)** - JavaScript runtime documentation

---

## Real-World Examples

### zeystudios - Photography Print Shop

A photography portfolio with 23 prints, curated collections, and shopping cart.

**Before:** 400+ lines of repeated HTML
**After:** 10 lines of HTML + type-safe data binding

```css
@type Print {
    id: string;
    title: string;
    subtitle: string;
    image: url;
    prices: { S: number; M: number; L: number; };
}

@data prints: Print[] {
    src: "/data/prints.json";
}

.gallery {
    @each(prints) {
        template: "print-card";
        /* ... */
    }
}
```

### ikarchitecte - Architecture Firm

Portfolio site with projects, services, and responsive design.

**Before:** Manual HTML for each project
**After:** Data-driven with automatic animations

```css
@data projects: Project[] {
    src: "/data/projects.json";
}

.projects-grid {
    @each(projects) {
        template: "project-card";
        /* ... */
    }

    > project-card {
        @scroll reveal(&reveal) {
            opacity: 0 -> 1;
            stagger: 0.15 first;
        }
    }
}
```

### jallete - E-commerce Store

Nursing wear shop with product packs, testimonials, and FAQ.

**Features:**
- Nested data structures (packs with items)
- localStorage cart management
- Cross-referenced product lookups
- State-driven UI

---

## How It Works

### Compilation Pipeline

1. **Parse** - Parse `.st` files into an AST
2. **Validate** - Type-check data, verify templates exist
3. **Optimize** - Combine animations, remove duplicates
4. **Generate** - Output JavaScript and CSS

### Type Safety

Spacetime validates your data at compile time:

```css
@type Product {
    price: number;
}

@data products: Product[] {
    src: "/data/products.json";
}
```

If `products.json` has `"price": "10"` (string), compilation fails with:

```
Error: Type mismatch in products.json
  Expected: number
  Got: string
  Field: price
```

### Runtime Architecture

Generated code uses a lightweight runtime library:

- **DataLoader** - Fetches and caches data
- **TemplateEngine** - Clones and populates templates
- **Filters** - Data transformation functions
- **EventBus** - State machine integration

All runtime code is tree-shakeable and optimized.

---

## Browser Support

- Chrome/Edge 90+
- Firefox 88+
- Safari 14+

Requires ES6+ support (modules, template literals, async/await).

---

## Performance

### Compile Time

- Small sites (< 10 components): < 100ms
- Medium sites (< 50 components): < 500ms
- Large sites (< 200 components): < 2s

### Runtime

- Template instantiation: ~0.1ms per item
- Data loading: Network-bound (cached aggressively)
- Animation overhead: Minimal (uses native APIs)

### Bundle Size

- Runtime library: ~8KB gzipped
- Generated code: Scales linearly with features used

---

## Development

### Building from Source

```bash
git clone https://github.com/your-repo/spacetime.git
cd spacetime
cargo build --release
```

### Running Tests

```bash
cargo test
```

### Debug Logging

Enable trace-level logging with the `RUST_LOG` environment variable:

```bash
# All trace output
RUST_LOG=trace cargo run -- serve projects/<name>/

# Only macro expansion/resolution traces
RUST_LOG=spacetime::pipeline=trace,spacetime::metasystem=trace cargo run -- serve projects/<name>/
```

### Project Structure

```
spacetime/
  src/
    parser/        - .st file parsing
    codegen/       - Code generation
    validator/     - Type checking
    runtime/       - Rust runtime utilities
  public/
    runtime/       - JavaScript runtime library
  stdlib/          - Macros and primitives shipped with Spacetime
  docs/            - Documentation
  examples/        - Example .st files
  tests/fixtures/  - Compiler integration test fixtures
  projects/        - Your private Spacetime websites (gitignored;
                     separate git repo). `cargo run -- init <name>`
                     creates `projects/<name>/`.
```

---

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

---

## License

- **Compiler, tooling, docs, and everything else in this repository:** [GNU Affero General Public License v3.0 or later](https://www.gnu.org/licenses/agpl-3.0.html) — see [LICENSE](./LICENSE).
- **`stdlib/` (the Spacetime standard library):** [Eclipse Public License v2.0](https://www.eclipse.org/legal/epl-2.0/) — see [stdlib/LICENSE](./stdlib/LICENSE). Pages you write in Spacetime import stdlib; the weaker copyleft there keeps your own pages unencumbered.
- **Vendored third-party code** (`stdlib/**/vendor/`, `.patches/serde*`) remains under its own upstream license — see each vendored project.

Copyright (C) 2026 undefine and contributors.

---

## Acknowledgments

Spacetime is inspired by:

- CSS Animations and Transitions
- React's declarative approach
- Framer Motion's animation API
- TypeScript's type system

---

## Links

- **Documentation:** [./docs](./docs)
- **Examples:** [./examples](./examples)
- **Issues:** GitHub Issues
- **Discussions:** GitHub Discussions

---

Built with Rust. Designed for simplicity. Built for performance.
