# Compile-Time Analysis

Spacetime's data binding system performs comprehensive compile-time analysis to catch errors before runtime.

---

## Table of Contents

1. [Type Checking](#1-type-checking)
2. [JSON Schema Validation](#2-json-schema-validation)
3. [Binding Validation](#3-binding-validation)
4. [Orphan Detection](#4-orphan-detection)
5. [Error Codes Reference](#5-error-codes-reference)
6. [Compile Analysis Report](#6-compile-analysis-report)

---

## 1. Type Checking

The compiler validates all `@type` definitions and their usage.

### Type Resolution

```css
@type Author {
    name: string;
    email: string;
}

@type Post {
    title: string;
    author: Author;     /* Reference to Author type */
}
```

The compiler:
1. Builds a type registry from all `@type` definitions
2. Resolves type references
3. Detects circular dependencies

### Circular Dependency Detection

```css
@type A {
    b: B;
}

@type B {
    a: A;       /* Circular reference */
}
```

```
error[E0101]: Circular type dependency detected
  --> styles.st:1:1
   |
 1 | @type A {
   | ^^^^^^^^
   |
   = note: A -> B -> A
   = help: Break the cycle by making one reference optional (?) or using string IDs
```

### Property Path Validation

```css
@each(prints) {
    [slot="title"]: $.title;           /* ✓ Valid: Print has 'title' */
    [slot="author"]: $.author.name;    /* ✓ Valid: nested path */
    [slot="price"]: $.price;           /* ✗ Invalid: Print has no 'price' */
}
```

```
error[E0401]: Property 'price' does not exist on type 'Print'
  --> zeystudios.st:45:23
   |
45 |         [slot="price"]: $.price;
   |                         ^^^^^^^^
   |
   = help: Did you mean 'prices'?
   = note: Available properties: id, title, subtitle, image, prices, featured, tags
```

---

## 2. JSON Schema Validation

The compiler generates JSON Schema from `@type` definitions and validates data files at build time.

### Schema Generation

```css
@type Print {
    id: string;
    title: string;
    subtitle: string;
    image: url;
    prices: {
        S: number;
        M: number;
        L: number;
    };
    featured?: boolean;
    tags?: string[];
}
```

Generates:

```json
{
    "$schema": "http://json-schema.org/draft-07/schema#",
    "type": "object",
    "required": ["id", "title", "subtitle", "image", "prices"],
    "properties": {
        "id": { "type": "string" },
        "title": { "type": "string" },
        "subtitle": { "type": "string" },
        "image": { "type": "string", "format": "uri" },
        "prices": {
            "type": "object",
            "required": ["S", "M", "L"],
            "properties": {
                "S": { "type": "number" },
                "M": { "type": "number" },
                "L": { "type": "number" }
            }
        },
        "featured": { "type": "boolean" },
        "tags": {
            "type": "array",
            "items": { "type": "string" }
        }
    }
}
```

### Validation Errors

**Missing required field:**

```json
{
    "id": "IN-003",
    "title": "Between Worlds"
    /* Missing: subtitle, image, prices */
}
```

```
error[E0501]: JSON schema mismatch in '/data/prints.json'
  --> /data/prints.json:1:1
   |
 1 | {
   | ^
   |
   = error: Missing required field 'subtitle'
   = error: Missing required field 'image'
   = error: Missing required field 'prices'
   = note: Schema defined at zeystudios.st:5 (@type Print)
```

**Wrong type:**

```json
{
    "id": "IN-003",
    "title": "Between Worlds",
    "subtitle": "Surfer in morning mist",
    "image": "/prints/IN-003.jpg",
    "prices": 95
}
```

```
error[E0502]: Type mismatch in '/data/prints.json'
  --> /data/prints.json:6:14
   |
 6 |     "prices": 95
   |               ^^
   |
   = expected: object with properties S, M, L
   = found: number
   = note: Schema defined at zeystudios.st:9 (prices field)
```

**Invalid enum value:**

```css
@type CartItem {
    size: "S" | "M" | "L";
}
```

```json
{
    "size": "XL"
}
```

```
error[E0503]: Invalid enum value in '/data/cart.json'
  --> /data/cart.json:2:12
   |
 2 |     "size": "XL"
   |             ^^^^
   |
   = expected: one of "S", "M", "L"
   = found: "XL"
```

---

## 3. Binding Validation

The compiler validates all bindings in `@each` blocks.

### Template Existence

```css
@each(prints) {
    template: "zey-prnt";       /* ✗ Typo */
}
```

```
error[E0402]: Template 'zey-prnt' does not exist
  --> zeystudios.st:38:19
   |
38 |         template: "zey-prnt";
   |                   ^^^^^^^^^^
   |
   = help: Did you mean 'zey-print'?
   = note: Available templates: zey-print, zey-hero, zey-gallery, zey-nav, zey-footer
```

### Slot Existence

```css
@each(prints) {
    template: "zey-print";

    [slot="caption"]: $.subtitle;   /* ✗ No 'caption' slot */
}
```

```
error[E0403]: Slot 'caption' does not exist in template 'zey-print'
  --> zeystudios.st:42:10
   |
42 |         [slot="caption"]: $.subtitle;
   |         ^^^^^^^^^^^^^^^^^
   |
   = help: Did you mean 'subtitle'?
   = note: Available slots in 'zey-print': image, title, subtitle, cta
```

### Filter Type Checking

```css
@each(prints) {
    [slot="title"]: $.title | currency("$");    /* ✗ currency expects number */
}
```

```
error[E0404]: Type mismatch for filter 'currency'
  --> zeystudios.st:43:29
   |
43 |         [slot="title"]: $.title | currency("$");
   |                         ^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = expected: number
   = found: string ($.title is string)
   = note: Filter 'currency' formats numbers as currency strings
```

### Cross-Reference Validation

```css
@fn getPrint(id: string): Print? {
    return prints.find(p => p.id == id);
}

.cart-items {
    @each(cart) {
        @let print = getPrnt($.printId);    /* ✗ Typo in function name */
    }
}
```

```
error[E0405]: Function 'getPrnt' does not exist
  --> zeystudios.st:67:22
   |
67 |         @let print = getPrnt($.printId);
   |                      ^^^^^^^
   |
   = help: Did you mean 'getPrint'?
   = note: Available functions: getPrint, priceFor
```

---

## 4. Orphan Detection

The compiler detects unused definitions that may indicate errors or dead code.

### Unused Data Sources

```css
@data prints: Print[] {
    src: "/data/prints.json";
}

@data curations: Curation[] {
    src: "/data/curations.json";
}

/* Only prints is used in @each */
.gallery {
    @each(prints) { /* ... */ }
}
```

```
warning[W0201]: Data source 'curations' is defined but never used
  --> zeystudios.st:7:1
   |
 7 | @data curations: Curation[] {
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = help: Remove unused data source or add @each(curations) binding
```

### Unused Templates

```
warning[W0202]: Template 'zey-footer' is defined but never instantiated
  --> templates.html:145:1
   |
   = help: This template is never referenced in any @each or template: directive
```

### Unused Reactive State Classes

```css
$status <- "loading";

.gallery {
    .is-loading: $status == "loading";
    .is-ready:   $status == "ready";
    .is-error:   $status == "error";      /* Never activated */
}

/* No code sets $status to "error" */
```

```
warning[W0301]: Reactive state class '.gallery.is-error' is never activated
  --> zeystudios.st:52:5
   |
52 |     .is-error:   $status == "error";
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = help: Set $status to "error" somewhere, or remove the unused class
   = note: Signal $status is declared at zeystudios.st:48
```

### Dead Bindings

```css
@each(prints) {
    template: "zey-print";

    [slot="badge"]: $.badge;    /* Print type has no 'badge' field */
}
```

This is caught as a type error (E0401), not a warning.

---

## 5. Error Codes Reference

### Errors (E0xxx)

| Code | Description |
|------|-------------|
| **E0101** | Circular type dependency |
| **E0102** | Unknown type reference |
| **E0401** | Property does not exist on type |
| **E0402** | Template does not exist |
| **E0403** | Slot does not exist in template |
| **E0404** | Type mismatch (filter, binding) |
| **E0405** | Function does not exist |
| **E0406** | Wrong number of function arguments |
| **E0407** | Invalid function argument type |
| **E0501** | JSON schema mismatch (missing field) |
| **E0502** | JSON schema mismatch (wrong type) |
| **E0503** | JSON schema mismatch (invalid enum) |
| **E0504** | JSON file not found |
| **E0505** | JSON parse error |

### Warnings (W0xxx)

| Code | Description |
|------|-------------|
| **W0201** | Unused data source |
| **W0202** | Unused template |
| **W0203** | Unused type definition |
| **W0204** | Unused helper function |
| **W0301** | Reactive state class is never activated |
| **W0302** | Signal value has no matching reactive state class |
| **W0303** | Duplicate reactive state class definition |

---

## 6. Compile Analysis Report

When compilation succeeds, Spacetime outputs a comprehensive analysis report.

### Report Format

```
══════════════════════════════════════════════════════════════
  SPACETIME COMPILE REPORT: zeystudios
══════════════════════════════════════════════════════════════

Types:
  ✓ Price (3 fields)
  ✓ Print (7 fields, 2 optional)
  ✓ Curation (7 fields)
  ✓ CartItem (3 fields)

Data Sources:
  ✓ prints: Print[] → /data/prints.json (validated, 23 items)
  ✓ curations: Curation[] → /data/curations.json (validated, 6 items)
  ✓ cart: CartItem[] → localStorage (runtime)

Computed:
  ✓ featuredPrints: Print[] (derived from prints)
  ✓ cartTotal: number (derived from cart)
  ✓ cartCount: number (derived from cart)

Functions:
  ✓ getPrint(id: string): Print?
  ✓ priceFor(printId: string, size: "S"|"M"|"L"): number

Templates:
  ✓ zey-print (slots: image, title, subtitle, cta)
  ✓ zey-hero (slots: background, title, subtitle, cta)
  ✓ zey-curation-card (slots: image, title, description)
  ✓ zey-cart-item (slots: image, title, size, quantity, price)
  ⚠ zey-footer (defined but unused)

Bindings:
  ✓ .zey-gallery → prints (23 instances)
  ✓ .curations-grid → curations (6 instances)
  ✓ .cart-items → cart (runtime)

Reactive State Classes:
  ✓ .zey-gallery: $status → .is-loading, .is-ready (all values matched)
  ✓ .cart-items: $count → .is-empty, .has-items (all values matched)
  ⚠ .cart-drawer: signal value 'open' has no matching reactive class

Cross-References:
  ✓ cart.printId → prints.id (validated)
  ✓ curations.prints → prints.id (validated)

══════════════════════════════════════════════════════════════
  WARNINGS: 2 | ERRORS: 0
══════════════════════════════════════════════════════════════

warning[W0202]: Template 'zey-footer' defined but never used
  --> templates.html:156

warning[W0302]: Signal value 'open' in .cart-drawer has no matching reactive class
  --> zeystudios.st:89
  = help: Add .is-open: $drawer == "open" or remove the unused value

══════════════════════════════════════════════════════════════
  BUILD SUCCEEDED
══════════════════════════════════════════════════════════════
```

### Report Sections

| Section | Description |
|---------|-------------|
| **Types** | All `@type` definitions with field counts |
| **Data Sources** | All `@data` sources with item counts (from JSON) |
| **Computed** | All `@computed` definitions with dependencies |
| **Functions** | All `@fn` definitions with signatures |
| **Templates** | All templates with slot lists |
| **Bindings** | All `@each` bindings with instance counts |
| **Reactive State Classes** | All signals and their matching reactive classes |
| **Cross-References** | ID lookups between data sources |

### Symbols

| Symbol | Meaning |
|--------|---------|
| ✓ | Valid, no issues |
| ⚠ | Warning (non-blocking) |
| ✗ | Error (blocks compilation) |

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success (may have warnings) |
| 1 | Compilation failed (has errors) |

---

## IDE Integration

The compiler outputs diagnostics in a format compatible with common editors.

### LSP (Language Server Protocol)

The Spacetime language server provides:
- Real-time error highlighting
- Autocomplete for property paths (`$.`)
- Go-to-definition for types and functions
- Hover information for bindings

### Error Format

```json
{
    "file": "zeystudios.st",
    "line": 45,
    "column": 23,
    "endLine": 45,
    "endColumn": 31,
    "severity": "error",
    "code": "E0401",
    "message": "Property 'price' does not exist on type 'Print'",
    "help": "Did you mean 'prices'?",
    "notes": [
        "Available properties: id, title, subtitle, image, prices, featured, tags"
    ]
}
```
