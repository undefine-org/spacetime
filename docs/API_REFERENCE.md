# Spacetime Data Binding API Reference

**Complete reference for all data binding directives, types, filters, and error codes.**

---

## Table of Contents

1. [@type - Type Definitions](#type---type-definitions)
2. [@data - Data Sources](#data---data-sources)
3. [@computed - Computed Data](#computed---computed-data)
4. [@fn - Helper Functions](#fn---helper-functions)
5. [@each - Template Iteration](#each---template-iteration)
6. [@let - Local Variables](#let---local-variables)
7. [@bind - Single Element Binding](#bind---single-element-binding)
8. [Filters](#filters)
9. [Special Variables](#special-variables)
10. [Data Events](#data-events)
11. [Error Codes](#error-codes)
12. [JavaScript Runtime API](#javascript-runtime-api)

---

## @type - Type Definitions

Define the structure of your data with type safety.

### Syntax

```css
@type TypeName {
    field: type;
    optionalField?: type;
}
```

### Primitive Types

| Type | Description | Example Values |
|------|-------------|----------------|
| `string` | Text data | `"hello"`, `""`, `"IN-003"` |
| `number` | Numeric values | `42`, `3.14`, `-100`, `0` |
| `boolean` | True/false | `true`, `false` |
| `url` | URL strings | `"/images/photo.jpg"`, `"https://..."` |
| `color` | Color values | `"#ff0000"`, `"rgb(255,0,0)"`, `"blue"` |

### Array Types

Append `[]` to any type:

```css
@type Gallery {
    images: string[];           /* Array of strings */
    tags: string[];
    featured: Print[];          /* Array of Print objects */
    scores: number[];
}
```

### Object Types

Define inline nested structures:

```css
@type Product {
    name: string;
    metadata: {
        created: string;
        author: {
            name: string;
            email: string;
        };
    };
}
```

### Optional Fields

Use `?` after field name:

```css
@type Product {
    id: string;             /* Required */
    name: string;           /* Required */
    description?: string;   /* Optional */
    tags?: string[];        /* Optional array */
}
```

### Union Types (Enums)

Restrict to specific string values:

```css
@type Order {
    status: "pending" | "processing" | "shipped" | "delivered";
}

@type Product {
    size: "S" | "M" | "L" | "XL";
}
```

### Type References

Reference other types:

```css
@type Author {
    name: string;
}

@type Post {
    title: string;
    author: Author;         /* Type reference */
}
```

### Examples

```css
/* Simple type */
@type Product {
    id: string;
    name: string;
    price: number;
}

/* Complex nested type */
@type CartItem {
    productId: string;
    quantity: number;
    size: "S" | "M" | "L";
    customization?: {
        text: string;
        color: color;
    };
}

/* Type with references */
@type Order {
    id: string;
    items: CartItem[];
    customer: Customer;
    status: "pending" | "shipped" | "delivered";
}
```

---

## @data - Data Sources

Define where and how to load data.

### Syntax

```css
@data sourceName: Type {
    src: source;
    /* optional parameters */
}
```

### Source Types

#### JSON File

```css
@data products: Product[] {
    src: "/data/products.json";
}
```

#### localStorage

```css
@data cart: CartItem[] {
    src: localStorage("cart-key");
    default: [];                        /* Fallback if empty */
}

@data settings: Settings {
    src: localStorage("user-settings");
    default: { theme: "light" };
}
```

#### Inline Data

```css
@data sizes: Size[] {
    src: inline;
    value: [
        { "code": "S", "label": "Small" },
        { "code": "M", "label": "Medium" }
    ];
}
```

### Parameters

#### `src` (required)

The data source.

**Type:** `string | localStorage(key) | inline`

```css
src: "/data/file.json";             /* URL */
src: localStorage("key");            /* localStorage */
src: inline;                         /* Inline (requires value) */
```

#### `default` (optional)

Default value if source is empty or doesn't exist.

**Type:** Same as data type

**Only for:** `localStorage` sources

```css
@data cart: CartItem[] {
    src: localStorage("cart");
    default: [];
}
```

#### `cache` (optional)

How long to cache the data.

**Type:** `duration | "none" | "forever"`

**Default:** `"5m"`

```css
cache: 5s;                  /* 5 seconds */
cache: 30m;                 /* 30 minutes */
cache: 2h;                  /* 2 hours */
cache: 7d;                  /* 7 days */
cache: none;                /* No caching */
cache: forever;             /* Cache indefinitely */
```

#### `refresh` (optional)

Auto-refresh strategy.

**Type:** `duration | "on-focus"`

```css
refresh: 30s;               /* Poll every 30 seconds */
refresh: 5m;                /* Poll every 5 minutes */
refresh: on-focus;          /* Refresh when tab gains focus */
```

### Examples

```css
/* Simple JSON source */
@data products: Product[] {
    src: "/data/products.json";
}

/* With caching */
@data products: Product[] {
    src: "/data/products.json";
    cache: 1h;
}

/* Live data, no cache */
@data liveStats: Stats {
    src: "/api/stats";
    cache: none;
    refresh: 10s;
}

/* localStorage with default */
@data cart: CartItem[] {
    src: localStorage("shopping-cart");
    default: [];
}

/* Inline static data */
@data navLinks: NavLink[] {
    src: inline;
    value: [
        { "href": "/", "label": "Home" },
        { "href": "/about", "label": "About" }
    ];
}
```

---

## @computed - Computed Data

Derive new data from existing sources.

### Syntax

```css
@computed name: Type {
    from: sourceData;
    /* operations */
}
```

### Operations

#### `where` - Filter

Filter items matching a condition.

```css
@computed featuredProducts: Product[] {
    from: products;
    where: $.featured == true;
}

@computed affordableProducts: Product[] {
    from: products;
    where: $.price < 100;
}

@computed moroccanPrints: Print[] {
    from: prints;
    where: $.tags.includes("morocco");
}
```

#### `sort` - Sort

Sort items by a field.

```css
@computed sortedByPrice: Product[] {
    from: products;
    sort: $.price asc;              /* Ascending */
}

@computed sortedByDate: Product[] {
    from: products;
    sort: $.createdAt desc;         /* Descending */
}
```

#### `limit` - Limit

Limit number of results.

```css
@computed topProducts: Product[] {
    from: products;
    limit: 10;
}

@computed featured: Product[] {
    from: products;
    where: $.featured == true;
    limit: 6;
}
```

#### `reduce` - Aggregate

Reduce to a single value.

```css
@computed cartTotal: number {
    from: cart;
    reduce: (sum, item) => sum + item.price * item.quantity;
    initial: 0;
}

@computed itemCount: number {
    from: cart;
    reduce: (count, item) => count + item.quantity;
    initial: 0;
}
```

### Examples

```css
/* Filter + sort + limit */
@computed topRatedProducts: Product[] {
    from: products;
    where: $.rating >= 4;
    sort: $.rating desc;
    limit: 5;
}

/* Aggregate */
@computed totalRevenue: number {
    from: orders;
    reduce: (sum, order) => sum + order.total;
    initial: 0;
}

/* Complex filter */
@computed availableInStock: Product[] {
    from: products;
    where: $.inStock == true && $.quantity > 0;
}
```

---

## @fn - Helper Functions

Define reusable functions for data operations.

### Syntax

```css
@fn functionName(param: Type, ...): ReturnType {
    /* function body */
}
```

### Parameters

Functions can have typed parameters:

```css
@fn getPrint(id: string): Print? {
    return prints.find(p => p.id == id);
}

@fn priceFor(printId: string, size: "S" | "M" | "L"): number {
    let print = getPrint(printId);
    return print?.prices[size] ?? 0;
}
```

### Return Types

Specify the return type after parameters:

- `Type` - Returns that type
- `Type?` - May return that type or null/undefined

```css
@fn findProduct(id: string): Product? {
    return products.find(p => p.id == id);
}

@fn calculateDiscount(price: number, percent: number): number {
    return price * (percent / 100);
}

@fn isInCart(productId: string): boolean {
    return cart.some(item => item.productId == productId);
}
```

### Usage in Bindings

```css
.cart-items {
    @each(cart) {
        @let product = findProduct($.productId);

        [slot="name"]: product.name;
        [slot="price"]: priceFor($.productId, $.size) | currency("$");
    }
}
```

### Examples

```css
/* Lookup function */
@fn getCategory(id: string): Category? {
    return categories.find(c => c.id == id);
}

/* Calculation function */
@fn formatPrice(amount: number, currency: string): string {
    return currency + amount.toFixed(2);
}

/* Validation function */
@fn hasTag(product: Product, tag: string): boolean {
    return product.tags?.includes(tag) ?? false;
}

/* Complex logic */
@fn calculateShipping(total: number, country: string): number {
    if (total >= 100) return 0;
    if (country == "US") return 5.99;
    return 12.99;
}
```

---

## @each - Template Iteration

Generate elements from an array of data.

### Syntax

```css
.container {
    @each(dataSource) {
        template: "template-id";

        /* bindings */
    }
}
```

### Binding Types

#### Slot Text Content

Bind text to a slot:

```css
[slot="title"]: $.title;
[slot="description"]: $.description;
```

#### Slot Attributes

Bind attributes to a slot:

```css
[slot="image"] {
    src: $.imageUrl;
    alt: $.title;
    loading: "lazy";
}
```

#### Host Bindings

Bind to the generated element itself:

```css
:host {
    data-id: $.id;
    data-category: $.category;
    class: $.featured ? "featured" : "";
}
```

### Property Paths

Access nested properties with dot notation:

```css
$.title                     /* Direct property */
$.author.name               /* Nested property */
$.metadata.created          /* Deeply nested */
$.prices.S                  /* Object key */
```

### Filters in Bindings

Apply filters with `|`:

```css
[slot="price"]: $.price | currency("$");
[slot="date"]: $.createdAt | date("short");
[slot="title"]: $.title | truncate(50) | uppercase;
```

### Conditional Bindings

Use ternary operators:

```css
:host {
    class: $.featured ? "featured" : "";
    data-stock: $.inStock ? "available" : "out";
}

[slot="badge"]: $.discount > 0 ? "On Sale" : "";
```

### Nested @each

For nested arrays:

```css
.categories {
    @each(categories) {
        template: "category";

        [slot="name"]: $.name;

        [slot="products"] {
            @each($.products) {
                template: "product-card";

                [slot="name"]: $.name;
            }
        }
    }
}
```

### Examples

```css
/* Basic binding */
.products {
    @each(products) {
        template: "product-card";

        [slot="image"] {
            src: $.imageUrl;
            alt: $.name;
        }
        [slot="name"]: $.name;
        [slot="price"]: $.price | currency("$");

        :host {
            data-id: $.id;
        }
    }
}

/* With filters and conditionals */
.posts {
    @each(posts) {
        template: "blog-post";

        [slot="title"]: $.title | truncate(60);
        [slot="date"]: $.publishedAt | date("medium");
        [slot="excerpt"]: $.content | truncate(200);

        :host {
            class: $.featured ? "featured" : "";
            data-category: $.category;
        }
    }
}

/* Cross-reference */
.cart {
    @each(cart) {
        template: "cart-item";

        @let product = products.find(p => p.id == $.productId);

        [slot="name"]: product.name;
        [slot="image"] { src: product.imageUrl; }
        [slot="quantity"]: $.quantity;
        [slot="total"]: product.price * $.quantity | currency("$");
    }
}
```

---

## @let - Local Variables

Define local variables within `@each` blocks.

### Syntax

```css
@each(data) {
    @let varName = expression;

    /* use varName in bindings */
}
```

### Use Cases

#### Cross-Reference Lookup

```css
@each(cart) {
    @let product = products.find(p => p.id == $.productId);

    [slot="name"]: product.name;
    [slot="image"] { src: product.imageUrl; }
}
```

#### Calculations

```css
@each(items) {
    @let total = $.price * $.quantity;
    @let discount = total * 0.1;
    @let finalPrice = total - discount;

    [slot="total"]: finalPrice | currency("$");
}
```

#### Conditional Values

```css
@each(products) {
    @let badgeText = $.discount > 0 ? "Sale" : ($.featured ? "Featured" : "");

    [slot="badge"]: badgeText;
}
```

### Multiple @let

You can define multiple variables:

```css
@each(orders) {
    @let customer = customers.find(c => c.id == $.customerId);
    @let itemCount = $.items.length;
    @let total = $.items.reduce((sum, item) => sum + item.price, 0);

    [slot="customer"]: customer.name;
    [slot="items"]: itemCount + " items";
    [slot="total"]: total | currency("$");
}
```

---

## @bind - Single Element Binding

Bind data to a single element (not a list).

### Syntax

```css
.element {
    @bind {
        text: expression;
        attr: value;
    }
}
```

### Text Binding

```css
.cart-total {
    @bind {
        text: cartTotal | currency("$");
    }
}

.item-count {
    @bind {
        text: cart.length + " items";
    }
}
```

### Attribute Binding

```css
.cart-icon {
    @bind {
        data-count: cart.length;
        class: cart.length > 0 ? "has-items" : "empty";
    }
}
```

### Examples

```css
/* Display computed total */
.order-total {
    @bind {
        text: orderTotal | currency("$");
    }
}

/* Conditional class */
.cart-badge {
    @bind {
        text: cart.length;
        class: cart.length > 0 ? "visible" : "hidden";
    }
}

/* Multiple attributes */
.status-indicator {
    @bind {
        text: order.status | capitalize;
        class: "status-" + order.status;
        data-order-id: order.id;
    }
}
```

---

## Filters

Transform data inline with the pipe `|` operator.

### Syntax

```css
value | filterName
value | filterName(arg1, arg2)
value | filter1 | filter2 | filter3
```

### Built-in Filters

#### `json`

Serialize to JSON string.

**Signature:** `json(value: any): string`

```css
$.metadata | json           /* '{"key":"value"}' */
$.prices | json             /* '{"S":95,"M":195}' */
```

#### `currency(symbol)`

Format number as currency.

**Signature:** `currency(value: number, symbol: string): string`

```css
$.price | currency("$")     /* "$95.00" */
$.price | currency("€")     /* "€95.00" */
$.price | currency("dhs")   /* "95 dhs" */
$.price | currency("")      /* "95.00" */
```

#### `date(format)`

Format date string.

**Signature:** `date(value: string, format: string): string`

**Formats:**
- `"short"` - "Dec 16"
- `"medium"` - "Dec 16, 2024"
- `"long"` - "December 16, 2024"
- `"iso"` - "2024-12-16"
- `"relative"` - "2 days ago"

```css
$.createdAt | date("short")     /* "Dec 16" */
$.createdAt | date("medium")    /* "Dec 16, 2024" */
$.createdAt | date("relative")  /* "2 days ago" */
```

#### `default(fallback)`

Provide fallback for null/undefined.

**Signature:** `default(value: T?, fallback: T): T`

```css
$.description | default("No description")
$.featured | default(false)
$.tags | default([])
```

#### `count`

Get array length.

**Signature:** `count(value: any[]): number`

```css
$.items | count             /* 5 */
$.tags | count              /* 3 */
```

#### `truncate(length)`

Truncate string to length.

**Signature:** `truncate(value: string, length: number): string`

```css
$.title | truncate(50)      /* "This is a long title that gets tr..." */
$.description | truncate(100)
```

#### `if(trueVal, falseVal)`

Conditional value.

**Signature:** `if(condition: boolean, trueVal: any, falseVal: any): any`

```css
$.inStock | if("Available", "Out of Stock")
$.featured | if("⭐", "")
$.discount > 0 | if("On Sale", "")
```

#### `uppercase`

Convert to uppercase.

**Signature:** `uppercase(value: string): string`

```css
$.code | uppercase          /* "ABC123" */
$.status | uppercase        /* "PENDING" */
```

#### `lowercase`

Convert to lowercase.

**Signature:** `lowercase(value: string): string`

```css
$.email | lowercase         /* "user@example.com" */
```

#### `capitalize`

Capitalize first letter.

**Signature:** `capitalize(value: string): string`

```css
$.name | capitalize         /* "John doe" -> "John doe" */
$.status | capitalize       /* "pending" -> "Pending" */
```

#### `pluralize(singular, plural)`

Smart pluralization.

**Signature:** `pluralize(count: number, singular: string, plural: string): string`

```css
$.quantity | pluralize("item", "items")
/* 1 -> "item", 2 -> "items" */

$.count | pluralize("person", "people")
```

### Filter Chaining

Apply multiple filters:

```css
$.title | truncate(50) | uppercase
$.price | default(0) | currency("$")
$.description | truncate(100) | capitalize
```

Filters are applied left to right.

---

## Special Variables

Available within `@each` blocks.

### `$` - Current Item

The current item in the iteration.

```css
@each(products) {
    [slot="name"]: $.name;
    [slot="price"]: $.price;
}
```

### `$._index` - Index

Zero-based position in the array.

**Type:** `number`

```css
@each(items) {
    [slot="number"]: $._index + 1;      /* 1, 2, 3... */

    :host {
        data-index: $._index;            /* 0, 1, 2... */
    }
}
```

### `$._first` - Is First

Boolean indicating if this is the first item.

**Type:** `boolean`

```css
@each(items) {
    :host {
        class: $._first ? "first" : "";
    }
}
```

### `$._last` - Is Last

Boolean indicating if this is the last item.

**Type:** `boolean`

```css
@each(items) {
    :host {
        class: $._last ? "last" : "";
    }
}
```

### `$._count` - Total Count

Total number of items in the array.

**Type:** `number`

```css
@each(items) {
    [slot="position"]: ($._index + 1) + " of " + $._count;
    /* "1 of 10", "2 of 10", ... */
}
```

### Examples

```css
/* Using all special variables */
@each(products) {
    template: "product-card";

    [slot="position"]: "Item " + ($._index + 1) + " of " + $._count;

    :host {
        data-index: $._index;
        class: $._first ? "first" : ($._last ? "last" : "");
    }
}

/* Conditional styling based on position */
@each(testimonials) {
    :host {
        class: $._index % 2 == 0 ? "even" : "odd";
    }
}
```

---

## Data Events

Events emitted during data loading lifecycle.

### Event Types

#### `data:{source}:loaded`

Fired when data loads successfully with items.

**Event Detail:** `{ source: string, data: any[], count: number }`

```javascript
document.addEventListener('data:products:loaded', (event) => {
    console.log('Loaded', event.detail.count, 'products');
});
```

#### `data:{source}:empty`

Fired when data loads but array is empty.

**Event Detail:** `{ source: string }`

```javascript
document.addEventListener('data:products:empty', (event) => {
    console.log('No products found');
});
```

#### `data:{source}:error`

Fired when data fails to load.

**Event Detail:** `{ source: string, error: Error }`

```javascript
document.addEventListener('data:products:error', (event) => {
    console.error('Load error:', event.detail.error);
});
```

### Signal-Based State

Data sources expose status signals automatically. Use them to drive `@state(when:)` or reactive classes instead of an imperative state machine.

```spacetime
@data fetch $prints Print[] : "/data/prints.json";

.gallery {
    data-state: $prints_loading ? "loading" : ($prints_error ? "error" : ($prints.length == 0 ? "empty" : "ready"));

    @state(when: "loading") { opacity: 0.5; }
    @state(when: "ready")   { opacity: 1; }
    @state(when: "empty")   { /* no items UI */ }
    @state(when: "error")   { background: #fee; }
}
```

### Reactive Classes

Declare a CSS class that is toggled by a signal expression:

```spacetime
@data inline $open boolean : false;

.modal {
    .is-open: $open;
    @on &.click { $open <- !$open; }
}

.modal.is-open {
    opacity: 1;
    pointer-events: auto;
}
```

### `@state(when:)` — State CSS

Apply CSS properties when an element's `data-state` attribute matches. The attribute value can come from any signal expression.

```spacetime
.item {
    data-state: $status;

    @state(when: "loading") { opacity: 0.5; }
    @state(when: "ready")   { opacity: 1; }
}
```

### `@view` — Multi-State DOM Dispatch

For mutually-exclusive states, swap DOM based on a string/number signal:

```spacetime
@data inline $step string : "shipping";

.checkout {
    @view $step {
        "shipping"  => &shipping-form();
        "payment"   => &payment-form();
        "confirm"   => &confirmation();
    }
}
```

### `@socket` — Typed-Union Pattern Match

`@socket` produces a typed-union signal. Use `@state(when:)` with `is Variant` to pattern-match:

```spacetime
component socket-demo {
    @use socket(&self, url: "ws://localhost:8080") as $ws

    @state(when: $ws is Disconnected) {
        .status { background: gray; }
    }

    @state(when: $ws is Connected { $send, $received }) {
        .status { background: green; }
    }
}
```

---

## Error Codes

### Compile-Time Errors

#### `E001` - Type Not Found

**Message:** `Type 'TypeName' not found`

**Cause:** Referenced a type that doesn't exist.

**Fix:** Define the type or check spelling.

```css
/* Error */
@data products: Product[] { }       /* Product not defined */

/* Fix */
@type Product {
    name: string;
}
@data products: Product[] { }
```

#### `E002` - Field Missing

**Message:** `Field 'fieldName' not found in type 'TypeName'`

**Cause:** JSON data has a field not in the type definition.

**Fix:** Add field to type or remove from JSON.

```css
/* Error */
@type Product { name: string; }
/* JSON has: { "name": "x", "price": 10 } */

/* Fix */
@type Product {
    name: string;
    price: number;
}
```

#### `E003` - Type Mismatch

**Message:** `Expected type 'TypeA', got 'TypeB'`

**Cause:** JSON value doesn't match type definition.

**Fix:** Correct the JSON or update the type.

```css
/* Error */
@type Product { price: number; }
/* JSON has: { "price": "10" } */  /* String, not number */

/* Fix - update JSON */
{ "price": 10 }
```

#### `E004` - Template Not Found

**Message:** `Template 'template-id' not found`

**Cause:** Referenced template doesn't exist in HTML.

**Fix:** Add template or fix ID.

```css
/* Error */
@each(items) {
    template: "item-card";          /* No template with this ID */
}

/* Fix - add to HTML */
```html
<template id="item-card">
  <!-- ... -->
</template>
```

#### `E005` - Data Source Not Found

**Message:** `Data source 'sourceName' not found`

**Cause:** Referenced data that wasn't defined.

**Fix:** Define the data source.

```css
/* Error */
@each(products) { }                 /* products not defined */

/* Fix */
@data products: Product[] {
    src: "/data/products.json";
}
```

#### `E006` - Invalid Filter

**Message:** `Filter 'filterName' not found`

**Cause:** Used a filter that doesn't exist.

**Fix:** Check filter name spelling or use a different filter.

```css
/* Error */
[slot="price"]: $.price | money("$");   /* No 'money' filter */

/* Fix */
[slot="price"]: $.price | currency("$");
```

#### `E007` - Invalid Filter Arguments

**Message:** `Filter 'filterName' expects N arguments, got M`

**Cause:** Wrong number of arguments to filter.

**Fix:** Check filter signature.

```css
/* Error */
[slot="price"]: $.price | currency();   /* Missing argument */

/* Fix */
[slot="price"]: $.price | currency("$");
```

#### `E008` - Circular Type Reference

**Message:** `Circular reference detected in type 'TypeName'`

**Cause:** Type references itself directly or indirectly.

**Fix:** Break the circular reference.

```css
/* Error */
@type Node {
    value: string;
    child: Node;            /* Circular */
}

/* Fix */
@type Node {
    value: string;
    child?: Node;           /* Optional breaks infinite loop */
}
```

### Runtime Errors

#### `R001` - Fetch Failed

**Message:** `Failed to fetch data from 'url'`

**Cause:** Network error, 404, CORS, etc.

**Fix:** Check URL, server, and CORS settings.

#### `R002` - Invalid JSON

**Message:** `Invalid JSON in 'url'`

**Cause:** JSON is malformed.

**Fix:** Validate JSON with a linter.

#### `R003` - localStorage Error

**Message:** `Failed to read from localStorage key 'key'`

**Cause:** localStorage disabled or quota exceeded.

**Fix:** Check browser settings or reduce data size.

#### `R004` - Template Clone Failed

**Message:** `Failed to clone template 'template-id'`

**Cause:** Template element not found or malformed.

**Fix:** Ensure template exists in HTML.

#### `R005` - Binding Failed

**Message:** `Failed to bind slot 'slotName'`

**Cause:** Slot element not found in template.

**Fix:** Add slot to template or check selector.

---

## Type Compatibility

### Assignability

| From → To | `string` | `number` | `boolean` | `url` | `color` | `any[]` | `object` |
|-----------|----------|----------|-----------|-------|---------|---------|----------|
| `string` | ✓ | ✗ | ✗ | ✓ | ✓ | ✗ | ✗ |
| `number` | ✗ | ✓ | ✗ | ✗ | ✗ | ✗ | ✗ |
| `boolean` | ✗ | ✗ | ✓ | ✗ | ✗ | ✗ | ✗ |
| `url` | ✓ | ✗ | ✗ | ✓ | ✗ | ✗ | ✗ |
| `color` | ✓ | ✗ | ✗ | ✗ | ✓ | ✗ | ✗ |

`url` and `color` are validated `string` subtypes.

### Optional vs Required

```css
/* Required field */
@type Product {
    name: string;           /* Must exist in JSON */
}

/* Optional field */
@type Product {
    description?: string;   /* May be missing */
}
```

When accessing optional fields, use safe operators:

```css
[slot="desc"]: $.description | default("No description")
[slot="author"]: $.author?.name | default("Unknown")
```

---

## Performance Notes

### Cache Strategy

For best performance:

- Use `cache: forever` for static data
- Use `cache: 1h` for data that changes occasionally
- Use `cache: none` only for live/real-time data

### Template Instantiation

- Templates are cloned, not re-parsed
- Slot lookups are cached per template
- Large lists (1000+ items) may benefit from pagination

### localStorage

- localStorage reads are synchronous
- Keep stored data under 5MB
- Use JSON compression for large datasets

---

## JavaScript Runtime API

Spacetime generates JavaScript code that exposes global objects for data access.

### SpacetimeData

Holds all loaded data sources.

```javascript
// Access loaded data
const products = SpacetimeData.products;     // Array of products
const cart = SpacetimeData.cart;             // Array of cart items

// Check if data is loaded
if (SpacetimeData.products) {
    console.log('Products loaded:', SpacetimeData.products.length);
}
```

### SpacetimeComputed

Holds computed (derived) data.

```javascript
// Access computed data after it's ready
const featured = SpacetimeComputed.featuredProducts;
const total = SpacetimeComputed.cartTotal;

// Wait for computed data to be ready
document.addEventListener('computed:featuredProducts:ready', () => {
    console.log('Featured products:', SpacetimeComputed.featuredProducts);
});
```

### SpacetimeFunctions

Contains all user-defined `@fn` functions.

```javascript
// Call functions directly
const formatted = SpacetimeFunctions.formatPrice(19.99);  // "$19.99"
const product = SpacetimeFunctions.getProduct('prod-1');

// Functions are available immediately after script load
console.log(SpacetimeFunctions.add(2, 3));  // 5
```

### SpacetimeRuntime

Utility functions for template handling.

```javascript
// Find a template by ID
const template = SpacetimeRuntime.findTemplate('product-card');

// Wait for a template to be available (async)
const template = await SpacetimeRuntime.waitForTemplate('product-card');
```

### Generated Function Patterns

#### Data Loading

```javascript
// Generated: load_{dataName}()
async function load_products() {
    const response = await fetch('/data/products.json');
    SpacetimeData.products = await response.json();
    document.dispatchEvent(new CustomEvent('data:products:loaded', {
        detail: { count: SpacetimeData.products.length }
    }));
}
```

#### Computed Calculation

```javascript
// Generated: compute_{computedName}()
function compute_featuredProducts() {
    let result = SpacetimeData.products;
    result = result.filter($ => $.featured == true);
    result = result.slice().sort((a, b) => b.price - a.price);
    result = result.slice(0, 5);
    SpacetimeComputed.featuredProducts = result;
    document.dispatchEvent(new CustomEvent('computed:featuredProducts:ready'));
}
```

#### Rendering

```javascript
// Generated: render_{selector}_{dataSource}()
function render_productList_products() {
    const container = document.querySelector('.product-list');
    const template = SpacetimeRuntime.findTemplate('product-card');
    const data_0 = SpacetimeData.products;

    data_0.forEach((item_0, index_0) => {
        const instance = template.content.cloneNode(true);
        // ... slot bindings
        container.appendChild(instance.firstElementChild);
    });
}
```

### Computed Events

Events dispatched by computed data:

| Event | When | Detail |
|-------|------|--------|
| `computed:{name}:ready` | Computed data calculated | `{}` |

Example usage:

```javascript
document.addEventListener('computed:featuredProducts:ready', () => {
    console.log('Featured products ready:', SpacetimeComputed.featuredProducts);
});
```

### Reactive State with Signals

Use data and computed events to update the same `$status` signal:

```spacetime
$status <- "loading";

@on data:products:loaded { $status <- "loaded"; }
@on computed:featuredProducts:ready { $status <- "ready"; }

.product-grid {
    .is-loading: $status == "loading";
    .is-loaded: $status == "loaded";
    .is-ready: $status == "ready";
}
```

---

## See Also

- [Data Binding Guide](./DATA_BINDING_GUIDE.md) - Step-by-step tutorial
- [Examples](./DATA_SYSTEM_EXAMPLES.md) - Real-world examples
- [Migration Guide](./MIGRATION.md) - Migrating existing sites
