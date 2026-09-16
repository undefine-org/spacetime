# Spacetime Data Binding System

**A declarative, type-safe system for binding data to templates in Spacetime.**

---

## Table of Contents

1. [Overview](#1-overview)
2. [Type System](#2-type-system)
3. [Data Sources](#3-data-sources)
4. [Computed Data](#4-computed-data)
5. [Helper Functions](#5-helper-functions)
6. [Template Binding with @each](#6-template-binding-with-each)
7. [Filters](#7-filters)
8. [Reactive State with Signals](#8-reactive-state-with-signals)
9. [Complete Example](#9-complete-example)

---

## 1. Overview

### The Problem

Current Spacetime sites have repetitive, manually-duplicated HTML:

```html
<!-- Repeated 23 times in zeystudios gallery -->
<zey-print data-id="IN-003" data-prices='{"S":95,"M":195,"L":395}'>
  <img slot="image" src="/prints/IN-003.jpg" alt="Between Worlds">
  <span slot="title">Between Worlds</span>
  <span slot="subtitle">Surfer in morning mist, Morocco</span>
</zey-print>
```

This is:
- **Error-prone**: Copy-paste mistakes, inconsistent data
- **Hard to maintain**: Changing price structure requires editing every item
- **Not scalable**: Adding 100 items means 100 copy-paste operations

### The Solution

Spacetime Data Binding separates **data** from **templates**:

```json
// /data/prints.json
[
  {
    "id": "IN-003",
    "title": "Between Worlds",
    "subtitle": "Surfer in morning mist, Morocco",
    "image": "/prints/IN-003.jpg",
    "prices": { "S": 95, "M": 195, "L": 395 }
  }
]
```

```css
/* zeystudios.st */
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

```html
<!-- index.html — now just one line -->
<div class="zey-gallery"></div>
```

### Key Benefits

1. **Type Safety**: Catch errors at compile time, not runtime
2. **Single Source of Truth**: Data lives in JSON, templates in HTML, behavior in .st
3. **Compile-Time Validation**: Missing fields, wrong types, orphan templates
4. **Scalability**: Add 1000 items by updating JSON, not HTML
5. **Animations Just Work**: Generated elements inherit Spacetime animations

---

## 2. Type System

### Basic Syntax

```css
@type TypeName {
    field1: type;
    field2: type;
    optionalField?: type;
}
```

### Primitive Types

| Type | Description | Example Values |
|------|-------------|----------------|
| `string` | Text | `"Hello"`, `"IN-003"` |
| `number` | Numeric | `42`, `3.14`, `-100` |
| `boolean` | True/false | `true`, `false` |
| `url` | URL string | `"/images/photo.jpg"`, `"https://..."` |
| `color` | Color value | `"#ff0000"`, `"rgb(255,0,0)"` |

### Object Types

Inline object definitions:

```css
@type Product {
    name: string;
    prices: {
        small: number;
        medium: number;
        large: number;
    };
    metadata: {
        created: string;
        author: {
            name: string;
            email: string;
        };
    };
}
```

### Array Types

Append `[]` to any type:

```css
@type Gallery {
    title: string;
    images: string[];           /* Array of strings */
    prints: Print[];            /* Array of Print objects */
    tags: string[];
}
```

### Optional Fields

Use `?` after the field name:

```css
@type Print {
    id: string;                 /* Required */
    title: string;              /* Required */
    subtitle?: string;          /* Optional */
    featured?: boolean;         /* Optional, defaults to undefined */
    tags?: string[];            /* Optional array */
}
```

### Union Types (Enums)

Restrict to specific string values:

```css
@type CartItem {
    printId: string;
    size: "S" | "M" | "L" | "XL";
    quantity: number;
}

@type Order {
    status: "pending" | "processing" | "shipped" | "delivered";
}
```

### Type References

Types can reference other types:

```css
@type Author {
    name: string;
    email: string;
}

@type Post {
    title: string;
    author: Author;             /* Reference to Author type */
    comments: Comment[];        /* Array of Comment type */
}

@type Comment {
    text: string;
    author: Author;
}
```

### Complete Type Example

```css
@type Price {
    S: number;
    M: number;
    L: number;
}

@type Print {
    id: string;
    title: string;
    subtitle: string;
    image: url;
    prices: Price;
    featured?: boolean;
    tags?: string[];
    curation?: string;          /* ID reference to Curation */
}

@type Curation {
    id: string;
    slug: string;
    title: string;
    description: string;
    hero: url;
    accent: color;
    prints: string[];           /* Array of Print IDs */
}
```

---

## 3. Data Sources

### Basic Syntax

```css
@data sourceName: Type {
    src: "path/to/data.json";
    /* options */
}
```

### JSON File Source

```css
@data prints: Print[] {
    src: "/data/prints.json";
}

@data curations: Curation[] {
    src: "/data/curations.json";
}
```

### localStorage Source

```css
@data cart: CartItem[] {
    src: localStorage("zey-cart");
    default: [];                /* Default if key doesn't exist */
}

@data preferences: UserPrefs {
    src: localStorage("user-prefs");
    default: {
        theme: "light",
        currency: "USD"
    };
}
```

### Inline Source (Embedded)

For small, static data:

```css
@data sizes: Size[] {
    src: inline;
    value: [
        { "code": "S", "label": "Small", "dimensions": "8 × 10 in" },
        { "code": "M", "label": "Medium", "dimensions": "16 × 20 in" },
        { "code": "L", "label": "Large", "dimensions": "24 × 30 in" }
    ];
}
```

### Cache Options

```css
@data prints: Print[] {
    src: "/data/prints.json";
    cache: 1h;                  /* Cache for 1 hour */
}

@data liveData: Stats {
    src: "/api/stats";
    cache: none;                /* Always fetch fresh */
}

@data staticContent: Content {
    src: "/data/content.json";
    cache: forever;             /* Cache indefinitely */
}
```

### Refresh Options

```css
@data notifications: Notification[] {
    src: "/api/notifications";
    refresh: 30s;               /* Poll every 30 seconds */
}

@data cart: CartItem[] {
    src: localStorage("cart");
    refresh: on-focus;          /* Refresh when tab gains focus */
}
```

---

## 4. Computed Data

Derive new data from existing sources.

### Filtering

```css
@computed featuredPrints: Print[] {
    from: prints;
    where: $.featured == true;
}

@computed affordablePrints: Print[] {
    from: prints;
    where: $.prices.S < 100;
}

@computed moroccanPrints: Print[] {
    from: prints;
    where: $.tags.includes("morocco");
}
```

### Sorting

```css
@computed printsByDate: Print[] {
    from: prints;
    sort: $.createdAt desc;
}

@computed printsByPrice: Print[] {
    from: prints;
    sort: $.prices.S asc;
}
```

### Limiting

```css
@computed topPrints: Print[] {
    from: prints;
    where: $.featured == true;
    sort: $.sales desc;
    limit: 6;
}
```

### Combining Operations

```css
@computed heroCarousel: Print[] {
    from: prints;
    where: $.featured == true;
    sort: $.order asc;
    limit: 5;
}
```

### Aggregation

```css
@computed cartTotal: number {
    from: cart;
    reduce: (sum, item) => sum + priceFor(item.printId, item.size) * item.quantity;
    initial: 0;
}

@computed itemCount: number {
    from: cart;
    reduce: (count, item) => count + item.quantity;
    initial: 0;
}
```

---

## 5. Helper Functions

Reusable logic for computed data and bindings.

### Basic Syntax

```css
@fn functionName(param: Type, param2: Type): ReturnType {
    /* function body */
}
```

### Examples

```css
/* Look up a print by ID */
@fn getPrint(id: string): Print? {
    return prints.find(p => p.id == id);
}

/* Get price for a print and size */
@fn priceFor(printId: string, size: "S" | "M" | "L"): number {
    let print = getPrint(printId);
    return print?.prices[size] ?? 0;
}

/* Format price with currency */
@fn formatPrice(amount: number, currency: string): string {
    return currency + amount.toFixed(2);
}

/* Check if print is in cart */
@fn isInCart(printId: string): boolean {
    return cart.some(item => item.printId == printId);
}
```

### Using Functions in Bindings

```css
.cart-item {
    @each(cart) {
        template: "cart-item";

        @let print = getPrint($.printId);

        [slot="image"] { src: print.image; }
        [slot="title"]: print.title;
        [slot="price"]: priceFor($.printId, $.size) | currency("$");
        [slot="quantity"]: $.quantity;
    }
}
```

---

## 6. Template Binding with @each

### Basic Syntax

```css
.container {
    @each(dataSource) {
        template: "template-name";

        /* Slot bindings */
        [slot="name"]: $.property;

        /* Attribute bindings */
        [slot="name"] {
            attr: $.property;
        }

        /* Host (root element) bindings */
        :host {
            data-id: $.id;
        }
    }
}
```

### Property Paths

Access nested data with `$.path.to.property`:

```css
@each(prints) {
    [slot="title"]: $.title;
    [slot="author"]: $.metadata.author.name;
    [slot="price"]: $.prices.S;
}
```

### Special Variables

| Variable | Description |
|----------|-------------|
| `$` | Current item in iteration |
| `$._index` | Zero-based index |
| `$._first` | Boolean: is first item |
| `$._last` | Boolean: is last item |
| `$._count` | Total number of items |

```css
@each(prints) {
    :host {
        data-index: $._index;
        class: $._first ? "first" : ($._last ? "last" : "");
    }
}
```

### Slot Binding Syntax

**Text content:**
```css
[slot="title"]: $.title;
```

**Single attribute:**
```css
[slot="image"] { src: $.image; }
```

**Multiple attributes:**
```css
[slot="image"] {
    src: $.image;
    alt: $.title;
    loading: "lazy";
}
```

### Host Binding

Bind to the generated component's root element:

```css
@each(prints) {
    template: "zey-print";

    :host {
        data-id: $.id;
        data-prices: $.prices | json;
        data-featured: $.featured | default(false);
        class: $.featured ? "featured" : "";
    }
}
```

### Cross-Reference with @let

Look up related data:

```css
.cart-items {
    @each(cart) {
        template: "cart-item";

        /* Look up the print for this cart item */
        @let print = prints.find(p => p.id == $.printId);

        [slot="image"] { src: print.image; }
        [slot="title"]: print.title;
        [slot="size"]: $.size;
        [slot="quantity"]: $.quantity;
        [slot="price"]: print.prices[$.size] * $.quantity | currency("$");
    }
}
```

### Nested @each

For nested data structures:

```css
@type Pack {
    id: string;
    title: string;
    items: PackItem[];
}

@type PackItem {
    name: string;
    price: number;
}

.pack-list {
    @each(packs) {
        template: "pack-card";

        [slot="title"]: $.title;

        [slot="items"] {
            @each($.items) {
                template: "pack-item";

                [slot="name"]: $.name;
                [slot="price"]: $.price | currency("dhs");
            }
        }
    }
}
```

---

## 7. Filters

Transform data inline with the `|` pipe syntax.

### Built-in Filters

| Filter | Description | Example |
|--------|-------------|---------|
| `json` | Serialize to JSON | `$.prices \| json` → `'{"S":95}'` |
| `currency(symbol)` | Format as currency | `$.price \| currency("$")` → `"$95.00"` |
| `date(format)` | Format date | `$.created \| date("short")` → `"Dec 16"` |
| `default(value)` | Fallback value | `$.subtitle \| default("No description")` |
| `count` | Array length | `$.items \| count` → `3` |
| `truncate(n)` | Truncate string | `$.title \| truncate(20)` → `"Between Wor..."` |
| `if(true, false)` | Conditional | `$.inStock \| if("Available", "Sold Out")` |
| `uppercase` | Uppercase | `$.code \| uppercase` → `"ABC"` |
| `lowercase` | Lowercase | `$.name \| lowercase` → `"abc"` |
| `capitalize` | Capitalize first | `$.name \| capitalize` → `"Hello world"` |
| `pluralize(s, p)` | Pluralize | `$.count \| pluralize("item", "items")` |

### Filter Chaining

```css
[slot="title"]: $.title | truncate(30) | uppercase;
[slot="price"]: $.price | default(0) | currency("$");
```

### Date Format Options

```css
$.date | date("short")      /* "Dec 16" */
$.date | date("medium")     /* "Dec 16, 2024" */
$.date | date("long")       /* "December 16, 2024" */
$.date | date("iso")        /* "2024-12-16" */
$.date | date("relative")   /* "2 days ago" */
```

### Currency Format

```css
$.price | currency("$")     /* "$95.00" */
$.price | currency("€")     /* "€95.00" */
$.price | currency("dhs")   /* "95 dhs" */
$.price | currency("")      /* "95.00" */
```

---

## 8. Reactive State with Signals

Data loading status is a signal. Track it with a `$`-signal, expose it through reactive classes, and drive `@state(when:)` via a reactive `data-state` attribute.

### Signal-Driven Loading States

```css
.gallery {
    @data prints: Print[] {
        src: "/data/prints.json";
    }

    $loadState <- "loading";

    data-state: $loadState;

    .is-loading: $loadState == "loading";
    .is-empty:   $loadState == "empty";
    .is-ready:   $loadState == "ready";
    .is-error:   $loadState == "error";

    @state(when: "loading") {
        min-height: 400px;
        opacity: 0.5;
    }

    @state(when: "empty") {
        /* Show "no items" message */
    }

    @state(when: "ready") {
        opacity: 1;
    }

    @state(when: "error") {
        /* Show error state */
    }

    /* Data events update the signal */
    @on data:prints:loaded { $loadState <- "ready"; }
    @on data:prints:empty  { $loadState <- "empty"; }
    @on data:prints:error   { $loadState <- "error"; }

    @each(prints) {
        template: "print-card";
        /* ... bindings ... */
    }
}
```


### Data Events

| Event | Description |
|-------|-------------|
| `data:{source}:loaded` | Data loaded successfully with items |
| `data:{source}:empty` | Data loaded but array is empty |
| `data:{source}:error` | Data failed to load |
| `data:{source}:updated` | Data was updated (reactive) |

### Animating Generated Children

Children generated by `@each` automatically participate in animations:

```css
.gallery {
    @each(prints) {
        template: "print-card";
        /* ... */
    }

    /* Stagger animation for generated cards */
    > print-card {
        @scroll reveal(&quick-reveal) {
            opacity: 0 -> 1;
            translate-y: 40px -> 0;
            stagger: 0.08 first;
        }

        @on &.hover lift(300ms) {
            translate-y: 0 -> -8px;
        }
    }
}
```

---

## 9. Complete Example

Here's a full example showing all features working together:

### Type Definitions

```css
/* zeystudios.st */

/* ============================================
   TYPE DEFINITIONS
   ============================================ */

@type Price {
    S: number;
    M: number;
    L: number;
}

@type Print {
    id: string;
    title: string;
    subtitle: string;
    image: url;
    prices: Price;
    featured?: boolean;
    tags?: string[];
}

@type Curation {
    id: string;
    slug: string;
    title: string;
    description: string;
    hero: url;
    accent: color;
    prints: string[];
}

@type CartItem {
    printId: string;
    size: "S" | "M" | "L";
    quantity: number;
}

/* ============================================
   DATA SOURCES
   ============================================ */

@data prints: Print[] {
    src: "/data/prints.json";
}

@data curations: Curation[] {
    src: "/data/curations.json";
}

@data cart: CartItem[] {
    src: localStorage("zey-cart");
    default: [];
}

/* ============================================
   COMPUTED DATA
   ============================================ */

@computed featuredPrints: Print[] {
    from: prints;
    where: $.featured == true;
    limit: 6;
}

@computed cartTotal: number {
    from: cart;
    reduce: (sum, item) => sum + priceFor(item.printId, item.size) * item.quantity;
    initial: 0;
}

/* ============================================
   HELPER FUNCTIONS
   ============================================ */

@fn getPrint(id: string): Print? {
    return prints.find(p => p.id == id);
}

@fn priceFor(printId: string, size: "S" | "M" | "L"): number {
    let print = getPrint(printId);
    return print?.prices[size] ?? 0;
}

/* ============================================
   GALLERY BINDING
   ============================================ */

.zey-gallery {
    $loadState <- "loading";

    data-state: $loadState;

    .is-loading: $loadState == "loading";
    .is-empty:   $loadState == "empty";
    .is-ready:   $loadState == "ready";

    @state(when: "loading") {
        min-height: 400px;
        opacity: 0.5;
    }

    @state(when: "ready") {
        opacity: 1;
    }

    @state(when: "empty") {
        /* Nothing to show */
    }

    @on data:prints:loaded { $loadState <- "ready"; }
    @on data:prints:empty  { $loadState <- "empty"; }

    @each(prints) {
        template: "zey-print";

        [slot="image"] {
            src: $.image;
            alt: $.title;
            loading: "lazy";
        }
        [slot="title"]: $.title;
        [slot="subtitle"]: $.subtitle;

        :host {
            data-id: $.id;
            data-prices: $.prices | json;
            data-featured: $.featured | default(false);
        }
    }

    /* Animations for generated children */
    > zey-print {
        @scroll reveal(&quick-reveal) {
            opacity: 0 -> 1;
            translate-y: 40px -> 0;
            stagger: 0.08 first;
        }

        @on &.hover lift(300ms) {
            translate-y: 0 -> -8px;
            box-shadow: 0 4px 20px rgba(0,0,0,0.1) -> 0 12px 40px rgba(0,0,0,0.15);
        }
    }
}

/* ============================================
   CART BINDING
   ============================================ */

.cart-items {
    @each(cart) {
        template: "cart-item";

        @let print = getPrint($.printId);

        [slot="image"] { src: print.image; }
        [slot="title"]: print.title;
        [slot="size"]: $.size;
        [slot="quantity"]: $.quantity;
        [slot="price"]: priceFor($.printId, $.size) * $.quantity | currency("$");
    }
}

.cart-total {
    @bind {
        text: cartTotal | currency("$");
    }
}

/* ============================================
   CURATIONS BINDING
   ============================================ */

.curations-grid {
    @each(curations) {
        template: "curation-card";

        [slot="image"] { src: $.hero; }
        [slot="title"]: $.title;
        [slot="description"]: $.description;

        :host {
            data-slug: $.slug;
            style: "--accent: " + $.accent;
        }
    }
}
```

### Corresponding HTML

```html
<!-- index.html -->
<section class="gallery-section">
    <h2>All Prints</h2>
    <div class="zey-gallery">
        <!-- Populated by Spacetime from prints.json -->
    </div>
</section>

<section class="curations-section">
    <h2>Curated Collections</h2>
    <div class="curations-grid">
        <!-- Populated by Spacetime from curations.json -->
    </div>
</section>

<!-- Cart drawer -->
<aside class="cart-drawer">
    <h3>Your Cart</h3>
    <div class="cart-items">
        <!-- Populated by Spacetime from localStorage -->
    </div>
    <div class="cart-footer">
        <span>Total:</span>
        <span class="cart-total">$0.00</span>
    </div>
</aside>
```

### JSON Data Files

```json
// /data/prints.json
[
    {
        "id": "IN-003",
        "title": "Between Worlds",
        "subtitle": "Surfer in morning mist, Morocco",
        "image": "/prints/IN-003.jpg",
        "prices": { "S": 95, "M": 195, "L": 395 },
        "featured": true,
        "tags": ["morocco", "surf", "mist"]
    },
    {
        "id": "IN-004",
        "title": "Golden Hour",
        "subtitle": "Desert dunes at sunset",
        "image": "/prints/IN-004.jpg",
        "prices": { "S": 95, "M": 195, "L": 395 },
        "tags": ["desert", "sunset"]
    }
]
```

```json
// /data/curations.json
[
    {
        "id": "moroccan-soul",
        "slug": "moroccan-soul",
        "title": "Moroccan Soul",
        "description": "Vibrant colors and textures from the heart of Morocco",
        "hero": "/curations/moroccan-hero.jpg",
        "accent": "#d4a574",
        "prints": ["IN-003", "IN-007", "IN-012"]
    }
]
```

---

## Next Steps

- See [DATA_SYSTEM_EXAMPLES.md](./DATA_SYSTEM_EXAMPLES.md) for complete examples for each site
- See [COMPILE_TIME_ANALYSIS.md](./COMPILE_TIME_ANALYSIS.md) for error reporting and validation
- See [ARCHITECTURE.md](./ARCHITECTURE.md) for how this fits in the Spacetime compilation pipeline
