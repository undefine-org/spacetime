# BUG-000: `@each` + `@template` Rendering Completely Broken

**Affects:** All sites using `@each($source as $item) { &template-name($item); }`  
**Severity:** Critical — data-driven template rendering non-functional  
**Status:** Fixed (3 bugs, 4 files changed, 2 new tests)  
**Discovered while:** Building `site/adovasio/` — a luxury wedding photography portfolio  

---

## Overview

The `@each` macro combined with `@template` invocations inside its body was
completely non-functional. The compiled JavaScript contained a raw string instead
of the structured array the runtime expected, so no template instances were ever
created and the page rendered empty.

This was caused by a **3-part failure chain** — three independent bugs that each
had to be fixed for the feature to work end-to-end.

---

## The Failure Chain

```
@each($projects as $p) {
    &ado-project($p);
}
```

### Step 1 — Extraction (BUG-001)

`TemplateInvocationExtractor::extract()` was a stub. It delegated to
`ExprExtractor`, capturing the raw body text as a string:

```
CapturedValue::Expr("{\n        &ado-project")
```

**Expected:**
```
CapturedValue::Named({ "name": "ado-project", "args": ["$p"] })
```

→ See [BUG-001](./BUG-001-template-invocation-extractor-stub.md)

---

### Step 2 — Form Compilation (BUG-002)

Even after fixing the extractor, the form compiler's body-capture path didn't
loop for `+`/`*` quantifiers. The `@each` macro uses `$invocations:template_invocation+`
(one-or-more). The body-capture path extracted exactly one value and stored it
as a scalar, not as an `Array`:

```
CapturedValue::Named(...)       ← wrong: scalar
CapturedValue::Array([Named(...)]) ← correct: array
```

→ See [BUG-002](./BUG-002-body-capture-quantifier-loop.md)

---

### Step 3 — JS Serialization (BUG-003)

Even after fixing extraction and form compilation, `captured_to_js` had no
special case for the `{ "name": ..., "args": ... }` shape. A scalar `Named`
was serialized as a JS object literal instead of a single-element array:

```javascript
// Wrong:
const templateInvocations = { name: "ado-project", args: ["$p"] };

// Correct:
const templateInvocations = [{ name: "ado-project", args: ["$p"] }];
```

The runtime's `each-with-templates` primitive always expects an array.

→ See [BUG-003](./BUG-003-template-invocation-js-serialization.md)

---

## Files Changed

| File | Change |
|------|--------|
| `src/syntax/events/extractors/pattern.rs` | Fully implemented `TemplateInvocationExtractor::extract()` |
| `src/syntax/events/form_compiler.rs` | Added `ZeroOrMore \| OneOrMore` loop branch to body-capture path |
| `src/pipeline/expand.rs` | Added `is_template_invocation_map()` helper + special-case serialization in both `captured_to_js` and `captured_to_js_with_registry` |
| `src/pipeline/resolve.rs` | Minor: synthetic emit primitive for macros with `%emit js` blocks |

---

## Tests Added

| Test | File | Verifies |
|------|------|----------|
| `template_invocation_extractor_single_arg` | `extractors/pattern.rs:531` | Extractor parses `&name($arg)` → `Named({name, args})` |
| `template_invocation_extractor_empty_args` | `extractors/pattern.rs:558` | Extractor parses `&name()` → `Named({name, args: []})` |
| `template_invocation_body_capture_one_or_more_keeps_args` | `form_compiler.rs:2271` | Body capture with `+` produces `Array([Named(...)])` |
| `test_captured_to_js_template_invocation_array` | `expand.rs:2561` | `Array([Named])` serializes to `[{name, args}]` |
| `test_captured_to_js_template_invocation_named_wraps_array` | `expand.rs:2582` | Scalar `Named` also serializes to `[{name, args}]` |

Test count: 1855 → 1857 passing (2 new tests; the other 3 were pre-existing or
added as part of the same commit).

---

## Before / After

**Before (all 3 bugs present):**
```javascript
// Compiled output of @each { &ado-project($p); }
const templateInvocations = "{\n        &ado-project";
```

**After (all 3 bugs fixed):**
```javascript
const templateInvocations = [{ name: "ado-project", args: ["$p"] }];
```

The runtime's `each-with-templates` primitive then correctly:
1. Fetches the data source (`$projects`)
2. For each item, clones the `<template id="ado-project">` element
3. Passes `$p` as the bound variable
4. Appends the populated clone to the DOM

---

## How to Reproduce (Historical)

With the bugs present, any `.st` file containing:

```css
@each($data as $item) {
    &my-template($item);
}
```

Would compile to broken JS. The symptom was an empty page where template
instances should appear, with no JavaScript errors (the runtime received a
string and silently did nothing with it).
