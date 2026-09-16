# Metasystem Architecture

**The metasystem is Spacetime's compile-time macro system.** It defines how
`%primitive` and `%macro` definitions produce JavaScript and CSS at build time.
The `%` system is not present in runtime output -- it only governs code generation.

---

## Overview

Spacetime source files contain `@directive` macro calls (e.g., `@on-hover`,
`@cycle`, `@template`) inside CSS selector scopes. At compile time, these
directives are matched against `%macro` definitions loaded from the stdlib,
which in turn reference `%primitive` definitions that contain `%emit js { }`
and `%emit css { }` blocks. The compiler expands macros, resolves parameters,
generates IR, and emits the final JavaScript and CSS.

As of the pipeline unification (PLAN-001), there is ONE compilation path:

1. **Unified pipeline** -- ALL `@directive` calls (both selector-scoped and
   file-level like `@template`, `@test`, `@fixture`) flow through the 4-layer
   pipeline: Resolve → Sort → Expand → Emit.

   File-level directives are assigned `Phase::Global` (no element binding)
   and emit without selector wrapping. The metasystem `expand_macro()` is no
   longer called from `compiler.rs`.

   The metasystem's `expand_macro()` still exists for handling `%for`, `%states`,
   `%derives`, `%includes` body items — these are deferred to a future Evaluate
   layer (Layer 0 before Resolve).

```text
                        Spacetime Source (.st)
                               |
                            Parse
                               |
                          FormMatch[]
                        (all directives)
                               |
                     Unified Pipeline
                     ┌─────────────┐
                     │  Resolve     │  FormMatch → ResolvedPrimitive
                     │  Sort        │  Topological (phase, order)
                     │  Expand      │  ResolvedPrimitive → ExpandedPrimitive (typed IR)
                     │  Emit        │  ExpandedPrimitive → JS/CSS strings
                     └─────────────┘
                           |
                    CompiledSpacetime
                      { js, css }
```

---

## Module Structure

### `src/metasystem/`

| File | Purpose |
|------|---------|
| `mod.rs` | Public API: re-exports from compile, expand, registry, validate, diagnostics |
| `compile.rs` | `compile_macro_call()` -- compiles a single `@directive` via DeclarationGraph -> JS/CSS |
| `expand.rs` | `expand_macro()` -- full macro expansion with `MacroContext`, processes `%binds`/`%derives`/`%states`/`%emit`/`%on`/`%when`/`%for`/`%if` |
| `registry.rs` | `MetaRegistry` -- stores all `%primitive`, `%macro`, `%capture_type`, preset, and runtime registry definitions |
| `validate.rs` | `validate_primitive()`, `validate_macro()` -- static analysis of macro/primitive definitions |
| `diagnostics.rs` | `to_diagnostic()` -- converts metasystem errors to user-facing diagnostics with "did you mean?" suggestions |
| `incremental_cache.rs` | `IncrementalStdlibCache` -- mtime-based hot-reload of stdlib files during development |
| `relationships.rs` | `RelationshipRegistry` -- analyzes `%includes` and parameter forwarding between macros (used by LSP hover) |
| `tests.rs` | Module-level integration tests |

### `src/emit/`

| File | Purpose |
|------|---------|
| `mod.rs` | `EmitOptions`, `CompiledOutput`, `emit_fragment()` -- central dispatch for IR -> string |
| `js.rs` | `emit_stmts()`, `resolve_stmts()` -- JsStmt/JsExpr IR -> JavaScript strings |
| `css.rs` | `emit_all()`, `resolve_expr()` -- CssExpr IR -> CSS strings |
| `glsl.rs` | GlslExpr -> GLSL shader strings |
| `html.rs` | HtmlExpr -> HTML strings |
| `metasystem_codegen.rs` | `generate_primitive_ir()` -- processes `%emit` blocks in primitives into `GeneratedPrimitiveIR` (typed JS/CSS IR) |
| `js_parser.rs` | Parses JS emit block content into `JsStmt` IR with `Placeholder` nodes for `%param` references |
| `css_parser.rs` | Parses CSS emit block content into `CssExpr` IR |
| `emit_tokenizer.rs` | Shared tokenizer for emit block content |
| `sourcemap.rs` | Source Map V3 builder |
| `writer.rs` | `SourceMapWriter` -- writes output with source map tracking |

### `src/pipeline/`

| File | Purpose |
|------|---------|
| `mod.rs` | `compile()`, `compile_verbose()`, `CompileContext` -- orchestrates the 4-layer pipeline |
| `types.rs` | `FormMatch` adapter, `ResolvedPrimitive`, `ExpandedPrimitive`, `JsFragment`, `CssFragment`, `PipelineOutput`, `Phase` enum |
| `resolve.rs` | **Layer 1: Resolve** -- `resolve()`: FormMatch -> ResolvedPrimitive (macro lookup, argument binding, phase inference) |
| `sort.rs` | **Layer 2: Sort** -- `sort()`: stable sort by (Phase, order) |
| `expand.rs` | **Layer 3: Expand** -- `expand_typed()`: ResolvedPrimitive -> ExpandedPrimitive (runs `generate_primitive_ir()`, produces typed IR) |
| `emit.rs` | **Layer 4: Emit** -- `emit_typed()`: JsFragment/CssFragment -> PipelineOutput (scope-aware JS wrapping, runtime inclusion) |

### `src/compiler.rs`

The top-level `Compiler` builder and `compile_pipeline()` function that ties
everything together. Handles stdlib loading, user meta-definition registration,
file-level macro expansion, CSS passthrough, and runtime wrapping.

---

## The Pipeline (selector-scoped directives)

The pipeline compiles selector-scoped `@directive` calls that were parsed into
`FormMatch` values during syntax analysis. `FormMatch` is the unified AST type
that replaces all legacy special-case types (ValueDeclaration, ElementRefDecl,
DataDef, EachBlock, etc.).

```rust
// src/syntax/form_match.rs
pub struct FormMatch {
    pub macro_name: String,                       // e.g., "on-hover", "cycle"
    pub captures: HashMap<String, CapturedValue>, // captured parameters
    pub capture_spans: HashMap<String, SourceSpan>,
    pub selector: Option<String>,                 // CSS selector context
    pub span: SourceSpan,
}
```

### Layer 1: Resolve (`pipeline/resolve.rs`)

```rust
pub fn resolve(
    matches: &[FormMatch],
    meta_registry: &MetaRegistry,
) -> Result<Vec<ResolvedPrimitive>, ResolveError>
```

Looks up the `%macro` definition for each FormMatch, evaluates `%binds`
declarations to find which primitives to invoke, resolves captured values
into primitive arguments, infers phase (Global vs Selector) and execution
order, and validates `%requires` constraints.

### Layer 2: Sort (`pipeline/sort.rs`)

```rust
pub fn sort(mut primitives: Vec<ResolvedPrimitive>) -> Vec<ResolvedPrimitive>
```

Stable sort by `(Phase, order)`. Global-phase primitives (no `&element`
parameter) execute first; selector-phase primitives execute second. Within
each phase, the `order` field controls sequencing.

### Layer 3: Expand (`pipeline/expand.rs`)

```rust
pub fn expand_typed(
    primitives: &[ResolvedPrimitive],
    meta_registry: &MetaRegistry,
) -> Result<Vec<ExpandedPrimitive>, ExpandError>
```

For each `ResolvedPrimitive`, looks up the `%primitive` definition, calls
`generate_primitive_ir()` from `emit/metasystem_codegen.rs` to process the
`%emit js { }` and `%emit css { }` blocks. Produces typed IR (`Vec<JsStmt>`,
`Vec<CssExpr>`) with placeholder resolution -- `%param` references are
substituted with concrete values. The result is `ExpandedPrimitive` containing
`JsFragment`, `CssFragment`, exports, and build scripts.

### Layer 4: Emit (`pipeline/emit.rs`)

```rust
pub fn emit_typed(
    js_fragments: &[JsFragment],
    css_fragments: &[CssFragment],
) -> PipelineOutput
```

Converts typed IR to final output strings. JS fragments are wrapped according
to their `JsScope`:

- **IIFE**: `(function() { el_init; stmts; cleanup; })();` -- isolated scope
- **Block**: `{ el_init; stmts; cleanup; }` -- block-scoped
- **Inline**: raw statements, cleanup handled globally

Element initialization (`ElInit`) determines how `el` is bound:
- `Body` -> `const el = document.body;`
- `Selector(s)` -> `document.querySelectorAll(s).forEach(el => { ... });`
- `None` -> no element binding

CSS fragments are emitted via the CSS emitter (`emit/css.rs`).

```rust
pub fn emit_typed_with_runtime(
    js_fragments: &[JsFragment],
    css_fragments: &[CssFragment],
    include_runtime: bool,
) -> PipelineOutput
```

Same as `emit_typed` but wraps the JS output with the Spacetime runtime
(core signals, RAF loop, easing, color interpolation, stagger, templates, etc.).

---

## The Metasystem Expansion Path (file-level macros)

File-level `@directive` calls (those appearing outside CSS selector scopes,
like `@template &card($title) { ... }`) go through the metasystem expansion
path in `compiler.rs`. This path uses `expand_macro()` from
`src/metasystem/expand.rs`.

### `expand_macro()`

```rust
pub fn expand_macro(
    ctx: &mut MacroContext,
    macro_name: &str,
    args: &[PatternArg],
    span: &SourceSpan,
) -> Result<ExpandedDirectives, MacroExpansionError>
```

Creates a `MacroContext` with the registry, binds arguments from the call site
to the macro's `%form` parameters, then expands the macro body. The macro body
can contain:

- `%binds` -- invoke primitives, capture their outputs as signals
- `%derives` -- create computed signals from expressions
- `%states` -- define CSS state classes driven by signals
- `%emit js { }` / `%emit css { }` / `%emit build-js { }` -- emit code directly
- `%when` / `%for` / `%if` / `%elif` / `%else` -- conditional/iterative expansion
- `%on` -- event handlers
- `%animates` -- bind signals to CSS properties
- `%applies` -- inline style applications
- `%includes` -- compose other macros
- `%registers` -- register patterns (e.g., pattern matching CSS)
- `%resolves` -- resolve inherited bindings

Returns `ExpandedDirectives` containing `emitted_js`, `emitted_css`, and
`emitted_build_js` string vectors, along with state machines, transitions,
and transform rules.

### `compile_macro_call()`

```rust
// src/metasystem/compile.rs
pub fn compile_macro_call(
    registry: &MetaRegistry,
    selector: &str,
    macro_name: &str,
    args: &[PatternArg],
    span: &SourceSpan,
) -> Result<CompiledOutput, MacroExpansionError>
```

An alternative entry point that compiles a single macro call for a given CSS
selector. Builds a `DeclarationGraph` from the macro definition, then calls
`generate_code()` to produce JS and CSS. Used for selector-scoped expansion
when the full pipeline path is not applicable. Supports form-based macro
disambiguation with scoring.

The `DeclarationGraph` contains:
- `primitives: Vec<PrimitiveBinding>` -- from `%binds`
- `derives: Vec<DerivedSignal>` -- from `%derives`
- `states: Vec<StateDefinition>` -- from `%states` / `%registers`
- `animations: Vec<AnimationBinding>` -- from `%animates`
- `events: Vec<EventHandler>` -- from `%on`

---

## The Compiler (`src/compiler.rs`)

### Builder API

```rust
// From a parsed AST:
let compiled = Compiler::from_ast(&ast).compile();

// From a file on disk (reads, parses, resolves imports):
let compiled = Compiler::from_file(path, workspace_root)?.compile();

// With options:
let compiled = Compiler::from_ast(&ast)
    .trace(true)
    .debug(config)
    .fresh_registry()
    .compile();
```

### `compile_pipeline()` -- the orchestrator

This internal function in `compiler.rs` ties both paths together:

1. **Resolve registry** -- loads the stdlib `MetaRegistry` (cached, fresh, or
   caller-provided via `RegistrySource`)
2. **Register user meta definitions** -- primitives/macros/capture types from
   the user's `.st` file
3. **Pipeline path** -- passes `ast.matches` (pre-populated `FormMatch` values)
   through `pipeline::compile()`
4. **Metasystem expansion** -- iterates `ast.macro_calls` for file-level macros,
   expands each via `expand_macro()`, collects emitted JS/CSS/build-scripts
5. **Merge** -- prepends file-level JS (template definitions) before pipeline JS
   (template invocations) so definitions are registered before use
6. **CSS passthrough** -- emits static CSS declarations from scope blocks
7. **Runtime wrapping** -- wraps JS with the Spacetime runtime if enabled

Returns `CompiledSpacetime { js, css, build_scripts, pipeline_errors, ... }`.

### Stdlib Loading

```rust
pub fn load_stdlib_registry() -> (MetaRegistry, Vec<StdlibLoadError>)
pub fn cached_stdlib_registry() -> (MetaRegistry, Vec<StdlibLoadError>)
```

Loads `%primitive`, `%macro`, and `%capture_type` definitions from:
- `stdlib/capture-types/`
- `stdlib/runtime/`
- `stdlib/primitives/`
- `stdlib/syntax/`
- `stdlib/macros/`
- `stdlib/testing/`

Falls back to embedded stdlib (compiled into the binary) when filesystem
directories are not found. The `IncrementalStdlibCache` tracks file mtimes
for hot-reload during development.

---

## MetaRegistry (`src/metasystem/registry.rs`)

The `MetaRegistry` is the central store for all metasystem definitions:

```rust
pub struct MetaRegistry {
    primitives: HashMap<String, PrimitiveDefAst>,
    macros: HashMap<String, MacroDefAst>,
    presets: HashMap<String, ...>,
    transform_registry: TransformRegistry,
    runtime_registries: HashMap<String, ...>,
    capture_types: HashMap<String, ...>,
    provider_index: HashMap<String, Vec<String>>,
    // ...
}
```

Key methods:
- `register()` -- registers a `MetaDef` (primitive, macro, capture type, etc.)
- `get_primitive()` / `get_macro()` / `get_capture_type()` -- lookups by name
- `get_macro_by_form_directive()` -- finds macros whose `%form` creates a given `@directive`
- `get_all_macros_by_form_directive()` -- returns all overloads for a directive
- `rebuild_provider_index()` -- indexes which macros provide which signal bindings (for `%requires` validation and error messages)
- `load_stdlib_collecting_errors()` -- loads from a directory, collecting parse errors
- `load_from_defs()` -- loads from pre-parsed AST definitions
- `unregister_file()` -- removes all definitions from a source file (for incremental reload)

---

## Validation (`src/metasystem/validate.rs`)

Static analysis of metasystem definitions before they are used:

- `validate_primitive()` -- checks for duplicate exports, valid emit blocks, type references
- `validate_macro()` -- checks for unknown primitives in `%binds`, unbound variables, circular `%includes`, `%requires` satisfaction
- `validate_scope_macro_calls()` -- validates macro calls within a scope (used by LSP and WASM)

---

## Diagnostics (`src/metasystem/diagnostics.rs`)

Converts metasystem errors to structured `Diagnostic` values with:
- Error codes
- "Did you mean?" suggestions using Levenshtein distance
- Source spans for IDE integration

---

## Key Data Flow Example

Given this Spacetime source:

```spacetime
button {
    @on-hover { opacity: 0.8; }
}
```

1. **Parse**: The event-based parser produces a `FormMatch` with
   `macro_name: "on-hover"`, `captures: { "properties": StyleProperties([...]) }`,
   `selector: Some("button")`

2. **Resolve**: Looks up `%macro on-hover` in the registry, finds it binds
   the `pointer-hover` primitive with the captured properties. Creates a
   `ResolvedPrimitive` with phase=Selector, the bound arguments, and selector.

3. **Sort**: Orders by (Selector, inferred-order). Single primitive, so no reordering.

4. **Expand**: Calls `generate_primitive_ir()` on the `pointer-hover` primitive
   definition. Processes its `%emit js { }` block, substituting `%properties`
   with the captured style values. Produces `JsFragment` and `CssFragment`.

5. **Emit**: Wraps the JS in a `querySelectorAll('button').forEach(el => { ... })`
   block. Emits CSS rules. Wraps with runtime.

---

## Test Locations

Tests are distributed across inline module tests and test files:

| Area | Location |
|------|----------|
| Metasystem compile | `src/metasystem/compile.rs` (inline `#[cfg(test)]`) |
| Metasystem expand | `src/metasystem/expand.rs` (inline `#[cfg(test)]`) |
| Metasystem registry | `src/metasystem/registry.rs` (inline `#[cfg(test)]`) |
| Metasystem validate | `src/metasystem/validate.rs` (inline `#[cfg(test)]`) |
| Metasystem diagnostics | `src/metasystem/diagnostics.rs` (inline `#[cfg(test)]`) |
| Metasystem incremental cache | `src/metasystem/incremental_cache.rs` (inline `#[cfg(test)]`) |
| Metasystem relationships | `src/metasystem/relationships.rs` (inline `#[cfg(test)]`) |
| Cross-module metasystem | `src/metasystem/tests.rs` |
| Pipeline (all layers) | `src/pipeline/mod.rs` (inline `#[cfg(test)]`) |
| Pipeline resolve | `src/pipeline/resolve.rs` (inline `#[cfg(test)]`) |
| Pipeline sort | `src/pipeline/sort.rs` (inline `#[cfg(test)]`) |
| Pipeline expand | `src/pipeline/expand.rs` (inline `#[cfg(test)]`) |
| Pipeline emit | `src/pipeline/emit.rs` (inline `#[cfg(test)]`) |
| Pipeline integration | `src/pipeline/integration_tests.rs` |
| Emit JS | `src/emit/js.rs` (inline `#[cfg(test)]`) |
| Emit CSS | `src/emit/css.rs` (inline `#[cfg(test)]`) |
| Emit metasystem codegen | `src/emit/metasystem_codegen.rs` (inline `#[cfg(test)]`) |
| Compiler | `src/compiler.rs` (inline `#[cfg(test)]`) |
| Integration (compile + output) | `tests/integration/` |
| Headless (.test.st) macros | `tests/unit/macros/` (e.g., `cycle.test.st`, `loop.test.st`) |
| Headless (.test.st) primitives | `tests/unit/primitives/` |

Run tests:
```bash
cargo test --lib                              # Unit tests (1574+)
cargo test --test integration_tests           # Integration tests (442+)
cargo test --test v8_runtime_test             # JS runtime tests (139)
cargo run --features headless -- test tests/  # Headless Spacetime tests (341+)
```

---

## Adding New Metasystem Features

To add a new `@directive`:

1. **Define the primitive** in `stdlib/primitives/` -- write a `.st` file with
   `%primitive name { %params { ... } %emit js { ... } %emit css { ... } }`
2. **Define the macro** in `stdlib/macros/` -- write a `.st` file with
   `%macro { %form { @directive ... } %binds { ... } %states { ... } }`
3. **Define a capture type** (if needed) in `stdlib/capture-types/` for custom
   argument parsing
4. **The pipeline picks it up automatically** -- no Rust code changes needed
   for standard macros that follow the `%form` -> `%binds` -> `%emit` pattern

To modify the compilation infrastructure:

1. **IR types** live in `src/ir/` (JsStmt, JsExpr, CssExpr, etc.)
2. **Emit block parsing** lives in `src/emit/js_parser.rs` and `src/emit/css_parser.rs`
3. **Primitive IR generation** lives in `src/emit/metasystem_codegen.rs`
4. **Pipeline layers** each have their own file in `src/pipeline/`
5. **Macro expansion logic** lives in `src/metasystem/expand.rs`
