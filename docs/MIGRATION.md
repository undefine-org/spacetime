# Migration Guide: Adopting Data Binding in Spacetime

**A step-by-step guide for migrating existing Spacetime sites to the new data binding system.**

> **PLAN-076/079 (2026-07): mechanical migrations are now toolchain-applied.**
> Syntax retirements are declared as `%migration` capsule entries in the
> stdlib and applied FOR you — by the migrations pill in the dev dock (next
> to the host pill) or by `spacetime migrate <project>`. This guide remains
> the conceptual background for the FEAT-072 data-binding cutover; for the
> migration system itself (waves, `@version`, capsules), see
> [docs/language/migrations.md](language/migrations.md).
>
> **The 2026-06-09 `reactive-surface` wave** retired the paren-macro
> reactive surface. Per property:
>
> | retired | now |
> |---|---|
> | `@bind(text: $x)` | `text <- $x;` |
> | `@bind(attr: "src", value: $x)` | `` `src` <- $x; `` → `src <- $x;` |
> | `@bind(class: "open", when: $c)` | `.open: $c;` |
> | `@bind(style: "color", value: $v)` | `color: $v;` |
> | `@bind(visible: $v)` | `.shown: $v;` (or your own class) |
> | `@show(when: $c)` | `.hidden: !$c;` + a `.hidden { display: none }` rule |
> | `@input(bind: $q)` | `value <- $q;` + `@on &.input { $q <- &el.value; }` |
>
> The automatic rows are applied by the pill/CLI; `@show`/`@input` and
> multi-property `@bind` calls are **manual** (hint-guided — split them one
> property per statement).

---

## Table of Contents

1. [Overview](#overview)
2. [What's New](#whats-new)
3. [Retired Imperative State-Machine Directives](#retired-imperative-state-machine-directives)
4. [Should You Migrate?](#should-you-migrate)
5. [Migration Strategy](#migration-strategy)
6. [Step-by-Step Migration](#step-by-step-migration)
7. [Before & After Examples](#before--after-examples)
8. [Backward Compatibility](#backward-compatibility)
9. [Common Pitfalls](#common-pitfalls)
10. [FAQ](#faq)

---

## Overview

Spacetime's data binding system is a **new optional feature** that makes it easier to build data-driven sites. You can adopt it incrementally—you don't need to migrate your entire site at once.

### What This Guide Covers

- What's new in data binding
- How to decide what to migrate
- Step-by-step migration process
- Real-world before/after examples
- Backward compatibility guarantees

---

## What's New

### Data Binding Features

The data binding system introduces:

1. **Type Definitions** (`@type`) - Define your data structure with type safety
2. **Data Sources** (`@data`) - Load from JSON, localStorage, or inline
3. **Template Iteration** (`@each`) - Generate elements from data
4. **Filters** - Transform data inline (currency, dates, etc.)
5. **Computed Data** (`@computed`) - Derive data from other sources
6. **Helper Functions** (`@fn`) - Reusable logic
7. **Reactive Signals and Classes** (`$`) - Loading states, empty states, errors

### New Syntax

```css
/* Type definition */
@type Product {
    name: string;
    price: number;
}

/* Data source */
@data products: Product[] {
    src: "/data/products.json";
}

/* Template binding */
.product-list {
    @each(products) {
        template: "product-card";
        [slot="name"]: $.name;
        [slot="price"]: $.price | currency("$");
    }
}
```

---

## Retired Imperative State-Machine Directives

> PLAN-047 retired the imperative finite-state-machine directives `@state_machine(...)`, `@transition(from:..., to:..., on:...)`, and `@mutate(...)`. They are no longer recommended live syntax.

Replace them with **reactive `$`-signals + reactive classes** (or `@view $sig { ... }` for swapping DOM). Define a signal such as `$status <- "loading"`, update it from data events or user events, then drive classes with boolean expressions:

```css
.product-list {
    $status <- "loading";

    .is-loading: $status == "loading";
    .is-ready: $status == "ready";

    @on data:products:loaded {
        $status <- "ready";
    }
}

.product-list.is-loading {
    min-height: 400px;
    opacity: 0.5;
}

.product-list.is-ready {
    opacity: 1;
}
```

For mutually-exclusive states, use a string/number signal with per-state reactive classes (`.is-step-2: $step == 2`) or use `@view $sig { "a" => &tplA(); "b" => &tplB(); }` to swap DOM.

`@state(when: "...") { ... }` remains valid as the `data-st-state` CSS renderer, but the state should now be set through a reactive `data-state` attribute/class or a `$`-signal rather than the retired `@state_machine` directive.

### Old:

```css
.product-list {
    @state_machine(initial: "loading");

    @state(when: "loading") {
        min-height: 400px;
        opacity: 0.5;
    }

    @state(when: "ready") {
        opacity: 1;
    }

    @transition(from: "loading", to: "ready", on: "data:products:loaded");
}
```

### New:

```css
.product-list {
    $status <- "loading";

    .is-loading: $status == "loading";
    .is-ready: $status == "ready";

    @on data:products:loaded {
        $status <- "ready";
    }
}

.product-list.is-loading {
    min-height: 400px;
    opacity: 0.5;
}

.product-list.is-ready {
    opacity: 1;
}
```

## Should You Migrate?

### Good Candidates for Migration

Migrate if your site has:

- **Repeated HTML blocks** - Same structure copied many times
- **Data in HTML** - Prices, IDs, or structured data in attributes
- **Manual maintenance** - Adding items means copying HTML
- **Growing content** - You expect to add more items over time

Examples:
- Product galleries
- Blog post lists
- Team member pages
- Testimonial sections
- FAQ sections
- Navigation menus

### Keep As-Is

Don't migrate if:

- **Static, one-off content** - Hero sections, about pages
- **Complex custom layouts** - Each item is visually unique
- **Small, fixed lists** - 2-3 items that never change
- **Working fine** - If it ain't broke, don't fix it

---

## Migration Strategy

### Incremental Adoption

You can migrate one section at a time:

1. Start with the most repetitive section (e.g., product gallery)
2. Test thoroughly
3. Move to the next section
4. Repeat

Your site will work with a mix of old and new approaches.

### Migration Priority

Migrate in this order:

1. **High-value, high-repetition** - Product galleries, blog lists
2. **Medium repetition** - Testimonials, FAQ, team pages
3. **Low repetition** - Navigation, footer links
4. **Static content** - Don't migrate

---

## Step-by-Step Migration

### Example: Product Gallery

Let's migrate a product gallery from manual HTML to data binding.

#### Step 1: Identify Repeated Structure

**Before (manual HTML):**

```html
<!-- Repeated 20 times -->
<div class="product-card" data-id="prod-1">
  <img src="/images/prod-1.jpg" alt="Product 1">
  <h3>Product 1</h3>
  <p>Description of product 1</p>
  <span class="price">$29.99</span>
</div>

<div class="product-card" data-id="prod-2">
  <img src="/images/prod-2.jpg" alt="Product 2">
  <h3>Product 2</h3>
  <p>Description of product 2</p>
  <span class="price">$39.99</span>
</div>

<!-- ...18 more copies... -->
```

Identify what changes between items:
- `data-id`: "prod-1", "prod-2", etc.
- Image `src` and `alt`
- `h3` text
- `p` text
- Price

#### Step 2: Create a Template

Extract one item into a `<template>`:

```html
<template id="product-card">
  <div class="product-card">
    <img slot="image" src="" alt="">
    <h3 slot="name"></h3>
    <p slot="description"></p>
    <span slot="price" class="price"></span>
  </div>
</template>

<div class="product-list">
  <!-- Products will be generated here -->
</div>
```

Mark variable parts with `slot` attributes.

#### Step 3: Extract Data to JSON

Create `/data/products.json`:

```json
[
  {
    "id": "prod-1",
    "name": "Product 1",
    "description": "Description of product 1",
    "imageUrl": "/images/prod-1.jpg",
    "price": 29.99
  },
  {
    "id": "prod-2",
    "name": "Product 2",
    "description": "Description of product 2",
    "imageUrl": "/images/prod-2.jpg",
    "price": 39.99
  }
]
```

Copy data from your HTML into this structure.

#### Step 4: Define the Type

In your `.st` file:

```css
@type Product {
    id: string;
    name: string;
    description: string;
    imageUrl: url;
    price: number;
}
```

This matches your JSON structure.

#### Step 5: Register Data Source

```css
@data products: Product[] {
    src: "/data/products.json";
}
```

#### Step 6: Bind Template

```css
.product-list {
    $status <- "loading";

    .is-loading: $status == "loading";
    .is-ready: $status == "ready";

    @on data:products:loaded {
        $status <- "ready";
    }

    @each(products) {
        template: "product-card";

        [slot="image"] {
            src: $.imageUrl;
            alt: $.name;
            loading: "lazy";
        }
        [slot="name"]: $.name;
        [slot="description"]: $.description;
        [slot="price"]: $.price | currency("$");

        :host {
            data-id: $.id;
        }
    }

    /* Preserve your existing animations */
    > .product-card {
        @scroll reveal(&reveal) {
            opacity: 0 -> 1;
            translate-y: 40px -> 0;
            stagger: 0.1 first;
        }
    }
}

.product-list.is-loading {
    min-height: 400px;
    opacity: 0.5;
}

.product-list.is-ready {
    opacity: 1;
}
```

#### Step 7: Test

1. Compile your site
2. Check that products load
3. Verify all data appears correctly
4. Test animations still work

#### Step 8: Clean Up

Remove the old manual HTML.

---

## Before & After Examples

### Example 1: zeystudios Gallery

#### Before

**HTML (gallery.html):** 400+ lines

```html
<zey-print data-id="IN-003" data-prices='{"S":95,"M":195,"L":395}'>
  <img slot="image" src="/prints/IN-003.jpg" alt="Between Worlds">
  <span slot="title">Between Worlds</span>
  <span slot="subtitle">Surfer in morning mist, Morocco</span>
</zey-print>

<zey-print data-id="IN-004" data-prices='{"S":95,"M":195,"L":395}'>
  <img slot="image" src="/prints/IN-004.jpg" alt="Golden Hour">
  <span slot="title">Golden Hour</span>
  <span slot="subtitle">Desert dunes at sunset</span>
</zey-print>

<!-- ...repeated 21 more times... -->
```

**Issues:**
- Copy-paste errors (wrong ID, mismatched images)
- Hard to change price structure
- Can't easily filter or sort
- 400 lines of repetitive HTML

#### After

**HTML (gallery.html):** 10 lines

```html
<template id="zey-print">
  <zey-print>
    <img slot="image" src="" alt="">
    <span slot="title"></span>
    <span slot="subtitle"></span>
  </zey-print>
</template>

<div class="zey-gallery"></div>
```

**JSON (/data/prints.json):**

```json
[
  {
    "id": "IN-003",
    "title": "Between Worlds",
    "subtitle": "Surfer in morning mist, Morocco",
    "image": "/prints/IN-003.jpg",
    "prices": { "S": 95, "M": 195, "L": 395 }
  },
  {
    "id": "IN-004",
    "title": "Golden Hour",
    "subtitle": "Desert dunes at sunset",
    "image": "/prints/IN-004.jpg",
    "prices": { "S": 95, "M": 195, "L": 395 }
  }
]
```

**Spacetime (zeystudios.st):**

```css
@type Print {
    id: string;
    title: string;
    subtitle: string;
    image: url;
    prices: { S: number; M: number; L: number; };
}

@data prints: Print[] {
    src: "/data/prints.json";
}

.zey-gallery {
    @each(prints) {
        template: "zey-print";

        [slot="image"] {
            src: $.image;
            alt: $.title;
        }
        [slot="title"]: $.title;
        [slot="subtitle"]: $.subtitle;

        :host {
            data-id: $.id;
            data-prices: $.prices | json;
        }
    }
}
```

**Benefits:**
- 97% less HTML
- Type-safe (catches errors at compile time)
- Easy to add prints (just update JSON)
- Can filter, sort, paginate
- Animations still work

### Example 2: Shopping Cart with localStorage

#### Before

**HTML:**

```html
<div class="cart-items">
  <!-- Manually managed with JavaScript -->
</div>

<script>
  // 50+ lines of cart management code
  function updateCart() {
    const cart = JSON.parse(localStorage.getItem('cart') || '[]');
    const container = document.querySelector('.cart-items');
    container.innerHTML = '';

    cart.forEach(item => {
      const div = document.createElement('div');
      div.className = 'cart-item';
      div.innerHTML = `
        <img src="/products/${item.id}.jpg">
        <span>${item.name}</span>
        <span>$${item.price}</span>
      `;
      container.appendChild(div);
    });
  }

  updateCart();
  // More event listeners, etc.
</script>
```

#### After

**HTML:**

```html
<template id="cart-item">
  <div class="cart-item">
    <img slot="image" src="" alt="">
    <span slot="name"></span>
    <span slot="price"></span>
  </div>
</template>

<div class="cart-items"></div>
```

**Spacetime:**

```css
@type CartItem {
    productId: string;
    quantity: number;
}

@data cart: CartItem[] {
    src: localStorage("cart");
    default: [];
}

@data products: Product[] {
    src: "/data/products.json";
}

.cart-items {
    $cartStatus <- "checking";

    .is-empty: $cartStatus == "empty";
    .has-items: $cartStatus == "has-items";

    @on data:cart:empty {
        $cartStatus <- "empty";
    }

    @on data:cart:loaded {
        $cartStatus <- "has-items";
    }

    @each(cart) {
        template: "cart-item";

        @let product = products.find(p => p.id == $.productId);

        [slot="image"] {
            src: product.image;
            alt: product.name;
        }
        [slot="name"]: product.name;
        [slot="price"]: product.price * $.quantity | currency("$");
    }
}

.cart-items.is-empty::after {
    content: "Your cart is empty";
}
```

**Benefits:**
- No manual DOM manipulation
- Automatic reactivity (updates when localStorage changes)
- Type-safe cross-references
- Loading/empty states built-in

---

## Backward Compatibility

### Guaranteed Compatible

These features work alongside data binding:

- **All existing animations** - `@scroll`, `@on`, etc.
- **Reactive signals and classes** - Data events integrate seamlessly
- **Custom elements** - Generated elements work like manual ones
- **CSS** - All existing styles apply

### No Breaking Changes

Spacetime data binding is **100% additive** for sites that have not used the retired directives:

- Existing data binding syntax still works
- Sites without data binding compile identically
- Incremental adoption is safe

The only retired syntax is the imperative finite-state-machine family (`@state_machine`, `@transition`, `@mutate`). See [Retired Imperative State-Machine Directives](#retired-imperative-state-machine-directives) for the recommended replacement.

### Version Requirements

- **Spacetime version:** 0.9.0+
- **Browser support:** Same as before (ES6+)

### Internal Improvements (v0.9.5+)

The data binding system now uses a unified 7-phase compilation pipeline in `compile_macro_data_bindings()`. This is an internal change that:

- **No syntax changes** - All features work exactly the same
- **Better ordering** - SpacetimeLocal, element refs, @data, @computed, @fn, @each, and @on are processed in dependency order
- **Cleaner output** - Generated JavaScript is more organized
- **Developer-friendly** - Single entry point for all data binding code generation

For implementation details, see [src/metasystem/ARCHITECTURE.md](../src/metasystem/ARCHITECTURE.md).

---

## Common Pitfalls

### Pitfall 1: Forgetting Optional Fields

**Problem:**

```css
@type Product {
    description: string;        /* Required */
}
```

But your JSON has:

```json
{ "name": "Product", "description": null }
```

**Error:** Type mismatch

**Solution:**

```css
@type Product {
    description?: string;       /* Optional */
}
```

### Pitfall 2: Wrong Template ID

**Problem:**

```css
@each(products) {
    template: "product-card";   /* HTML has id="product-item" */
}
```

**Error:** Template not found

**Solution:** Match IDs exactly:

```html
<template id="product-card">
```

### Pitfall 3: Missing Slots

**Problem:**

```css
[slot="price"]: $.price;
```

But template has:

```html
<span class="price"></span>    <!-- No slot attribute -->
```

**Error:** Binding failed

**Solution:**

```html
<span slot="price" class="price"></span>
```

### Pitfall 4: Incorrect JSON Path

**Problem:**

```css
src: "data/products.json";      /* Missing leading / */
```

**Error:** 404 Not Found

**Solution:**

```css
src: "/data/products.json";     /* Absolute path */
```

### Pitfall 5: Filter Syntax

**Problem:**

```css
[slot="price"]: $.price | currency($);     /* Missing quotes */
```

**Error:** Invalid filter arguments

**Solution:**

```css
[slot="price"]: $.price | currency("$");   /* String argument */
```

---

## FAQ

### Can I mix old and new approaches?

**Yes!** You can have some sections using data binding and others using manual HTML. They coexist peacefully.

### Do I need to migrate everything?

**No.** Migrate only what makes sense. Static content can stay as-is.

### Will my animations still work?

**Yes.** Elements generated by `@each` work exactly like manual elements. All animations apply.

### What about SEO?

Generated content is rendered at build time (if using static generation) or client-side (if dynamic). For SEO-critical content, consider:

- Server-side rendering (if available)
- Pre-rendering the page
- Using static data (not dynamically loaded)

### Can I still use JavaScript?

**Yes.** Data binding doesn't replace JavaScript—it complements it. You can still:

- Add event listeners
- Manipulate the DOM
- Use external libraries

### What if I need custom logic?

Use `@fn` helper functions or `@computed` data for complex logic. For very custom cases, JavaScript is still available.

### How do I debug?

1. Enable debug mode: `?debug=spacetime` in URL
2. Check browser console for errors
3. Use compile-time validation (catches most errors)

### What about performance?

Data binding is optimized:

- Templates are cloned, not re-parsed
- Slot lookups are cached
- Large lists (1000+) perform well

For very large datasets, consider pagination.

### Can I use this with a CMS?

**Yes.** Your CMS can:

- Generate the JSON data files
- Update them when content changes
- Trigger rebuilds

Many headless CMSs work great with this approach.

### What if my data changes frequently?

Use the `refresh` option:

```css
@data liveData: Stats {
    src: "/api/stats";
    cache: none;
    refresh: 30s;           /* Poll every 30 seconds */
}
```

---

## Migration Checklist

Use this checklist when migrating a section:

- [ ] Identify repeated HTML structure
- [ ] Create template with slot attributes
- [ ] Extract data to JSON
- [ ] Define type matching JSON structure
- [ ] Register data source
- [ ] Write `@each` binding
- [ ] Add loading states (if needed)
- [ ] Test compilation
- [ ] Verify data loads correctly
- [ ] Check animations still work
- [ ] Test responsive layout
- [ ] Remove old HTML
- [ ] Commit changes

---

## Getting Help

If you get stuck:

1. Check the [Data Binding Guide](./DATA_BINDING_GUIDE.md)
2. Review the [API Reference](./API_REFERENCE.md)
3. Look at [Examples](./DATA_SYSTEM_EXAMPLES.md)
4. Check error messages (compile-time errors are detailed)

---

## Summary

### Key Takeaways

1. **Data binding is optional** - Migrate only what makes sense
2. **100% backward compatible** - Old code still works
3. **Incremental adoption** - One section at a time
4. **Type-safe** - Errors caught at compile time
5. **Animations preserved** - Everything still works

### Next Steps

1. Read the [Data Binding Guide](./DATA_BINDING_GUIDE.md)
2. Try migrating one small section
3. Expand to more sections as you get comfortable
4. Enjoy cleaner, more maintainable code!

Happy migrating!
