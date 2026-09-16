# Parser Extension Specification

This document specifies the grammar extensions needed to support the `%` macro system.

## New Token: `%`

The percent sign introduces compile-time constructs. It is only valid at the top level of `.st` files in stdlib and library code.

```pest
// New token
PERCENT = { "%" }

// Meta constructs (% prefix)
meta_construct = {
    primitive_def |
    macro_def |
    preset_def |     // Existing, but now also %preset
    blocktype_def    // Custom block types
}
```

## Primitive Definition

```pest
primitive_def = {
    "%primitive" ~ identifier ~ primitive_params ~ "{" ~
        primitive_body ~
    "}"
}

primitive_params = {
    "(" ~ primitive_param_list? ~ ")"
}

primitive_param_list = {
    primitive_param ~ ("," ~ primitive_param)*
}

primitive_param = {
    element_ref ~ |                                    // &el
    identifier ~ ":" ~ type_annotation ~ ("=" ~ expr)?  // name: type = default
}

primitive_body = {
    emit_block ~ cleanup_block? ~ exports_block
}

emit_block = {
    "%emit" ~ "js" ~ "{" ~ js_code ~ "}"
}

cleanup_block = {
    "%cleanup" ~ "{" ~ js_code ~ "}"
}

exports_block = {
    "%exports" ~ "{" ~ export_decl* ~ "}"
}

export_decl = {
    binding_ref ~ ":" ~ type_annotation
}
```

## Macro Definition

```pest
macro_def = {
    "%macro" ~ identifier ~ "{" ~
        creates_clause? ~
        form_clause ~
        macro_body_clauses ~
    "}"
}

creates_clause = {
    "%creates" ~ "@" ~ identifier
}

form_clause = {
    "%form" ~ "{" ~ form_pattern ~ "}"
}

form_pattern = {
    "@" ~ identifier ~ form_params? ~ form_block?
}

form_params = {
    "(" ~ form_param_list? ~ ")"
}

form_param = {
    identifier ~ ":" ~ "$" ~ identifier ~ ":" ~ type_annotation ~ ("=" ~ expr)?
}

form_block = {
    "{" ~ form_block_content ~ "}"
}

form_block_content = {
    "$" ~ identifier ~ ":" ~ block_type ~ "?"?
}

macro_body_clauses = {
    (binds_clause | derives_clause | when_clause | animates_clause |
     timeline_clause | states_clause | registers_clause | on_clause)*
}

binds_clause = {
    "%binds" ~ "{" ~ bind_stmt* ~ "}"
}

bind_stmt = {
    identifier ~ "(" ~ args ~ ")" ~ "->" ~ "{" ~ binding_list ~ "}"
}

derives_clause = {
    "%derives" ~ "{" ~ derive_stmt* ~ "}"
}

derive_stmt = {
    "$" ~ identifier ~ ":" ~ expr
}

when_clause = {
    "%when" ~ expr ~ "{" ~ macro_body_clauses ~ "}"
}

animates_clause = {
    "%animates" ~ ("on" ~ element_ref)? ~ "{" ~ animation_property* ~ "}"
}

timeline_clause = {
    "%timeline" ~ "(" ~ identifier ~ ("," ~ expr)? ~ ")" ~ "{" ~ timeline_body ~ "}"
}

states_clause = {
    "%states" ~ "{" ~ state_def* ~ "}"
}

registers_clause = {
    "%registers" ~ register_type ~ "(" ~ args ~ ")" ~ "{" ~ register_body ~ "}"
}

on_clause = {
    "%on" ~ expr ~ "{" ~ macro_body_clauses ~ "}"
}
```

## Type Annotations

```pest
type_annotation = {
    primitive_type |
    element_type |
    binding_type |
    union_type |
    array_type |
    block_type |
    typeref
}

primitive_type = {
    "ident" | "string" | "number" | "bool" | "time" | "length" |
    "color" | "easing" | "selector" | "expr" | "fn"
}

element_type = { "element" }

binding_type = { "binding" }

union_type = {
    "(" ~ string_literal ~ ("|" ~ string_literal)* ~ ")"
}

array_type = {
    type_annotation ~ "[]"
}

block_type = {
    "keyframes" | "properties" | "template" | "fields" | "states"
}
```

## Symbol References

```pest
// Existing (runtime)
binding_ref = { "$" ~ identifier ~ ("." ~ identifier)* }
element_ref = { "&" ~ identifier }

// New (compile-time splicing)
meta_binding_ref = { "%" ~ "$" ~ identifier }    // Splice a value
meta_element_ref = { "%" ~ "&" ~ identifier }    // Splice an element ref
meta_param_ref = { "%" ~ identifier }             // Splice a parameter

// Yield syntax (in primitives)
yield_stmt = {
    "%yield" ~ expr ~ "->" ~ binding_ref
}
```

## AST Extensions

### New AST Types

```rust
/// %primitive definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveDef {
    pub name: String,
    pub params: Vec<PrimitiveParam>,
    pub emit: EmitBlock,
    pub cleanup: Option<String>,
    pub exports: Vec<ExportDecl>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveParam {
    pub name: String,
    pub is_element: bool,
    pub type_annotation: TypeAnnotation,
    pub default: Option<Expr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmitBlock {
    pub language: String,  // "js"
    pub code: String,
    pub yields: Vec<YieldStmt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YieldStmt {
    pub expr: String,
    pub binding: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportDecl {
    pub binding: String,
    pub type_annotation: TypeAnnotation,
}

/// %macro definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroDef {
    pub name: String,
    pub creates: Option<String>,
    pub form: FormClause,
    pub binds: Vec<BindStmt>,
    pub derives: Vec<DeriveStmt>,
    pub whens: Vec<WhenClause>,
    pub animates: Vec<AnimatesClause>,
    pub timelines: Vec<TimelineClause>,
    pub states: Option<StatesClause>,
    pub registers: Vec<RegistersClause>,
    pub ons: Vec<OnClause>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormClause {
    pub pattern: FormPattern,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormPattern {
    pub directive_name: String,
    pub params: Vec<FormParam>,
    pub block: Option<FormBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormParam {
    pub name: Option<String>,      // Named param like "duration:"
    pub capture: String,           // $duration
    pub type_annotation: TypeAnnotation,
    pub default: Option<Expr>,
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormBlock {
    pub captures: Vec<FormBlockCapture>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormBlockCapture {
    pub name: String,
    pub block_type: BlockType,
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeAnnotation {
    Primitive(PrimitiveType),
    Element,
    Binding,
    Union(Vec<String>),
    Array(Box<TypeAnnotation>),
    Block(BlockType),
    TypeRef(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrimitiveType {
    Ident,
    String,
    Number,
    Bool,
    Time,
    Length,
    Color,
    Easing,
    Selector,
    Expr,
    Fn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlockType {
    Keyframes,
    Properties,
    Template,
    Fields,
    States,
}
```

## Compilation Pipeline

```
┌─────────────────────────────────────────────────────────────┐
│  1. PARSE                                                   │
│  Read .st files, build AST including PrimitiveDef/MacroDef  │
└─────────────────────────────────┬───────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────┐
│  2. REGISTER                                                │
│  Build macro registry: name -> MacroDef                     │
│  Build primitive registry: name -> PrimitiveDef             │
└─────────────────────────────────┬───────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────┐
│  3. EXPAND                                                  │
│  For each @directive in user code:                          │
│  - Match against %form patterns                             │
│  - Extract typed captures                                   │
│  - Expand macro into declaration graph                      │
└─────────────────────────────────┬───────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────┐
│  4. RESOLVE                                                 │
│  Resolve %binds to primitives                               │
│  Connect primitive $exports to macro bindings               │
└─────────────────────────────────┬───────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────┐
│  5. CODEGEN                                                 │
│  Emit JavaScript from primitives' %emit js blocks           │
│  Wire up reactive bindings                                  │
│  Generate optimized runtime code                            │
└─────────────────────────────────────────────────────────────┘
```

## Implementation Priority

### Phase 1: Grammar Extension
1. Add `%` token to lexer
2. Add `primitive_def` and `macro_def` rules
3. Add type annotation rules

### Phase 2: AST Types
1. Create new AST structs in `src/parser/mod.rs`
2. Update parser to produce new AST nodes

### Phase 3: Macro Registry
1. Create `src/macros/registry.rs`
2. Implement macro lookup by `@directive` name
3. Implement form pattern matching

### Phase 4: Macro Expansion
1. Create `src/macros/expand.rs`
2. Implement capture extraction from user code
3. Implement declaration graph construction

### Phase 5: Primitive Resolution
1. Create `src/primitives/registry.rs`
2. Implement primitive lookup
3. Implement binding connection

### Phase 6: Code Generation
1. Extend `src/codegen.rs`
2. Emit JavaScript from `%emit js` blocks
3. Generate reactive binding wiring

## Backward Compatibility

The `%` constructs are only valid in stdlib and library files. User `.st` files continue to use `@`, `$`, `&` syntax unchanged.

Existing `@` directives (like `@scroll`, `@on &.hover`, etc.) will be redefined using the new `%macro` system, but their external interface remains identical.
