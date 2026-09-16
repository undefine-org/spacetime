# Spacetime Documentation Index

**Complete guide to all documentation for the Spacetime data binding system.**

---

## Getting Started

### For Users

Start here if you're new to Spacetime data binding:

1. **[Data Binding Guide](./DATA_BINDING_GUIDE.md)**
   - Step-by-step tutorial for beginners
   - Learn by building real examples
   - Common patterns and recipes
   - Troubleshooting guide

2. **[Working Example](../examples/data-binding/)**
   - Complete, runnable example
   - Demonstrates all major features
   - Easy to customize and learn from

### For Developers Migrating

If you have an existing Spacetime site:

3. **[Migration Guide](./MIGRATION.md)**
   - What's new in data binding
   - Should you migrate?
   - Step-by-step migration process
   - Before/after examples
   - Backward compatibility notes

---

## Reference Documentation

### Compiler Inspection

- **[`inspect` Layer Provenance](./INSPECT_PIPELINE.html)**
  - Mermaid maps of every `cargo run -- inspect --layer` output
  - Distinguishes parser projections from production compiler artifacts
  - Links each CLI view to its implementation source

### Complete API Reference

4. **[API Reference](./API_REFERENCE.md)**
   - All `@type` primitives and syntax
   - All `@data` options
   - All `@each` binding patterns
   - All built-in filters
   - Special variables
   - Data events
   - Error codes with explanations

### Technical Specifications

5. **[Data System Specification](./DATA_SYSTEM.md)**
   - Technical overview
   - Type system details
   - Data sources and caching
   - Computed data
   - Helper functions
   - Template binding
   - State machine integration

6. **[Data System Examples](./DATA_SYSTEM_EXAMPLES.md)**
   - Complete examples for real websites
   - zeystudios (photography shop)
   - ikarchitecte (architecture firm)
   - jallete (e-commerce)
   - Before/after comparisons

### Runtime Library

7. **[Runtime API](../public/runtime/README.md)**
   - JavaScript runtime documentation
   - API for programmatic usage
   - Advanced features
   - Performance optimization

---

## Quick Links by Topic

### Learning Path

**Absolute Beginner → Advanced User**

1. Read: [Data Binding Guide](./DATA_BINDING_GUIDE.md) (Getting Started section)
2. Try: [Working Example](../examples/data-binding/)
3. Build: Your first data-bound component
4. Reference: [API Reference](./API_REFERENCE.md) as needed
5. Explore: [Real-World Examples](./DATA_SYSTEM_EXAMPLES.md)

### By Feature

#### Types
- [Guide: Defining Types](./DATA_BINDING_GUIDE.md#defining-types)
- [Reference: @type](./API_REFERENCE.md#type---type-definitions)
- [Spec: Type System](./DATA_SYSTEM.md#2-type-system)

#### Data Sources
- [Guide: Creating Data Files](./DATA_BINDING_GUIDE.md#creating-data-files)
- [Reference: @data](./API_REFERENCE.md#data---data-sources)
- [Spec: Data Sources](./DATA_SYSTEM.md#3-data-sources)

#### Template Binding
- [Guide: Using @each](./DATA_BINDING_GUIDE.md#using-each-for-lists)
- [Reference: @each](./API_REFERENCE.md#each---template-iteration)
- [Spec: Template Binding](./DATA_SYSTEM.md#6-template-binding-with-each)

#### Filters
- [Guide: Filters and Transformations](./DATA_BINDING_GUIDE.md#filters-and-transformations)
- [Reference: Filters](./API_REFERENCE.md#filters)
- [Spec: Filters](./DATA_SYSTEM.md#7-filters)

#### Reactive State
- [Guide: Reactive State for Loading](./DATA_BINDING_GUIDE.md#reactive-state-for-loading)
- [Reference: Data Events](./API_REFERENCE.md#data-events)
- [Spec: Reactive State with Signals](./DATA_SYSTEM.md#8-reactive-state-with-signals)

### By Use Case

#### Building a Product Gallery
1. [Guide: Pattern 1 - Product Gallery](./DATA_BINDING_GUIDE.md#pattern-1-product-gallery)
2. [Example: zeystudios Gallery](./DATA_SYSTEM_EXAMPLES.md#1-zeystudios)
3. [Working Demo](../examples/data-binding/)

#### Shopping Cart
1. [Guide: Pattern 2 - Shopping Cart](./DATA_BINDING_GUIDE.md#pattern-2-shopping-cart)
2. [Example: zeystudios Cart](./DATA_SYSTEM_EXAMPLES.md#cart-binding)

#### Blog Posts / Articles
1. [Guide: Pattern 3 - Featured Items](./DATA_BINDING_GUIDE.md#pattern-3-featured-items-filter)
2. [Example: ikarchitecte Projects](./DATA_SYSTEM_EXAMPLES.md#2-ikarchitecte)

#### Testimonials
1. [Guide: Pattern 5 - Testimonials](./DATA_BINDING_GUIDE.md#pattern-5-testimonials)
2. [Example: jallete Testimonials](./DATA_SYSTEM_EXAMPLES.md#testimonials-binding)

#### E-commerce Packs
1. [Example: jallete Packs](./DATA_SYSTEM_EXAMPLES.md#3-jallete)

---

## Documentation by Audience

### I'm a Designer/Frontend Developer

**You want to:** Build beautiful, data-driven sites without complex backend code

**Start with:**
1. [Data Binding Guide](./DATA_BINDING_GUIDE.md)
2. [Working Example](../examples/data-binding/)
3. [Common Patterns](./DATA_BINDING_GUIDE.md#common-patterns)

**Reference:**
- [Filters](./API_REFERENCE.md#filters) for data formatting
- [Error Codes](./API_REFERENCE.md#error-codes) when stuck

### I'm a Full-Stack Developer

**You want to:** Understand the system deeply and build complex features

**Start with:**
1. [Data System Specification](./DATA_SYSTEM.md)
2. [API Reference](./API_REFERENCE.md)
3. [Runtime API](../public/runtime/README.md)

**Reference:**
- [Computed Data](./API_REFERENCE.md#computed---computed-data)
- [Helper Functions](./API_REFERENCE.md#fn---helper-functions)
- [Type System](./DATA_SYSTEM.md#2-type-system)

### I Have an Existing Spacetime Site

**You want to:** Migrate to data binding without breaking things

**Start with:**
1. [Migration Guide](./MIGRATION.md)
2. [Before/After Examples](./MIGRATION.md#before--after-examples)
3. [Backward Compatibility](./MIGRATION.md#backward-compatibility)

**Reference:**
- [Migration Strategy](./MIGRATION.md#migration-strategy)
- [Common Pitfalls](./MIGRATION.md#common-pitfalls)

### I'm Debugging an Issue

**You need:** Quick answers to specific problems

**Go to:**
1. [Troubleshooting](./DATA_BINDING_GUIDE.md#troubleshooting)
2. [Error Codes](./API_REFERENCE.md#error-codes)
3. [Common Pitfalls](./MIGRATION.md#common-pitfalls)

---

## Document Summaries

### DATA_BINDING_GUIDE.md
**Audience:** Beginners to intermediate users
**Format:** Tutorial-style walkthrough
**Length:** ~8,000 words
**What you'll learn:** How to use data binding from scratch

### API_REFERENCE.md
**Audience:** All users
**Format:** Reference manual
**Length:** ~12,000 words
**What you'll learn:** Complete syntax and options for every feature

### MIGRATION.md
**Audience:** Existing Spacetime users
**Format:** Step-by-step guide
**Length:** ~5,000 words
**What you'll learn:** How to migrate existing sites safely

### DATA_SYSTEM.md
**Audience:** Advanced users, contributors
**Format:** Technical specification
**Length:** ~10,000 words
**What you'll learn:** How the system works internally

### DATA_SYSTEM_EXAMPLES.md
**Audience:** Intermediate to advanced users
**Format:** Real-world examples
**Length:** ~10,000 words
**What you'll learn:** How to build complete features

### Runtime README
**Audience:** JavaScript developers
**Format:** API documentation
**Length:** ~4,000 words
**What you'll learn:** How to use the runtime library directly

---

## Cheat Sheets

### Quick Syntax Reference

```css
/* Define a type */
@type Product {
    name: string;
    price: number;
    tags?: string[];
}

/* Load data */
@data products: Product[] {
    src: "/data/products.json";
}

/* Bind to template */
.product-list {
    @each(products) {
        template: "product-card";
        [slot="name"]: $.name;
        [slot="price"]: $.price | currency("$");
    }
}
```

### Common Filters

```css
$.price | currency("$")         /* $95.00 */
$.date | date("short")          /* "Dec 16" */
$.title | truncate(50)          /* "Long title..." */
$.inStock | if("Yes", "No")     /* Conditional */
$.tags | count                  /* 3 */
$.text | uppercase              /* "HELLO" */
```

### Reactive State Pattern

```css
/* Signal set via data event or @on &.click */
$status <- "loading"

.container {
    data-state: $status;

    @state(when: "loading") { opacity: 0.5; }
    @state(when: "ready")   { opacity: 1; }
}
```

---

## Contributing to Documentation

Found an error or want to improve the docs?

1. All docs are in `./`
2. Use clear, simple language
3. Include working code examples
4. Add cross-references to related topics
5. Update this index when adding new docs

---

## Version History

- **v1.0** (Current) - Initial data binding documentation
  - Complete guide for users
  - API reference
  - Migration guide
  - Real-world examples
  - Working demo

---

## Quick Access

### Most Used Documents

1. [Data Binding Guide](./DATA_BINDING_GUIDE.md) - Start here
2. [API Reference](./API_REFERENCE.md) - Look up syntax
3. [Troubleshooting](./DATA_BINDING_GUIDE.md#troubleshooting) - Fix problems

### Example Code

- [Working Demo](../examples/data-binding/)
- [Real Sites](./DATA_SYSTEM_EXAMPLES.md)

---

**Need help?** Start with the [Data Binding Guide](./DATA_BINDING_GUIDE.md) and refer to the [API Reference](./API_REFERENCE.md) as needed.
