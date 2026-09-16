# Parser → FormMatch Direct: Analysis

## Current Architecture

```
Source Code
    ↓
Parser (Chumsky) → Hardcoded AST Types
    │                 ├── ValueDeclaration ($count number: 0;)
    │                 ├── DataDef (@data products: Product[] { src: "/api" })
    │                 ├── ComputedDef (@computed filtered: ... { where: ... })
    │                 ├── FnDef (@fn formatPrice(x: number): string { ... })
    │                 ├── EachBlock (@each $products { ... })
    │                 └── OnMutationBlock (@on &.click $btn { ... })
    ↓
Adapter (convert.rs) → Vec<FormMatch>
    ↓
Pipeline: Resolve → Sort → Expand → Emit
```

**Problem:** Two representations of the same data. Adapter exists only to convert between them.

---

## Target Architecture

```
Source Code
    ↓
Parser (Chumsky) → Vec<FormMatch> directly
    │                 ├── FormMatch { macro_name: "local-state", captures: {name, type, value} }
    │                 ├── FormMatch { macro_name: "data", captures: {name, type, src} }
    │                 ├── FormMatch { macro_name: "computed", captures: {name, source, ...} }
    │                 └── ...
    ↓
Pipeline: Resolve → Sort → Expand → Emit
```

---

## What It Takes

### Option A: Hardcoded Parsers → FormMatch

Keep the existing parsers but have them return FormMatch instead of specialized AST types.

**Example - ValueDeclaration parser:**
```rust
// Before: Returns ValueDeclaration
fn value_declaration() -> impl Parser<ValueDeclaration> {
    just('$')
        .ignore_then(ident())
        .then(type_expr())
        .then_ignore(just(':'))
        .then(expr())
        .then_ignore(just(';'))
        .map(|((name, type_expr), value)| ValueDeclaration { name, type_expr, value, span })
}

// After: Returns FormMatch
fn value_declaration() -> impl Parser<FormMatch> {
    just('$')
        .ignore_then(ident())
        .then(type_expr())
        .then_ignore(just(':'))
        .then(expr())
        .then_ignore(just(';'))
        .map_with(|((name, type_expr), value), e| FormMatch {
            macro_name: "local-state".to_string(),
            captures: HashMap::from([
                ("name".into(), CapturedValue::Ident(name)),
                ("type".into(), CapturedValue::TypeRef(type_expr.to_string())),
                ("value".into(), CapturedValue::Json(value)),
            ]),
            selector: None,
            span: e.span(),
        })
}
```

**Pros:**
- Minimal structural change
- Existing parser logic preserved
- Clear path from current code

**Cons:**
- Still hardcoded - adding new syntax requires parser changes
- Doesn't leverage `%form` patterns from stdlib

---

### Option B: Pattern-Based Universal Parser

Parser uses `%form` patterns from stdlib to dynamically match syntax.

**Flow:**
1. Bootstrap: Load stdlib, extract `%form` patterns into `SyntaxRegistry`
2. Parse: For each statement, try matching against registered patterns
3. On match: Produce FormMatch with captured values

**Example:**
```rust
// SyntaxRegistry has pattern: "$name:ident $type:ident : $value:expr ;"
// Parser sees: "$count number: 0;"
// Pattern matches, produces:
FormMatch {
    macro_name: "local-state",
    captures: {
        "name": Ident("count"),
        "type": Ident("number"),
        "value": Json(Number("0"))
    }
}
```

**Pros:**
- True extensibility - new syntax via stdlib, not parser changes
- Single source of truth (stdlib defines both pattern and behavior)
- Cleaner architecture

**Cons:**
- More complex implementation
- Pattern matching performance considerations
- Bootstrap dependency (need to parse stdlib first)

---

## Impact on IR and Subsequent Layers

### IR Changes: None

The intermediate representations stay the same:

```
FormMatch → [Resolve] → ResolvedPrimitive → [Sort] → [Expand] → IRFragment → [Emit] → Output
```

FormMatch is already the input to Resolve. The change is only in how FormMatch is produced.

### Resolve Layer: No Change

Already works with FormMatch:
```rust
pub fn resolve(matches: &[FormMatch], registry: &MetaRegistry) -> Vec<ResolvedPrimitive>
```

### Sort Layer: No Change

Works with ResolvedPrimitive, unaffected.

### Expand Layer: No Change

Works with ResolvedPrimitive, unaffected.

### Emit Layer: No Change

Works with IRFragment, unaffected.

---

## Diagnostic Implications

### Current Diagnostics

With specialized AST types:
```
Error: Invalid type expression
  --> file.st:5:12
   |
5  |   $count invalid: 0;
   |          ^^^^^^^ expected type like 'number', 'string', etc.
```

The parser knows it's parsing a `ValueDeclaration`, so errors are contextual.

### With FormMatch Direct

**Challenge 1: Pattern Ambiguity**

If multiple patterns could match:
```st
$count number: 0;  # Could be local-state or something else
```

The error message needs to explain which patterns were tried:
```
Error: No pattern matched
  --> file.st:5:1
   |
5  | $count number: 0;
   | ^^^^^^^^^^^^^^^^^
   |
   = note: tried patterns:
     - local-state: expected ';' at position 16
     - computed: expected 'from' keyword
```

**Challenge 2: Capture Type Errors**

When a capture doesn't match expected type:
```st
$count "not-a-type": 0;  # type should be ident, got string
```

Need to report:
```
Error: Type mismatch in pattern capture
  --> file.st:5:8
   |
5  | $count "not-a-type": 0;
   |        ^^^^^^^^^^^^
   |
   = expected: identifier (for 'type' capture)
   = found: string literal
```

**Challenge 3: Span Preservation**

FormMatch needs fine-grained spans:
```rust
pub struct FormMatch {
    pub macro_name: String,
    pub captures: HashMap<String, CapturedValue>,
    pub capture_spans: HashMap<String, SourceSpan>,  // NEW: span per capture
    pub selector: Option<String>,
    pub span: SourceSpan,  // Span of entire match
}
```

This enables pointing to the specific capture that caused an error.

---

## Diagnostic Completeness

### What We Lose

1. **Structural validation during parsing**
   - Currently: Parser rejects `@data { }` (missing required fields) at parse time
   - With FormMatch: Rejection happens in Resolve layer

2. **IDE autocomplete context**
   - Currently: Parser state tells IDE "user is typing a type expression"
   - With FormMatch: Less context available during typing

3. **Specialized error recovery**
   - Currently: Each parser has custom recovery for its syntax
   - With FormMatch: Generic pattern mismatch errors

### What We Gain

1. **Unified error format**
   - All syntax errors become "pattern did not match"
   - Consistent error structure

2. **Better "did you mean" suggestions**
   - Can show all similar patterns that almost matched
   - "Did you mean @data instead of @dat?"

3. **Runtime-extensible validation**
   - New syntax patterns can be added without recompiling parser
   - Validation rules come from stdlib

---

## Recommended Approach

### Phase 1: Option A (Hardcoded → FormMatch)

1. Modify existing parsers to return FormMatch
2. Delete specialized AST types (ValueDeclaration, etc.)
3. Delete adapter layer
4. Keep diagnostic quality

**Effort:** Medium (refactor existing code)
**Risk:** Low (preserves existing behavior)

### Phase 2: Option B (Pattern-Based, Later)

1. Implement pattern matching engine
2. Load patterns from stdlib
3. Gradually migrate syntax definitions
4. Eventually remove hardcoded parsers

**Effort:** High (new system)
**Risk:** Medium (new behavior to validate)

---

## Code Deletions (Phase 1)

| File | Deletions |
|------|-----------|
| `src/parser/ast.rs` | ~300 lines (ValueDeclaration, DataDef, ComputedDef, FnDef, EachBlock, OnMutationBlock) |
| `src/pipeline/adapter.rs` | ~550 lines (entire file) |
| `src/parser/chumsky/scopes.rs` | Modify parsers to return FormMatch |
| `src/parser/chumsky/toplevel.rs` | Modify parsers to return FormMatch |

**Estimated net deletion:** ~800-1000 lines

---

## Summary

| Aspect | Impact |
|--------|--------|
| **IR** | No change |
| **Resolve** | No change |
| **Sort** | No change |
| **Expand** | No change |
| **Emit** | No change |
| **Diagnostics** | Requires span-per-capture, loses some parse-time validation |
| **Extensibility** | Better with pattern-based (Phase 2) |
| **Code size** | ~800-1000 line reduction |
