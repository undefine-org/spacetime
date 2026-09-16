# BUG-002: Body-Capture Path Didn't Loop for `+`/`*` Quantifiers

**Component:** `src/syntax/events/form_compiler.rs`  
**Severity:** Critical — `@each` body with multiple template invocations silently dropped all but the first  
**Status:** Fixed  

---

## Summary

The form compiler's body-capture path handled `Required` and `Optional` capture
modifiers but did not loop for `ZeroOrMore` (`*`) or `OneOrMore` (`+`) modifiers.
It extracted exactly one value and returned it as a scalar, not as an `Array`.

---

## Symptom

The `@each` macro form declares its body capture as:

```
{ $invocations:template_invocation+ }
```

The `+` means "one or more". With the bug, even a single invocation was returned
as a bare `CapturedValue::Named(...)` instead of `CapturedValue::Array([Named(...)])`.

The downstream serializer (see BUG-003) expected an array. When it received a
scalar `Named` map, it produced a JS object literal instead of an array:

```javascript
// Wrong (scalar Named, no loop):
const templateInvocations = { name: "ado-project", args: ["$p"] };

// Correct (Array of Named, with loop):
const templateInvocations = [{ name: "ado-project", args: ["$p"] }];
```

---

## Root Cause

In `form_compiler.rs`, the inline-element capture dispatch had three branches:

```rust
match modifier {
    CaptureModifier::Required => { /* extract once */ }
    CaptureModifier::Optional => { /* try once, ok if missing */ }
    // ZeroOrMore / OneOrMore were MISSING — fell through silently
}
```

The `ZeroOrMore` / `OneOrMore` case was absent from the body-capture path (the
path that processes tokens inside `{ ... }` blocks). The inline-param path
already had the loop (lines 604–638), but the body path did not.

---

## Fix

The `ZeroOrMore | OneOrMore` branch was added to the body-capture dispatch in
`src/syntax/events/form_compiler.rs` (lines 604–638):

```rust
CaptureModifier::ZeroOrMore | CaptureModifier::OneOrMore => {
    let mut values = Vec::new();
    if let Some(ext) = extractor {
        loop {
            cursor.skip_trivia();
            let remaining = cursor.remaining();
            if remaining.is_empty() { break; }
            if let Some((value, consumed)) = ext.extract(remaining, source) {
                values.push(value);
                cursor.advance(consumed);
                cursor.skip_trivia();
                cursor.eat_kind(SyntaxKind::COMMA);
            } else {
                break;
            }
        }
    }

    if modifier == CaptureModifier::OneOrMore && values.is_empty() {
        return Err(MatchFailure { /* ... */ });
    }

    captures.insert(var_name.to_string(), CapturedValue::Array(values));
    Ok(())
}
```

Key behaviors:
- Loops until the extractor fails to match
- Skips trivia (whitespace/comments) between items
- Eats optional comma separators
- Enforces the `+` (one-or-more) constraint: returns `MatchFailure` if zero items found
- Always stores result as `CapturedValue::Array(values)` — even for a single item

---

## Validation

A unit test was added to `src/syntax/events/form_compiler.rs` at line 2271:

### `template_invocation_body_capture_one_or_more_keeps_args`

```rust
let source = "@each { &ado-project($p); }";
// Form with body_capture: "{ $invocations:template_invocation+ }"
// ...
let invocations = fm.captures.get("invocations").unwrap();

match invocations {
    CapturedValue::Array(items) => {
        assert_eq!(items.len(), 1);
        match &items[0] {
            CapturedValue::Named(map) => {
                assert_eq!(map.get("name"), Some(&String("ado-project")));
                assert_eq!(map.get("args"), Some(&Array([String("$p")])));
            }
        }
    }
}
```

This test verifies that:
1. The result is an `Array` (not a bare `Named`)
2. The single item inside is a `Named` map with correct `name` and `args`
3. The argument `$p` is preserved through the full capture pipeline

Test passes as part of `cargo test --lib`.

---

## Related Bugs

- **BUG-001**: `TemplateInvocationExtractor` was a stub (must be fixed first)
- **BUG-003**: `captured_to_js` had no serialization case for `Named({name, args})`
