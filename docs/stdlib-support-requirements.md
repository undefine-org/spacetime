# Stdlib Support Requirements

This document lists what's needed for the parser to fully support stdlib macro/primitive definitions.

## Current Status

The stdlib files in `stdlib/syntax/`, `stdlib/macros/`, `stdlib/primitives/` cannot be fully parsed because they use metasystem syntax that the parser doesn't completely support.

**Result:** The MetaRegistry remains empty, causing the pipeline to use heuristic fallbacks instead of actual macro definitions.

---

## 1. Parser Gaps

### 1.1 `%emit` Options (FIXED - removed from stdlib)

```st
# Was:
%emit js(order: 1) { ... }

# Now (simplified):
%emit js { ... }
```

**Status:** Worked around by removing options from stdlib files. Parser doesn't support emit options.

---

### 1.2 `%form` Clause Pattern Syntax

The parser needs to handle the inline pattern syntax inside `%form` blocks:

```st
%form { $name:ident $type:ident : $value:expr ; }
```

**Pattern elements to support:**
| Element | Example | Meaning |
|---------|---------|---------|
| `$name:ident` | Variable capture as identifier |
| `$value:expr` | Variable capture as expression |
| `$src:string` | Variable capture as string literal |
| `$refresh:time?` | Optional variable capture |
| `$default:any?` | Optional with any type |
| `$content:block?` | Optional block capture |
| Literal tokens | `:`, `;`, `@`, `(`, `)` | Exact match |

**Complex form example:**
```st
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
```

**Parser file:** `src/parser/chumsky/metasystem.rs`

---

### 1.3 `%registers` Clause

```st
%registers data_source {
  name: $name
  src: $src
  refresh: $refresh
  default: $default
}
```

Registers metadata about the macro invocation for later use (e.g., by LSP, validation).

**Status:** Not implemented in parser.

---

### 1.4 `%binds` with Signal Aliasing

```st
%binds {
  data-source(name: $name, src: $src) -> {
    $data as $$name,
    $loading as ${$name}-loading,
    $error as ${$name}-error
  }
}
```

The `-> { ... }` syntax allows remapping signals from the primitive to user-visible names.

**Status:** Parser handles basic `%binds`, but not the `-> { ... }` aliasing syntax.

---

### 1.5 `%derives` Clause

```st
%derives {
  draggable(target: &self)
}
```

Derives/inherits behavior from other macros.

**Parser file:** `src/parser/chumsky/metasystem.rs` - `derives_clause()`

**Status:** Partially implemented, needs verification.

---

### 1.6 `%states` Clause

```st
%states {
  idle { ... }
  dragging {
    %derives {
      position-tracker(&self)
    }
  }
}
```

Defines state-specific behaviors within a macro.

**Status:** Partially implemented, needs verification.

---

### 1.7 `%exports` Clause

```st
%exports {
  $value: any
}
```

Declares what signals/values a primitive exports.

**Parser file:** `src/parser/chumsky/metasystem.rs` - `exports_clause()`

**Status:** Implemented, seems to work.

---

### 1.8 Template String Interpolation in Emit

```js
%emit js {
  document.dispatchEvent(new CustomEvent(`local:${prop}:updated`, { ... }));
}
```

The JS template literals with `${...}` should be passed through verbatim.

**Status:** Works, but brace matching might be fragile.

---

## 2. Testing Checklist

To verify stdlib support:

```rust
#[test]
fn test_parse_all_stdlib_files() {
    let dirs = ["stdlib/syntax", "stdlib/macros", "stdlib/primitives"];
    for dir in dirs {
        for entry in walkdir::WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            if entry.path().extension() == Some("st".as_ref()) {
                let content = std::fs::read_to_string(entry.path()).unwrap();
                let result = parse(&content);
                assert!(result.is_ok(), "Failed to parse {}: {:?}", entry.path().display(), result.err());
            }
        }
    }
}
```

---

## 3. Priority Order

1. **`%form` pattern syntax** - Critical for macro matching
2. **`%binds` signal aliasing** - Needed for data macros
3. **`%registers`** - Needed for LSP/validation
4. **`%derives`** - Needed for composition
5. **`%states`** - Needed for stateful macros

---

## 4. Files to Modify

| File | Changes Needed |
|------|----------------|
| `src/parser/chumsky/metasystem.rs` | Add pattern parsing to `form_clause()` |
| `src/parser/meta_ast.rs` | Add `FormPattern` AST type if needed |
| `src/metasystem/registry.rs` | Verify registration works once parsing works |

---

## 5. Alternative: Separate Metasystem Parser

Instead of extending the main Chumsky parser, consider a dedicated metasystem parser:

```rust
// src/parser/meta_parser.rs
pub fn parse_meta_file(content: &str) -> Result<Vec<MetaDef>, ParseError> {
    // Simpler parser focused only on %primitive, %macro syntax
}
```

This would:
- Avoid polluting the main parser with metasystem complexity
- Be easier to maintain
- Only run on stdlib files, not user code
