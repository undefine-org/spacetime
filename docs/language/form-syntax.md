# Spacetime Form Syntax

> The same grammar also powers `%match` in syntax migrations — old syntax rewritten as stdlib data. See [Syntax Migrations](./migrations.md).

The `%form` declaration specifies the syntactic shape a macro accepts. It's pattern matching for syntax — you declare what the `@` directive looks like, and the compiler extracts typed values.

## Basic Structure

```st
%form {
  @directive-name($param:type, named: $value:type = default) {
    $body:blocktype
  }
}
```

---

## Capture Syntax

### Simple Capture

```st
$name:type
```

- `$name` — the binding name (available in macro body)
- `type` — the expected type

```st
%form {
  @fade-in($duration:time)
}

// User writes: @fade-in(800ms)
// Captures: $duration = 800ms
```

### Named Parameters

```st
name: $capture:type
```

```st
%form {
  @fade-in(duration: $dur:time, threshold: $th:number)
}

// User writes: @fade-in(duration: 800ms, threshold: 0.5)
// Captures: $dur = 800ms, $th = 0.5
```

### Default Values

```st
$name:type = default
```

```st
%form {
  @fade-in($duration:time = 600ms, threshold: $th:number = 0.1)
}

// User writes: @fade-in
// Captures: $duration = 600ms, $th = 0.1

// User writes: @fade-in(800ms)
// Captures: $duration = 800ms, $th = 0.1
```

### Optional Captures

```st
$name:type?
```

Optional captures may be absent. Check with `$name?` in macro body.

```st
%form {
  @drag(bounds: $bounds:element?)
}

// User writes: @drag
// Captures: $bounds = none

// User writes: @drag(bounds: &container)
// Captures: $bounds = &container
```

---

## Type Annotations

### Primitive Types

| Type | Matches | Examples |
|------|---------|----------|
| `ident` | Identifier | `hero`, `fade_in`, `myName` |
| `string` | String literal | `"hello"`, `"/api/data"` |
| `number` | Numeric literal | `42`, `3.14`, `-10` |
| `bool` | Boolean | `true`, `false` |
| `time` | Duration | `300ms`, `2s`, `1.5s` |
| `length` | CSS length | `20px`, `2rem`, `50%` |
| `color` | CSS color | `red`, `#ff0000`, `oklch(...)` |
| `easing` | Easing function | `ease-out`, `spring(300, 20)` |

### Reference Types

| Type | Matches | Examples |
|------|---------|----------|
| `element` | Element reference | `&hero`, `&self`, `&container` |
| `binding` | Data binding | `$products`, `$user.name` |
| `selector` | CSS selector | `.card`, `#main`, `[slot="x"]` |
| `typeref` | Type reference | `Product`, `User[]` |

### Block Types

| Type | Matches |
|------|---------|
| `keyframes` | Animation declarations |
| `properties` | CSS properties |
| `template` | Child elements |
| `fields` | Type field definitions |
| `states` | State definitions |
| `expr` | Single expression |

### Compound Types

```st
// Arrays
$items:string[]              // array of strings
$points:number[]             // array of numbers

// Unions (literal types)
$axis:("x" | "y" | "both")   // one of these strings
$mode:("light" | "dark")

// Function types
$handler:fn                  // any function
$formatter:fn(number):string // typed function
```

---

## Block Captures

### Simple Block

```st
%form {
  @scroll $name:ident {
    $body:keyframes
  }
}

// User writes:
@scroll hero {
  opacity: 0 -> 1
  translate-y: 50px -> 0
}

// Captures:
// $name = hero
// $body = { opacity: 0 -> 1; translate-y: 50px -> 0 }
```

### Block with Metadata

Capture specific properties from within a block:

```st
%form {
  @scroll $name:ident {
    $body:keyframes
    range: $start:number to $end:number = 0 to 1
  }
}

// User writes:
@scroll hero {
  opacity: 0 -> 1
  range: 0.2 to 0.8
}

// Captures:
// $name = hero
// $body = { opacity: 0 -> 1 }
// $start = 0.2
// $end = 0.8
```

### Optional Block

```st
%form {
  @fade-in($duration:time = 600ms) {
    $body:keyframes?    // optional block
  }
}

// Valid: @fade-in(800ms)
// Valid: @fade-in(800ms) { opacity: 0.5 -> 1 }
```

### Multiple Block Sections

```st
%form {
  @each($source:binding) {
    $template:template

    :entering {
      $enterAnim:keyframes?
    }

    :exiting {
      $exitAnim:keyframes?
    }
  }
}

// User writes:
@each($products) {
  > product-card {
    [slot="name"]: $.name
  }

  :entering {
    opacity: 0 -> 1
  }
}

// Captures:
// $source = $products
// $template = { > product-card { ... } }
// $enterAnim = { opacity: 0 -> 1 }
// $exitAnim = none
```

---

## Pattern Variations

### Directive with Keyword

```st
%form {
  @on $event:ident $name:ident($duration:time) {
    $body:keyframes
  }
}

// Matches:
// @on hover cardLift(300ms) { ... }
// @on click buttonPress(200ms) { ... }
// @on visible fadeIn(800ms) { ... }

// Captures: $event, $name, $duration, $body
```

### Multiple Forms

A macro can accept multiple syntax variations:

```st
%macro on-event {
  %form {
    // Short form: just duration
    @on $event:ident $name:ident($duration:time) {
      $body:keyframes
    }
  }

  %form {
    // Long form: with options
    @on $event:ident $name:ident(
      $duration:time,
      delay: $delay:time = 0,
      easing: $ease:easing = ease-out
    ) {
      $body:keyframes
    }
  }
}
```

### Nested Directive Forms

```st
%form {
  @state(when: $when:expr) {
    $body:declarations
  }

  @on $event:ident {
    $action:statements
  }
}

// User writes:
$loading <- false

.form {
  @on &.click { $loading <- true; }

  @state(when: $loading) {
    opacity: 0.6;
    pointer-events: none;
  }
}
```

---

## Variadic Captures

### Rest Parameters

```st
%form {
  @animate($props:keyframes...) // captures remaining as keyframes
}
```

### Repeated Patterns

```st
%form {
  @keyframes $name:ident {
    ($percent:number% { $props:properties })+  // one or more
  }
}

// Matches:
@keyframes bounce {
  0% { transform: translateY(0) }
  50% { transform: translateY(-20px) }
  100% { transform: translateY(0) }
}
```

---

## Conditional Forms

### When Guards

```st
%form {
  @data $name:ident : $type:typeref {
    src: $url:string
    cache: $cache:duration = 0
  } when $url.startsWith("http")
}
```

### Type-Dependent Forms

```st
%form {
  // For array types
  @data $name:ident : $type:typeref[] {
    src: $url:string
  }
}

%form {
  // For single object types
  @data $name:ident : $type:typeref {
    src: $url:string
  }
}
```

---

## Form Validation

The compiler validates user code against forms:

### Type Mismatches

```st
%form { @fade-in($duration:time) }

@fade-in("hello")  // ERROR: expected time, got string
@fade-in(50)       // ERROR: expected time, got number (needs unit)
@fade-in(500ms)    // OK
```

### Missing Required Parameters

```st
%form { @scroll $name:ident { $body:keyframes } }

@scroll { opacity: 0 -> 1 }  // ERROR: missing required $name
@scroll hero { }             // ERROR: empty keyframes block
@scroll hero { opacity: 0 -> 1 }  // OK
```

### Unknown Parameters

```st
%form { @fade-in(duration: $d:time) }

@fade-in(duration: 500ms, speed: fast)  // ERROR: unknown parameter 'speed'
```

---

## Form Introspection

Forms enable tooling support:

### Auto-completion

The IDE knows what parameters are valid:
```st
@fade-in(|)  // suggests: duration, threshold, distance, easing
```

### Hover Documentation

Derived from form structure and JSDoc-style comments:
```st
%macro fade-in {
  /// Fade in animation triggered by visibility
  /// @param duration Animation duration
  /// @param threshold Visibility threshold (0-1)
  %form {
    @fade-in($duration:time = 600ms, threshold: $th:number = 0.1)
  }
}
```

### Signature Help

Shows parameter info as you type:
```st
@fade-in(800ms, |)
//       ↑ duration: time
//                   ↑ threshold: number = 0.1
```

---

## Full Example

```st
%macro parallax {
  /// Creates a parallax scrolling effect
  %creates @parallax

  %form {
    @parallax(
      // Movement speed multiplier (0 = fixed, 1 = scroll speed)
      speed: $speed:number = 0.5,

      // Axis of movement
      axis: $axis:("x" | "y") = "y",

      // Element to track scroll position from
      relative: $relative:element = &viewport
    )
  }

  %binds {
    scroll($relative, axis: $axis) -> { $progress }
  }

  %derives {
    $offset: $progress * $speed * 100
  }

  %animates {
    translate-$axis: $offset + "px"
  }
}

// User writes:
.background-image {
  @parallax(speed: 0.3, axis: "y")
}

// Compiler extracts:
// $speed = 0.3
// $axis = "y"
// $relative = &viewport (default)
```

---

## Summary

| Syntax | Meaning |
|--------|---------|
| `$name:type` | Capture with type |
| `$name:type = default` | Capture with default |
| `$name:type?` | Optional capture |
| `name: $capture:type` | Named parameter |
| `$body:blocktype` | Block capture |
| `("a" \| "b")` | Union/enum type |
| `type[]` | Array type |
| `(pattern)+` | One or more |
| `(pattern)*` | Zero or more |

Forms are the interface contract between macros and users — they define exactly what syntax is valid and how it maps to macro bindings.
