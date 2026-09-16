# Spacetime Data Binding Guide

**Learn how to build dynamic, data-driven websites with Spacetime's declarative data binding system.**

---

## Table of Contents

1. [Introduction](#introduction)
2. [Getting Started](#getting-started)
3. [Defining Types](#defining-types)
4. [Creating Data Files](#creating-data-files)
5. [Using @each for Lists](#using-each-for-lists)
6. [Computed Data with @computed](#computed-data-with-computed)
7. [Helper Functions with @fn](#helper-functions-with-fn)
8. [Filters and Transformations](#filters-and-transformations)
9. [Reactive State for Loading](#reactive-state-for-loading)
10. [Common Patterns](#common-patterns)
11. [Troubleshooting](#troubleshooting)

---

## Introduction

### Why Data Binding?

Imagine you're building a photography portfolio with 50 prints. Without data binding, you'd write HTML like this 50 times:

```html
<div class="print-card" data-id="IN-003">
  <img src="/prints/IN-003.jpg" alt="Between Worlds">
  <h3>Between Worlds</h3>
  <p>Surfer in morning mist, Morocco</p>
  <span class="price">$95</span>
</div>
```

This is tedious, error-prone, and hard to maintain. What if you want to change the price format? You'd need to edit 50 places.

**Data binding solves this** by separating your data from your presentation:

1. **Data** lives in JSON files
2. **Structure** lives in HTML templates
3. **Behavior** lives in .st files

### What You'll Learn

By the end of this guide, you'll know how to:

- Define type-safe data schemas
- Load data from JSON files or localStorage
- Generate HTML from templates automatically
- Apply filters to transform data
- Handle loading states with state machines
- Build real-world features like galleries and shopping carts

---

## Getting Started

### Your First Data-Bound Component

Let's build a simple product list in 4 steps.

#### Step 1: Create a Template

In your HTML file, define a template:

```html
<!-- index.html -->
<template id="product-card">
  <div class="product">
    <img slot="image" src="" alt="">
    <h3 slot="name"></h3>
    <p slot="price"></p>
  </div>
</template>

<div class="product-list">
  <!-- Products will be inserted here -->
</div>
```

The `slot` attributes mark where data will be inserted.

#### Step 2: Define the Data Type

In your .st file, describe what a product looks like:

```css
/* styles.st */

@type Product {
    id: string;
    name: string;
    price: number;
    imageUrl: url;
}
```

This tells Spacetime what fields to expect and what types they should be.

#### Step 3: Point to Your Data

Tell Spacetime where to load the data from:

```css
@data products: Product[] {
    src: "/data/products.json";
}
```

The `Product[]` means "an array of Product objects."

#### Step 4: Bind the Template

Connect your template to the data:

```css
.product-list {
    @each(products) {
        template: "product-card";

        [slot="image"] {
            src: $.imageUrl;
            alt: $.name;
        }
        [slot="name"]: $.name;
        [slot="price"]: $.price | currency("$");
    }
}
```

The `$` represents each product in the list. The `| currency("$")` is a filter that formats the number as money.

#### Step 5: Create the JSON Data

Create `/data/products.json`:

```json
[
  {
    "id": "prod-1",
    "name": "Ceramic Mug",
    "price": 24.99,
    "imageUrl": "/images/mug.jpg"
  },
  {
    "id": "prod-2",
    "name": "Wool Blanket",
    "price": 89.99,
    "imageUrl": "/images/blanket.jpg"
  }
]
```

That's it! When you compile and load your page, Spacetime will:

1. Load the products from JSON
2. Create a product card for each item
3. Insert the data into the slots
4. Add them to `.product-list`

---

## Defining Types

Types are your data's contract. They tell Spacetime what to expect and help catch errors early.

### Basic Type Definition

```css
@type TypeName {
    field: type;
    anotherField: type;
}
```

### Available Types

#### Primitives

```css
@type Product {
    name: string;           /* Text */
    price: number;          /* Numbers (int or float) */
    inStock: boolean;       /* true or false */
    imageUrl: url;          /* URL string */
    accentColor: color;     /* Color value */
}
```

#### Arrays

Add `[]` after any type:

```css
@type Gallery {
    images: string[];       /* Array of strings */
    tags: string[];
    prices: number[];
}
```

#### Objects

Define nested structures inline:

```css
@type Product {
    name: string;
    dimensions: {
        width: number;
        height: number;
        unit: string;
    };
    pricing: {
        regular: number;
        sale: number;
    };
}
```

#### Optional Fields

Use `?` for fields that might not exist:

```css
@type Product {
    name: string;           /* Required */
    description?: string;   /* Optional */
    featured?: boolean;     /* Optional, undefined if missing */
}
```

#### Enums (String Unions)

Restrict to specific values:

```css
@type Order {
    status: "pending" | "shipped" | "delivered";
    priority: "low" | "medium" | "high";
}

@type Product {
    size: "S" | "M" | "L" | "XL";
}
```

#### Type References

Types can reference other types:

```css
@type Author {
    name: string;
    email: string;
}

@type BlogPost {
    title: string;
    author: Author;         /* Reference to Author */
    tags: string[];
}
```

### Real-World Example

Here's a complete type definition for an e-commerce site:

```css
@type PriceBreakdown {
    S: number;
    M: number;
    L: number;
}

@type Product {
    id: string;
    name: string;
    description: string;
    images: url[];
    prices: PriceBreakdown;
    category: "prints" | "clothing" | "accessories";
    tags?: string[];
    featured?: boolean;
    inStock: boolean;
}
```

---

## Creating Data Files

### JSON Structure

Your JSON must match your type definition:

```css
/* Type */
@type Product {
    id: string;
    name: string;
    price: number;
}
```

```json
/* Data */
[
  {
    "id": "1",
    "name": "Product Name",
    "price": 29.99
  }
]
```

### File Organization

Organize data files in a `/data` folder:

```
your-site/
  data/
    products.json
    categories.json
    settings.json
  index.html
  styles.st
```

### Data Source Options

#### JSON File (Most Common)

```css
@data products: Product[] {
    src: "/data/products.json";
}
```

#### localStorage (For User Data)

```css
@data cart: CartItem[] {
    src: localStorage("shopping-cart");
    default: [];                /* If key doesn't exist */
}

@data settings: UserSettings {
    src: localStorage("user-settings");
    default: {
        theme: "light",
        language: "en"
    };
}
```

#### Inline Data (For Small, Static Lists)

```css
@data sizes: Size[] {
    src: inline;
    value: [
        { "code": "S", "label": "Small" },
        { "code": "M", "label": "Medium" },
        { "code": "L", "label": "Large" }
    ];
}
```

### Caching

Control how long data is cached:

```css
@data products: Product[] {
    src: "/data/products.json";
    cache: 1h;              /* Cache for 1 hour */
}

@data liveData: Stats {
    src: "/api/stats";
    cache: none;            /* Always fetch fresh */
}

@data staticContent: Page {
    src: "/data/about.json";
    cache: forever;         /* Cache indefinitely */
}
```

Cache duration formats:
- `5s` - 5 seconds
- `30m` - 30 minutes
- `2h` - 2 hours
- `7d` - 7 days
- `none` - No caching
- `forever` - Cache forever

---

## Using @each for Lists

The `@each` directive generates HTML for each item in an array.

### Basic Syntax

```css
.container {
    @each(dataSource) {
        template: "template-id";

        [slot="name"]: $.fieldName;
    }
}
```

### Slot Bindings

#### Text Content

```css
[slot="title"]: $.title;
[slot="description"]: $.description;
```

#### Attributes

```css
[slot="image"] {
    src: $.imageUrl;
    alt: $.title;
    loading: "lazy";
}
```

#### Host Element

The `:host` selector targets the generated element itself:

```css
@each(products) {
    template: "product-card";

    :host {
        data-id: $.id;
        data-category: $.category;
        class: $.featured ? "featured" : "";
    }
}
```

### Accessing Nested Data

Use dot notation:

```css
@each(products) {
    [slot="price"]: $.pricing.sale;
    [slot="width"]: $.dimensions.width;
    [slot="author-name"]: $.author.name;
}
```

### Special Variables

Inside `@each`, you have access to:

| Variable | Description | Type |
|----------|-------------|------|
| `$` | Current item | Object |
| `$._index` | Position (0-based) | Number |
| `$._first` | Is first item? | Boolean |
| `$._last` | Is last item? | Boolean |
| `$._count` | Total items | Number |

Example:

```css
@each(products) {
    template: "product-card";

    [slot="position"]: $._index + 1;

    :host {
        class: $._first ? "first" : ($._last ? "last" : "");
    }
}
```

### Cross-References with @let

Look up related data:

```css
@data products: Product[] {
    src: "/data/products.json";
}

@data cart: CartItem[] {
    src: localStorage("cart");
}

.cart-items {
    @each(cart) {
        template: "cart-item";

        /* Look up product by ID */
        @let product = products.find(p => p.id == $.productId);

        [slot="image"] { src: product.imageUrl; }
        [slot="name"]: product.name;
        [slot="quantity"]: $.quantity;
        [slot="total"]: product.price * $.quantity;
    }
}
```

### Nested @each

For nested data structures, you can nest `@each` blocks to render hierarchical data.

#### Basic Nesting

```css
@type Category {
    name: string;
    products: Product[];
}

@data categories: Category[] {
    src: "/data/categories.json";
}

.categories {
    @each(categories) {
        template: "category-section";

        [slot="title"]: $.name;

        /* Nested @each for products within each category */
        @each($.products) {
            template: "product-card";

            [slot="name"]: $.name;
            [slot="price"]: $.price;
        }
    }
}
```

#### How Nesting Works

In nested `@each` blocks:
- `$` always refers to the **current level's item**
- The outer item is accessed via the nested data source (`$.products` comes from the outer item)
- Each level generates its own template instances

#### Three-Level Nesting

For deeply nested data:

```css
@type Item {
    name: string;
    price: number;
}

@type SubCategory {
    name: string;
    items: Item[];
}

@type Department {
    name: string;
    subcategories: SubCategory[];
}

@data store: Department[] {
    src: "/data/store.json";
}

.store-catalog {
    @each(store) {
        template: "department-section";
        [slot="dept-name"]: $.name;

        @each($.subcategories) {
            template: "subcategory-section";
            [slot="subcat-name"]: $.name;

            @each($.items) {
                template: "item-card";
                [slot="item-name"]: $.name;
                [slot="item-price"]: $.price | currency("$");
            }
        }
    }
}
```

#### Nested @each with Attributes

You can bind attributes in nested loops:

```css
@type Image {
    url: url;
    caption: string;
}

@type Gallery {
    title: string;
    images: Image[];
}

@data galleries: Gallery[] {
    src: "/data/galleries.json";
}

.galleries {
    @each(galleries) {
        template: "gallery-section";
        [slot="title"]: $.title;

        @each($.images) {
            template: "image-card";

            [slot="img"] {
                src: $.url;
                alt: $.caption;
            }
            [slot="caption"]: $.caption;
        }
    }
}
```

#### Example JSON for Nested Data

```json
[
  {
    "name": "Electronics",
    "products": [
      { "name": "Laptop", "price": 999 },
      { "name": "Phone", "price": 699 }
    ]
  },
  {
    "name": "Clothing",
    "products": [
      { "name": "T-Shirt", "price": 29 },
      { "name": "Jeans", "price": 59 }
    ]
  }
]
```

---

## Computed Data with @computed

The `@computed` directive lets you derive new data from existing sources. It's perfect for filtering, sorting, limiting, and aggregating data.

### Why Use @computed?

Without `@computed`, you'd need JavaScript to filter or transform data. With `@computed`, you declare transformations in your .st file:

```css
/* Filter products to show only featured items */
@computed featuredProducts: Product[] {
    from: products;
    where: $.featured == true;
}

/* Use the computed data like any other data source */
.featured-section {
    @each(featuredProducts) {
        template: "product-card";
        [slot="name"]: $.name;
    }
}
```

### Filtering with `where`

The `where` option filters items matching a condition:

```css
@type Product {
    name: string;
    price: number;
    inStock: boolean;
    category: string;
}

@data products: Product[] {
    src: "/data/products.json";
}

/* Filter by boolean */
@computed availableProducts: Product[] {
    from: products;
    where: $.inStock == true;
}

/* Filter by comparison */
@computed affordableProducts: Product[] {
    from: products;
    where: $.price < 50;
}

/* Filter by string match */
@computed clothingProducts: Product[] {
    from: products;
    where: $.category == "clothing";
}
```

### Sorting with `sort`

The `sort` option orders items by a field:

```css
/* Sort ascending (lowest first) */
@computed cheapestFirst: Product[] {
    from: products;
    sort: $.price asc;
}

/* Sort descending (highest first) */
@computed newestFirst: Product[] {
    from: products;
    sort: $.createdAt desc;
}

/* Alphabetical sort */
@computed alphabetical: Product[] {
    from: products;
    sort: $.name asc;
}
```

### Limiting with `limit`

The `limit` option restricts the number of results:

```css
/* Show only top 5 */
@computed topFive: Product[] {
    from: products;
    limit: 5;
}

/* Combine with sort for "top N" patterns */
@computed topRated: Product[] {
    from: products;
    sort: $.rating desc;
    limit: 3;
}
```

### Combining Operations

You can chain multiple operations:

```css
/* Filter + Sort + Limit */
@computed featuredDeals: Product[] {
    from: products;
    where: $.featured == true && $.discount > 0;
    sort: $.discount desc;
    limit: 6;
}

/* In-stock items, sorted by price, limited */
@computed budgetPicks: Product[] {
    from: products;
    where: $.inStock == true && $.price < 30;
    sort: $.price asc;
    limit: 10;
}
```

### Aggregating with `reduce`

The `reduce` option computes a single value from all items:

```css
@type CartItem {
    productId: string;
    price: number;
    quantity: number;
}

@data cart: CartItem[] {
    src: localStorage("cart");
    default: [];
}

/* Sum all prices */
@computed cartTotal: number {
    from: cart;
    reduce: (sum, item) => sum + item.price * item.quantity;
    initial: 0;
}

/* Count total items */
@computed itemCount: number {
    from: cart;
    reduce: (count, item) => count + item.quantity;
    initial: 0;
}
```

Use aggregated values in bindings:

```css
.cart-summary {
    @bind {
        text: "Total: " + cartTotal | currency("$");
    }
}

.cart-badge {
    @bind {
        text: itemCount;
    }
}
```

### Event Lifecycle

Computed data sources are signals. Drive state from their loading / ready status just like any other signal:

```css
@data query $featuredProducts Product[] from $products {
    where: $.featured == true;
    limit: 6;
}

.product-grid {
    data-state: $featuredProducts_loading ? "loading" : "ready";

    /* Styling for loading state */
    @state(when: "loading") { opacity: 0.5; }
    @state(when: "ready")   { opacity: 1; }

    @each($featuredProducts as $product) {
        template: "product-card";
        /* ... */
    }
}
```

---

## Helper Functions with @fn

The `@fn` directive lets you define reusable functions for data operations and formatting.

### Why Use @fn?

Functions encapsulate logic you'd otherwise repeat:

```css
/* Define once */
@fn formatPrice(price: number): string {
    return '$' + price.toFixed(2);
}

/* Use everywhere */
.products {
    @each(products) {
        [slot="price"]: formatPrice($.price);
    }
}

.cart {
    @each(cart) {
        [slot="total"]: formatPrice($.price * $.quantity);
    }
}
```

### Basic Function Definition

```css
@fn functionName(param1: Type, param2: Type): ReturnType {
    /* JavaScript function body */
}
```

### Examples

#### Formatting Functions

```css
/* Currency formatting */
@fn formatPrice(amount: number): string {
    return '$' + amount.toFixed(2);
}

/* Date formatting */
@fn formatDate(dateStr: string): string {
    const date = new Date(dateStr);
    return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
}

/* Percentage */
@fn formatPercent(value: number): string {
    return Math.round(value * 100) + '%';
}
```

#### Calculation Functions

```css
/* Calculate discount */
@fn discountedPrice(price: number, discount: number): number {
    return price * (1 - discount / 100);
}

/* Tax calculation */
@fn withTax(price: number, rate: number): number {
    return price * (1 + rate);
}
```

#### Lookup Functions

```css
@data products: Product[] {
    src: "/data/products.json";
}

/* Find a product by ID */
@fn getProduct(id: string): Product {
    return SpacetimeData.products.find(p => p.id === id);
}

/* Get category name */
@fn getCategoryName(categoryId: string): string {
    const categories = { 'prints': 'Art Prints', 'clothing': 'Apparel' };
    return categories[categoryId] || 'Other';
}
```

### Using Functions in Bindings

Functions can be called anywhere expressions are allowed:

```css
.products {
    @each(products) {
        template: "product-card";

        /* In slot text */
        [slot="price"]: formatPrice($.price);

        /* With calculations */
        [slot="sale-price"]: formatPrice(discountedPrice($.price, $.discount));

        /* In conditionals */
        :host {
            class: $.discount > 0 ? "on-sale" : "";
        }
    }
}
```

### Functions with Computed Data

Functions work seamlessly with `@computed`:

```css
@fn isAffordable(price: number): boolean {
    return price < 50;
}

/* Note: Use where expression directly, not function call in where */
@computed affordableProducts: Product[] {
    from: products;
    where: $.price < 50;
}
```

---

## Filters and Transformations

Filters transform data inline using the `|` pipe syntax.

### Basic Usage

```css
[slot="price"]: $.price | currency("$");
[slot="title"]: $.title | uppercase;
```

### Built-in Filters

#### `currency(symbol)`

Format numbers as currency:

```css
$.price | currency("$")     /* $95.00 */
$.price | currency("€")     /* €95.00 */
$.price | currency("dhs")   /* 95 dhs */
```

#### `date(format)`

Format dates:

```css
$.createdAt | date("short")     /* "Dec 16" */
$.createdAt | date("medium")    /* "Dec 16, 2024" */
$.createdAt | date("long")      /* "December 16, 2024" */
$.createdAt | date("iso")       /* "2024-12-16" */
$.createdAt | date("relative")  /* "2 days ago" */
```

#### `default(fallback)`

Provide a fallback value:

```css
$.description | default("No description available")
$.featured | default(false)
```

#### `json`

Serialize to JSON:

```css
$.metadata | json           /* '{"key":"value"}' */
$.prices | json             /* '{"S":95,"M":195}' */
```

#### `truncate(length)`

Truncate strings:

```css
$.description | truncate(100)   /* "Lorem ipsum dolor..." */
$.title | truncate(50)
```

#### `if(trueVal, falseVal)`

Conditional output:

```css
$.inStock | if("In Stock", "Out of Stock")
$.featured | if("⭐ Featured", "")
```

#### `uppercase`, `lowercase`, `capitalize`

Text transformations:

```css
$.code | uppercase              /* "ABC123" */
$.name | lowercase              /* "product name" */
$.title | capitalize            /* "Product name" */
```

#### `count`

Get array length:

```css
$.images | count                /* 5 */
$.tags | count                  /* 3 */
```

#### `pluralize(singular, plural)`

Smart pluralization:

```css
$.quantity | pluralize("item", "items")
/* 1 → "item", 2 → "items" */
```

### Chaining Filters

Apply multiple filters in sequence:

```css
[slot="title"]: $.title | truncate(50) | uppercase;
[slot="price"]: $.price | default(0) | currency("$");
[slot="date"]: $.date | date("short") | uppercase;
```

---

## Reactive State for Loading

Use reactive `$`-signals to handle loading, empty, and error states. Fetch sources expose `<name>_loading` and `<name>_error` signals automatically, and the array itself tells you when it is empty.

### Basic Pattern

```css
@data fetch $products Product[] : "/data/products.json";

.product-list {
    data-state: $products_loading ? "loading" : ($products_error ? "error" : ($products.length == 0 ? "empty" : "ready"));

    @state(when: "loading") {
        min-height: 400px;
        opacity: 0.5;
    }

    @state(when: "ready") {
        opacity: 1;
    }

    @state(when: "empty") {
        /* Show "no products" message */
    }

    @state(when: "error") {
        background: #fee;
    }

    @each($products as $product) {
        template: "product-card";
        /* ... */
    }
}
```

### Data Status Signals

| Signal | Meaning |
|----------|---------|
| `$products_loading` | `true` while the source is fetching |
| `$products_error` | Error value if the fetch failed, otherwise null |
| `$products.length == 0` | Data loaded but array is empty |
| `$products` | The loaded array (ready state) |

### Empty State Pattern

```css
.gallery {
    data-state: $products_loading ? "loading" : ($products.length == 0 ? "empty" : "ready");

    @state(when: "empty") {
        &::after {
            content: "No items to display";
            display: block;
            text-align: center;
            padding: 60px 20px;
            color: #999;
        }
    }
}
```

### Loading Spinner Pattern

```css
.product-list {
    data-state: $products_loading ? "loading" : "ready";

    @state(when: "loading") {
        &::before {
            content: "";
            display: block;
            width: 40px;
            height: 40px;
            margin: 40px auto;
            border: 3px solid #eee;
            border-top-color: #333;
            border-radius: 50%;
            animation: spin 0.8s linear infinite;
        }
    }

    @keyframes spin {
        to { transform: rotate(360deg); }
    }
}
```

### Alternative: reactive classes

You can also drive state with CSS classes directly, without `@state(when:)`:

```css
.product-list {
    .is-loading: $products_loading;
    .is-empty:   !$products_loading && $products.length == 0;
    .is-error:   $products_error != null;
}

.product-list.is-loading { opacity: 0.5; min-height: 400px; }
.product-list.is-empty::after { content: "No items to display"; }
.product-list.is-error   { background: #fee; }
```

## Common Patterns

### Pattern 1: Product Gallery

```css
@type Product {
    id: string;
    name: string;
    image: url;
    price: number;
}

@data fetch $products Product[] : "/data/products.json";

.product-grid {
    data-state: $products_loading ? "loading" : "ready";

    @state(when: "loading") { opacity: 0.5; }
    @state(when: "ready") { opacity: 1; }

    @each($products as $product) {
        template: "product-card";

        [slot="image"] {
            src: $product.image;
            alt: $product.name;
            loading: "lazy";
        }
        [slot="name"]: $product.name;
        [slot="price"]: $product.price | currency("$");

        :host {
            data-id: $product.id;
        }
    }

    /* Stagger animation */
    > product-card {
        @scroll reveal(&reveal) {
            opacity: 0 -> 1;
            translate-y: 40px -> 0;
            stagger: 0.1 first;
        }
    }
}
```

### Pattern 2: Shopping Cart

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
    @each(cart) {
        template: "cart-item";

        @let product = products.find(p => p.id == $.productId);

        [slot="image"] { src: product.image; }
        [slot="name"]: product.name;
        [slot="quantity"]: $.quantity;
        [slot="price"]: product.price * $.quantity | currency("$");
    }
}
```

### Pattern 3: Featured Items Filter

```css
@data allPosts: BlogPost[] {
    src: "/data/posts.json";
}

@computed featuredPosts: BlogPost[] {
    from: allPosts;
    where: $.featured == true;
    limit: 3;
}

.featured-posts {
    @each(featuredPosts) {
        template: "post-card";
        /* ... */
    }
}
```

### Pattern 4: Category Navigation

```css
@data categories: Category[] {
    src: inline;
    value: [
        { "id": "prints", "label": "Prints", "icon": "🖼️" },
        { "id": "clothing", "label": "Clothing", "icon": "👕" },
        { "id": "accessories", "label": "Accessories", "icon": "🎒" }
    ];
}

.category-nav {
    @each(categories) {
        template: "nav-link";

        [slot="icon"]: $.icon;
        [slot="label"]: $.label;

        :host {
            href: "#" + $.id;
        }
    }
}
```

### Pattern 5: Testimonials

```css
@type Testimonial {
    quote: string;
    author: string;
    role: string;
    avatar?: url;
}

@data testimonials: Testimonial[] {
    src: "/data/testimonials.json";
}

.testimonials {
    @each(testimonials) {
        template: "testimonial-card";

        [slot="quote"]: $.quote;
        [slot="author"]: $.author;
        [slot="role"]: $.role;
        [slot="avatar"] {
            src: $.avatar | default("/images/default-avatar.png");
            alt: $.author;
        }
    }

    > testimonial-card {
        @scroll reveal(&reveal) {
            opacity: 0 -> 1;
            stagger: 0.15 first;
        }
    }
}
```

---

## Troubleshooting

### Problem: "Template not found"

**Error:** `Template 'product-card' not found`

**Solution:** Make sure your template has the correct ID:

```html
<template id="product-card">
  <!-- Must match template: "product-card" -->
</template>
```

### Problem: "Field missing in data"

**Error:** `Field 'imageUrl' missing in Product`

**Solution:** Either:

1. Add the field to your JSON:
   ```json
   { "imageUrl": "/images/product.jpg" }
   ```

2. Make it optional in your type:
   ```css
   @type Product {
       imageUrl?: url;
   }
   ```

3. Use a default filter:
   ```css
   [slot="image"] {
       src: $.imageUrl | default("/images/placeholder.png");
   }
   ```

### Problem: "Data won't load"

**Checklist:**

1. Check the file path is correct
2. Verify JSON is valid (use a JSON validator)
3. Check browser console for errors
4. Ensure CORS allows the request (if loading from different domain)

### Problem: "Filters not working"

**Common mistakes:**

```css
/* Wrong */
[slot="price"]: $.price | currency($);

/* Right */
[slot="price"]: $.price | currency("$");
```

Filters need strings in quotes.

### Problem: "Reactive state not updating"

**Solution:** Signals only update when you assign to them with `<-`, and state expressions re-evaluate when their signal dependencies change. Data fetches automatically expose a `$<name>_loading` signal, so read that signal directly:

```css
/* Right */
@data fetch $products Product[] : "/data/products.json";

.product-grid {
    data-state: $products_loading ? "loading" : "ready";
}

/* or with reactive classes */
.product-grid {
    .is-loading: $products_loading;
}
.product-grid.is-loading { /* loading styles */ }
```

> **Note:** The `@transition` directive was retired. It is not part of the signal idiom; use a signal with reactive classes or `data-state` instead.

### Problem: "Cross-reference returns undefined"

**Solution:** Ensure both data sources are loaded:

```css
/* Both must be loaded before @each runs */
@data products: Product[] { src: "/data/products.json"; }
@data cart: CartItem[] { src: localStorage("cart"); }

.cart {
    @each(cart) {
        @let product = products.find(p => p.id == $.productId);
        /* product might be undefined if not found */
        [slot="name"]: product?.name | default("Unknown");
    }
}
```

---

## Next Steps

You now know the fundamentals of Spacetime data binding! Here's where to go next:

- **[API Reference](./API_REFERENCE.md)** - Complete reference for all directives and filters
- **[Examples](./DATA_SYSTEM_EXAMPLES.md)** - Real-world examples from actual sites
- **[Migration Guide](./MIGRATION.md)** - Moving existing sites to data binding

## Questions?

If you run into issues or have questions, check the [Troubleshooting](#troubleshooting) section or review the [API Reference](./API_REFERENCE.md).

Happy building!
