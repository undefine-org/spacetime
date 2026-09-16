# BUG-001: `TemplateInvocationExtractor` Was a Stub

**Component:** `src/syntax/events/extractors/pattern.rs`  
**Severity:** Critical — `@each` + `@template` data binding completely non-functional  
**Status:** Fixed  

---

## Summary

`TemplateInvocationExtractor::extract()` was not implemented. It delegated to
`ExprExtractor`, which captured the raw source text of the body block as a plain
string instead of parsing the `&name($arg1, $arg2)` syntax into structured data.

---

## Symptom

Given a Spacetime file like:

```css
@each($projects as $p) {
    &ado-project($p);
}
```

The compiled JavaScript contained:

```javascript
const templateInvocations = "{\n        &ado-project";  // raw string!
```

The runtime's `each-with-templates` primitive expects an **array of objects**:

```javascript
const templateInvocations = [{ name: "ado-project", args: ["$p"] }];
```

Because the value was a string, no templates were ever instantiated. The project
gallery rendered empty.

---

## Root Cause

`TemplateInvocationExtractor` existed as a struct but its `extract()` method
simply fell through to `ExprExtractor`, capturing the entire body block as a
raw `CapturedValue::Expr("{\n        &ado-project")`.

The `@each` macro form declares:

```
$invocations:template_invocation+
```

This means: "capture one or more template invocations". The extractor is
responsible for parsing `&name(args...)` syntax. Because it was a stub, it
returned the raw text instead.

---

## Fix

`TemplateInvocationExtractor::extract()` was fully implemented in
`src/syntax/events/extractors/pattern.rs` (lines 234–347).

The new implementation:

1. Skips leading whitespace
2. Expects an `AMPERSAND` token (`&`)
3. Reads the template name (stops at `(`, `;`, `,`, `}`, or EOF)
4. If a `(` follows, parses comma-separated arguments until the matching `)`
5. Skips an optional trailing `;`
6. Returns `CapturedValue::Named({ "name": String(name), "args": Array([...]) })`

```rust
// Before (stub):
fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
    ExprExtractor.extract(tokens, source)  // captured raw text!
}

// After (real implementation):
fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
    // ... parse &name($arg1, $arg2) ...
    let mut invocation = HashMap::new();
    invocation.insert("name".to_string(), CapturedValue::String(name));
    invocation.insert("args".to_string(), CapturedValue::Array(args));
    Some((CapturedValue::Named(invocation), pos))
}
```

---

## Validation

Two unit tests were added to `src/syntax/events/extractors/pattern.rs`:

### `template_invocation_extractor_single_arg` (line 531)

```rust
let source = "&ado-project($p);";
// ...
assert_eq!(result, Some((CapturedValue::Named({
    "name" => String("ado-project"),
    "args" => Array([String("$p")])
}), 7)));
```

### `template_invocation_extractor_empty_args` (line 558)

```rust
let source = "&product-card();";
// ...
assert_eq!(result, Some((CapturedValue::Named({
    "name" => String("product-card"),
    "args" => Array([])
}), 5)));
```

Both tests pass as part of `cargo test --lib`.

---

## Related Bugs

This bug is part of a 3-part failure chain. Even after fixing the extractor,
the output was still wrong because of:

- **BUG-002**: Form compiler body-capture path didn't loop for `+`/`*` quantifiers
- **BUG-003**: `captured_to_js` had no serialization case for `Named({name, args})`
