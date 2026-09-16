# Spacetime Data Binding Runtime

A reusable JavaScript runtime library for Spacetime's data binding system. Provides data loading, template instantiation, filters, and reactive updates.

## Features

- **Data Loading**: Fetch from URLs, localStorage, or inline data
- **Caching**: Configurable cache strategies (duration-based or forever)
- **Refresh Strategies**: Polling, on-focus, or manual refresh
- **Template Engine**: Clone and populate HTML templates with data
- **Filters**: Built-in data transformation functions
- **State Machine Integration**: Trigger state transitions based on data events
- **Reactive Updates**: Observe localStorage changes
- **Debug Mode**: Detailed console logging

## Installation

### Option 1: Script Tag (Global)

```html
<script src="/runtime/data-binding.js"></script>
<script>
  // Runtime is available globally
  console.log(SpacetimeRuntime);
</script>
```

### Option 2: ES Module

```javascript
import {
  SpacetimeDataLoader,
  SpacetimeTemplateEngine,
  SpacetimeFilters,
  SpacetimeRuntime
} from './runtime/data-binding.js';
```

## Quick Start

### 1. Define a Template

```html
<template id="product-card">
  <div class="product-card">
    <img slot="image" src="" alt="">
    <h3 slot="title"></h3>
    <p slot="description"></p>
    <span slot="price"></span>
  </div>
</template>

<div id="product-grid"></div>
```

### 2. Register and Load Data

```javascript
// Register a data source
SpacetimeRuntime.registerData('products', {
  src: '/api/products.json',
  cache: '5m'  // Cache for 5 minutes
});

// Load the data
const products = await SpacetimeRuntime.loadData('products');
```

### 3. Instantiate Templates

```javascript
const container = document.getElementById('product-grid');

const bindings = {
  slots: {
    title: '$.name',
    description: '$.description'
  },
  slotAttrs: {
    image: {
      src: '$.imageUrl',
      alt: '$.name'
    }
  },
  host: {
    'data-id': '$.id'
  }
};

SpacetimeRuntime.engine.instantiate(
  'product-card',
  products,
  bindings,
  container
);
```

## API Reference

### SpacetimeRuntime

Main coordinator object for the runtime.

#### Methods

##### `registerData(name, options)`

Register a data source.

```javascript
SpacetimeRuntime.registerData('prints', {
  src: '/data/prints.json',
  cache: '1h',           // Cache duration: 5s, 30m, 1h, 2d, forever, none
  refresh: '30s'         // Refresh interval or 'on-focus'
});
```

**Options:**
- `src`: String (URL), Object (`{ type: 'localStorage', key: 'key' }`), or Object (`{ type: 'inline', value: [...] }`)
- `cache`: Cache duration (e.g., "5m", "1h", "forever", "none")
- `refresh`: Refresh strategy (duration string or "on-focus")
- `default`: Default value if source is empty (for localStorage)

##### `loadData(name)`

Load a registered data source.

```javascript
const data = await SpacetimeRuntime.loadData('products');
```

##### `loadAll()`

Load all registered data sources.

```javascript
await SpacetimeRuntime.loadAll();
```

##### `getData(name)`

Get loaded data by name.

```javascript
const products = SpacetimeRuntime.getData('products');
```

##### `resolve(context, expr)`

Resolve a cross-reference expression.

```javascript
const context = { $: { printId: 'IN-003' } };
const print = SpacetimeRuntime.resolve(
  context,
  'prints.find(p => p.id === $.printId)'
);
```

##### `transitionOnDataEvent(eventName, selector, targetState)`

Trigger state transition when data event fires.

```javascript
SpacetimeRuntime.transitionOnDataEvent(
  'data:prints:loaded',
  '.gallery',
  'ready'
);
```

### SpacetimeDataLoader

Handles loading data from various sources.

```javascript
const loader = new SpacetimeDataLoader('products', {
  src: '/api/products.json',
  cache: '5m'
});

const data = await loader.load();
```

#### Methods

- `load()`: Load data (uses cache if valid)
- `refresh()`: Force reload (bypass cache)
- `addEventListener(type, callback)`: Listen for loader events
- `destroy()`: Clean up resources

### SpacetimeTemplateEngine

Instantiates and binds data to templates.

```javascript
const engine = new SpacetimeTemplateEngine();

engine.instantiate(templateId, data, bindings, container, options);
```

#### Methods

##### `instantiate(templateId, data, bindings, container, options)`

Create instances from template and data.

**Parameters:**
- `templateId`: ID of the `<template>` element
- `data`: Array of data items
- `bindings`: Binding configuration object
- `container`: Container element to append instances
- `options`: Additional options
  - `clear`: Clear container first (default: true)
  - `preserveScroll`: Maintain scroll position (default: false)

**Bindings object:**
```javascript
{
  let: {
    // @let variables
    discountPrice: (ctx) => ctx.$.price * 0.9
  },
  slots: {
    // Text content bindings
    title: '$.name',
    price: 'discountPrice'
  },
  slotAttrs: {
    // Attribute bindings
    image: {
      src: '$.imageUrl',
      alt: '$.name'
    }
  },
  host: {
    // Root element attributes
    'data-id': '$.id',
    'class': '$.category'
  }
}
```

##### `bindSlot(root, slotName, expr, context)`

Bind text content to a slot.

##### `bindAttribute(element, attrName, value)`

Bind attribute to an element.

### SpacetimeFilters

Built-in filter functions.

#### Available Filters

| Filter | Description | Example |
|--------|-------------|---------|
| `json` | Serialize to JSON | `SpacetimeFilters.json({ a: 1 })` → `'{"a":1}'` |
| `currency(symbol)` | Format as currency | `SpacetimeFilters.currency(95, '$')` → `'$95.00'` |
| `date(format)` | Format date | `SpacetimeFilters.date(date, 'short')` → `'Dec 16'` |
| `default(value)` | Fallback value | `SpacetimeFilters.default(null, 'N/A')` → `'N/A'` |
| `count` | Array length | `SpacetimeFilters.count([1,2,3])` → `3` |
| `truncate(n)` | Truncate string | `SpacetimeFilters.truncate('Hello World', 5)` → `'Hello...'` |
| `if(true, false)` | Conditional | `SpacetimeFilters.if(true, 'Yes', 'No')` → `'Yes'` |
| `uppercase` | Uppercase | `SpacetimeFilters.uppercase('hello')` → `'HELLO'` |
| `lowercase` | Lowercase | `SpacetimeFilters.lowercase('HELLO')` → `'hello'` |
| `capitalize` | Capitalize first | `SpacetimeFilters.capitalize('hello')` → `'Hello'` |
| `pluralize(s, p)` | Pluralize | `SpacetimeFilters.pluralize(1, 'item', 'items')` → `'item'` |

#### Date Format Options

- `short`: "Dec 16"
- `medium`: "Dec 16, 2024"
- `long`: "December 16, 2024"
- `iso`: "2024-12-16"
- `relative`: "2 days ago"

#### Using Filters Programmatically

```javascript
// Single filter
const formatted = SpacetimeFilters.currency(99.99, '$');

// Filter chain
const result = applyFilters(value, [
  { name: 'truncate', args: [20] },
  { name: 'uppercase', args: [] }
]);
```

## Data Events

The runtime emits events for data lifecycle:

### Event Names

- `data:{source}:loaded` - Data loaded successfully with items
- `data:{source}:empty` - Data loaded but array is empty
- `data:{source}:error` - Data failed to load

### Listening for Events

```javascript
document.addEventListener('data:products:loaded', (event) => {
  console.log('Products loaded:', event.detail);
});

document.addEventListener('data:products:error', (event) => {
  console.error('Load error:', event.detail);
});
```

## Special Variables

When using bindings, these special variables are available:

| Variable | Description |
|----------|-------------|
| `$` | Current item in iteration |
| `_index` | Zero-based index |
| `_first` | Boolean: is first item |
| `_last` | Boolean: is last item |
| `_count` | Total number of items |

Example:
```javascript
{
  host: {
    'data-index': '$._index',
    'class': (ctx) => ctx._first ? 'first' : ''
  }
}
```

## Examples

### localStorage Source

```javascript
SpacetimeRuntime.registerData('cart', {
  src: { type: 'localStorage', key: 'cart-items' },
  default: []
});

await SpacetimeRuntime.loadData('cart');
```

### Inline Data

```javascript
SpacetimeRuntime.registerData('sizes', {
  src: {
    type: 'inline',
    value: [
      { code: 'S', label: 'Small' },
      { code: 'M', label: 'Medium' },
      { code: 'L', label: 'Large' }
    ]
  }
});
```

### With Filters

```javascript
const instances = SpacetimeRuntime.engine.instantiate(
  'product-card',
  products,
  bindings,
  container
);

// Apply filters manually
instances.forEach((instance, i) => {
  const product = products[i];
  const priceSlot = instance.querySelector('[slot="price"]');
  priceSlot.textContent = SpacetimeFilters.currency(product.price, '$');
});
```

### Cross-Reference Lookup

```javascript
// Cart items reference prints
const context = { $: { printId: 'IN-003' } };
const print = SpacetimeRuntime.resolve(
  context,
  'prints.find(p => p.id === $.printId)'
);

console.log('Found print:', print.title);
```

### Reactive Updates

```javascript
// Watch localStorage for changes
SpacetimeRuntime.watchLocalStorage('cart-items', (newValue, oldValue) => {
  console.log('Cart updated:', newValue);
  // Re-render cart UI
});
```

### State Machine Integration

```javascript
// Setup transitions
SpacetimeRuntime.transitionOnDataEvent('data:prints:loaded', '.gallery', 'ready');
SpacetimeRuntime.transitionOnDataEvent('data:prints:empty', '.gallery', 'empty');
SpacetimeRuntime.transitionOnDataEvent('data:prints:error', '.gallery', 'error');

// CSS states
.gallery.state-loading { opacity: 0.5; }
.gallery.state-ready { opacity: 1; }
.gallery.state-error { background: red; }
```

## Debug Mode

Enable debug mode to see detailed console logs:

```javascript
// Via localStorage
localStorage.setItem('spacetime-debug', 'true');

// Via URL
// ?debug=spacetime
```

This will log:
- Data loading operations
- Cache hits/misses
- Template instantiation
- Event dispatching

## Testing

Run the test suite by opening `/runtime/data-binding.test.html` in a browser.

View the live example at `/runtime/example.html`.

## Browser Support

- Modern browsers (Chrome, Firefox, Safari, Edge)
- Requires ES6+ support
- Uses Fetch API, CustomEvent, and template elements

## Integration with Generated Code

This runtime is designed to work alongside code generated by Spacetime's compiler (`/src/codegen.rs`). The compiler generates inline code that uses these runtime utilities.

Generated code example:
```javascript
// Generated by Spacetime compiler
document.addEventListener('data:prints:loaded', (event) => {
  const container = document.querySelector('.gallery');
  const template = document.getElementById('print-card');
  const data = SpacetimeData.prints;

  // Uses runtime engine
  SpacetimeRuntime.engine.instantiate(
    'print-card',
    data,
    bindings,
    container
  );
});
```

## License

MIT
