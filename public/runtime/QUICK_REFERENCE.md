# Spacetime Data Binding - Quick Reference

One-page cheat sheet for the runtime API.

## Setup

```html
<script src="/runtime/data-binding.js"></script>
```

## Data Sources

### URL
```javascript
SpacetimeRuntime.registerData('products', {
  src: '/api/products.json',
  cache: '5m'  // 5s, 30m, 1h, 2d, forever, none
});
```

### localStorage
```javascript
SpacetimeRuntime.registerData('cart', {
  src: { type: 'localStorage', key: 'cart' },
  default: []
});
```

### Inline
```javascript
SpacetimeRuntime.registerData('sizes', {
  src: { type: 'inline', value: [...] }
});
```

## Loading Data

```javascript
// Single source
const data = await SpacetimeRuntime.loadData('products');

// All sources
await SpacetimeRuntime.loadAll();

// Get loaded data
const products = SpacetimeRuntime.getData('products');

// Refresh (bypass cache)
const loader = SpacetimeRuntime.loaders.get('products');
await loader.refresh();
```

## Templates

### HTML Template
```html
<template id="card">
  <div>
    <img slot="image" src="" alt="">
    <h3 slot="title"></h3>
    <p slot="description"></p>
  </div>
</template>

<div id="container"></div>
```

### Instantiate
```javascript
SpacetimeRuntime.engine.instantiate(
  'card',          // template ID
  data,            // array of items
  bindings,        // binding config
  container        // container element
);
```

### Bindings Config
```javascript
const bindings = {
  // Computed values (@let)
  let: {
    discount: (ctx) => ctx.$.price * 0.9
  },

  // Text content
  slots: {
    title: '$.name',
    description: '$.desc'
  },

  // Attributes
  slotAttrs: {
    image: {
      src: '$.imageUrl',
      alt: '$.name',
      loading: 'lazy'
    }
  },

  // Root element attributes
  host: {
    'data-id': '$.id',
    'class': '$.category'
  }
};
```

## Filters

```javascript
// Usage
SpacetimeFilters.currency(99.99, '$')        // "$99.99"
SpacetimeFilters.date(date, 'short')         // "Dec 16"
SpacetimeFilters.truncate('Hello', 5)        // "Hello..."
SpacetimeFilters.default(null, 'N/A')        // "N/A"
SpacetimeFilters.uppercase('hello')          // "HELLO"
SpacetimeFilters.if(true, 'Yes', 'No')       // "Yes"

// Chain filters
applyFilters('hello world', [
  { name: 'truncate', args: [5] },
  { name: 'uppercase', args: [] }
]);  // "HELLO..."
```

### All Filters
| Filter | Args | Example |
|--------|------|---------|
| `json` | - | `{ a: 1 }` → `'{"a":1}'` |
| `currency` | symbol | `95` → `'$95.00'` |
| `date` | format | `date` → `'Dec 16'` |
| `default` | fallback | `null` → `'N/A'` |
| `count` | - | `[1,2,3]` → `3` |
| `truncate` | length | `'Hello World'` → `'Hello...'` |
| `if` | true, false | `true` → `'Yes'` |
| `uppercase` | - | `'hello'` → `'HELLO'` |
| `lowercase` | - | `'HELLO'` → `'hello'` |
| `capitalize` | - | `'hello'` → `'Hello'` |
| `pluralize` | singular, plural | `1` → `'item'` |

## Events

### Listen for Data Events
```javascript
document.addEventListener('data:products:loaded', (e) => {
  console.log('Data:', e.detail);
});

document.addEventListener('data:products:empty', () => {
  console.log('No data');
});

document.addEventListener('data:products:error', (e) => {
  console.error('Error:', e.detail);
});
```

### Event Types
- `data:{source}:loaded` - Data loaded with items
- `data:{source}:empty` - Data loaded but empty
- `data:{source}:error` - Load failed

## State Machine Integration

```javascript
SpacetimeRuntime.transitionOnDataEvent(
  'data:products:loaded',  // event name
  '.gallery',              // selector
  'ready'                  // target state
);
```

```css
.gallery.state-loading { opacity: 0.5; }
.gallery.state-ready { opacity: 1; }
.gallery.state-error { background: red; }
```

## Special Variables

In bindings, these are available:

| Variable | Description |
|----------|-------------|
| `$` | Current item |
| `$._index` | Index (0-based) |
| `$._first` | Is first item? |
| `$._last` | Is last item? |
| `$._count` | Total items |

```javascript
host: {
  'data-index': '$._index',
  'class': (ctx) => ctx._first ? 'first' : ''
}
```

## Cross-References

```javascript
// Find related item
const context = { $: { printId: 'IN-003' } };
const print = SpacetimeRuntime.resolve(
  context,
  'prints.find(p => p.id === $.printId)'
);
```

## Reactive Updates

```javascript
// Watch localStorage
SpacetimeRuntime.watchLocalStorage('cart', (newVal, oldVal) => {
  console.log('Cart updated:', newVal);
  // Re-render
});
```

## Refresh Strategies

```javascript
// Polling (every 30 seconds)
SpacetimeRuntime.registerData('live', {
  src: '/api/live',
  refresh: '30s'
});

// On window focus
SpacetimeRuntime.registerData('cart', {
  src: { type: 'localStorage', key: 'cart' },
  refresh: 'on-focus'
});
```

## Debouncing

```javascript
const debouncedUpdate = SpacetimeRuntime.debounce(() => {
  // Update UI
}, 300);

// Call multiple times, runs once after 300ms
debouncedUpdate();
debouncedUpdate();
debouncedUpdate();
```

## Debug Mode

```javascript
// Enable debug logging
localStorage.setItem('spacetime-debug', 'true');

// Or via URL: ?debug=spacetime
```

## Complete Example

```html
<!DOCTYPE html>
<html>
<head>
  <script src="/runtime/data-binding.js"></script>
</head>
<body>
  <div id="gallery"></div>

  <template id="card">
    <div class="card">
      <img slot="image" src="" alt="">
      <h3 slot="title"></h3>
      <span slot="price"></span>
    </div>
  </template>

  <script>
    // Register data
    SpacetimeRuntime.registerData('prints', {
      src: '/data/prints.json',
      cache: '5m'
    });

    // Setup state transitions
    SpacetimeRuntime.transitionOnDataEvent(
      'data:prints:loaded',
      '#gallery',
      'ready'
    );

    // Load and render
    document.addEventListener('DOMContentLoaded', async () => {
      const prints = await SpacetimeRuntime.loadData('prints');

      const instances = SpacetimeRuntime.engine.instantiate(
        'card',
        prints,
        {
          slots: { title: '$.title' },
          slotAttrs: {
            image: { src: '$.image', alt: '$.title' }
          },
          host: { 'data-id': '$.id' }
        },
        document.getElementById('gallery')
      );

      // Apply filters
      instances.forEach((el, i) => {
        const priceSlot = el.querySelector('[slot="price"]');
        priceSlot.textContent = SpacetimeFilters.currency(
          prints[i].price,
          '$'
        );
      });
    });
  </script>
</body>
</html>
```

## Common Patterns

### Loading State
```javascript
const gallery = document.getElementById('gallery');
gallery.classList.add('loading');

await SpacetimeRuntime.loadData('products');

gallery.classList.remove('loading');
gallery.classList.add('loaded');
```

### Error Handling
```javascript
try {
  await SpacetimeRuntime.loadData('products');
} catch (err) {
  console.error('Load failed:', err);
  // Show error UI
}
```

### Multiple Data Sources
```javascript
const [products, categories, cart] = await Promise.all([
  SpacetimeRuntime.loadData('products'),
  SpacetimeRuntime.loadData('categories'),
  SpacetimeRuntime.loadData('cart')
]);
```

### Conditional Rendering
```javascript
const prints = await SpacetimeRuntime.loadData('prints');

if (prints.length === 0) {
  container.innerHTML = '<p>No prints available</p>';
} else {
  SpacetimeRuntime.engine.instantiate('card', prints, bindings, container);
}
```

## Resources

- **API Docs**: `README.md`
- **Integration Guide**: `INTEGRATION.md`
- **Tests**: `data-binding.test.html`
- **Example**: `example.html`
- **Spec**: `/docs/DATA_SYSTEM.md`
