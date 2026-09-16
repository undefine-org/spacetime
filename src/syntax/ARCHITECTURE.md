# Syntax Pipeline Architecture

## 1. Overview

### The Core Principle: %form is the Unique Reference of Syntax

All user-facing syntax in Spacetime is defined via **%form patterns** in `.st` files. There are no hardcoded parsers for specific syntax constructs. Instead:

- Each `%macro` declares a `%form` pattern that defines how it appears in source code
- The pattern specifies the prefix, inline elements, captures, and structure
- A `SyntaxRegistry` indexes all forms by their prefix character for O(1) lookup
- Pattern matching determines which macro matches the input

**Example:**

```spacetime
%macro local-state {
  %form { $name:ident $type:ident : $value:expr ; }
  %binds { local-state-impl(name: $name, initial: $value) }
}
```

This single `%form` definition:
- Establishes `$` as the prefix for local state
- Defines captures: `name`, `type`, `value`
- Enables parsing: `$count number: 0;`
- No hardcoded parser needed

### Universal Syntax Definition

Every syntax construct follows this pattern:

| Syntax | Prefix | Example Form |
|--------|--------|--------------|
| Local state | `$` | `$name:ident $type:ident : $value:expr ;` |
| Element ref | `&` | `&name:ident $selector:selector ;` |
| Directives | `@` | `@data($url:string)` |
| Presets | `~` | `~preset-name` |

### Prefix-Indexed Registry

The `SyntaxRegistry` uses prefix-based indexing for efficient lookup:

```
'$' → [local-state, ...]
'&' → [element-ref, ...]
'@' → [data-fetch, computed, animate, each, ...]
'~' → [preset refs...]
```

When parsing `$count number: 0;`:
1. Extract prefix: `$`
2. Get candidate forms for `$` → O(1) lookup
3. Try each form in specificity order (most specific first)
4. Return first match as `FormMatch`

## 2. Components

### SyntaxRegistry

**Purpose:** Store and index %form patterns for efficient lookup during parsing.

**Key Methods:**

```rust
impl SyntaxRegistry {
    /// Register a macro's %form pattern
    pub fn register(&mut self, macro_def: &MacroDefAst)

    /// Get all forms for a given prefix (e.g., '$', '@', '&')
    pub fn forms_for_prefix(&self, prefix: char) -> &[RegisteredForm]

    /// Check if a prefix has registered forms
    pub fn has_prefix(&self, prefix: char) -> bool

    /// Get total form count
    pub fn form_count(&self) -> usize
}
```

**Specificity Ordering:**

Forms are sorted by specificity (descending) to ensure more specific patterns match first:

```rust
fn calculate_specificity(form: &FormClause) -> u32 {
    // Literals (e.g., ":") = 5 points each
    // Captures (e.g., $name:ident) = 2 points each
    // Params = 3 points each
    // More complex forms = higher specificity
}
```

**Example:**
```
@data($name:ident : $type:ident)  // specificity: 25
@data($name:ident)                 // specificity: 15
```

The first form matches first since it's more specific.

### FormMatch

**Purpose:** The unified AST type that replaces all special-case types (ValueDeclaration, ElementRefDecl, MacroCallAst, etc.).

**Structure:**

```rust
pub struct FormMatch {
    /// Which %macro matched (e.g., "local-state", "element-ref")
    pub macro_name: String,

    /// Captured values from the pattern
    pub captures: HashMap<String, CapturedValue>,

    /// CSS selector if in scope block
    pub selector: Option<String>,

    /// Source span for errors
    pub span: SourceSpan,
}
```

**Captured Values:**

All capture types are represented as `CapturedValue` enum variants:

```rust
pub enum CapturedValue {
    Ident(String),        // $name:ident → "count"
    String(String),       // $url:string → "/api/data"
    Number(f64),          // $threshold:number → 0.5
    Bool(bool),           // $enabled:bool → true
    Time(u32),            // $duration:time → 500 (ms)
    Length(LengthValue),  // $distance:length → 20px
    Selector(String),     // $target:selector → ".hero"
    Binding(String),      // $data:binding → "$products"
    Element(String),      // $ref:element → "&button"
    Expr(String),         // $value:expr → "x + 1"
    TypeRef(String),      // $type:typeref → "Product[]"
    Preset(String),       // $preset:preset → "~ease-out"
    Json(JsonValue),      // Complex values
    Block(Vec<FormMatch>), // Nested blocks
    Array(Vec<CapturedValue>), // Arrays
    Named(HashMap<String, CapturedValue>), // Key-value pairs
}
```

**Helper Methods:**

```rust
impl FormMatch {
    pub fn get_ident(&self, name: &str) -> Option<&str>
    pub fn get_string(&self, name: &str) -> Option<&str>
    pub fn get_number(&self, name: &str) -> Option<f64>
    pub fn get_time(&self, name: &str) -> Option<u32>
    pub fn get_binding(&self, name: &str) -> Option<&str>
    pub fn get_block(&self, name: &str) -> Option<&[FormMatch]>
    // ... etc for each type
}
```

### Event-Based Parser (MatchSink)

**Purpose:** Pattern matching engine that matches parsed events against %form definitions via a three-layer architecture.

**Architecture:**

```
Layer 3: MatchSink (Sink impl) — context management, form dispatch
Layer 2: FormCompiler          — FormClause -> compiled matcher (at registration)
Layer 1: CaptureExtractor      — per-type extraction from token slices
```

**API:**

```rust
pub fn parse_matches(source: &str, registry: &SyntaxRegistry) -> (Vec<FormMatch>, Vec<MatchDiagnostic>) {
    // 1. Lex source into tokens
    // 2. Build Input, run event-based parser
    // 3. Process events through MatchSink
    // 4. Return matches + diagnostics
}
```

Each capture type has a dedicated extractor operating on `&[TokenData]` slices.
Forms are tried in specificity order with early exit on match.

### Bootstrap

**Purpose:** Load stdlib .st files and build the SyntaxRegistry at compile time.

**Process:**

```rust
pub fn bootstrap_stdlib(stdlib_path: &Path) -> Result<SyntaxRegistry, BootstrapError> {
    let mut registry = SyntaxRegistry::new();

    // 1. Recursively load all .st files
    let macros = load_macros_from_dir(stdlib_path)?;

    // 2. Register each macro's %form pattern
    for macro_def in macros {
        registry.register(&macro_def);
    }

    Ok(registry)
}
```

**Skipped Files:**

- `examples/` directories - contain user code, not metasystem definitions
- `*.test.st` files - test files that use macros, don't define them

**Error Handling:**

Parse errors in individual files are logged but don't stop the bootstrap:

```rust
// Continue loading other files even if one fails
let _ = load_macros_from_file(&path, macros);
```

This allows stdlib loading to succeed even with experimental/broken files.

## 3. The 5-Layer Pipeline

The syntax system is Layer 0 and 1 of the full compilation pipeline:

### Layer 0: Bootstrap → SyntaxRegistry

**Input:** `stdlib/*.st` files
**Output:** `SyntaxRegistry`
**Module:** `src/syntax/bootstrap.rs`

```rust
let registry = bootstrap_stdlib(Path::new("./stdlib"))?;
```

Loads all macro definitions and builds the form registry.

### Layer 1: Parse → Vec&lt;FormMatch&gt;

**Input:** Source code + SyntaxRegistry
**Output:** `Vec<FormMatch>`
**Module:** `src/syntax/events/` (MatchSink)

```rust
let (matches, diagnostics) = events::parse_matches(source, &registry);
```

Event-based parser lexes, parses, and matches in a single pass via MatchSink.

### Layer 2: Resolve → Vec&lt;ResolvedPrimitive&gt;

**Input:** `Vec<FormMatch>` + MetaRegistry
**Output:** `Vec<ResolvedPrimitive>`
**Module:** `src/pipeline/resolve.rs`

```rust
pub struct ResolvedPrimitive {
    pub primitive_name: String,
    pub args: BoundArgs,
    pub phase: Phase,
    pub order: u32,
    pub selector: Option<String>,
}
```

Applies `%binds` clauses from macros to determine:
- Which primitive(s) to call
- Argument bindings (capture name → parameter name)
- Execution phase (Global, Selector, etc.)
- Execution order

### Layer 3: Sort → Sorted by (phase, order)

**Input:** `Vec<ResolvedPrimitive>`
**Output:** `Vec<ResolvedPrimitive>` (sorted)
**Module:** `src/pipeline/sort.rs`

```rust
primitives.sort_by_key(|p| (p.phase, p.order));
```

Ensures correct execution order:
1. Global phase first
2. Selector phase second
3. Within each phase, lower order runs first

### Layer 4: Expand → Vec&lt;IRFragment&gt;

**Input:** `Vec<ResolvedPrimitive>` + MetaRegistry
**Output:** `Vec<IRFragment>`
**Module:** `src/pipeline/expand.rs`

```rust
pub struct IRFragment {
    pub kind: FragmentKind, // CSS, JS, HTML
    pub content: String,
    pub order: u32,
    pub selector: Option<String>,
}
```

Evaluates primitive `%emit` blocks:
- Substitutes bound arguments (e.g., `%name` → `"count"`)
- Generates IR code (CSS, JS, HTML)

### Layer 5: Emit → String output

**Input:** `Vec<IRFragment>`
**Output:** `PipelineOutput { js, css, html }`
**Module:** `src/pipeline/emit.rs`

```rust
pub struct PipelineOutput {
    pub js: String,
    pub css: String,
    pub html: String,
}
```

Concatenates fragments by kind and adds runtime boilerplate.

## 4. Defining New Syntax

To add new syntax, create a `.st` file in `stdlib/`:

### Example: Local State (`$variable`)

**File:** `stdlib/syntax/local-state.st`

```spacetime
%macro local-state {
  // Define the syntax pattern
  %form { $name:ident $type:ident : $value:expr ; }

  // Bind to primitive implementation
  %binds {
    local-state-impl(name: $name, initial: $value)
  }
}

// Primitive implementation
%primitive local-state-impl(name: ident, initial: any) {
  %emit js(order: 1) {
    if (!window._localState) window._localState = {};
    window._localState[%name] = %initial;
  }
}
```

**Result:**
- Input: `$count number: 0;`
- Parsed as: `FormMatch { macro_name: "local-state", captures: { name: "count", value: "0" } }`
- Resolved: `local-state-impl(name: "count", initial: 0)`
- Emitted: `window._localState["count"] = 0;`

### Example: Element Reference (`&element`)

**File:** `stdlib/syntax/element-ref.st`

```spacetime
%macro element-ref {
  // Define the syntax pattern
  %form { &name:ident $selector:selector ; }

  // Bind to primitive with &self reference
  %binds {
    element-ref-impl(&self, name: $name, selector: $selector)
  }
}

%primitive element-ref-impl(&self, name: ident, selector: selector) {
  %emit css {
    :root {
      --st-%name-width: 0px;
      --st-%name-height: 0px;
    }
  }

  %emit js(order: 2) {
    const el = document.querySelector(%selector);
    // Set up ResizeObserver...
  }
}
```

**Result:**
- Input: `&header .header;`
- Parsed as: `FormMatch { macro_name: "element-ref", captures: { name: "header", selector: ".header" } }`
- Emitted: CSS vars + JS observer setup

### Example: Directive with Parameters

**File:** `stdlib/macros/fade-in.st`

```spacetime
%macro fade-in {
  %form {
    @fade-in(
      duration: $duration:time = 600ms,
      threshold: $threshold:number = 0.1,
      distance: $distance:length = 20px
    )
  }

  %binds {
    intersection(&self, threshold: $threshold, once: true) -> { $visible }
  }

  %when $visible {
    // Animation implementation
  }
}
```

**Features:**
- Named parameters with defaults
- Multiple capture types (time, number, length)
- Conditional blocks via `%when`

## 5. Architecture Status

The migration to `FormMatch` as the sole AST representation is complete. All legacy
special-case types (`ValueDeclaration`, `ElementRefDecl`, `MacroCallAst`) have been
removed. The syntax pipeline is the only parsing path:

- **SyntaxRegistry** indexes all `%form` patterns by prefix
- **MatchSink** (event-based parser) produces `FormMatch` values
- **FormMatch** is the universal AST type consumed by the pipeline
- No hardcoded parsers remain — all syntax is defined in `.st` files

## 6. Key Concepts

### %form Pattern Syntax

```
%form {
  prefix
  inline_element1
  inline_element2
  (param1, param2)
  { body_capture }
}
```

**Elements:**

- **Prefix:** First token (e.g., `@data`, `$`, `&`)
- **Inline elements:** Captures or literals (e.g., `:`, `=`)
- **Params:** Parenthesized parameters (optional)
- **Body capture:** Block content (optional)

### Capture Types

| Type | Example | Parsed As |
|------|---------|-----------|
| `:ident` | `count` | `Ident("count")` |
| `:string` | `"/api"` | `String("/api")` |
| `:number` | `0.5` | `Number(0.5)` |
| `:bool` | `true` | `Bool(true)` |
| `:time` | `500ms` | `Time(500)` |
| `:length` | `20px` | `Length(20.0, "px")` |
| `:selector` | `.hero` | `Selector(".hero")` |
| `:binding` | `$data` | `Binding("$data")` |
| `:element` | `&btn` | `Element("&btn")` |
| `:expr` | `x + 1` | `Expr("x + 1")` |
| `:typeref` | `Product[]` | `TypeRef("Product[]")` |

### %binds Clause

Maps form captures to primitive parameters:

```spacetime
%form { $name:ident : $value:expr }

%binds {
  local-state-impl(
    name: $name,      // Capture → parameter
    initial: $value
  )
}
```

### Default Values

Captures can have defaults:

```spacetime
%form {
  @fade-in(
    duration: $duration:time = 600ms,
    threshold: $threshold:number = 0.1
  )
}
```

Allows: `@fade-in(duration: 1s)` (threshold uses default)

## 7. Performance Characteristics

- **Prefix lookup:** O(1) - direct HashMap access
- **Form matching:** O(n) where n = forms for that prefix
- **Specificity ordering:** Most specific forms tried first
- **Bootstrap:** One-time cost at compiler initialization
- **Pattern matching:** Linear scan with early exit on match

## 8. Error Handling

### Parse Errors

When no form matches:

```rust
let (matches, diagnostics) = events::parse_matches(input, &registry);
// diagnostics contains info about unmatched statements
```

### Bootstrap Errors

Files with parse errors are skipped:

```rust
let _ = load_macros_from_file(&path, macros);
// Continue loading other files
```

### Validation

Forms are validated during registration:

- Must have a prefix
- Captures must have valid types
- No duplicate capture names
- %binds must reference existing primitives

## 9. Testing

### Unit Tests

Each component has comprehensive tests:

- `src/syntax/registry.rs`: Form registration and lookup
- `src/syntax/events/`: Event-based parser, MatchSink, extractors
- `src/syntax/bootstrap.rs`: File loading and macro extraction

### Integration Tests

`src/pipeline/integration_tests.rs`:

```rust
#[test]
fn test_full_pipeline() {
    let registry = bootstrap_stdlib(Path::new("./stdlib"))?;
    let (matches, _) = events::parse_matches(source, &registry);
    let output = compile(&matches, &context)?;

    assert!(output.js.contains("expected code"));
}
```

### Example Tests

```rust
#[test]
fn test_local_state_parsing() {
    let input = "$count number: 0;";
    let (matches, _) = events::parse_matches(input, &registry);
    let matched = matches.first().unwrap();

    assert_eq!(matched.macro_name, "local-state");
    assert_eq!(matched.get_ident("name"), Some("count"));
}
```

## 10. Future Enhancements

### Planned Features

1. **Recursive block parsing** - Parse nested FormMatches in block captures
2. **Custom capture types** - Allow .st files to define new capture types
3. **Pattern composition** - Reuse patterns across forms
4. **Better error messages** - Show which forms were tried and why they failed
5. **Form validation** - Compile-time checks for invalid patterns

### Optimization Opportunities

1. **Lazy bootstrap** - Only load forms when needed
2. **Compiled patterns** - Pre-compile regex patterns for captures
3. **Prefix trie** - Use trie instead of HashMap for multi-char prefixes
4. **Parallel parsing** - Parse multiple statements concurrently

---

## Summary

The syntax pipeline achieves universal syntax definition through:

1. **%form patterns** define all syntax in `.st` files
2. **SyntaxRegistry** provides O(1) prefix-based lookup
3. **FormMatch** serves as the universal AST type
4. **Pattern matching** replaces hardcoded parsers
5. **Bootstrap** loads stdlib and builds the registry

This architecture enables:
- Adding new syntax without touching Rust code
- Consistent parsing for all constructs
- Easy migration from legacy parsers
- Better extensibility and maintainability
