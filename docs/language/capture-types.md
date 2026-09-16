# Capture Types

Capture types define how the Spacetime parser recognizes and extracts values from source code. They are the building blocks of the `%form` pattern matching system.

## Overview

When you write a macro with `%form`, each capture like `$name:ident` or `$duration:time` uses a capture type to parse the input:

```st
%macro fade-in {
  %creates @fade-in
  %form { @fade-in($duration:time) }
  //                        ^^^^
  //                        capture type
}
```

**There are two kinds of capture types:**

1. **Kernel types** — implemented in Rust, because they need real lexical
   analysis (tokenization, escapes, operator parsing).
2. **Grammar types** — defined in Spacetime with `%capture_type`, including
   every CSS value type.

---

## Kernel Capture Types

These stay in Rust because a grammar cannot express them: they need
tokenization, escape handling, or expression parsing.

| Type | Description | Examples |
|------|-------------|----------|
| `ident` | Identifier | `fade-in`, `myVar`, `x` |
| `string` | Quoted string | `"hello"`, `'world'` |
| `number` | Numeric literal | `42`, `3.14`, `-1` |
| `bool` | Boolean | `true`, `false` |
| `expr` | JavaScript expression | `$.name`, `$count + 1` |
| `binding` | Signal binding | `$visible`, `$count` |
| `element` | Element reference | `&self`, `&container` |
| `selector` | CSS selector | `.card`, `#hero` |
| `event` | Event name | `click`, `hover`, `visible` |
| `typeref` | Type reference | `Product`, `string[]` |

---

## CSS Value Types Are Grammars, Not Rust

`color`, `length`, `duration`/`time`, `angle`, `percentage` and `easing` used to
be in the table above. They are now **ordinary `%capture_type` productions**
living in [`stdlib/capture-types/css-values.st`](../../stdlib/capture-types/css-values.st),
and nothing about them is special-cased in the compiler (PLAN-122).

| Type | Description | Examples |
|------|-------------|----------|
| `color` | CSS colour | `#ff0000`, `#fffa`, `oklch(...)`, `transparent` |
| `length` | CSS length | `20px`, `2rem`, `50%` |
| `duration` / `time` | Duration | `300ms`, `1.5s`, `5m` |
| `angle` | Angle | `45deg`, `0.5turn` |
| `percentage` | Percentage | `50%` |
| `easing` | Easing curve | `ease-in-out`, `cubic-bezier(...)` |

This is what makes them **extensible from stdlib**. Adding a CSS value type is
adding a grammar plus a row in [`stdlib/scalars/types.st`](../../stdlib/scalars/types.st)
— no Rust change:

```spacetime
%capture_type resolution_unit { "dpi" | "dppx" }
%capture_type resolution { ( $n:number $u:resolution_unit ) | ( $ref:binding ) }

%scalar_type resolution {
  %capture "resolution"     // the grammar that validates its literals
  %schema  "string"          // how it serializes
  %format  "resolution"
  %zero    ""
  %widget  "text"            // which admin control edits it
  %docs    "A screen resolution."
}
```

Two consequences worth knowing:

- **A malformed value is a parse error with a span.** `#e8ee1` is refused
  because the grammar says a hex colour is 3, 4, 6 or 8 digits — and those
  permitted lengths are data (`{3|4|6|8}`), which is why `#fffa` and
  `#11223344` still pass.
- **A scalar's captured value is its source text**, always: `8px` captures as
  the string `"8px"`, not as a record of the grammar's parts. The named parts
  inside a production exist to VALIDATE, not to restructure. The `%capture`
  column is what tells the compiler a production is a scalar, so a grammar
  without a `%scalar_type` row will capture as a record instead.

---

## Custom Capture Types (`%capture_type`)

For structured patterns that combine primitives, you can define capture types in Spacetime itself:

```st
%capture_type param_list {
  ( ( $name:binding | &name:element ) "?"? ","? )*
}
```

This creates a new capture type `param_list` that can be used in `%form`:

```st
%macro template {
  %form { @template &$name:ident($params:param_list) { $body:html_block } }
  //                              ^^^^^^^^^^^^^^^^^
  //                              custom capture type
}
```

### Syntax

```st
%capture_type name {
  pattern
}
```

**Pattern elements:**

| Element | Meaning |
|---------|---------|
| `$var:type` | Capture a value of the given type |
| `&var:element` | Capture an element reference |
| `"literal"` | Match a literal string |
| `( ... )` | Grouping |
| `\|` | Choice (alternatives) |
| `?` | Optional (zero or one) |
| `*` | Zero or more |
| `+` | One or more |

### Example: param_list

The `param_list` capture type parses template parameter declarations:

```st
// stdlib/capture-types/param_list.st

// Template parameter list capture type
// Parses: $title, &content, $footer?
//
// Example matches:
//   $title                    -> [{ name: "title", kind: "binding", optional: false }]
//   $a, &b, $c?               -> [binding a, element b, optional binding c]
//   ()                        -> [] (empty list allowed)
//
%capture_type param_list {
  ( ( $name:binding | &name:element ) "?"? ","? )*
}
```

**Usage:**

```st
// Define a template with parameters
@template &card($title, &content, $footer?) {
  <article>
    <h2>$title</h2>
    <div class="body">&content</div>
    <footer>$footer</footer>
  </article>
}

// Use the template
.hero {
  &card("Welcome", <p>Hello world</p>)
}
```

The `param_list` capture type extracts:
- `$title` — binding parameter (required)
- `&content` — element parameter (required)
- `$footer` — binding parameter (optional, marked with `?`)

---

## How Custom Types Work

When the parser encounters a custom capture type:

1. **Lookup** — The pattern is retrieved from the MetaRegistry
2. **Match** — The pattern matcher evaluates the pattern against input
3. **Convert** — Captured values are converted to the appropriate `CapturedValue`

```
Input: "$title, &content, $footer?"
         │
         ▼
┌─────────────────────────────────┐
│  Pattern: ( ( $:binding | &:element ) "?"? ","? )*  │
└─────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────┐
│  CapturedValue::ParamList([    │
│    { name: "title", kind: Binding, optional: false },
│    { name: "content", kind: Element, optional: false },
│    { name: "footer", kind: Binding, optional: true }
│  ])                             │
└─────────────────────────────────┘
```

---

## Creating a Custom Capture Type

### Step 1: Identify the Pattern

Look for repeating syntactic structures in your macros. Good candidates:
- Lists of similar items
- Key-value pairs
- Structured declarations

**Not good candidates:**
- Complex nested expressions (use `expr`)
- HTML/CSS parsing (use specialized types)
- Anything requiring complex tokenization

### Step 2: Write the Pattern

Start with examples of what you want to match:

```
// I want to match:
$a
$a, $b
$a, &b, $c?
(empty)
```

Then write a pattern that covers all cases:

```st
%capture_type my_params {
  ( ( $name:binding | &name:element ) "?"? ","? )*
}
```

### Step 3: Place in stdlib

Create a file in `stdlib/capture-types/`:

```
stdlib/
  capture-types/
    param_list.st    <- your new capture type
    my_params.st
```

The bootstrap process automatically loads all `.st` files from this directory.

### Step 4: Use in %form

Reference your capture type in macro form patterns:

```st
%macro my_directive {
  %form { @my-directive($params:my_params) }
}
```

---

## Migration Candidates

These structured types could potentially be migrated from Rust to `%capture_type`:

| Type | Pattern | Status |
|------|---------|--------|
| **param_list** | `( ($:binding \| &:element) "?"? ","? )*` | Migrated |
| **states** | `( $name:ident "{" $props:properties "}" )*` | Candidate |
| **template_invocation** | `"&" $name:ident "(" $args:param_list ")"` | Candidate |

**Must stay in Rust:**

| Type | Reason |
|------|--------|
| `properties` | Complex CSS parsing with transitions (`->`) |
| `keyframes` | Animation interpolation syntax |
| `html_block` | DOM parsing + interpolation |
| `expr` | JavaScript expression parsing |
| All primitives | Lexical analysis |

---

## Testing Custom Capture Types

### Unit Tests

Test the pattern matcher directly:

```rust
#[test]
fn test_param_list_parses_single_binding() {
    let result = parse_capture("$title)", &CaptureType::ParamList);
    assert!(result.is_some());
    // Verify captured value
}
```

### Runtime Tests (BoaJS)

Test that templates using the capture type work end-to-end:

```rust
#[test]
fn test_template_with_param_list() {
    let result = compile_and_run(r#"
        @template &greet($name) {
            <span>Hello $name</span>
        }

        .target {
            &greet("World")
        }
    "#);

    assert!(result.html.contains("Hello World"));
}
```

### LSP Tests

Verify the developer experience:

```rust
#[test]
fn test_hover_shows_param_list_type() {
    let hover = get_hover_at("@template &card($params:param_list)", position);
    assert!(hover.contains("param_list"));
}
```

---

## File Locations

| File | Purpose |
|------|---------|
| `stdlib/capture-types/*.st` | Custom capture type definitions |
| `src/syntax/events/` | Event-based parser (MatchSink + extractors) |
| `src/syntax/stdlib_registry.rs` | Stdlib capture type lookup |
| `src/parser/meta_ast.rs` | `CapturePatternAst` definition |
| `tests/v8_runtime/param_list_tests.rs` | Runtime tests |
| `tests/lsp/capture_type_test.rs` | LSP integration tests |

---

## See Also

- [Form Syntax](./form-syntax.md) — How `%form` patterns work
- [Macros](./macros.md) — Macro system overview
- [Primitives](./primitives.md) — Browser API wrappers
