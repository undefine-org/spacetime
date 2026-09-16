# Spacetime Symbol System

Spacetime uses four primary symbols to create a clear separation between compile-time metaprogramming and runtime behavior.

## The Four Symbols

| Symbol | Domain | Phase | Visibility |
|--------|--------|-------|------------|
| `$` | Data bindings | Runtime | End users |
| `&` | Element references | Runtime | End users |
| `@` | Directives | Runtime | End users |
| `%` | Macros/Meta | Compile-time | Library authors |

## Symbol Details

### `$` — Data Bindings

The dollar sign represents reactive data values at runtime.

```st
// Data context
$products          // a data source
$.name             // property access on current item
$.price | currency // piped through formatter

// Loop metadata
$._index           // current index
$._first           // is first item
$._last            // is last item
$._count           // total count

// Scoped state (inside selector blocks)
.counter {
    $count number: 0;
}
```

**Key principle:** `$` values are reactive. When they change, dependent properties update automatically.

### `&` — Element References

The ampersand represents DOM element references.

```st
// Element binding (inside selector blocks)
.page {
    &hero .hero-section;
    &cards .card;  // NodeList
}

// Element facets
&hero.rect.top        // bounding rect
&hero.rect.width
&hero.scroll.y        // scroll position
&hero.scroll.progress // normalized 0-1

// Self-reference in scopes
.card {
  &self              // refers to .card element
}
```

**Key principle:** `&` references are live bindings to DOM elements with typed facet access.

### `@` — Directives

The at-sign represents behavioral directives that end users write.

```st
// Animation directives
@fade-in(duration: 800ms)
@scroll hero-reveal { opacity: 0 -> 1 }
@on &.hover lift(300ms) { translate-y: 0 -> -8px }

// Data directives
@type Product { id: string; name: string }
@data products: Product[] { src: "/api/products" }
@each($products) { ... }

// State directives
@state(when: "editing") { outline: 2px dashed }
```

**Key principle:** Users compose `@` directives to build interactive experiences. They never need to understand the underlying implementation.

### `%` — Macros (Compile-time)

The percent sign is the metaprogramming symbol, used only by library authors.

```st
// Define a primitive (wraps JS)
%primitive intersection(&el, threshold: 0.5) {
  %emit js { new IntersectionObserver(...) }
  %exports { $visible: bool, $ratio: number }
}

// Define a macro (composes primitives)
%macro fade-in {
  %form { @fade-in($duration:time = 600ms) }
  %binds { intersection(&self) -> { $visible } }
  %when $visible { ... }
}
```

**Key principle:** `%` constructs are erased after compilation. They define the `@` directives that users write, but users never see or write `%` syntax.

---

## Visibility Rules

```
┌─────────────────────────────────────────────┐
│  % METALANGUAGE                             │
│  Visible to: stdlib authors, library authors│
│  Erased at: compile time                    │
└──────────────────────┬──────────────────────┘
                       │ defines
                       ▼
┌─────────────────────────────────────────────┐
│  @ DIRECTIVES                               │
│  Visible to: all users                      │
│  Executes at: runtime                       │
└──────────────────────┬──────────────────────┘
                       │ uses
                       ▼
┌─────────────────────────────────────────────┐
│  $ DATA  +  & ELEMENTS                      │
│  Visible to: all users                      │
│  Reactive at: runtime                       │
└─────────────────────────────────────────────┘
```

### Who Writes What

| Role | Writes | Example |
|------|--------|---------|
| **End User** | `@`, `$`, `&` | `@fade-in(duration: 800ms)` |
| **Component Author** | `@`, `$`, `&` | Building reusable components |
| **Library Author** | `%`, `@`, `$`, `&` | Creating new directives |
| **Stdlib Author** | `%` | Defining core primitives |

---

## Compile-Time vs Runtime

### Compile-Time (`%`)

- Macro expansion
- Syntax transformation
- Type checking
- Declaration graph construction
- **Result:** Clean runtime code with no `%` artifacts

### Runtime (`@`, `$`, `&`)

- Observer creation (IntersectionObserver, etc.)
- Event binding
- Reactive updates
- Animation execution
- DOM manipulation

---

## Design Rationale

### Why Four Symbols?

1. **Clarity of intent** — Each symbol immediately tells you what domain you're in
2. **Phase separation** — `%` is compile-time, others are runtime
3. **Minimal collision** — Chosen to avoid CSS/JS conflicts
4. **Memorable mnemonics:**
   - `$` = "values" (like shell variables)
   - `&` = "references" (like C/Rust references)
   - `@` = "at this element, do this" (like decorators)
   - `%` = "template/placeholder" (like printf)

### Why Hide `%` from Users?

The metaprogramming layer adds cognitive overhead. By restricting `%` to library authors:

1. **Simpler mental model** for end users
2. **Cleaner syntax** in application code
3. **Better tooling** — IDEs can provide richer support for `@` directives
4. **Easier debugging** — no macro-expanded code to trace through

### The Rust Analogy

| Rust | Spacetime |
|------|-----------|
| `macro_rules!` / `#[proc_macro]` | `%primitive` / `%macro` |
| `foo!()` invocation | `@foo()` invocation |
| Users write clean Rust | Users write clean `@`/`$`/`&` |

But unlike Rust's procedural macros, Spacetime's macros are **declarative** — they describe wiring, not token manipulation.

---

## Examples

### User Code (Clean)

```st
.hero {
  @fade-in(duration: 800ms, threshold: 0.2)

  > .title {
    @on &.hover title-glow(300ms) {
      text-shadow: none -> 0 0 20px gold
    }
  }
}

.product-grid {
  @each($products) {
    > product-card {
      [slot="name"]: $.name
      [slot="price"]: $.price | currency("$")
    }
  }
}

.draggable {
  @drag(axis: "both", bounds: &container)
}
```

### Library Code (Meta)

```st
%primitive intersection(&el, threshold: 0.5, once: false) {
  %emit js {
    const obs = new IntersectionObserver((entries) => {
      for (const e of entries) {
        %yield e.isIntersecting -> $visible;
        %yield e.intersectionRatio -> $ratio;
        if (%once && e.isIntersecting) obs.disconnect();
      }
    }, { threshold: %threshold });
    obs.observe(%&el);
    %cleanup { obs.disconnect(); }
  }
  %exports { $visible: bool, $ratio: number }
}

%macro fade-in {
  %creates @fade-in

  %form {
    @fade-in($duration:time = 600ms, threshold: $th:number = 0.1)
  }

  %binds {
    intersection(&self, threshold: $th, once: true) -> { $visible }
  }

  %when $visible {
    %animates {
      opacity: 0 -> 1
    }
    %timing {
      duration: $duration
      easing: ease-out
    }
  }
}
```

---

## Summary

| Symbol | Mnemonic | Phase | User Sees |
|--------|----------|-------|-----------|
| `$` | Values | Runtime | Yes |
| `&` | References | Runtime | Yes |
| `@` | Directives | Runtime | Yes |
| `%` | Templates | Compile | No |

The symbol system creates a clean separation of concerns: users work with a simple, powerful DSL (`@`, `$`, `&`), while library authors have full metaprogramming capabilities (`%`) to extend the language.
