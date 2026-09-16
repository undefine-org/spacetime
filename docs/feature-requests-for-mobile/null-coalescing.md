# Feature Request: Null Coalescing Operator (`??`)

## Problem Statement

Mobile gesture macros often need to provide default values for optional parameters. Currently, this requires verbose ternary expressions:

```spacetime
%derives {
  // Current workaround - verbose and hard to read
  $effectiveBounds: $bounds != none ? $bounds : &self.parentElement
  $effectiveThreshold: $threshold != none ? $threshold : 50
  $effectiveName: $name != "" ? $name : "default"
}
```

JavaScript's null coalescing operator (`??`) provides a cleaner syntax:

```javascript
// JavaScript equivalent
const effectiveBounds = bounds ?? self.parentElement;
const effectiveThreshold = threshold ?? 50;
```

### Desired Syntax (Does Not Compile)

```spacetime
%derives {
  $effectiveBounds: $bounds ?? &self.parentElement
  $effectiveThreshold: $threshold ?? 50
  $effectiveName: $name ?? "default"
}

// In macro parameters
%form {
  @gesture(
    bounds: $bounds:element? = none,
    threshold: $threshold:number? = none
  )
}

// Derived with null coalescing
%derives {
  $actualThreshold: $threshold ?? 10
}
```

**Parser Error**: The grammar does not recognize `??` as a valid operator.

## Proposed Syntax

### In `%derives` Expressions

```spacetime
%derives {
  // Simple null coalescing
  $value: $optional ?? "default"

  // Chained null coalescing
  $value: $first ?? $second ?? "fallback"

  // With property access
  $width: $bounds?.width ?? 100

  // With function calls
  $name: getName() ?? "anonymous"
}
```

### In `%emit js` Blocks

The `??` operator should pass through unchanged to JavaScript since it's a native JS operator:

```spacetime
%emit js {
  const value = %optional ?? "default";
  const threshold = %threshold ?? 50;
}
```

### In Register Values

```spacetime
%registers binding($name) {
  value: $value ?? null,
  loading: false
}
```

### In State Property Values

```spacetime
%states {
  active when $isActive {
    opacity: $customOpacity ?? 1
    transform: $customTransform ?? none
  }
}
```

## Implementation Details

### 1. Grammar Changes (`src/parser/grammar.pest`)

#### Add Null Coalescing Operator

```pest
// Add to operator precedence (lower than || but higher than ternary)
derive_expr = @{ derive_expr_part+ }
derive_expr_part = @{ derive_expr_braces | derive_expr_non_brace }

// Alternative: Parse ?? explicitly for validation
derive_expr_with_coalesce = {
    derive_primary ~ ("??" ~ derive_primary)*
}

derive_primary = {
    derive_ternary
    | derive_value
}

// Null coalescing operator
null_coalesce_op = { "??" }

// Update meta state value to allow ??
meta_state_value = @{ meta_state_value_part+ }
meta_state_value_part = @{
    !(";" | "}" | (NEWLINE ~ WHITESPACE* ~ property_name ~ ":")) ~ ANY
}
// Note: This already allows ?? since it matches ANY character
```

The grammar currently uses raw string capture for `derive_expr` and `meta_state_value`, which means `??` already passes through. However, we should:

1. **Validate** that `??` is used correctly (has operands on both sides)
2. **Provide LSP support** (autocomplete, hover info)
3. **Document** the operator officially

#### For Explicit Parsing (Optional Enhancement)

If we want to parse and validate `??` expressions:

```pest
// Coalesce expression: expr ?? default ?? fallback
coalesce_expr = { coalesce_term ~ (null_coalesce_op ~ coalesce_term)* }
coalesce_term = { conditional_expr | primary_expr }
null_coalesce_op = { "??" }

// Optional chaining (separate feature but related)
optional_chain = { "?." }
optional_property_access = { "$" ~ identifier ~ optional_chain ~ identifier }
```

### 2. AST Changes (`src/parser/ast.rs` / `src/parser/meta_ast.rs`)

If we parse `??` explicitly, add AST nodes:

```rust
/// Expression with null coalescing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeriveExpr {
    /// Raw expression string (existing behavior)
    Raw(String),
    /// Parsed null coalescing: left ?? right ?? fallback
    NullCoalesce {
        operands: Vec<DeriveExpr>,
    },
    /// Variable reference
    Variable(String),
    /// Literal value
    Literal(DeriveValue),
    /// Property access: $obj.prop
    PropertyAccess {
        base: Box<DeriveExpr>,
        property: String,
    },
    /// Optional property access: $obj?.prop
    OptionalAccess {
        base: Box<DeriveExpr>,
        property: String,
    },
    /// Function call
    FunctionCall {
        name: String,
        args: Vec<DeriveExpr>,
    },
    /// Ternary: condition ? then : else
    Ternary {
        condition: Box<DeriveExpr>,
        then_expr: Box<DeriveExpr>,
        else_expr: Box<DeriveExpr>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeriveValue {
    Number(f64),
    String(String),
    Bool(bool),
    Null,
    Identifier(String),
}
```

### 3. Codegen Changes (`src/metasystem/codegen.rs`)

#### Pass-Through Behavior

Since `derive_expr` captures raw strings, `??` already passes through to JavaScript. The current implementation:

```rust
/// %derives { $name: expression }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeriveDecl {
    pub name: String,
    pub expr: String,  // Raw expression - ?? passes through
    pub span: SourceSpan,
}
```

No changes needed for basic functionality.

#### Enhanced Codegen (Optional)

If we want to add validation or transformation:

```rust
fn validate_derive_expr(expr: &str) -> Result<(), String> {
    // Check for balanced ??
    let parts: Vec<&str> = expr.split("??").collect();
    for part in &parts {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            return Err("Empty operand in null coalescing expression".to_string());
        }
    }
    Ok(())
}

fn transform_coalesce_for_older_browsers(expr: &str) -> String {
    // For browsers without ?? support, transform to:
    // (x ?? y) => (x != null ? x : y)
    // This is typically not needed as ?? has wide support
    expr.to_string()
}
```

### 4. Validation and Diagnostics

Add validation in the validation pass:

```rust
// In src/validation/mod.rs or similar

fn validate_derive_decl(decl: &DeriveDecl, diagnostics: &mut Vec<Diagnostic>) {
    let expr = &decl.expr;

    // Check for ?? usage
    if expr.contains("??") {
        // Validate operands
        let parts: Vec<&str> = expr.split("??").collect();
        for (i, part) in parts.iter().enumerate() {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                diagnostics.push(Diagnostic {
                    severity: DiagnosticSeverity::Error,
                    message: format!(
                        "Missing {} operand in null coalescing expression",
                        if i == 0 { "left" } else { "right" }
                    ),
                    span: decl.span,
                });
            }
        }
    }
}
```

### 5. LSP Support

#### Autocomplete

When typing `$variable ??`, suggest:
- Default values based on variable type
- Common patterns like `"default"`, `0`, `false`, `null`

```rust
// In src/lsp/completion.rs

fn complete_after_null_coalesce(
    var_name: &str,
    var_type: Option<&str>,
) -> Vec<CompletionItem> {
    let mut items = vec![];

    match var_type {
        Some("number") => {
            items.push(completion_item("0", "Default number"));
            items.push(completion_item("1", "Default number"));
        }
        Some("string") => {
            items.push(completion_item("\"\"", "Empty string"));
            items.push(completion_item("\"default\"", "Default string"));
        }
        Some("bool") => {
            items.push(completion_item("false", "Default false"));
            items.push(completion_item("true", "Default true"));
        }
        Some(t) if t.ends_with("?") => {
            items.push(completion_item("null", "Null default"));
        }
        _ => {
            items.push(completion_item("null", "Null default"));
        }
    }

    items
}
```

#### Hover Information

```rust
fn hover_null_coalesce() -> String {
    r#"**Null Coalescing Operator** (`??`)

Returns the left operand if it is not `null` or `undefined`,
otherwise returns the right operand.

**Example:**
```spacetime
%derives {
  $value: $optional ?? "default"
}
```

**Equivalent JavaScript:**
```javascript
const value = optional ?? "default";
```

**Chaining:**
```spacetime
$value: $first ?? $second ?? "fallback"
```
"#.to_string()
}
```

## Related Feature: Optional Chaining (`?.`)

While implementing `??`, consider also adding optional chaining:

```spacetime
%derives {
  // Optional property access
  $width: $bounds?.rect?.width ?? 100

  // Optional method call
  $result: $handler?.() ?? defaultHandler()
}
```

### Grammar Addition for Optional Chaining

```pest
optional_chain_access = {
    "$" ~ identifier ~ ("?." ~ identifier)+
}

optional_method_call = {
    "$" ~ identifier ~ "?." ~ "(" ~ expr_args? ~ ")"
}
```

## Example Usage

### Mobile Gesture with Defaults

```spacetime
%macro swipe-to-action {
  %creates @swipe-to-action

  %form {
    @swipe-to-action(
      threshold: $threshold:number? = none,
      direction: $direction:("left" | "right" | "both")? = none,
      haptic: $haptic:bool? = none
    ) {
      on-action: $onAction:expr?
    }
  }

  %binds {
    gesture(&self) -> { $deltaX, $active }
  }

  %derives {
    // Use null coalescing for cleaner defaults
    $effectiveThreshold: $threshold ?? 100
    $effectiveDirection: $direction ?? "both"
    $effectiveHaptic: $haptic ?? true

    // Derived values
    $progress: Math.abs($deltaX) / $effectiveThreshold
    $shouldTrigger: $progress >= 1

    // Direction check
    $isValidDirection: $effectiveDirection == "both"
      || ($effectiveDirection == "left" && $deltaX < 0)
      || ($effectiveDirection == "right" && $deltaX > 0)
  }

  %on $active -> false {
    %if $shouldTrigger && $isValidDirection {
      %if $effectiveHaptic {
        navigator.vibrate?.(10)
      }
      $onAction?.()
    }
  }
}
```

### Optional Element References

```spacetime
%macro scrollable {
  %form {
    @scrollable(
      container: $container:element? = none,
      snap: $snap:bool? = none
    )
  }

  %binds {
    scroll($container ?? &self) -> { $scrollY, $scrollX }
  }

  %derives {
    $effectiveSnap: $snap ?? false
  }
}
```

### Chained Coalescing

```spacetime
%derives {
  // Try multiple sources in order
  $theme: $userTheme ?? $systemTheme ?? "light"

  // With property access
  $color: $config?.theme?.primary ?? $defaults?.primary ?? "#000"
}
```

## Migration / Compatibility Notes

### Backward Compatibility

1. **Fully backward compatible**: `??` is new syntax that doesn't conflict with existing code
2. **Ternary still works**: Existing `$x != none ? $x : default` patterns continue to work
3. **Optional migration**: Can migrate existing code gradually

### Migration Helper

Consider adding a codemod or lint rule:

```rust
// Detect patterns that could use ??
fn suggest_null_coalesce(expr: &str) -> Option<String> {
    // Pattern: $x != none ? $x : default
    let pattern = Regex::new(r"\$(\w+)\s*!=\s*none\s*\?\s*\$\1\s*:\s*(.+)").unwrap();
    if let Some(caps) = pattern.captures(expr) {
        let var = &caps[1];
        let default = &caps[2];
        return Some(format!("${} ?? {}", var, default));
    }
    None
}
```

### Browser Support

The `??` operator is supported in:
- Chrome 80+
- Firefox 72+
- Safari 13.1+
- Edge 80+

For older browsers, the codegen could optionally transform:
```javascript
// Input
const x = a ?? b;

// Output (for old browsers)
const x = a != null ? a : b;
```

## Testing Checklist

- [ ] `??` parses correctly in `%derives` expressions
- [ ] `??` passes through to JS in `%emit` blocks
- [ ] Chained `??` works: `$a ?? $b ?? "c"`
- [ ] `??` with property access: `$obj.prop ?? default`
- [ ] `??` with function calls: `fn() ?? default`
- [ ] Validation catches missing operands
- [ ] LSP provides hover info for `??`
- [ ] LSP autocompletes after `??`
- [ ] Error messages are helpful for malformed `??` expressions

## Files to Modify

| File | Changes |
|------|---------|
| `src/parser/grammar.pest` | Document that `??` is supported (already passes through) |
| `src/validation/mod.rs` | Add validation for `??` expressions |
| `src/lsp/completion.rs` | Add autocomplete after `??` |
| `src/lsp/hover.rs` | Add hover info for `??` |
| `docs/language-reference.md` | Document `??` operator |

## Priority

**Medium-High**: This is a quality-of-life improvement that makes macro code much more readable. The implementation is minimal since `??` already passes through to JavaScript. Main work is validation and LSP support.
