# Data Binding Example

A complete working example demonstrating Spacetime's data binding features.

## What This Demonstrates

- Type-safe data definitions
- Loading data from JSON
- Template iteration with `@each`
- Filters (currency, date, truncate)
- State machines for loading states
- Cross-references between data sources
- localStorage for user data
- Computed data and helper functions

## Features

### Product Gallery

- Load products from JSON
- Display with images, names, and prices
- Filter by category
- Scroll animations

### Shopping Cart

- Persist cart in localStorage
- Cross-reference products
- Calculate totals
- Empty state handling

### Testimonials

- Inline data source
- Simple template iteration
- Stagger animations

## Project Structure

```
data-binding/
  index.html          - Main page
  styles.st           - Spacetime definitions
  data/
    products.json     - Product data
  README.md           - This file
```

## How to Run

1. **Compile the Spacetime file:**
   ```bash
   cd examples/data-binding
   spacetime compile styles.st
   ```

2. **Serve the directory:**
   ```bash
   # Using Python
   python -m http.server 8000

   # Or using Node
   npx serve .

   # Or any static file server
   ```

3. **Open in browser:**
   ```
   http://localhost:8000
   ```

## Code Walkthrough

### 1. Type Definitions

We start by defining our data structures:

```css
@type Product {
    id: string;
    name: string;
    description: string;
    price: number;
    category: string;
    imageUrl: url;
    inStock: boolean;
}
```

This gives us compile-time type safety.

### 2. Data Sources

We load products from JSON:

```css
@data products: Product[] {
    src: "/data/products.json";
    cache: 1h;
}
```

And persist the cart in localStorage:

```css
@data cart: CartItem[] {
    src: localStorage("demo-cart");
    default: [];
}
```

### 3. Template Binding

We use `@each` to generate product cards:

```css
.product-grid {
    @each(products) {
        template: "product-card";

        [slot="image"] {
            src: $.imageUrl;
            alt: $.name;
        }
        [slot="name"]: $.name;
        [slot="description"]: $.description | truncate(100);
        [slot="price"]: $.price | currency("$");
    }
}
```

### 4. State Machines

Handle loading states elegantly:

```css
.product-grid {
    .is-loading: $products == null;
    .is-ready: $products.length > 0;
    opacity: 1;
}
.product-grid.is-loading {
    opacity: 0.5;
    min-height: 400px;
}
```

### 5. Computed Data

Calculate cart total automatically:

```css
@computed cartTotal: number {
    from: cart;
    reduce: (sum, item) => {
        let product = getProduct(item.productId);
        return sum + (product.price * item.quantity);
    };
    initial: 0;
}
```

## Customization

### Add More Products

Edit `data/products.json`:

```json
{
    "id": "new-product",
    "name": "New Product",
    "description": "A great new product",
    "price": 49.99,
    "category": "electronics",
    "imageUrl": "/images/new-product.jpg",
    "inStock": true
}
```

No code changes needed!

### Change Price Format

Update the filter:

```css
[slot="price"]: $.price | currency("€");   /* Euros */
[slot="price"]: $.price | currency("£");   /* Pounds */
```

### Add Filtering

Use computed data:

```css
@computed electronicsProducts: Product[] {
    from: products;
    where: $.category == "electronics";
}

.electronics-section {
    @each(electronicsProducts) {
        template: "product-card";
        /* ... */
    }
}
```

## Debugging

### Enable Debug Mode

Add to URL:
```
?debug=spacetime
```

This enables console logging for:
- Data loading
- Template instantiation
- State transitions
- Event dispatching

### Common Issues

**Products not showing:**
- Check browser console for errors
- Verify `data/products.json` path is correct
- Ensure JSON is valid

**Filters not working:**
- Check filter syntax (strings need quotes)
- Verify filter name is correct

**Template not found:**
- Verify `<template id="...">` matches `template: "..."`
- Check for typos

## Next Steps

Try these exercises:

1. **Add a search feature** - Filter products by name
2. **Add sorting** - Sort by price or name
3. **Add pagination** - Show 10 products at a time
4. **Add categories** - Group products by category
5. **Add to cart** - Implement cart functionality
6. **Add wishlist** - Another localStorage data source

## Learn More

- [Data Binding Guide](../../docs/DATA_BINDING_GUIDE.md)
- [API Reference](../../docs/API_REFERENCE.md)
- [More Examples](../../docs/DATA_SYSTEM_EXAMPLES.md)
