# %form Syntax Guide

**From Beginner to Expert**

The `%form` clause is the pattern-matching heart of Spacetime's macro system. It's based on a simple but powerful principle: **one syntax to rule them all**. Instead of hardcoding dozens of special syntax forms into the parser, Spacetime uses `%form` to let macros define their own syntax patterns. The compiler then uses these patterns to parse user code and extract captured values.

This guide takes you from your first pattern to mastering advanced techniques used throughout the stdlib.

---

## Table of Contents

1. [Introduction - Why %form Exists](#introduction---why-form-exists)
2. [Quick Start](#quick-start)
3. [Capture Types](#capture-types)
   - [Basic Types](#basic-types-ident-string-number-bool)
   - [Time and Length Types](#time-and-length-types)
   - [Spacetime-Specific Types](#spacetime-specific-types)
   - [Block Types](#block-types)
4. [Modifiers](#modifiers)
5. [Default Values](#default-values)
6. [Literals](#literals)
7. [Pattern Prefixes](#pattern-prefixes)
8. [Parameters](#parameters)
9. [Body Captures](#body-captures)
10. [Aliasing](#aliasing)
11. [Union Types](#union-types)
12. [Pattern Match Syntax](#pattern-match-syntax)
13. [Advanced Patterns from Stdlib](#advanced-patterns-from-stdlib)
14. [How the Matcher Works](#how-the-matcher-works)
15. [Best Practices](#best-practices)
16. [Quick Reference](#quick-reference)

---

## Introduction - Why %form Exists

Traditional CSS preprocessors and DSLs have a fixed syntax. If you want `@keyframes`, `@media`, or `@mixin`, the parser must know about each one. Adding new features means changing the parser itself.

Spacetime takes a different approach. The core parser understands only the **structure** of macros - that they have a `%form` clause, a `%binds` clause, etc. The actual **syntax** that users write is defined by the macros themselves.

This means:

- **Extensibility**: New syntax can be added just by writing new macros in `.st` files
- **Consistency**: All custom syntax follows the same matching rules
- **Clarity**: Looking at a macro's `%form` tells you exactly what syntax it accepts
- **Power**: Complex patterns like `@each($items as $item, key: $item.id)` are just patterns, not parser magic

When you write:

```st
$count number: 0;
```

The compiler doesn't "know" about local state syntax. Instead, it finds the `local-state` macro whose `%form` pattern matches that input, extracts the captures, and passes them to the macro's body for processing.

---

## Quick Start

Let's write a simple macro that defines custom syntax:

```st
%macro greet {
  %form { @greet($name:string) }

  %emit js {
    console.log("Hello, " + %name + "!");
  }
}
```

This macro:
1. Defines a pattern: `@greet($name:string)`
2. Captures the string argument into `$name`
3. Uses `$name` in the JavaScript emission

Users can now write:

```st
@greet("World")
```

And the compiler will generate:

```js
console.log("Hello, " + "World" + "!");
```

### A Real Example: Local State

Here's the actual `local-state` macro from stdlib:

```st
%macro local-state {
  %form { $name:ident $type:ident : $value:expr ; }

  %binds {
    local-state-impl(name: $name, initial: $value)
  }
}
```

This pattern matches:

```st
$count number: 0;
$name string: "hello";
$items Product[]: [];
```

Breaking down the pattern:
- `$name:ident` - captures an identifier (like `count`)
- `$type:ident` - captures the type name (like `number`)
- `:` - a literal colon that must appear
- `$value:expr` - captures any expression (like `0`)
- `;` - a literal semicolon that must appear

The captured values are then passed to the `local-state-impl` primitive.

---

## Capture Types

Captures are the core of `%form` patterns. They use the syntax `$name:type` to extract values from user code.

### Anatomy of a Capture

```
$duration:time
    ^       ^
    |       |
    |       +-- capture type (what kind of value to match)
    +---------- capture name (variable to bind the value to)
```

### Basic Types: `:ident`, `:string`, `:number`, `:bool`

These are the fundamental building blocks:

#### `:ident` - Identifiers

Matches names like variables, function names, or keywords.

```st
%form { @animate $name:ident }
```

Matches:
- `@animate fadeIn`
- `@animate slide-up`
- `@animate btn_primary`

Rules:
- Starts with a letter or underscore
- Can contain letters, digits, underscores, and hyphens
- Case-sensitive

#### `:string` - String Literals

Matches quoted text.

```st
%form { @import $path:string }
```

Matches:
- `@import "styles.css"`
- `@import './components.st'`

Rules:
- Single or double quotes
- Supports escape sequences (`\"`, `\'`, `\\`)

#### `:number` - Numeric Values

Matches integers and floating-point numbers.

```st
%form { @opacity($value:number) }
```

Matches:
- `@opacity(0.5)`
- `@opacity(100)`
- `@opacity(-0.1)`

Rules:
- Optional negative sign
- Optional decimal point
- No units (use `:time` or `:length` for values with units)

#### `:bool` - Boolean Values

Matches `true` or `false`.

```st
%form { @autoplay($enabled:bool) }
```

Matches:
- `@autoplay(true)`
- `@autoplay(false)`

### Time and Length Types

#### `:time` - Time Durations

Matches durations with units.

```st
%form { @delay($wait:time) }
```

Matches:
- `@delay(300ms)` - milliseconds
- `@delay(1.5s)` - seconds
- `@delay(100us)` - microseconds

The captured value is always converted to milliseconds internally.

#### `:duration` - Same as `:time`

An alias for `:time`, used when the semantic meaning is "duration" rather than "point in time".

#### `:length` - CSS Length Values

Matches values with CSS length units.

```st
%form { @offset($x:length, $y:length) }
```

Matches:
- `@offset(20px, 10px)`
- `@offset(1rem, 0.5em)`
- `@offset(50%, 100vh)`

Supported units: `px`, `em`, `rem`, `%`, `vh`, `vw`, `vmin`, `vmax`, and more.

#### `:easing` - Easing Functions

Matches easing function names or `cubic-bezier()` calls.

```st
%form { @animate(easing: $ease:easing) }
```

Matches:
- `@animate(easing: ease-out)`
- `@animate(easing: linear)`
- `@animate(easing: cubic-bezier(0.4, 0, 0.2, 1))`

### Spacetime-Specific Types

#### `:expr` - Expressions

The most flexible type - matches any expression until a delimiter.

```st
%form { @computed($formula:expr) }
```

Matches:
- `@computed($price * 1.1)`
- `@computed($items.filter(x => x.active))`
- `@computed($a + $b * $c)`

Expressions capture until they hit:
- A semicolon `;`
- A comma `,` (at depth 0)
- A closing paren `)` or brace `}` (at depth 0)

This makes `:expr` suitable for capturing complex JavaScript-like expressions.

#### `:binding` - Data Bindings

Matches reactive data references starting with `$`.

```st
%form { @each($source:binding as $item:ident) }
```

Matches:
- `@each($products as $p)`
- `@each($user.orders as $order)`
- `@each($data.items[0] as $first)`

Rules:
- Must start with `$`
- Can include property paths (`.property`)
- Can include array access (`[index]`)

#### `:element` - Element References

Matches element references starting with `&`.

```st
%form { @resize(bounds: $container:element) }
```

Matches:
- `@resize(bounds: &panel)`
- `@resize(bounds: &self)`
- `@resize(bounds: &slider-track)`

The special `&self` refers to the current element.

#### `:selector` - CSS Selectors

Matches CSS selector syntax.

```st
%form { @target($sel:selector) }
```

Matches:
- `@target(.card)`
- `@target(#main-content)`
- `@target([data-active])`
- `@target(.nav > .item:first-child)`

Parsing stops at whitespace, comma, or opening brace/paren.

#### `:typeref` - Type References

Matches type names, optionally with array brackets.

```st
%form { @data $name:ident : $type:typeref }
```

Matches:
- `@data products: Product`
- `@data items: string[]`
- `@data users: User[]`

#### `:preset` - Preset References

Matches preset names starting with `~`.

```st
%form { @animate(easing: $ease:preset) }
```

Matches:
- `@animate(easing: ~spring-bounce)`
- `@animate(easing: ~ease-smooth)`

### Block Types

These capture structured content between braces.

#### `:keyframes` - Animation Keyframes

Captures animation property transitions.

```st
%form { @hover { $body:keyframes } }
```

Captures content like:
```st
opacity: 0 -> 1
translate-y: 20px -> 0
scale: 0.9 -> 1 -> 1.02 -> 1
```

#### `:properties` - CSS Properties

Captures CSS property declarations.

```st
%form { @state(when: $condition:string) { $styles:properties } }
```

Captures content like:
```st
opacity: 0.5;
pointer-events: none;
background: rgba(0,0,0,0.5);
```

#### `:template` - Template Content

Captures child element templates.

```st
%form { @for($i:ident in $range:expr) { $template:template } }
```

#### `:html_block` - HTML Content

Captures HTML with interpolation.

```st
%form { @template &$name:ident { $html:html_block } }
```

Captures content like:
```html
<div class="card" data-id="$item.id">
  <h3>$item.title</h3>
  <p>$item.description</p>
</div>
```

#### `:param_list` - Parameter Definitions

Captures parameter lists in template definitions.

```st
%form { @template &$name:ident($params:param_list) }
```

Captures content like:
```st
$title, &content, $subtitle?
```

#### `:template_invocation` - Template Calls

Captures calls to templates.

```st
%form { @each { $calls:template_invocation+ } }
```

Captures content like:
```st
&card($item);
&header($item.title);
```

#### `:state_block` - Reactive State Blocks

Captures CSS rules tied to a reactive state.

```st
%form { @state(when: $condition:expr) { $properties:properties } }
```

Captures content like:
```st
@state(when: $status == "loading") {
  opacity: 0.5;
}
```

#### `:event_handler` - Event Handler Blocks

Captures event handler directives with a body.

```st
%form { @on $event:ident($params:param_list) { $properties:properties } }
```

Captures content like:
```st
@on &.click($target:ident) {
  cursor: pointer;
}
```

#### `:binding_target` - Iterator Bindings

Captures source bindings for iteration macros.

```st
%form { @each($source:binding as $item:ident) }
```

Captures content like:
```st
$products as $p
$items as $item
```

---

## Modifiers

Modifiers control how many times a capture can match.

### Optional: `?`

Matches zero or one time. If not present, the value is `None`/`null`.

```st
%form {
  @load $name:ident (
    delay: $delay:time?,
    threshold: $threshold:number?
  )
}
```

All these match:
- `@load fadeIn()` - both optional params missing
- `@load fadeIn(delay: 300ms)` - only delay provided
- `@load fadeIn(threshold: 0.5)` - only threshold provided
- `@load fadeIn(delay: 300ms, threshold: 0.5)` - both provided

Use optional captures when:
- A parameter has a sensible default
- The feature is an enhancement, not required
- You want to support minimal and full syntax variants

### Zero or More: `*`

Matches zero or more times, resulting in an array.

```st
%form {
  @each($source:binding as $item:ident) {
    $invocations:template_invocation*
  }
}
```

The `$invocations*` captures all template calls in the body - could be none, one, or many.

### One or More: `+`

Matches one or more times, requiring at least one match.

```st
%form {
  @each($source:binding as $item:ident) {
    $invocations:template_invocation+
  }
}
```

The `$invocations+` requires at least one template invocation - an empty body would not match.

### Combining Modifiers

You can combine modifiers with block captures:

```st
%form {
  @value-change {
    :entering {
      $enterAnim:keyframes?    // Optional keyframes
    }
    :exiting {
      $exitAnim:keyframes?     // Optional keyframes
    }
  }
}
```

---

## Default Values

Provide default values with `= value` after the type:

```st
%form {
  @scroll $name:ident (
    start: $start:number = 0,
    end: $end:number = 1,
    scrub: $scrub:bool = true,
    easing: $easing:ident = linear
  )
}
```

Now users can omit parameters they're happy with the defaults for:

```st
@scroll parallax(end: 0.5) { ... }
// start defaults to 0, scrub to true, easing to linear
```

### Supported Default Value Types

| Value Type | Example |
|------------|---------|
| Numbers | `= 0`, `= 3.14`, `= -1` |
| Strings | `= "default"`, `= 'value'` |
| Booleans | `= true`, `= false` |
| Time values | `= 300ms`, `= 1s`, `= 0ms` |
| Length values | `= 20px`, `= 1rem` |
| Identifiers | `= linear`, `= ease-out` |
| Empty array | `= []` |
| None/null | `= none` |

### Default vs Optional

- **Default** (`= value`): Parameter is optional AND has a fallback value
- **Optional** (`?`): Parameter can be omitted, value will be null/none

```st
// With default - $delay will be 0ms if not provided
delay: $delay:time = 0ms

// Optional - $delay will be null/none if not provided
delay: $delay:time?
```

---

## Literals

Literals in patterns match exact tokens. They're not captured but must be present in user code.

### Common Literal Tokens

| Literal | Purpose | Example Pattern |
|---------|---------|-----------------|
| `:` | Name/value separator | `$name:ident : $value:expr` |
| `;` | Statement terminator | `$decl:expr ;` |
| `(` `)` | Parameter delimiters | `@fn($args:expr)` |
| `{` `}` | Block delimiters | `@block { $body:properties }` |
| `,` | List separator | `$a:expr , $b:expr` |
| `->` | Transition arrow | `$from:number -> $to:number` |
| `<-` | Assignment arrow | `$target:ident <- $value:expr` |
| `as` | Aliasing keyword | `$source:binding as $item:ident` |
| `is` | Pattern matching | `$signal:ident is $variant:ident` |
| `in` | Iteration | `$var:ident in $range:expr` |
| `..` | Range operator | `$start:number .. $end:number` |

### Example: Semicolon as Statement Terminator

```st
%form { $name:ident $type:ident : $value:expr ; }
```

User code must include the semicolon:

```st
$count number: 0;
              ^-- literal semicolon required
```

### Example: Keywords in Iteration

```st
%form { @for($var:ident in $start:number .. $end:number) }
```

Matches: `@for($i in 1 .. 5)`
- `in` and `..` are literals
- `$var`, `$start`, `$end` are captures

---

## Pattern Prefixes

The first character of a pattern determines its "prefix" - this helps the matcher quickly find candidate patterns.

### Directive Prefix: `@`

The most common pattern type. Matches `@directive-name` syntax.

```st
%form { @toggle($active:binding) }
```

Matches: `@toggle($isOpen)`

### Bound Prefix: `$`

Matches patterns starting with a binding sigil.

```st
%form { $name:ident $type:ident : $value:expr ; }
```

Matches: `$count number: 0;`

### Element Prefix: `&`

Matches patterns starting with an element reference.

```st
%form { &name:ident $selector:selector ; }
```

Matches: `&header .header;`

### Why Prefixes Matter

The matcher uses prefixes to efficiently find candidate patterns. When parsing `@data(...)`, it only considers patterns that start with `@`. This makes matching fast even with many registered patterns.

---

## Parameters

Parameters appear inside parentheses and support several styles.

### Named Parameters

Most readable for macros with many options:

```st
%form {
  @load $name:ident (
    duration: $duration:time = 1000ms,
    delay: $delay:time = 0ms,
    easing: $easing:ident = ease-out
  )
}
```

User writes:

```st
@load fadeIn(duration: 600ms, delay: 200ms)
```

### Positional Parameters

Simpler for macros with few arguments:

```st
%form { @repeat($count:number) { $template:template } }
```

User writes:

```st
@repeat(5) { ... }
```

### Mixed Parameters

Combine positional and named:

```st
%form {
  @each(
    $source:binding as $item:ident,
    key: $key:expr = $_index
  )
}
```

User writes:

```st
@each($products as $p, key: $p.id)
```

The first part is positional, `key:` is named.

### Parameter Keywords

Use keywords like `as`, `in`, `to` within parameter lists:

```st
%form { @each($source:binding as $item:ident) }
%form { @for($var:ident in $start:number .. $end:number) }
%form { @state(when: $condition:expr) }
```

---

## Body Captures

Capture entire blocks of content with brace-delimited bodies.

### Basic Body Capture

```st
%form {
  @hover $name:ident($duration:time) {
    $body:keyframes
  }
}
```

Matches:

```st
@hover lift(300ms) {
  translate-y: 0 -> -8px
  opacity: 0.8 -> 1
}
```

The entire block content is captured into `$body`.

### Nested Structured Bodies

Some patterns have multiple named sub-blocks:

```st
%form {
  @each($source:binding as $item:ident) {
    $invocations:template_invocation+

    :entering {
      $enterAnim:keyframes?
    }

    :exiting {
      $exitAnim:keyframes?
    }

    :move {
      $moveAnim:keyframes?
    }
  }
}
```

This captures:
- `$invocations` - the template calls (required, at least one)
- `$enterAnim` - enter animation keyframes (optional)
- `$exitAnim` - exit animation keyframes (optional)
- `$moveAnim` - move animation keyframes (optional)

User writes:

```st
@each($items as $item) {
  &card($item);

  :entering {
    opacity: 0 -> 1
    scale: 0.9 -> 1
  }

  :exiting {
    opacity: 1 -> 0
  }
}
```

---

## Aliasing

The `as` keyword creates aliases for captured values.

### In Form Patterns

Used for data iteration - the user chooses the iteration variable name:

```st
%form { @each($source:binding as $item:ident) }
```

User writes `@each($products as $product)`:
- `$source` captures `$products` (the data source)
- `$item` captures `$product` (the user's chosen name for each item)

### In %binds Output

Rename primitive outputs for user convenience:

```st
%binds {
  data-source(name: $name, src: $src) -> {
    $data as $$name,
    $loading as ${$name}-loading,
    $error as ${$name}-error
  }
}
```

If the user writes `@data products(...)`, then:
- `$data` becomes `$products`
- `$loading` becomes `$products-loading`
- `$error` becomes `$products-error`

### Socket/State Aliasing

Let users name their state bindings:

```st
%form { @socket(url: $url:expr) as $state:ident }
```

User writes:

```st
@socket(url: "/ws") as $ws
```

The socket state is now accessible as `$ws`.

---

## Union Types

Match one of several literal options.

### Basic Union Type

```st
%form {
  @drag(axis: $axis:("x" | "y" | "both") = "both")
}
```

The `$axis` capture will be exactly one of: `"x"`, `"y"`, or `"both"`.

Matches:
- `@drag(axis: "x")`
- `@drag(axis: "y")`
- `@drag(axis: "both")`
- `@drag()` - uses default `"both"`

### Union with Unquoted Values

For identifier-like values:

```st
%form {
  @sort(direction: $dir:(asc | desc) = asc)
}
```

Matches:
- `@sort(direction: asc)`
- `@sort(direction: desc)`

### Union in Complex Patterns

```st
%form {
  @view $signal:ident {
    $branches:("a" | "b" | "c") => $template:template_invocation
  }
}
```

Matches:
- `@view $mode { "a" => &tplA(); "b" => &tplB(); }`
- `@view $mode { "c" => &tplC(); }`

---

## Pattern Match Syntax

For matching typed union variants, like WebSocket connection states.

### Simple Pattern Match

```st
%form {
  @state(when: $signal:ident is $variant:ident) {
    $properties:properties
  }
}
```

Matches:

```st
@state(when: $ws is Disconnected) {
  opacity: 0.5;
}
```

### Pattern Match with Destructuring

```st
%form {
  @state(when: $signal:ident is $variant:ident { $bindings:ident* }) {
    $properties:properties
  }
}
```

Matches:

```st
@state(when: $ws is Connected { $send, $received }) {
  opacity: 1;
}
```

The `$bindings` captures `["$send", "$received"]` - the destructured variables from the variant.

---

## Advanced Patterns from Stdlib

Let's examine some real patterns from the standard library.

### Timeline Macros: Multiple Parameters with Defaults

```st
%macro scroll-timeline {
  %form {
    @scroll $name:ident (
      start: $start:number = 0,
      end: $end:number = 1,
      scrub: $scrub:bool = true,
      trigger: $trigger:selector?,
      easing: $easing:ident = linear,
      stagger: $stagger:number = 0,
      staggerFrom: $staggerFrom:string = "first"
    ) {
      $body:keyframes
    }
  }
  // ...
}
```

This showcases:
- Inline capture after directive (`$name:ident`)
- Named parameters with defaults
- Optional parameter without default (`$trigger:selector?`)
- Body capture for keyframes

### Data Macro: Nested Block Structure

```st
%macro data-fetch {
  %form {
    @data(
      src: $src:string,
      as: $name:ident = data,
      refresh: $refresh:time?,
      default: $default:any?
    ) {
      $content:block?
    }
  }
  // ...
}
```

This showcases:
- Parameter aliasing (`as: $name`)
- Optional block content (`$content:block?`)

### Each Macro: Complex Iteration Pattern

```st
%macro each {
  %form {
    @each(
      $source:binding as $item:ident,
      key: $key:expr = $_index
    ) {
      $invocations:template_invocation+

      :entering {
        $enterAnim:keyframes?
      }

      :exiting {
        $exitAnim:keyframes?
      }

      :move {
        $moveAnim:keyframes?
      }
    }
  }
  // ...
}
```

This showcases:
- Positional + named mixed parameters
- `as` keyword for iteration variable naming
- `+` modifier requiring at least one invocation
- Nested pseudo-selector blocks
- Multiple optional body captures

### Test Framework: Flexible Assertion Pattern

```st
%macro then {
  %form {
    @then $target:selector should $assertion:ident $expected:expr?
  }
  // ...
}
```

This showcases:
- Multiple inline captures
- The `should` keyword as literal
- Optional expected value for assertions that don't need one

---

## How the Matcher Works

Understanding the matcher helps you write better patterns.

### The Matching Algorithm

When the compiler encounters a statement like `$count number: 0;`, here's what happens:

1. **Extract Prefix**: Get the first character (`$`)

2. **Find Candidates**: Look up all registered forms with that prefix

3. **Try Each Form**: For each candidate (most specific first):
   - Try to match the directive name
   - Match inline elements (captures and literals)
   - Match parameters if present
   - Match body if present

4. **Return First Match**: The first successful match wins

### What "Most Specific First" Means

Forms are sorted by specificity. A pattern with more literals is more specific than one with more captures. This ensures:

```st
%form { @data-source($url:string) }  // More specific
%form { @data($name:expr) }          // Less specific
```

The first pattern matches `@data-source(...)`, the second matches other `@data*` patterns.

### How Captures Parse

Each capture type has specific parsing rules:

| Type | Parsing Rule |
|------|-------------|
| `:ident` | Letter/underscore, then letters/digits/underscores/hyphens |
| `:string` | Opening quote, content until closing quote (respects escapes) |
| `:number` | Optional `-`, digits, optional `.` and more digits |
| `:time` | Number followed by `ms`, `s`, or `us` |
| `:length` | Number followed by CSS unit (`px`, `%`, `em`, etc.) |
| `:expr` | Content until delimiter (`;`, `,`, `)`, `}` at depth 0) |
| `:selector` | Content until whitespace, `,`, `{`, or `(` |
| `:binding` | `$` followed by identifier and optional `.prop` or `[index]` |
| `:element` | `&` followed by identifier |
| `:preset` | `~` followed by identifier |

### Whitespace Handling

The matcher is whitespace-tolerant:

```st
// All of these match the same pattern:
$count number: 0;
$count  number :  0 ;
$count   number   :   0   ;
```

Whitespace is trimmed between tokens automatically.

### Block Extraction

For body captures, the matcher counts brace depth:

```st
@hover {
  opacity: 0 -> 1      // depth = 1
  @nested {            // depth = 2
    scale: 1 -> 1.1
  }                    // depth = 1
}                      // depth = 0, block ends
```

Everything between the opening `{` and the matching closing `}` is captured.

---

## Best Practices

### 1. Use Descriptive Capture Names

```st
// Good - clear what each capture represents
$duration:time, $easing:ident, $threshold:number

// Avoid - cryptic abbreviations
$d:time, $e:ident, $t:number
```

### 2. Provide Sensible Defaults

```st
%form {
  @load $name:ident (
    duration: $duration:time = 1000ms,  // Reasonable default
    delay: $delay:time = 0ms,           // No delay by default
    easing: $easing:ident = ease-out    // Common easing choice
  )
}
```

### 3. Make Optional What Can Be Optional

If a feature is an enhancement rather than core functionality, make it optional:

```st
// Good - trigger is only needed sometimes
trigger: $trigger:selector?

// This would be annoying if users had to specify a trigger every time
trigger: $trigger:selector
```

### 4. Use Appropriate Capture Types

Match the semantic meaning:

```st
// For CSS properties, use :properties
$styles:properties

// For animation keyframes, use :keyframes
$body:keyframes

// For HTML content, use :html_block
$html:html_block

// For template calls, use :template_invocation
$calls:template_invocation+
```

### 5. Group Related Parameters

Keep related parameters together:

```st
%form {
  @scroll $name:ident (
    // Scroll position parameters
    start: $start:number = 0,
    end: $end:number = 1,
    scrub: $scrub:bool = true,

    // Animation parameters
    easing: $easing:ident = linear,
    stagger: $stagger:number = 0,
    staggerFrom: $staggerFrom:string = "first"
  )
}
```

### 6. Document Expected Syntax

Add examples in comments:

```st
/// Makes an element draggable.
///
/// @example basic
/// .card { @drag }
///
/// @example constrained to x-axis
/// .slider { @drag(axis: "x") }
///
/// @example with bounds
/// .handle { @drag(axis: "x", bounds: &track) }

%macro drag {
  %form { ... }
}
```

### 7. Consider Pattern Specificity

More specific patterns should be defined first or use longer directive names:

```st
// These won't conflict - different directive names
%form { @state-machine(...) }  // Matches @state-machine
%form { @state(...) }          // Matches @state
```

---

## Quick Reference

### Capture Type Cheat Sheet

| Type | Example Match | Use For |
|------|---------------|---------|
| `:ident` | `fadeIn`, `my-var` | Names, identifiers |
| `:string` | `"hello"`, `'world'` | Quoted text |
| `:number` | `42`, `3.14`, `-1` | Numeric values |
| `:bool` | `true`, `false` | Boolean flags |
| `:time` | `300ms`, `1.5s` | Durations |
| `:length` | `20px`, `50%` | CSS lengths |
| `:expr` | `$a + $b`, `fn()` | Expressions |
| `:binding` | `$data`, `$user.name` | Reactive bindings |
| `:element` | `&header`, `&self` | Element references |
| `:selector` | `.card`, `#main` | CSS selectors |
| `:typeref` | `Product`, `string[]` | Type references |
| `:preset` | `~ease-out` | Preset references |
| `:properties` | `name: string; price: number;` | Type field definitions |
| `:params` | `a: number, b: number = 0` | Function parameters |
| `:keyframes` | `opacity: 0 -> 1` | Animation keyframes |
| `Union` | `"x" \| "y" \| "both"` | One of several variants |
| `PatternMatch` | `$ws is Connected { $send }` | Typed variant matching |
| `:html_block` | `<div>...</div>` | HTML content |

### Modifier Cheat Sheet

| Modifier | Meaning | Captured Value |
|----------|---------|----------------|
| (none) | Required, exactly one | Single value |
| `?` | Optional (0 or 1) | Value or null |
| `*` | Zero or more | Array |
| `+` | One or more | Array (non-empty) |

### Default Value Cheat Sheet

```st
$num:number = 0
$str:string = "default"
$flag:bool = true
$dur:time = 300ms
$len:length = 20px
$id:ident = linear
$arr:expr = []
$opt:expr = none
```

---

## New Capture Types (Phase 1 Implementation)

The following capture types were recently added to support complex stdlib patterns:

### `:properties` - Type Field Definitions

Captures structured type field definitions with optional fields and array types.

```st
%form { @type $name:ident { $fields:properties } }
```

Matches:
```st
@type Product {
  id: number;
  name: string;
  price: number;
  active?: boolean;
  tags: string[];
}
```

Each property is parsed into a `PropertyDef` struct with:
- `name: String` - the field name
- `type_ref: String` - the type reference (supports arrays like `string[]`)
- `optional: bool` - whether the field is optional (marked with `?`)

### `:params` - Function Parameter Lists

Captures function parameter definitions with optional default values.

```st
%form { @fn $name:ident($params:params) { $body:expr } }
```

Matches:
```st
@fn calculate(a: number, b: number = 0, c: string = "default") {
  return a + b;
}
```

Each parameter is parsed into a `ParamDef` struct with:
- `name: String` - the parameter name
- `type_ref: String` - the type reference
- `default: Option<String>` - optional default value

### `:keyframes` - Animation Keyframes

Captures animation keyframe transitions with multi-step support.

```st
%form { @hover { $anim:keyframes } }
```

Matches:
```st
@hover {
  opacity: 0 -> 1;
  scale: 0.9 -> 1 -> 1.05 -> 1;
  translate-y: 20px -> 0;
  transform: scale(0.9) -> scale(1);
}
```

Each keyframe is parsed into a `KeyframeDef` struct with:
- `property: String` - the CSS property name
- `values: Vec<String>` - the transition values (from -> to -> ...)

### Union Types: `("x" | "y" | "both")`

Match one of several literal string variants.

```st
%form {
  @drag(axis: $axis:("x" | "y" | "both") = "both")
}
```

Matches:
- `@drag(axis: "x")`
- `@drag(axis: "y")`
- `@drag(axis: "both")`

The captured value is a `CapturedValue::Ident` with the matched variant.

### Pattern Matching: `$signal is Variant { $bindings }`

Match typed union variants with optional destructuring.

```st
%form {
  @state(when: $condition:pattern_match) { $styles:properties }
}
```

Matches:
```st
// Simple variant match
@state(when: $ws is Disconnected) {
  opacity: 0.5;
}

// Variant match with destructuring
@state(when: $ws is Connected { $send, $received }) {
  opacity: 1;
}
```

The pattern is captured as a string representation: `"$ws is Connected { $send, $received }"`.

### Implementation Status

All five new capture types are fully implemented with comprehensive tests:

| Capture Type | Parser Function | Tests | Status |
|--------------|----------------|-------|---------|
| `:properties` | `parse_properties()` | 5+ unit tests | Complete |
| `:params` | `parse_params_capture()` | 4+ unit tests | Complete |
| `:keyframes` | `parse_keyframes()` | 3+ unit tests | Complete |
| `Union` | `parse_union()` | 5+ unit tests | Complete |
| `PatternMatch` | `parse_pattern_match()` | 4+ unit tests | Complete |

### CapturedValue Variants

The new capture types return structured variants:

```rust
pub enum CapturedValue {
    // ...existing variants...

    // New structured variants
    Properties(Vec<PropertyDef>),
    Params(Vec<ParamDef>),
    Keyframes(Vec<KeyframeDef>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropertyDef {
    pub name: String,
    pub type_ref: String,
    pub optional: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParamDef {
    pub name: String,
    pub type_ref: String,
    pub default: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeyframeDef {
    pub property: String,
    pub values: Vec<String>,
}
```

## Summary

The `%form` clause is powerful and flexible:

- **Captures** (`$name:type`) extract values from user code
- **Types** range from simple (`:ident`, `:number`) to complex (`:keyframes`, `:html_block`)
- **Modifiers** (`?`, `*`, `+`) control optionality and repetition
- **Defaults** (`= value`) provide fallback values
- **Literals** (`:`, `;`, `as`, `in`) match exact tokens
- **Prefixes** (`@`, `$`, `&`) determine pattern type and enable fast matching
- **Parameters** support named, positional, and mixed styles
- **Body captures** extract block content
- **Union types** match one of several options
- **Pattern matching** matches typed union variants with destructuring
- **Structured captures** (`:properties`, `:params`, `:keyframes`) provide typed field access
- **Aliasing** lets users name their own bindings

Master these concepts and you can define any custom syntax for Spacetime macros. The "one syntax to rule them all" principle means that every macro - from simple presets to complex state machines - uses the same pattern-matching foundation.
