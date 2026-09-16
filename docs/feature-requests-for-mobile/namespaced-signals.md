# Feature Request: Namespaced Signals

## Problem Statement

Mobile gesture patterns require grouping related signals under a common namespace for organization and clarity. For example, a pan gesture naturally produces `$pan.deltaX`, `$pan.deltaY`, `$pan.velocity`, etc. Similarly, a swipe gesture produces `$swipe.progress`, `$swipe.direction`, etc.

Currently, Spacetime's `%exports` and `%derives` blocks only support flat signal names like `$deltaX`, leading to:
1. **Name collisions** when multiple gestures are active on the same element
2. **Poor discoverability** - related signals are not grouped
3. **Verbose naming** - prefixes like `$panDeltaX` are needed to avoid collisions

### Current Syntax (Does Not Compile)

```spacetime
%primitive pan-gesture(&el) {
  %emit js {
    %yield dx -> $pan.deltaX;
    %yield dy -> $pan.deltaY;
  }

  %exports {
    $pan.deltaX: number
    $pan.deltaY: number
  }
}

%macro swipe {
  %derives {
    $swipe.progress: $pan.deltaX / $swipe.threshold
    $swipe.active: $swipe.progress > 0.3
  }
}
```

**Parser Error**: The grammar does not recognize `$namespace.property` syntax in signal declarations.

## Proposed Syntax

### In `%exports` Block

```spacetime
%exports {
  // Flat exports (existing, unchanged)
  $active: bool
  $deltaX: number

  // Namespaced exports (new)
  $pan.deltaX: number
  $pan.deltaY: number
  $pan.velocity: number

  // Nested namespaces (optional, future)
  $gesture.pan.startX: number
}
```

### In `%derives` Block

```spacetime
%derives {
  // Reference namespaced signals
  $swipe.progress: $pan.deltaX / $threshold
  $swipe.direction: $pan.deltaX > 0 ? "right" : "left"

  // Cross-namespace references
  $combined.movement: $pan.deltaX + $pinch.scale * 100
}
```

### In `%yield` Statements

```spacetime
%emit js {
  %yield deltaX -> $pan.deltaX;
  %yield Math.sqrt(dx*dx + dy*dy) -> $pan.distance;
}
```

### In `%binds` Output Aliases

```spacetime
%binds {
  gesture(&self) -> {
    $deltaX as $pan.deltaX,
    $deltaY as $pan.deltaY,
    $active as $pan.active
  }
}
```

## Implementation Details

### 1. Grammar Changes (`src/parser/grammar.pest`)

#### Modify `export_decl` Rule

```pest
// Current
export_decl = { "$" ~ identifier ~ ":" ~ type_annotation ~ inline_comment? }

// Proposed
export_decl = { "$" ~ namespaced_identifier ~ ":" ~ type_annotation ~ inline_comment? }

// New rule for namespaced identifiers
namespaced_identifier = @{ identifier ~ ("." ~ identifier)* }
```

#### Modify `derive_decl` Rule

```pest
// Current
derive_decl = { "$" ~ identifier ~ ":" ~ derive_expr }

// Proposed
derive_decl = { "$" ~ namespaced_identifier ~ ":" ~ derive_expr }
```

#### Update Variable References

The existing `meta_var_path` rule already supports dot notation for property access:
```pest
meta_var_path = { "$" ~ identifier ~ meta_var_path_segment* }
meta_var_path_segment = { meta_var_array_access | meta_var_prop_access }
meta_var_prop_access = { "." ~ identifier }
```

This needs to be unified with the signal declaration syntax.

### 2. AST Changes (`src/parser/ast.rs` and `src/parser/meta_ast.rs`)

#### Modify `ExportDecl`

```rust
// Current
pub struct ExportDecl {
    pub name: String,           // "deltaX"
    pub type_expr: ExportTypeExpr,
    pub optional: bool,
}

// Proposed
pub struct ExportDecl {
    pub name: SignalName,       // SignalName::Namespaced or SignalName::Simple
    pub type_expr: ExportTypeExpr,
    pub optional: bool,
}

/// Signal name - either simple or namespaced
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SignalName {
    /// Simple name: $active
    Simple(String),
    /// Namespaced: $pan.deltaX, $gesture.pan.startX
    Namespaced { namespace: Vec<String>, property: String },
}

impl SignalName {
    /// Get the full path as a string (e.g., "pan.deltaX")
    pub fn full_path(&self) -> String {
        match self {
            SignalName::Simple(name) => name.clone(),
            SignalName::Namespaced { namespace, property } => {
                let mut path = namespace.join(".");
                path.push('.');
                path.push_str(property);
                path
            }
        }
    }

    /// Get the JS-safe variable name (dots replaced with underscores)
    pub fn js_safe_name(&self) -> String {
        self.full_path().replace('.', "_")
    }
}
```

#### Modify `DeriveDecl`

```rust
// Current
pub struct DeriveDecl {
    pub name: String,
    pub expr: String,
    pub span: SourceSpan,
}

// Proposed
pub struct DeriveDecl {
    pub name: SignalName,
    pub expr: String,
    pub span: SourceSpan,
}
```

### 3. Parser Changes (`src/parser/mod.rs`)

Add a parsing function for namespaced identifiers:

```rust
fn parse_namespaced_identifier(pair: pest::iterators::Pair<Rule>) -> SignalName {
    let text = pair.as_str();
    if let Some(dot_pos) = text.find('.') {
        let parts: Vec<&str> = text.split('.').collect();
        let (namespace, property) = parts.split_at(parts.len() - 1);
        SignalName::Namespaced {
            namespace: namespace.iter().map(|s| s.to_string()).collect(),
            property: property[0].to_string(),
        }
    } else {
        SignalName::Simple(text.to_string())
    }
}
```

### 4. Codegen Changes (`src/metasystem/codegen.rs`)

#### Update `%yield` Transformation

```rust
// Current: %yield expr -> $var => ST.set(el, 'var', expr)
// New: %yield expr -> $pan.deltaX => ST.set(el, 'pan.deltaX', expr)

static YIELD_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    // Updated regex to support dotted names
    Regex::new(r"%yield\s+(.+?)\s*->\s*\$([a-zA-Z_][a-zA-Z0-9_]*(?:\.[a-zA-Z_][a-zA-Z0-9_]*)*)").unwrap()
});

pub fn transform_yields(code: &str) -> String {
    YIELD_REGEX.replace_all(code, |caps: &regex::Captures| {
        let expr = caps.get(1).unwrap().as_str().trim();
        let signal_path = caps.get(2).unwrap().as_str();
        // Use the full path as the signal key
        format!("ST.set(el, '{}', {})", signal_path, expr)
    }).to_string()
}
```

#### Update Signal References in Derives

```rust
fn generate_derive_watcher(
    name: &SignalName,
    dependencies: &[&SignalName],
    expression: &str,
) -> String {
    let signal_key = name.full_path();

    if dependencies.len() == 1 {
        format!(
            "ST.watch(el, '{}', (v) => ST.set(el, '{}', {}));\n",
            dependencies[0].full_path(), signal_key, expression
        )
    } else {
        let deps_array = dependencies
            .iter()
            .map(|d| format!("'{}'", d.full_path()))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "ST.watchAll(el, [{}], (v) => ST.set(el, '{}', {}));\n",
            deps_array, signal_key, expression
        )
    }
}
```

### 5. Runtime Changes (`public/runtime/st-minimal.js`)

The runtime `ST.set` and `ST.get` already use string keys, so namespaced signals like `'pan.deltaX'` will work without modification. However, for better organization, consider:

```javascript
// Optional: Helper to get all signals in a namespace
ST.getNamespace = (el, namespace) => {
    const signals = el.__st_signals || {};
    const result = {};
    const prefix = namespace + '.';
    for (const key in signals) {
        if (key.startsWith(prefix)) {
            result[key.slice(prefix.length)] = signals[key];
        }
    }
    return result;
};

// Usage: const pan = ST.getNamespace(el, 'pan');
// Returns: { deltaX: 10, deltaY: 5, velocity: 100 }
```

## Example Usage

### Mobile Pan Gesture Primitive

```spacetime
%primitive pan-gesture(
  &el,
  threshold: number = 0
) {
  %emit js {
    let startX = 0, startY = 0;
    let active = false;

    const onTouchStart = (e) => {
      const touch = e.touches[0];
      startX = touch.clientX;
      startY = touch.clientY;
      %yield 0 -> $pan.deltaX;
      %yield 0 -> $pan.deltaY;
    };

    const onTouchMove = (e) => {
      const touch = e.touches[0];
      const dx = touch.clientX - startX;
      const dy = touch.clientY - startY;

      if (!active && Math.abs(dx) > %threshold) {
        active = true;
        %yield true -> $pan.active;
      }

      if (active) {
        %yield dx -> $pan.deltaX;
        %yield dy -> $pan.deltaY;
        %yield Math.sqrt(dx*dx + dy*dy) -> $pan.distance;
      }
    };

    const onTouchEnd = () => {
      active = false;
      %yield false -> $pan.active;
    };

    el.addEventListener('touchstart', onTouchStart);
    el.addEventListener('touchmove', onTouchMove);
    el.addEventListener('touchend', onTouchEnd);

    %cleanup {
      el.removeEventListener('touchstart', onTouchStart);
      el.removeEventListener('touchmove', onTouchMove);
      el.removeEventListener('touchend', onTouchEnd);
    }
  }

  %exports {
    $pan.active: bool
    $pan.deltaX: number
    $pan.deltaY: number
    $pan.distance: number
  }
}
```

### Macro Using Namespaced Signals

```spacetime
%macro swipe-to-dismiss {
  %creates @swipe-to-dismiss

  %form {
    @swipe-to-dismiss(
      threshold: $threshold:number = 100,
      direction: $direction:("x" | "y" | "both") = "x"
    ) {
      on-dismiss: $onDismiss:expr?
    }
  }

  %binds {
    pan-gesture(&self) -> {
      $active as $pan.active,
      $deltaX as $pan.deltaX,
      $deltaY as $pan.deltaY
    }
  }

  %derives {
    $swipe.progress: $direction == "y"
      ? abs($pan.deltaY) / $threshold
      : abs($pan.deltaX) / $threshold
    $swipe.shouldDismiss: $swipe.progress >= 1.0
    $swipe.direction: $pan.deltaX > 0 ? "right" : "left"
  }

  %states {
    idle { opacity: 1; transform: translateX(0); }
    swiping when $pan.active {
      opacity: 1 - $swipe.progress * 0.5;
    }
  }

  %animates {
    translate-x: $pan.deltaX
  }

  %on $pan.active -> false {
    %if $swipe.shouldDismiss {
      $onDismiss?.($swipe.direction)
    }
  }
}
```

## Migration / Compatibility Notes

### Backward Compatibility

1. **Existing flat signals continue to work**: `$deltaX` remains valid
2. **No breaking changes to existing code**: The parser accepts both flat and namespaced
3. **Runtime is unchanged**: Signals are stored as string keys regardless

### Migration Path

1. **Phase 1**: Add grammar support for namespaced signals
2. **Phase 2**: Update AST types to use `SignalName`
3. **Phase 3**: Update codegen to handle namespaced signals
4. **Phase 4**: Add LSP support for autocomplete on namespaces

### LSP Considerations

The Language Server should be updated to:
1. Provide autocomplete for known namespaces (e.g., typing `$pan.` shows available signals)
2. Show namespace documentation on hover
3. Support "Go to Definition" for namespaced signals
4. Validate that referenced signals exist in their namespace

## Testing Checklist

- [ ] Parser accepts `$namespace.property` in `%exports`
- [ ] Parser accepts `$namespace.property` in `%derives`
- [ ] Parser accepts `$namespace.property` in `%yield`
- [ ] Parser accepts `$namespace.property` in bind output aliases
- [ ] Codegen produces correct `ST.set(el, 'namespace.property', ...)` calls
- [ ] Codegen produces correct watcher dependencies for namespaced signals
- [ ] Runtime stores and retrieves namespaced signals correctly
- [ ] Existing flat signals continue to work
- [ ] LSP provides autocomplete for namespaces

## Files to Modify

| File | Changes |
|------|---------|
| `src/parser/grammar.pest` | Add `namespaced_identifier` rule, update `export_decl`, `derive_decl` |
| `src/parser/ast.rs` | Add `SignalName` enum |
| `src/parser/meta_ast.rs` | Update `ExportDecl`, `DeriveDecl` to use `SignalName` |
| `src/parser/mod.rs` | Add `parse_namespaced_identifier` function |
| `src/metasystem/codegen.rs` | Update `YIELD_REGEX`, signal reference handling |
| `src/lsp/completion.rs` | Add namespace autocomplete support |
| `public/runtime/st-minimal.js` | Optional: Add `ST.getNamespace` helper |
