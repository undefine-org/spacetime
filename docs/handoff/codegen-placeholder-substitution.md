# Codegen Placeholder Substitution Issue

## Status: Pre-existing Bug (3 failing tests)

This document describes a pre-existing issue in the codegen/emit system that was discovered during the form parsing refactor but was **not introduced by those changes**.

## Failing Tests

| Test | Error |
|------|-------|
| `test_default_param_used` | `%name` placeholder not substituted with default "World" |
| `test_for_loop_expansion` | `%$b.sel` and `%$b.prop` not resolved in `%for` loop |
| `test_provided_param_works` | `%url` not substituted with provided "/api/data" |

## What These Tests Verify

The `%emit` block system supports placeholders that should be substituted with parameter values:

```rust
// test_default_param_used
%emit js { console.log('Hello %name'); }  // %name should become "World" (default)

// test_provided_param_works
%emit js { fetch('%url'); }               // %url should become "/api/data"

// test_for_loop_expansion
%for $b in %bindings {
    el.querySelector('%$b.sel').textContent = item.%$b.prop;
}
```

## Root Cause Analysis

The issue is in how placeholders are substituted during codegen/emit:

1. **Default params**: `%name` should be replaced with the default value if no arg provided
2. **Provided params**: `%url` should be replaced with the provided argument value
3. **For loop vars**: `%$b.sel` should resolve loop variable field access

## Likely Culprits

| File | What to Check |
|------|---------------|
| `src/metasystem/codegen.rs` | `generate_primitive_js()`, placeholder substitution |
| `src/emit/js_emit.rs` | `resolve_stmts()`, `EmitContext::with_param()` |
| `src/emit/js_parser.rs` | `%for` loop parsing and expansion |

## Investigation Steps

1. Check `generate_primitive_js()` for how `%name` placeholders are substituted
2. Verify `PrimitiveArgs` params are being passed correctly to emit
3. Check `%for` loop resolution in `js_emit::resolve_stmts()`
4. Add debug logging to trace placeholder resolution

## Notes

- These tests were failing before the form parsing refactor (commit `887fa28`)
- The form parsing changes only affected the parser layer, not codegen
- Fixing these requires understanding the emit/codegen pipeline
