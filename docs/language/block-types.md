# Spacetime Block Types

Block types define what kind of content can appear inside `{ }` blocks in Spacetime. The parser uses block type annotations to correctly parse and validate block contents.

## Why Block Types?

Different directives expect different kinds of content:

```st
// Keyframes — property animations
@scroll hero {
  opacity: 0 -> 1
  translate-y: 50px -> 0
}

// Properties — static CSS
@state(when: "editing") {
  outline: 2px dashed blue
  background: #ffffcc
}

// Template — child elements
@each($products) {
  > product-card {
    [slot="name"]: $.name
  }
}

// Fields — type definitions
@type Product {
  id: string
  name: string
  price: number
}
```

Block types tell the compiler what to expect, enabling proper parsing and type checking.

---

## Block Type Annotations

In `%form` declarations, use `$name:blocktype` to annotate captured blocks:

```st
%form {
  @scroll $name:ident {
    $body:keyframes      // expects keyframe declarations
    range: $r:range?
  }
}

%form {
  @state(when: $condition:string) {
    $styles:properties   // expects CSS properties
  }
}

%form {
  @each($source:binding) {
    $template:template   // expects child elements
  }
}
```

---

## The Six Block Types

### 1. `keyframes`

Property animations with transition syntax.

```st
// Example content
opacity: 0 -> 1
translate-y: 50px -> 0
scale: 0.9 -> 1 -> 1.05 -> 1
color: red -> blue in oklch

// With explicit keyframes
opacity: {
  0%: 0
  50%: 1
  100%: 0.8
}

// With inline easing
translate-y: 50px -> 0 with spring(300, 20)
```

**Syntax elements:**
- `property: start -> end` — two-value transition
- `property: a -> b -> c` — multi-step transition
- `property: { 0%: v1; 50%: v2; 100%: v3 }` — explicit keyframes
- `... with easing` — inline easing
- `... in colorspace` — color interpolation space

**Used by:** `@scroll`, `@on hover/click/visible`, `@loop`, `:entering`, `:exiting`

---

### 2. `properties`

Static CSS property declarations.

```st
// Example content
outline: 2px dashed blue
background: #ffffcc
cursor: text
user-select: text
box-shadow: 0 4px 12px rgba(0,0,0,0.15)
```

**Syntax elements:**
- Standard CSS `property: value` pairs
- No transition syntax (`->`)
- Can include CSS functions, colors, lengths, etc.

**Used by:** `@state`, `:host`, state machine states

---

### 3. `template`

Child element declarations with data binding.

```st
// Example content
> product-card {
  [slot="name"]: $.name
  [slot="price"]: $.price | currency("$")
  [slot="image"]: $.thumbnail

  @fade-in(duration: 300ms)
}

> .empty-state @if $_count == 0 {
  "No products found"
}
```

**Syntax elements:**
- `> selector { }` — child element scope
- `[slot="name"]: expression` — slot binding
- `$.property` — data binding
- `| filter` — value transformation
- Nested directives allowed

**Used by:** `@each`, `@for`, `@if`, component templates

---

### 4. `fields`

Type field declarations.

```st
// Example content
id: string
name: string
price: number
tags: string[]
inStock: bool
description?: string    // optional field
metadata: {             // nested object
  createdAt: date
  updatedAt: date
}
category: "electronics" | "clothing" | "food"  // union type
```

**Syntax elements:**
- `name: type` — required field
- `name?: type` — optional field
- `type[]` — array type
- `{ ... }` — nested object type
- `"a" | "b"` — union/enum type

**Built-in types:** `string`, `number`, `bool`, `date`, `url`, `any`

**Used by:** `@type`

---

### 5. `states`

State definitions used by reactive interaction directives.

```st
// Example content
idle {
  cursor: grab
  background: white
}

dragging when $active {
  cursor: grabbing
  background: #f0f0f0
  user-select: none
}

disabled when $isDisabled {
  opacity: 0.5
  pointer-events: none
}
```

**Syntax elements:**
- `stateName { properties }` — state with CSS
- `stateName when $condition { }` — conditional state
- Can include nested animations

**Used by:** `@drag`, `@state`, custom interaction macros

For purely reactive UIs, prefer a `$`-signal and reactive classes instead of an explicit state machine:

```st
$status <- "loading"

.gallery {
  .is-loading: $status == "loading";
}

.gallery.is-loading {
  opacity: 0.7
}
```


---

### 6. `expr`

Single expression for computed values.

```st
// Example content
$.price * 0.9
$visible ? 1 : 0
clamp(0, $progress * 100, 100)
&container.rect.width - 20
```

**Syntax elements:**
- Arithmetic: `+`, `-`, `*`, `/`
- Ternary: `condition ? a : b`
- Function calls: `clamp()`, `min()`, `max()`
- Property access: `$.field`, `&el.facet.prop`
- Comparisons: `==`, `!=`, `<`, `>`, `<=`, `>=`

**Used by:** Parameter defaults, computed properties, `@let`, inline expressions

---

## Block Type Composition

Some blocks can contain other block types:

### `keyframes` + metadata

```st
@scroll hero {
  // Keyframes
  opacity: 0 -> 1
  translate-y: 50px -> 0

  // Metadata (not keyframes, but parsed specially)
  range: 0.1 to 0.6
  easing: ease-out
}
```

### `template` + directives

```st
@each($products) {
  // Template content
  > product-card {
    [slot="name"]: $.name

    // Nested directive (contains keyframes)
    @on &.hover lift(200ms) {
      translate-y: 0 -> -4px
    }
  }

  // Lifecycle animations (keyframes blocks)
  :entering {
    opacity: 0 -> 1
  }

  :exiting {
    opacity: 1 -> 0
  }
}
```

### `states` + properties + keyframes

```st
@drag {
  // States block
  idle {
    cursor: grab           // properties
  }

  dragging when $active {
    cursor: grabbing       // properties
    scale: 1 -> 1.02       // can include animations
  }
}
```

---

## Parsing Strategy

The parser determines block type from context:

1. **Explicit annotation** in macro's `%form`:
   ```st
   %form { @foo { $body:keyframes } }
   ```

2. **Directive name** for built-in directives:
   ```st
   @type Product { ... }  // always 'fields'
   @state(...) { ... }    // always 'properties'
   ```

3. **Content inspection** for ambiguous cases:
   - Contains `->` → likely `keyframes`
   - Contains `> selector` → likely `template`
   - Contains `name: type` pattern → likely `fields`

---

## Type Checking

Block types enable compile-time validation:

```st
// ERROR: 'opacity: 0 -> 1' is keyframes syntax, not allowed in properties
@state(when: "active") {
  opacity: 0 -> 1  // Should be: opacity: 1
}

// ERROR: missing required field type
@type Product {
  name           // Should be: name: string
}

// ERROR: invalid transition syntax in template
@each($items) {
  > .item {
    opacity: 0 -> 1  // Should use @fade-in directive instead
  }
}
```

---

## Custom Block Types

Library authors can define custom block types for domain-specific content:

```st
%blocktype validation {
  // Define valid syntax patterns
  %allows {
    $field:ident : $rules:validation-rules
  }

  // Example valid content:
  // email: required, email, max(255)
  // age: required, number, min(0), max(150)
}

%macro form-validation {
  %form {
    @validate {
      $rules:validation   // uses custom block type
    }
  }
}
```

---

## Summary

| Block Type | Contains | Example Directive |
|------------|----------|-------------------|
| `keyframes` | Property animations | `@scroll`, `@on &.hover` |
| `properties` | Static CSS | `@state`, `:host` |
| `template` | Child elements + bindings | `@each`, `@for` |
| `fields` | Type definitions | `@type` |
| `states` | Reactive state definitions | `@drag`, `@state` |
| `expr` | Single expression | `@let`, defaults |

Block types are the grammar within blocks — they tell the parser what syntax to expect and enable rich validation and tooling support.
