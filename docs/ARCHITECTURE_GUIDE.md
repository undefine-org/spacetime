# Spacetime Architecture Guide: A Reverse-Engineering Perspective

This guide explains Spacetime's architecture by working backwards from the final output (JavaScript, CSS, GLSL, HTML strings) to the original source code. This reverse-engineering approach helps developers understand how the pieces fit together by following the data flow in reverse.

```
+------------------+     +------------------+     +------------------+
|   Layer 5        |     |   Layer 4        |     |   Layer 3        |
|   EMIT           |<----|   IR             |<----|   EXPAND         |
|                  |     |                  |     |                  |
|  js::emit()      |     |  JsExpr          |     |  expand_macro()  |
|  css::emit()     |     |  CssExpr         |     |  MacroContext    |
|  glsl::emit()    |     |  GlslExpr        |     |  MetaRegistry    |
|  html::emit()    |     |  CodeFragment    |     |  codegen         |
+------------------+     +------------------+     +------------------+
        ^                        ^                        ^
        |                        |                        |
+------------------+     +------------------+
|   Layer 2        |     |   Layer 1        |
|   PARSE          |<----|   SOURCE         |
|                  |     |                  |
|  grammar.pest    |     |  .st files       |
|  StFile          |     |  % $ & symbols   |
|  MacroCallAst    |     |  @directives     |
|  ScopeBlock      |     |                  |
+------------------+     +------------------+
```

---

## Layer 5: Emit (`src/emit/`)

**What comes out**: Final JavaScript, CSS, GLSL, and HTML strings ready for the browser.

The emit layer is the ONLY place where IR types get converted to final output strings. All stringification is centralized here for:
- Consistent formatting
- Easy minification toggle
- Future extensibility (source maps, etc.)

### EmitOptions

```rust
pub struct EmitOptions {
    pub minify: bool,     // Remove whitespace, shorten names
    pub indent: String,   // Indentation string (ignored if minify=true)
    pub newline: String,  // Line ending (ignored if minify=true)
}

// Factory methods
EmitOptions::pretty()   // Human-readable output
EmitOptions::minified() // Production-ready compact output
```

### Emitters

Each language has its own emitter module:

| Module | Function | Input | Output |
|--------|----------|-------|--------|
| `js.rs` | `emit_expr()`, `emit_stmt()`, `emit_stmts()` | `JsExpr`, `JsStmt` | JavaScript code |
| `css.rs` | `emit()`, `emit_all()` | `CssExpr` | CSS rules |
| `glsl.rs` | `emit()` | `GlslExpr` | GLSL shader code |
| `html.rs` | `emit()`, `emit_all()` | `HtmlExpr` | HTML markup |

### JavaScript Emission Example

```rust
// In js.rs
pub fn emit_expr(expr: &JsExpr, opts: &EmitOptions) -> String {
    match expr {
        JsExpr::Lit(lit) => emit_lit(lit),
        JsExpr::Var(name) => name.clone(),
        JsExpr::Call { callee, args } => {
            let args_str = args.iter()
                .map(|a| emit_expr(a, opts))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}({})", emit_expr(callee, opts), args_str)
        }
        // ... other variants
    }
}
```

### Fragment Dispatch

The top-level `emit_fragment()` function routes code fragments to the appropriate emitter:

```rust
pub fn emit_fragment(fragment: &CodeFragment, opts: &EmitOptions) -> String {
    match &fragment.kind {
        FragmentKind::Js(stmts) => js::emit_stmts(stmts, opts),
        FragmentKind::Css(exprs) => css::emit_all(exprs, opts),
        FragmentKind::Glsl(expr) => glsl::emit(expr, opts),
        FragmentKind::Html(exprs) => html::emit_all(exprs, opts),
    }
}
```

### CompiledOutput

The final assembled output:

```rust
pub struct CompiledOutput {
    pub js: String,
    pub css: String,
    pub glsl: Option<String>,
    pub html: Option<String>,
}
```

---

## Layer 4: IR (`src/ir/`)

**Purpose**: Intermediate Representation types that enable deferred stringification and introspection.

```
Expand -> IR -> Emit (strings generated only at emit stage)
```

### Why IR?

IR types provide:
1. **Introspection**: Code can be analyzed before becoming strings
2. **Transformation**: Optimizations and modifications without string manipulation
3. **Deferred Stringification**: Format decisions made at final emit time
4. **Type Safety**: Structured representation catches errors early

### JsExpr Variants

```rust
pub enum JsExpr {
    // Literals
    Lit(JsLit),                                    // "hello", 42, true

    // References
    Var(String),                                   // foo
    Prop { obj: Box<JsExpr>, prop: String },       // obj.prop
    Index { arr: Box<JsExpr>, idx: Box<JsExpr> },  // arr[idx]

    // Calls
    Call { callee: Box<JsExpr>, args: Vec<JsExpr> },
    Method { obj: Box<JsExpr>, method: String, args: Vec<JsExpr> },

    // Functions
    Arrow { params: Vec<String>, body: Box<JsExpr> },
    Block { stmts: Vec<JsStmt>, expr: Option<Box<JsExpr>> },

    // Templates
    Template { parts: Vec<TemplatePart> },         // `Hello ${name}`

    // Operations
    Binary { left: Box<JsExpr>, op: BinOp, right: Box<JsExpr> },
    Unary { op: UnaryOp, expr: Box<JsExpr> },
    Ternary { cond: Box<JsExpr>, then_: Box<JsExpr>, else_: Box<JsExpr> },

    // Data structures
    Object(Vec<(String, JsExpr)>),
    Array(Vec<JsExpr>),

    // Escape hatch
    Raw(String),                                   // Passthrough for complex cases
}
```

### When Each JsExpr Variant is Used

| Variant | Use Case | Example Output |
|---------|----------|----------------|
| `Lit` | Constants | `"hello"`, `42`, `true` |
| `Var` | Variable references | `progress` |
| `Prop` | Property access | `el.style` |
| `Method` | Method calls | `el.addEventListener(...)` |
| `Arrow` | Inline callbacks | `e => e.target.value` |
| `Template` | String interpolation | `` `${x}px` `` |
| `Binary` | Math/logic | `progress * 100` |
| `Raw` | Complex JS passthrough | WebGL code |

### JsStmt Variants

```rust
pub enum JsStmt {
    Decl { kind: DeclKind, name: String, init: Option<JsExpr> },
    Expr(JsExpr),
    If { cond: JsExpr, then_: Vec<JsStmt>, else_: Option<Vec<JsStmt>> },
    ForOf { var: String, iter: JsExpr, body: Vec<JsStmt> },
    For { init: Option<Box<JsStmt>>, cond: Option<JsExpr>, update: Option<JsExpr>, body: Vec<JsStmt> },
    Return(Option<JsExpr>),
    Break,
    Continue,
    Raw(String),
}
```

### CSS and Other IR Types

```rust
pub enum CssExpr {
    Rule { selector: String, declarations: Vec<CssDecl> },
    Keyframes { name: String, frames: Vec<KeyframeBlock> },
    Raw(String),
}

pub enum GlslExpr {
    Program { vertex: String, fragment: String },
    Raw(String),
}

pub enum HtmlExpr {
    Element { tag: String, attrs: Vec<(String, String)>, children: Vec<HtmlExpr> },
    Text(String),
    Raw(String),
}
```

### CodeFragment

Wraps IR with metadata:

```rust
pub struct CodeFragment {
    pub kind: FragmentKind,
    pub deps: Vec<String>,  // Signal dependencies
}

pub enum FragmentKind {
    Js(Vec<JsStmt>),
    Css(Vec<CssExpr>),
    Glsl(GlslExpr),
    Html(Vec<HtmlExpr>),
}
```

---

## Layer 3: Expand (`src/metasystem/`)

**Purpose**: Macro expansion via `%binds`, `%derives`, `%states`, and primitive code generation.

### Architecture Overview

```
Spacetime Source -> Parse -> Macro Expansion -> Declaration Graph -> Codegen
                                   |                    |
                              %binds {}           Primitives
                              %derives {}         -> JS code
                              %states {}          -> CSS
```

### MetaRegistry

The registry holds all primitive and macro definitions:

```rust
pub struct MetaRegistry {
    primitives: HashMap<String, PrimitiveDefAst>,
    macros: HashMap<String, MacroDefAst>,
    presets: HashMap<String, MetaPresetDefAst>,
}

impl MetaRegistry {
    pub fn get_primitive(&self, name: &str) -> Option<&PrimitiveDefAst>;
    pub fn get_macro(&self, name: &str) -> Option<&MacroDefAst>;
    pub fn get_macro_by_creates(&self, directive_name: &str) -> Option<&MacroDefAst>;
    pub fn load_stdlib_from_dir(&mut self, dir: &Path) -> Result<(), MetaRegistryError>;
}
```

### MacroContext

Context for macro expansion with variable bindings:

```rust
pub struct MacroContext<'a> {
    registry: &'a MetaRegistry,
    bindings: HashMap<String, MacroValue>,
    stack: Vec<String>,              // For cycle detection
    current_span: SourceSpan,
    pub nested_css: Vec<String>,     // CSS from nested expansions
    pub nested_js: Vec<String>,      // JS from nested expansions
    pub scope_selector: Option<String>,
}
```

### MacroValue

Values during macro expansion:

```rust
pub enum MacroValue {
    String(String),
    Number(f64),
    Bool(bool),
    Ident(String),
    List(Vec<MacroValue>),
    Properties(Vec<(String, String)>),
    MacroBody(MacroCallBody),
    Expr(String),
}
```

### The expand_macro Function

```rust
pub fn expand_macro(
    ctx: &mut MacroContext,
    macro_name: &str,
    args: &[PatternArg],
    span: &SourceSpan,
) -> Result<ExpandedDirectives, MacroExpansionError> {
    // 1. Look up macro definition
    let macro_def = ctx.registry.get_macro(macro_name)
        .or_else(|| ctx.registry.get_macro_by_creates(macro_name))?;

    // 2. Enter macro (cycle detection)
    ctx.enter_macro(macro_name)?;

    // 3. Bind arguments from form pattern
    ctx.bind_args(&macro_def, args)?;

    // 4. Process %binds -> invoke primitives
    for bind in &macro_def.binds {
        process_bind(ctx, bind)?;
    }

    // 5. Process %derives
    for derive in &macro_def.derives {
        process_derive(ctx, derive)?;
    }

    // 6. Expand body items
    let mut result = ExpandedDirectives::new();
    for item in &macro_def.body {
        result.extend(expand_body_item(ctx, item)?);
    }

    ctx.exit_macro();
    Ok(result)
}
```

### Primitive Code Generation (codegen.rs)

Transforms `%primitive` definitions into JavaScript:

```rust
// Transformation rules:
// | Placeholder       | Replacement                    |
// |-------------------|--------------------------------|
// | `%&el`            | `el` (element reference)       |
// | `%param`          | Literal value from binding     |
// | `%yield expr -> $var` | `ST.set(el, 'var', expr)` |
// | `%cleanup { ... }`    | Collected for cleanup fn  |

pub fn generate_primitive_js(
    primitive: &PrimitiveDefAst,
    args: &PrimitiveArgs,
) -> GeneratedPrimitive {
    // Process emit blocks
    // Transform placeholders
    // Extract cleanup code
    // Return setup JS, CSS, cleanup, and exports
}
```

### How %emit Blocks Work

Primitives contain `%emit` blocks that generate JS/CSS:

```spacetime
%primitive scroll(&el, axis: ("x" | "y" | "both") = "y") {
  %emit js {
    const el = %&el;                    // -> const el = el;
    const handleScroll = () => {
      %yield scrollY -> $y;             // -> ST.set(el, 'y', scrollY);
    };
    target.addEventListener('scroll', handleScroll);

    %cleanup {
      target.removeEventListener('scroll', handleScroll);
    }
  }

  %exports {
    $y: number
    $progress: number
  }
}
```

---

## Layer 2: Parse (`src/parser/`)

**Purpose**: Convert `.st` text files into AST structures.

### The Grammar (grammar.pest)

Spacetime uses PEG (Parsing Expression Grammar) via the Pest library:

```pest
// Top-level file structure
file = { SOI ~ (import | preset_def | pattern_def | meta_def |
               fn_def | local_def | file_level_macro_call | scope_block)* ~ EOI }

// Scope blocks: .selector { content }
scope_block = { selector ~ "{" ~ scope_content ~ "}" }

// Metasystem definitions
meta_def = { primitive_def | macro_def | meta_preset_def }
primitive_def = { "%primitive" ~ identifier ~ "(" ~ params ~ ")" ~ "{" ~ body ~ "}" }
macro_def = { "%macro" ~ identifier ~ "{" ~ macro_body ~ "}" }

// Generic macro calls: @directive args { body }
generic_macro_call = {
    "@" ~ macro_call_name
    ~ macro_call_inline_args?
    ~ macro_call_as_clause?
    ~ macro_call_block?
}
```

### AST Types (ast.rs)

**Syntactic AST types** - capture syntax only, not semantics:

```rust
/// Root of a parsed .st file
pub struct StFile {
    pub imports: Vec<String>,
    pub presets: Vec<PresetDef>,
    pub patterns: Vec<PatternDef>,
    pub meta_defs: Vec<meta_ast::MetaDef>,
    pub types: Vec<TypeDef>,
    pub data: Vec<DataDef>,
    pub computed: Vec<ComputedDef>,
    pub functions: Vec<FnDef>,
    pub scopes: Vec<ScopeBlock>,
    pub macro_calls: Vec<MacroCallAst>,
    pub span: SourceSpan,
}
```

### MacroCallAst

Generic capture of any `@directive` syntax:

```rust
pub struct MacroCallAst {
    pub name: String,                              // e.g., "scroll", "load"
    pub variant: Option<String>,                   // for @name.variant syntax
    pub args: Vec<MacroCallArg>,                   // inline arguments
    pub as_clause: Option<(String, Option<String>)>, // "as name: Type"
    pub body: Option<MacroCallBody>,               // block content
    pub span: SourceSpan,
}

pub enum MacroCallArg {
    String(String),        // "hello"
    Selector(String),      // .class, #id
    Ident(String),         // click, hover
    Number(f64),           // 100, 3.14
    Time(u32),             // 500ms, 2s
    Named(String, Box<MacroCallArg>),  // name: value
    Keyword(String),       // should, returns
    Array(Vec<MacroCallArg>),
    Expr(String),          // (1 + 1 === 2)
}
```

### ScopeBlock

Represents `.selector { content }` blocks:

```rust
pub struct ScopeBlock {
    pub selector: String,
    pub behavior: BehaviorBlock,
    pub each_blocks: Vec<EachBlock>,
    pub css_declarations: Vec<CssDeclaration>,
    pub nested_scopes: Vec<NestedScope>,
    pub on_mutations: Vec<OnMutationBlock>,
    pub element_ref_decls: Vec<ElementRefDecl>,
    pub value_declarations: Vec<ValueDeclaration>,
    pub macro_calls: Vec<MacroCallAst>,
    pub span: SourceSpan,
}
```

### How @scroll Becomes MacroCallAst

```
Source:                           Parsed AST:
----------------------------------------
@scroll intro(duration: 1s) {    MacroCallAst {
  opacity: 0 -> 1                  name: "scroll",
}                                  args: [
                                     Named("duration", Time(1000))
                                   ],
                                   body: Some(MacroCallBody {
                                     properties: [
                                       ("opacity", "0 -> 1")
                                     ],
                                     macro_calls: [],
                                   }),
                                 }
```

---

## Layer 1: Source (`.st` files)

**Purpose**: Human-authored Spacetime code.

### The Three Symbols

Spacetime uses three special symbols:

| Symbol | Name | Purpose | Example |
|--------|------|---------|---------|
| `%` | Meta | Compile-time definitions | `%primitive`, `%macro`, `%emit` |
| `$` | Signal | Reactive data | `$progress`, `$x`, `$visible` |
| `&` | Reference | Element references | `&el`, `&self`, `&title` |

### % Meta System

The `%` symbol defines compile-time constructs:

```spacetime
// Primitive definition (emits JS/CSS at compile time)
%primitive scroll(&el, axis: ("x" | "y") = "y") {
  %emit js {
    el.addEventListener('scroll', () => {
      %yield el.scrollTop -> $y;
    });
  }
  %exports {
    $y: number
  }
}

// Macro definition (composes primitives)
%macro scroll-timeline {
  %creates @scroll

  %form {
    @scroll $name:ident(start: $start:number = 0) {
      $body:keyframes
    }
  }

  %binds {
    scroll-driver(&self, start: $start) -> { $progress }
    apply-animations(&self, animations: $body)
  }
}
```

### $ Signals

Reactive values that flow through the system:

```spacetime
// Primitive exports signals
%exports {
  $progress: number
  $visible: bool
}

// Signals in scope blocks
.hero {
  $count number: 0;    // Value declaration

  @load intro(duration: 1s) {
    opacity: 0 -> 1
  }
}
```

### & References

Element references for targeting DOM nodes:

```spacetime
.container {
  // Reference declaration
  &title [slot=title];
  &description [slot=description] {
    font-size: 1.2em;
  }

  // Using references
  @on &.click {
    &title.textContent = "Clicked!";
  }
}
```

### User-Facing Syntax Examples

**Timeline Animation (scroll-driven)**:
```spacetime
.hero-image {
  @scroll parallax(start: 0, end: 1, scrub: true) {
    opacity: 0 -> 1
    translate-y: 50px -> 0
  }
}
```

**Load Animation**:
```spacetime
.fade-in {
  @load intro(delay: 200ms, duration: 600ms) {
    opacity: 0 -> 1
    translate-y: 20px -> 0
  }
}
```

**Reactive State**:
```spacetime
$editing <- false

.card {
  .card__body { .is-editing: $editing == true; }
  .edit-btn { @on &.click { $editing <- true; } }
  .done-btn { @on &.click { $editing <- false; } }
}

.card__body.is-editing {
  outline: 2px dashed teal;
}
```

**Data Binding**:
```spacetime
@data Products: Product[] {
  source: "/api/products.json"
  transform: json
  cache: 5m
}

.product-list {
  @each $products {
    template: <product-card>;

    [slot=name]: $.name;
    [slot=price]: $.price | currency;
  }
}
```

---

## Follow a Feature: @scroll from Source to Output

Let's trace how `@scroll` works end-to-end:

### 1. Source Code (Layer 1)

```spacetime
// stdlib/macros/timeline.st
%macro scroll-timeline {
  %creates @scroll

  %form {
    @scroll $name:ident(start: $start:number = 0, end: $end:number = 0) {
      $body:keyframes
    }
  }

  %binds {
    scroll-driver(&self, name: $name, start: $start, end: $end) -> { $progress }
    apply-animations(&self, driver: $name, animations: $body)
  }
}
```

User writes:
```spacetime
.hero {
  @scroll intro(start: 0, end: 1) {
    opacity: 0 -> 1
  }
}
```

### 2. Parsing (Layer 2)

The parser produces:
```rust
ScopeBlock {
    selector: ".hero",
    macro_calls: vec![
        MacroCallAst {
            name: "scroll",
            args: vec![
                MacroCallArg::Ident("intro"),
                MacroCallArg::Named("start", MacroCallArg::Number(0.0)),
                MacroCallArg::Named("end", MacroCallArg::Number(1.0)),
            ],
            body: Some(MacroCallBody {
                properties: vec![("opacity", "0 -> 1")],
                ..
            }),
        }
    ],
}
```

### 3. Macro Expansion (Layer 3)

```rust
// expand_macro("scroll", args, span)

// 1. Look up macro by %creates @scroll
let macro_def = registry.get_macro_by_creates("scroll")?;  // -> scroll-timeline

// 2. Bind form parameters
ctx.set_binding("name", MacroValue::Ident("intro"));
ctx.set_binding("start", MacroValue::Number(0.0));
ctx.set_binding("end", MacroValue::Number(1.0));
ctx.set_binding("body", MacroValue::Properties([("opacity", "0 -> 1")]));

// 3. Process %binds
// First bind: scroll-driver primitive
process_bind(ctx, BindDecl {
    primitive: "scroll-driver",
    args: [
        BindArg::Element("&self"),
        BindArg::Named("name", BindValue::Variable("name")),
        BindArg::Named("start", BindValue::Variable("start")),
        // ...
    ],
    outputs: [BindOutput { name: "progress" }],
})?;

// Second bind: apply-animations primitive
process_bind(ctx, BindDecl {
    primitive: "apply-animations",
    args: [
        BindArg::Named("animations", BindValue::Variable("body")),
        // ...
    ],
})?;
```

### 4. Primitive Code Generation (Layer 3 -> Layer 4)

```rust
// In codegen.rs
let args = PrimitiveArgs::new()
    .element("el", "el")
    .param("name", "intro")
    .param("start", "0")
    .param("end", "1");

let generated = generate_primitive_js(&scroll_driver_primitive, &args);
// generated.setup contains JS with %placeholders replaced
```

The scroll-driver primitive's `%emit js` block:
```javascript
// Before transformation:
const opts = { start: %start, end: %end };
const obs = new IntersectionObserver(...);
%yield progress -> $%name;  // Note: %name gets substituted

// After transformation:
const opts = { start: 0, end: 1 };
const obs = new IntersectionObserver(...);
ST.set(el, 'intro', progress);  // Signal named after the timeline
```

### 5. IR Generation (Layer 4)

The generated JS strings could be wrapped in IR (for future optimization):
```rust
CodeFragment {
    kind: FragmentKind::Js(vec![
        JsStmt::Raw(generated.setup),
    ]),
    deps: vec!["intro"],  // This fragment depends on $intro signal
}
```

### 6. Emit (Layer 5)

```rust
let opts = EmitOptions::pretty();
let output = emit_fragment(&fragment, &opts);

// Final JavaScript string
println!("{}", output);
```

Final output:
```javascript
(function initScope() {
  const selector = '.hero';

  function tryInit() {
    const els = document.querySelectorAll(selector);
    if (els.length === 0) return false;

    els.forEach(el => {
      // scroll-driver setup
      const opts = { start: 0, end: 1 };
      const obs = new IntersectionObserver((entries) => {
        const progress = /* calculation */;
        ST.set(el, 'intro', progress);
      }, opts);
      obs.observe(el);

      // apply-animations setup
      ST.animate(el, 'intro', {
        properties: [{ property: 'opacity', keyframes: [{at: 0, value: '0'}, {at: 1, value: '1'}] }]
      });
    });
    return true;
  }

  if (tryInit()) return;

  const observer = new MutationObserver((_, obs) => {
    if (tryInit()) obs.disconnect();
  });
  observer.observe(document.documentElement, { childList: true, subtree: true });
})();
```

---

## Data Flow Diagram (Complete)

```
                        .st SOURCE FILE
                              |
                              v
+-------------------------------------------------------------------------+
|                         LAYER 2: PARSE                                   |
|                                                                          |
|   grammar.pest  ------>  pest::Parser  ------>  Raw Parse Tree          |
|                              |                                           |
|                              v                                           |
|                      AST Construction                                    |
|                              |                                           |
|         +--------------------+--------------------+                      |
|         |                    |                    |                      |
|         v                    v                    v                      |
|   ScopeBlock           MetaDef              MacroCallAst                 |
|   (selectors)      (% primitives)         (@ directives)                 |
+-------------------------------------------------------------------------+
                              |
                              v
+-------------------------------------------------------------------------+
|                       LAYER 3: EXPAND                                    |
|                                                                          |
|   MetaRegistry                                                           |
|      |                                                                   |
|      +-- primitives: HashMap<String, PrimitiveDefAst>                    |
|      +-- macros: HashMap<String, MacroDefAst>                            |
|                                                                          |
|   MacroContext                                                           |
|      |                                                                   |
|      +-- bindings: HashMap<String, MacroValue>                           |
|      +-- nested_css, nested_js: Vec<String>                              |
|                              |                                           |
|                              v                                           |
|                       expand_macro()                                     |
|                              |                                           |
|         +--------------------+--------------------+                      |
|         |                    |                    |                      |
|         v                    v                    v                      |
|     %binds             %derives              %states                     |
|   process_bind()     process_derive()    generate_state_css()            |
|         |                    |                    |                      |
|         v                    v                    v                      |
|   generate_primitive_js()                                                |
|         |                                                                |
|         +-- transform_params()     %param -> value                       |
|         +-- transform_element_refs()  %&el -> el                         |
|         +-- transform_yields()     %yield x -> $y  -> ST.set(el,'y',x)   |
+-------------------------------------------------------------------------+
                              |
                              v
+-------------------------------------------------------------------------+
|                         LAYER 4: IR                                      |
|                                                                          |
|   JsExpr, JsStmt, CssExpr, GlslExpr, HtmlExpr                            |
|                              |                                           |
|                              v                                           |
|   CodeFragment { kind: FragmentKind::Js(...), deps: [...] }              |
+-------------------------------------------------------------------------+
                              |
                              v
+-------------------------------------------------------------------------+
|                        LAYER 5: EMIT                                     |
|                                                                          |
|   EmitOptions { minify, indent, newline }                                |
|                              |                                           |
|                              v                                           |
|   emit_fragment() -----> match FragmentKind:                             |
|                          - Js  -> js::emit_stmts()                       |
|                          - Css -> css::emit_all()                        |
|                          - Glsl -> glsl::emit()                          |
|                          - Html -> html::emit_all()                      |
|                              |                                           |
|                              v                                           |
|   CompiledOutput { js: String, css: String, glsl: Option, html: Option } |
+-------------------------------------------------------------------------+
                              |
                              v
                      FINAL OUTPUT (JS/CSS/GLSL/HTML strings)
```

---

## Key Takeaways

1. **Separation of Concerns**: Each layer has a single responsibility:
   - Parse: Text -> AST
   - Expand: AST -> Generated Code (via primitives)
   - IR: Structured representation
   - Emit: IR -> Strings

2. **Primitives are the Foundation**: All runtime behavior comes from `%primitive` definitions. Macros compose primitives but don't emit code directly.

3. **Macros are Compile-Time**: The `%` metasystem runs at compile time. At runtime, only primitive-generated JS/CSS executes.

4. **Signals Flow Through**: The `$` reactive system connects primitives. One primitive yields signals (`%yield x -> $progress`), another consumes them.

5. **The Registry is Central**: `MetaRegistry` holds all primitive and macro definitions. Loading stdlib populates this registry.

6. **Generic Parsing**: All `@` directives parse as `MacroCallAst`. The metasystem determines their meaning by looking up `%creates @directive` clauses.
