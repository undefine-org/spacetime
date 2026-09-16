# Compiler debugging

Loaded on demand from `spacetime-design` SKILL.md when the symptom is
compile-time (parse error, macro expansion, registry mismatch).
# Spacetime Compiler Debugging

Debug compile-time issues: parsing, macros, stdlib, form patterns.

> **Note**: For runtime issues (animations, CSS, browser), use `runtime-debugging.md` instead.

## When to Use

- Parse errors in `.st` files
- Macro expansion failures or unexpected output
- Stdlib loading problems
- Form pattern matching issues
- `@fn`, `@data`, `@each` macro troubles
- "missing parameter" or "unexpected character" errors

## Quick Diagnosis Workflow

```bash
# 1. Check for errors
cargo run -- check <file.st>

# 2. Inspect how file parses
cargo run -- inspect --layer parse <file.st> --format json

# 3. Verify macro is in registry
cargo run -- inspect --layer registry --selector <macro-name>

# 4. Check expansion output
cargo run -- inspect --layer expansion <file.st>

# 5. Enable trace logging for details
RUST_LOG=spacetime::metasystem=trace cargo run -- check <file.st>
```

## CLI Inspection Commands

### Parse Layer

Shows how a file is parsed into AST:

```bash
cargo run -- inspect --layer parse <file.st>
cargo run -- inspect --layer parse <file.st> --format json
```

Output includes: import count, function definitions, scope blocks, file-level macro calls with args.

### Registry Layer

Shows loaded macros and primitives from stdlib:

```bash
cargo run -- inspect --layer registry
cargo run -- inspect --layer registry --selector fn
cargo run -- inspect --layer registry --format json
```

### Expansion Layer

Shows macro expansion results (emitted JS/CSS):

```bash
cargo run -- inspect --layer expansion <file.st>
```

### Check Command

Quick validation with warnings:

```bash
cargo run -- check <file.st>
```

## Trace Logging

Enable detailed logging per module:

```bash
# Metasystem (macro expansion)
RUST_LOG=spacetime::metasystem=trace cargo run -- check <file.st>

# Pipeline
RUST_LOG=spacetime::pipeline=trace cargo run -- check <file.st>

# Parser
RUST_LOG=spacetime::parser=trace cargo run -- check <file.st>

# Multiple modules
RUST_LOG=spacetime::metasystem=trace,spacetime::parser::chumsky::metasystem=trace cargo run -- check <file.st>
```

## Common Error Patterns

### "Parse error: expected '{'"

**Cause**: Form pattern body parsing failed.

**Fix**: Check `src/parser/chumsky/metasystem.rs` around `form_body_with_params()`. Look for unsupported syntax in the form body.

### "missing parameter 'X' for macro %Y"

**Cause**: Form matching succeeded but arg binding failed.

**Fix**: Check the form's expected captures in registry (`inspect --layer registry --selector <macro>`). Verify type compatibility.

**Key file**: `src/metasystem/expand.rs` - `bind_args()` function

### "Unexpected character 'X' in directive"

**Cause**: Form pattern lexer hit unexpected character.

**Fix**: Find the form pattern in stdlib that's failing. Check `form_pattern()` in `src/parser/chumsky/metasystem.rs`.

### Macro expansion produces no output

**Cause**: Body content not being passed correctly.

**Fix**: Check if body has content via parse inspection. Verify the body is added to pattern_args during macro expansion in `src/metasystem/expand.rs`.

### Brace-bearing regex literals in `%emit js` (BUG-022)

**Symptom**: Parse fails with `expected scope block, directive, or declaration at offset N` pointing inside the body of a `%primitive ... { %emit js { ... } }` block. Or, in earlier compiler revisions, `cargo run -- check` reported success but `cargo run -- inspect --layer ir <fixture>` showed the affected primitive with an empty `Expanded[N]` body and the emit JS contained nothing.

**Cause**: The lexer that tokenizes the source does not recognize JS regex literals. A pattern like `/:root\s*\{[^}]*\}/g` contains:

- `\{` -> tokens `BACKSLASH` + `L_BRACE` (depth + 1)
- `[^}]` -> contains an unescaped `}` that is *literal* inside a character class but is lexed as `R_BRACE` (depth - 1)
- `\}` -> tokens `BACKSLASH` + `R_BRACE` (depth - 1)

Net: 1 open, 2 closes. The emit-body parser closes the block too early and tries to parse the rest of the regex as top-level directives.

**Workaround** (until [[id:FUP-017-bug-022-structural-fix-regex-literal-awa]] structural lexer fix lands): rewrite the regex to avoid brace-bearing patterns, or build it from a string at runtime:

```js
// Bad: parser breaks on the literal regex.
var styleRe = /:root\s*\{[^}]*\}/g;

// Good: hide the braces from the lexer.
var styleRe = new RegExp(":root\\s*\\{[^}]*\\}", "g");
```

**Acceptance proof**: `src/metasystem/tests.rs::grammar::test_brace_regex_in_emit_body_surfaces_diagnostic` pins the loud-failure behavior. When the lexer is fixed to recognize regex literals, flip the assertion to `is_ok()` and add a body-content roundtrip check.

## Data Flow Overview

```
1. Source file (.st)
   |
   v
2. Event-based parser (events/mod.rs)
   -> FormMatch { pattern, captures }
   |
   v
3. Pipeline: Resolve → Sort → Expand
   -> IRFragment { js, css, cleanup }
   |
   v
4. Macro expansion (expand.rs)
   -> bind_args() matches form pattern
   -> MacroValue bindings created
   |
   v
5. Primitive evaluation + Emit
   -> JS/CSS emitted
```

### Type Transitions

| Stage | Args Type | Body Type |
|-------|-----------|-----------|
| Chumsky | `Vec<MacroCallArg>` | `MacroCallBody { raw_content }` |
| AST | `Vec<MacroCallArg>` | `MacroCallBody { properties, ... }` |
| Transform | `Vec<PatternArg>` | `PatternValue::MacroBody(...)` |
| Expand | `MacroValue` bindings | Extracted per capture type |

## Macro-Specific Debugging

### @fn Macro

**Form**: `@fn $name:ident($params:params) : $returnType:typeref { $body:expr }`

**Captures**: `$name` (Ident), `$params` (Params), `$returnType` (TypeRef), `$body` (Expr)

**Common issues**:
- Named args like `x: number` parsed incorrectly
- Body with only expressions not being passed
- Return type including colon prefix

### @data Macro

**Form**: `@data $name:ident : $type:typeref { $config:properties }`

**Captures**: `$name`, `$type`, `$config`

### @each Macro

**Form**: `@each($source:binding) as $item:ident { $body }`

**Captures**: `$source` (binding), `$item` (ident), as_clause handling

## Minimal Test File Technique

Isolate issues with minimal files:

```bash
echo '@fn test(): number { 1 }' > /tmp/test1.st
cargo run -- check /tmp/test1.st

echo '@fn test(x: number): number { x }' > /tmp/test2.st
cargo run -- check /tmp/test2.st
```

## Key Source Files

| File | Purpose |
|------|---------|
| `src/parser/chumsky/toplevel.rs` | File parsing entry |
| `src/parser/chumsky/macros.rs` | Macro call parsing |
| `src/parser/chumsky/metasystem.rs` | Form pattern parsing |
| `src/parser/chumsky/convert.rs` | AST conversion |
| `src/compiler.rs` | Compiler builder + pipeline orchestration |
| `src/metasystem/expand.rs` | Macro expansion, arg binding |
| `src/metasystem/registry.rs` | Macro/primitive registry |

## Debug Code Snippets

Add to `src/metasystem/expand.rs` in `bind_args()`:

```rust
eprintln!("[DEBUG bind_args] macro={}, inline_elements={}, params={}, body_capture={:?}",
    macro_def.name,
    form.inline_elements.len(),
    form.params.len(),
    form.body_capture);
```
