# Spacetime Macros

Macros are declarative compositions that build user-facing `@` directives from primitives. Unlike procedural macros that manipulate tokens, Spacetime macros declare *what* should be wired, and the compiler figures out *how*.

## The Declarative Difference

| Procedural Macros (Rust) | Declarative Macros (Spacetime) |
|--------------------------|--------------------------------|
| Code that writes code | Patterns that describe wiring |
| Manipulate tokens | Declare bindings |
| Compiler runs your code | Compiler interprets declarations |
| Opaque transformation | Analyzable structure |

---

## Macro Syntax

```st
%macro name {
  %creates @directive-name    // What @ directive this becomes

  %form {
    // Syntax pattern with typed captures
  }

  %binds {
    // Connect to primitives
  }

  %derives {
    // Computed values
  }

  %when condition {
    // Conditional activation
  }

  %animates {
    // Property targets
  }

  %timeline(name, duration) {
    // Animation composition
  }

  %states {
    // State machine integration
  }

  %registers type/binding {
    // Type or binding registration
  }

  %transforms {
    // Expression rewriting rules
  }
}
```

---

## Macro Constructs

### `%creates @name`

Declares what `@` directive this macro manifests as.

```st
%macro fade-in {
  %creates @fade-in

  // Now users can write: @fade-in(duration: 800ms)
}
```

### `%form { }`

Declares the syntactic shape the directive accepts. See [Form Syntax](./form-syntax.md) for details.

```st
%form {
  @fade-in($duration:time = 600ms, threshold: $th:number = 0.1) {
    $body:keyframes?    // optional keyframe block
  }
}
```

The `%form` declaration:
- Specifies parameter names, types, and defaults
- Captures block contents with type annotations
- Enables the compiler to parse and validate user code

### `%binds { }`

Connects to primitives and captures their reactive outputs.

```st
%binds {
  intersection(&self, threshold: $th, once: true) -> { $visible, $ratio }
  scroll(&container) -> { $progress }
  pointer(&self, events: ["enter", "leave"]) -> { $x, $y }
}
```

Syntax: `primitive(args) -> { $binding1, $binding2, ... }`

You can rename bindings:
```st
%binds {
  intersection(&self) -> { $visible as $isShown, $ratio as $amount }
}
```

### `%derives { }`

Computes derived values from primitive bindings.

```st
%derives {
  $offset: $progress * 100px
  $opacity: $visible ? 1 : 0
  $scale: 1 + ($ratio * 0.2)
  $clamped: clamp(0, $deltaX, &container.rect.width)
}
```

Derived values are reactive — they update when their dependencies change.

### `%when condition { }`

Activates a block when a condition is true.

```st
%when $visible {
  %animates {
    opacity: 0 -> 1
  }
}

%when $active {
  %states {
    dragging { cursor: grabbing }
  }
}
```

`%when` can be nested:
```st
%when $visible {
  %when $isDesktop {
    %animates { transform: scale(1.1) }
  }
}
```

### `%animates { }` / `%animates on &element { }`

Declares property animations.

```st
%animates {
  opacity: 0 -> 1
  translate-y: 20px -> 0
}

// Animate a different element
%animates on &target {
  scale: 1 -> 1.1
}
```

Animation values use Spacetime's keyframe syntax:
- `0 -> 1` — transition from 0 to 1
- `0 -> 1 -> 0.5` — multi-step
- `{ 0%: 0; 50%: 1; 100%: 0.5 }` — explicit keyframes
- `0 -> 1 with ease-out` — inline easing

### `%timeline($name, $duration) { }`

Creates a named timeline for complex animations.

```st
%timeline($name, $duration) {
  $body                    // splice captured keyframes
  easing: $easing ?? ease-out
  fill: forwards
}
```

For scroll-driven timelines:
```st
%timeline($name) {
  driver: $progress        // bind to scroll progress
  range: $start to $end
  $body
}
```

### `%states { }`

Integrates with Spacetime's state machine system.

```st
%states {
  idle {
    cursor: grab
  }
  dragging when $active {
    cursor: grabbing
    user-select: none
  }
  $customStates?           // splice user-provided states
}
```

State conditions use `when` clauses bound to reactive values.

### `%registers type/binding { }`

Registers types or bindings for use elsewhere.

```st
// Register a type
%registers type($name) {
  $fields
}

// Register a data binding
%registers binding($name, type: $type) {
  value: $data
  loading: $loading
  error: $error
}
```

### `%resolves { }`

Declares semantic symbol resolution via runtime registries. This enables macros to specify that captured symbols should be resolved through a named registry, which defines the runtime namespace.

```st
%resolves {
  $name -> functions
}
```

**Syntax:** `$symbol -> registry`

- **`$symbol`:** A captured variable from `%form` (e.g., `$name`)
- **`registry`:** The name of a runtime registry (e.g., `functions`, `filters`)

**How it works:**

When a macro with `%resolves` is expanded:
1. The captured variable (e.g., `$name = "formatPrice"`) is resolved
2. The registry namespace is looked up from `%runtime-registry` definitions
3. A transform rule is created using the registry's namespace

**Example - The `@fn` macro:**

```st
%macro fn {
  %creates @fn
  %scope file

  %form {
    @fn $name:ident($params:params) : $returnType:typeref {
      $body:expr
    }
  }

  %resolves {
    $name -> functions
  }

  %binds {
    fn-registry(name: $name, params: $params, body: $body, returnType: $returnType)
  }
}
```

When the user writes:
```st
@fn formatPrice(price: number): string {
  "$" + price.toFixed(2)
}

.product-card {
  @each(products) {
    [slot="price"]: formatPrice($.price);
  }
}
```

The `%resolves` clause ensures `formatPrice($.price)` becomes `ST.functions.formatPrice(item_0.price)` in the generated JavaScript, using the namespace from the `functions` runtime registry.

---

## Runtime Registries

Runtime registries define the runtime namespaces and how they work per target. Macros reference these by name via `%resolves`.

### `%runtime-registry name { }`

Declares a runtime registry with target-specific definitions:

```st
%runtime-registry functions {
  %target js {
    %namespace { ST.functions }
    %init { ST.functions = ST.functions || {}; }
    %register($name, $value) { ST.functions[$name] = $value; }
    %call($name, $args) { ST.functions[$name]($args) }
    %access($name) { ST.functions[$name] }
  }
}
```

**Registry operations:**

| Operation | Purpose |
|-----------|---------|
| `%namespace` | The runtime namespace (e.g., `ST.functions`) |
| `%init` | Initialization code emitted once |
| `%register` | Code to register a value |
| `%call` | Code pattern for calling |
| `%access` | Code pattern for accessing |

**Built-in registries** (defined in `stdlib/runtime/registries.st`):

- `functions` - User-defined `@fn` functions → `ST.functions`
- `filters` - Data transformation filters → `ST.filters`

**Multi-target support:**

Registries can define different implementations per target:

```st
%runtime-registry functions {
  %target js {
    %namespace { ST.functions }
    %call($name, $args) { ST.functions[$name]($args) }
  }

  %target wasm {
    %namespace { spacetime::functions }
    %call($name, $args) { (call_indirect $fn_table (i32.const $name) $args) }
  }
}
```

The macro doesn't change—just set `current_target` in the registry to switch output.

---

### `%on trigger { }`

Responds to events or state changes.

```st
%on $active -> false {
  // When drag ends
  $onDrop?.($deltaX, $deltaY)
}

%on $visible -> true {
  // When element becomes visible
  %emit visibility-event
}
```

---

## Complete Examples

### `%macro fade-in`

```st
%macro fade-in {
  %creates @fade-in

  %form {
    @fade-in(
      $duration:time = 600ms,
      threshold: $th:number = 0.1,
      distance: $dist:length = 20px,
      easing: $ease:easing = ease-out
    )
  }

  %binds {
    intersection(&self, threshold: $th, once: true) -> { $visible }
  }

  %when $visible {
    %animates {
      opacity: 0 -> 1
      translate-y: $dist -> 0
    }

    %timing {
      duration: $duration
      easing: $ease
    }
  }
}
```

**User writes:**
```st
.hero {
  @fade-in(duration: 800ms, distance: 40px)
}
```

---

### `%macro scroll-timeline`

```st
%macro scroll-timeline {
  %creates @scroll

  %form {
    @scroll $name:ident (trigger: $trig:element = &self) {
      $body:keyframes
      range: $start:number to $end:number = 0 to 1
    }
  }

  %binds {
    scroll($trig) -> { $progress }
  }

  %timeline($name) {
    driver: $progress
    range: $start to $end
    $body
  }
}
```

**User writes:**
```st
.hero {
  @scroll hero-reveal(trigger: &viewport) {
    opacity: 0 -> 1
    translate-y: 50px -> 0
    range: 0.1 to 0.6
  }
}
```

---

### `%macro drag`

```st
%macro drag {
  %creates @drag

  %form {
    @drag(
      axis: $axis:("x" | "y" | "both") = "both",
      bounds: $bounds:element? = none
    ) {
      $customStates:states?
      on-drop: $onDrop:expr?
    }
  }

  %binds {
    gesture(&self) -> { $active, $deltaX, $deltaY, $velocityX, $velocityY }
  }

  %derives {
    $x: $axis != "y" ? $deltaX : 0
    $y: $axis != "x" ? $deltaY : 0
    $pos: $bounds ? clamp($x, $y, $bounds) : ($x, $y)
  }

  %states {
    idle {
      cursor: grab
    }
    dragging when $active {
      cursor: grabbing
      user-select: none
    }
    $customStates?
  }

  %animates {
    translate: $pos
  }

  %on $active -> false {
    $onDrop?.($x, $y, $velocityX, $velocityY)
  }
}
```

**User writes:**
```st
.card {
  @drag(axis: "x", bounds: &container) {
    dragging { scale: 1.02; box-shadow: 0 8px 32px rgba(0,0,0,0.2) }
    on-drop: handleDrop($x, $y)
  }
}
```

---

### `%macro each`

```st
%macro each {
  %creates @each

  %form {
    @each($source:binding) {
      $template:template
      :entering { $enterAnim:keyframes? }
      :exiting { $exitAnim:keyframes? }
    }
  }

  %iterates $source {
    item: $
    index: $_index
    first: $_first
    last: $_last
    count: $_count
  }

  %template {
    $template
  }

  %transitions {
    enter: $enterAnim
    exit: $exitAnim
  }
}
```

**User writes:**
```st
.product-grid {
  @each($products) {
    > product-card {
      [slot="name"]: $.name
      [slot="price"]: $.price | currency("$")
      [slot="image"]: $.thumbnail
    }

    :entering {
      opacity: 0 -> 1
      scale: 0.9 -> 1
    }

    :exiting {
      opacity: 1 -> 0
      translate-x: 0 -> -20px
    }
  }
}
```

---

### `%macro type`

```st
%macro type {
  %creates @type

  %form {
    @type $name:ident {
      $fields:fields
    }
  }

  %registers type($name) {
    $fields
  }
}
```

**User writes:**
```st
@type Product {
  id: string
  name: string
  price: number
  tags: string[]
  inStock: bool
}
```

---

### `%macro data`

```st
%macro data {
  %creates @data

  %form {
    @data $name:ident : $type:typeref {
      src: $url:string
      cache: $cache:duration = 0
    }
  }

  %binds {
    fetch($url, cache: $cache) -> { $data, $loading, $error }
  }

  %registers binding($name, type: $type) {
    value: $data
    loading: $loading
    error: $error
  }
}
```

**User writes:**
```st
@data products: Product[] {
  src: "/api/products"
  cache: 5m
}
```

---

## How Macros Compose

Macros can use other macros:

```st
%macro hero-section {
  %creates @hero-section

  %form {
    @hero-section {
      $content:template
    }
  }

  // Compose with existing directives
  %includes {
    @fade-in(duration: 800ms, threshold: 0.2)
    @parallax(speed: 0.3)
  }

  %template {
    $content
  }
}
```

---

## The Compiler's View

When the compiler sees:
```st
.hero {
  @fade-in(duration: 800ms)
}
```

It:
1. Looks up `@fade-in` in the macro registry
2. Matches against `%form`
3. Captures: `$duration = 800ms`, `$th = 0.1` (default), etc.
4. Expands into a declaration graph:

```
.hero
└── @fade-in
    ├── %binds
    │   └── intersection(&self, threshold: 0.1, once: true)
    │       └── exports: { $visible }
    ├── %when $visible
    │   ├── %animates
    │   │   ├── opacity: 0 -> 1
    │   │   └── translate-y: 20px -> 0
    │   └── %timing
    │       ├── duration: 800ms
    │       └── easing: ease-out
```

5. Resolves primitives to JavaScript
6. Emits optimized runtime code

---

## Testing Macros

Test macros by verifying their declaration graph:

```ts : BAD
// TODO: This is WRONG.
// Testing in Spacetime should be done in a Spacetime way
// i.e. some macros/primitives that enable automated testing,
// ideally end-to-end and runnable from a CLI.

describe('%macro fade-in', () => {
  it('generates correct bindings', () => {
    const graph = expandMacro('@fade-in(duration: 800ms)', context);

    expect(graph.binds).toContainEqual({
      primitive: 'intersection',
      args: { threshold: 0.1, once: true },
      captures: ['$visible']
    });
  });

  it('applies timing from parameters', () => {
    const graph = expandMacro('@fade-in(duration: 1s, easing: spring(300))', context);

    expect(graph.timing.duration).toBe('1s');
    expect(graph.timing.easing).toBe('spring(300)');
  });

  it('uses defaults when parameters omitted', () => {
    const graph = expandMacro('@fade-in', context);

    expect(graph.timing.duration).toBe('600ms');
    expect(graph.timing.easing).toBe('ease-out');
  });
});
```

---

## Summary

| Construct | Purpose |
|-----------|---------|
| `%creates` | Names the `@` directive |
| `%form` | Declares syntax shape |
| `%binds` | Connects to primitives |
| `%derives` | Computes values |
| `%when` | Conditional activation |
| `%animates` | Property targets |
| `%timeline` | Animation composition |
| `%states` | State machine |
| `%registers` | Type/binding registration |
| `%resolves` | Symbol resolution via registry |
| `%on` | Event/change response |
| `%runtime-registry` | Define runtime namespace |

Macros are pure declaration. The compiler does the wiring.
