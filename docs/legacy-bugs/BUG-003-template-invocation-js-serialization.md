# BUG-003: `captured_to_js` Missing Serialization Case for Template Invocations

**Component:** `src/pipeline/expand.rs`  
**Severity:** Critical — template invocations serialized as JS objects instead of arrays  
**Status:** Fixed  

---

## Summary

`captured_to_js` (and its registry-aware variant `captured_to_js_with_registry`)
had no special case for `CapturedValue::Named` maps that represent template
invocations. They fell through to the generic `Named` branch, which serialized
the map as a plain JS object `{ name: "...", args: [...] }`. The runtime
primitive `each-with-templates` expects an **array** `[{ name: "...", args: [...] }]`.

---

## Symptom

After fixing BUG-001 and BUG-002, the extractor correctly produced:

```
CapturedValue::Array([
    Named({ "name": String("ado-project"), "args": Array([String("$p")]) })
])
```

But the serializer emitted:

```javascript
// Wrong — object, not array:
const templateInvocations = [{ name: "ado-project", args: ["$p"] }];
//                            ^^^ this part was correct...
```

Wait — actually the `Array` branch was also missing the special case, so it
emitted the inner `Named` as a generic object. The `each-with-templates`
primitive in `stdlib/primitives/template.st` iterates `%invocations` expecting
each element to have `.name` and `.args` properties, which it does — but the
**wrapping** was wrong when a single `Named` (not inside an `Array`) was passed.

The critical failure was when the body-capture returned a bare `Named` (before
BUG-002 was fixed): `captured_to_js` would emit `{ name: "...", args: [...] }`
(an object), and the runtime would try to iterate it as an array and fail silently.

---

## Root Cause

`captured_to_js` had this structure:

```rust
match value {
    CapturedValue::Array(items) => {
        // Generic: serialize each item recursively
        format!("[{}]", items.iter().map(captured_to_js).collect::<Vec<_>>().join(", "))
    }
    CapturedValue::Named(map) => {
        // Generic: serialize as JS object { key: value, ... }
        format!("{{ {} }}", pairs.join(", "))
    }
    // ...
}
```

Neither branch recognized the `{ "name": ..., "args": ... }` shape as a
template invocation. The `Named` branch produced a JS object literal, and the
`Array` branch recursively called `captured_to_js` on each `Named` item,
producing an array of objects — which is actually correct for the `Array` case.

The real problem was the `Named` (scalar) case: it produced `{ name: ..., args: ... }`
instead of `[{ name: ..., args: ... }]`. This matters because the runtime always
expects an array, even for a single invocation.

---

## Fix

Two changes were made to `src/pipeline/expand.rs`:

### 1. Added `is_template_invocation_map` helper (line 1464)

```rust
fn is_template_invocation_map(map: &HashMap<String, CapturedValue>) -> bool {
    map.len() == 2 && map.contains_key("name") && map.contains_key("args")
}
```

This identifies a `Named` map as a template invocation by its exact shape:
exactly two keys, `"name"` and `"args"`.

### 2. Special-cased `Array` of template invocations (lines 407–429)

```rust
CapturedValue::Array(items) => {
    if items.iter().all(|v| matches!(v, CapturedValue::Named(map) if is_template_invocation_map(map))) {
        // All items are template invocations — serialize as [{name, args}, ...]
        let items_str: Vec<String> = items.iter().filter_map(|v| {
            if let CapturedValue::Named(map) = v {
                let name = map.get("name")?;
                let args = map.get("args")?;
                Some(format!("{{ name: {}, args: {} }}",
                    captured_to_js_with_registry(name, meta_registry),
                    captured_to_js_with_registry(args, meta_registry)))
            } else { None }
        }).collect();
        return format!("[{}]", items_str.join(", "));
    }
    // ... generic array fallback
}
```

### 3. Special-cased scalar `Named` template invocation (lines 437–448)

```rust
CapturedValue::Named(map) => {
    if is_template_invocation_map(map) {
        let name = map.get("name").map(|v| captured_to_js_with_registry(v, meta_registry))
            .unwrap_or_else(|| "\"\"".to_string());
        let args = map.get("args").map(|v| captured_to_js_with_registry(v, meta_registry))
            .unwrap_or_else(|| "[]".to_string());
        // Wrap in array — runtime always expects [{ name, args }]
        return format!("[{{ name: {}, args: {} }}]", name, args);
    }
    // ... generic object fallback
}
```

The scalar `Named` case wraps the single invocation in an array, ensuring the
runtime always receives `[{ name: "...", args: [...] }]` regardless of whether
the body had one or many invocations.

The same changes were applied to both `captured_to_js` (the non-registry version,
lines 1540–1578) and `captured_to_js_with_registry` (lines 407–448).

---

## Final Output

Before all three fixes:

```javascript
const templateInvocations = "{\n        &ado-project";  // raw string
```

After all three fixes:

```javascript
const templateInvocations = [{ name: "ado-project", args: ["$p"] }];
```

---

## Validation

Two unit tests were added to `src/pipeline/expand.rs`:

### `test_captured_to_js_template_invocation_array` (line 2561)

```rust
let value = CapturedValue::Array(vec![
    CapturedValue::Named({
        "name" => String("ado-project"),
        "args" => Array([String("$p")])
    })
]);
let result = captured_to_js(&value);
assert_eq!(result, "[{ name: \"ado-project\", args: [\"$p\"] }]");
```

### `test_captured_to_js_template_invocation_named_wraps_array` (line 2582)

```rust
let value = CapturedValue::Named({
    "name" => String("ado-project"),
    "args" => Array([String("$p")])
});
let result = captured_to_js(&value);
// Single Named must also produce an array:
assert_eq!(result, "[{ name: \"ado-project\", args: [\"$p\"] }]");
```

Both tests pass as part of `cargo test --lib`.

---

## Related Bugs

- **BUG-001**: `TemplateInvocationExtractor` was a stub
- **BUG-002**: Form compiler body-capture path didn't loop for `+`/`*` quantifiers
