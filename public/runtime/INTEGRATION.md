# Runtime Integration Guide

This guide explains how Spacetime's compiler-generated code integrates with the runtime library.

## Overview

The Spacetime compilation pipeline has two parts:

1. **Compiler** (`src/codegen.rs`) - Generates data loading and binding code from `.st` files
2. **Runtime** (`public/runtime/data-binding.js`) - Provides reusable utilities

The compiler currently generates inline code, but it can be refactored to use the runtime library for better maintainability and smaller bundle sizes.

## Current Code Generation (Phase 4)

The compiler in `src/codegen.rs` currently generates inline JavaScript:

### Generated Data Loading Code

From a `.st` file with:
```css
@data prints: Print[] {
    src: "/data/prints.json";
}
```

The compiler generates:
```javascript
const SpacetimeData = {
  prints: null,
};

async function load_prints() {
  try {
    const response = await fetch('/data/prints.json');
    if (!response.ok) {
      document.dispatchEvent(new CustomEvent('data:prints:error', { detail: response.statusText }));
      return;
    }
    const data = await response.json();
    SpacetimeData.prints = data;
    if (Array.isArray(data) && data.length === 0) {
      document.dispatchEvent(new CustomEvent('data:prints:empty'));
    } else {
      document.dispatchEvent(new CustomEvent('data:prints:loaded', { detail: data }));
    }
  } catch (error) {
    console.error('Failed to load prints: ', error);
    document.dispatchEvent(new CustomEvent('data:prints:error', { detail: error.message }));
  }
}

document.addEventListener('DOMContentLoaded', async () => {
  await load_prints();
});
```

### Generated Template Instantiation Code

From:
```css
.zey-gallery {
    @each(prints) {
        template: "zey-print";
        [slot="image"] { src: $.image; alt: $.title; }
        [slot="title"]: $.title;
        :host { data-id: $.id; }
    }
}
```

The compiler generates:
```javascript
document.addEventListener('data:prints:loaded', (event) => {
  const container = document.querySelector('.zey-gallery');
  if (!container) return;

  const template = document.getElementById('zey-print');
  if (!template) {
    console.error('Template #zey-print not found');
    return;
  }

  container.innerHTML = '';

  const data = SpacetimeData.prints;
  if (!Array.isArray(data)) return;

  data.forEach((item, index) => {
    const instance = template.content.cloneNode(true);
    const root = instance.firstElementChild;

    const slot_image = root.querySelector('[slot="image"]');
    if (slot_image) {
      slot_image.setAttribute('src', item.image);
      slot_image.setAttribute('alt', item.title);
    }

    const slot_title = root.querySelector('[slot="title"]');
    if (slot_title) {
      slot_title.textContent = item.title;
    }

    root.setAttribute('data-id', item.id);

    container.appendChild(instance);
  });
});
```

## Integration with Runtime Library

The runtime library can replace much of this generated code. Here's how:

### Option 1: Use Runtime Directly (Recommended)

Modify `src/codegen.rs` to generate lighter code that uses the runtime:

**Generated code becomes:**
```javascript
// Include runtime (once per page)
// <script src="/runtime/data-binding.js"></script>

// Register data sources
SpacetimeRuntime.registerData('prints', {
  src: '/data/prints.json',
  cache: '5m'
});

// Setup bindings
document.addEventListener('DOMContentLoaded', async () => {
  await SpacetimeRuntime.loadData('prints');

  SpacetimeRuntime.engine.instantiate(
    'zey-print',
    SpacetimeRuntime.getData('prints'),
    {
      slots: {
        title: '$.title'
      },
      slotAttrs: {
        image: {
          src: '$.image',
          alt: '$.title'
        }
      },
      host: {
        'data-id': '$.id'
      }
    },
    document.querySelector('.zey-gallery')
  );
});
```

**Benefits:**
- Smaller generated code
- Centralized bug fixes in runtime
- Better caching and error handling
- Consistent behavior across sites

### Option 2: Hybrid Approach

Keep generated code but use runtime utilities for specific features:

```javascript
// Generated code uses runtime filters
const slot_price = root.querySelector('[slot="price"]');
if (slot_price) {
  slot_price.textContent = SpacetimeFilters.currency(item.price, '$');
}

// Generated code uses runtime for cross-references
const print = SpacetimeRuntime.resolve(
  { $: item },
  'prints.find(p => p.id === $.printId)'
);
```

## Updating the Compiler

To make the compiler generate runtime-based code, update `src/codegen.rs`:

### 1. Add Runtime Import

```rust
fn generate_runtime_import() -> String {
    r#"<script src="/runtime/data-binding.js"></script>"#.to_string()
}
```

### 2. Change Data Loading Generation

Replace `generate_data_loading_js()` with:

```rust
fn generate_data_loading_js_with_runtime(data_sources: &[DataDef]) -> String {
    let mut js = String::new();

    js.push_str("\n// Register data sources\n");

    for data in data_sources {
        let source_name = &data.name;
        let src_option = data.options.iter()
            .find(|opt| opt.key == "src")
            .expect(&format!("Data source '{}' missing 'src' option", source_name));

        js.push_str(&format!(
            "SpacetimeRuntime.registerData('{}', {{\n",
            source_name
        ));

        match &src_option.value {
            DataOptionValue::String(url) => {
                js.push_str(&format!("  src: '{}',\n", url));
            }
            DataOptionValue::LocalStorage(key) => {
                js.push_str(&format!("  src: {{ type: 'localStorage', key: '{}' }},\n", key));

                // Add default if present
                if let Some(default_opt) = data.options.iter().find(|opt| opt.key == "default") {
                    if let DataOptionValue::Json(json_str) = &default_opt.value {
                        js.push_str(&format!("  default: {},\n", json_str));
                    }
                }
            }
            _ => {}
        }

        // Add cache option if present
        if let Some(cache_opt) = data.options.iter().find(|opt| opt.key == "cache") {
            if let DataOptionValue::String(cache_val) = &cache_opt.value {
                js.push_str(&format!("  cache: '{}',\n", cache_val));
            }
        }

        js.push_str("});\n\n");
    }

    // Initialize
    js.push_str("document.addEventListener('DOMContentLoaded', async () => {\n");
    js.push_str("  await SpacetimeRuntime.loadAll();\n");
    js.push_str("});\n\n");

    js
}
```

### 3. Change Template Instantiation Generation

Replace `generate_template_instantiation_js()` with:

```rust
fn generate_template_instantiation_js_with_runtime(
    selector: &str,
    each: &EachBlock
) -> String {
    let mut js = String::new();

    let source = &each.source;
    let template_id = each.template.as_ref()
        .expect("@each block must have template specified");

    js.push_str(&format!(
        "\n// Template binding for {} using {}\n",
        selector, template_id
    ));

    js.push_str(&format!(
        "document.addEventListener('data:{}:loaded', () => {{\n",
        source
    ));

    js.push_str(&format!(
        "  const container = document.querySelector('{}');\n",
        selector
    ));
    js.push_str("  if (!container) return;\n\n");

    // Build bindings object
    js.push_str("  const bindings = {\n");

    // Generate slots
    js.push_str("    slots: {\n");
    for binding in &each.bindings {
        if let EachBinding::SlotText { slot, expr } = binding {
            let expr_js = expr_to_property_path(expr);
            js.push_str(&format!("      '{}': '{}',\n", slot, expr_js));
        }
    }
    js.push_str("    },\n");

    // Generate slotAttrs
    js.push_str("    slotAttrs: {\n");
    for binding in &each.bindings {
        if let EachBinding::SlotAttrs { slot, attrs } = binding {
            js.push_str(&format!("      '{}': {{\n", slot));
            for attr in attrs {
                let expr_js = expr_to_property_path(&attr.expr);
                js.push_str(&format!("        '{}': '{}',\n", attr.name, expr_js));
            }
            js.push_str("      },\n");
        }
    }
    js.push_str("    },\n");

    // Generate host
    js.push_str("    host: {\n");
    for binding in &each.bindings {
        if let EachBinding::Host { attrs } = binding {
            for attr in attrs {
                let expr_js = expr_to_property_path(&attr.expr);
                js.push_str(&format!("      '{}': '{}',\n", attr.name, expr_js));
            }
        }
    }
    js.push_str("    }\n");

    js.push_str("  };\n\n");

    // Call runtime
    js.push_str(&format!(
        "  SpacetimeRuntime.engine.instantiate('{}', SpacetimeRuntime.getData('{}'), bindings, container);\n",
        template_id, source
    ));

    js.push_str("});\n\n");

    js
}

fn expr_to_property_path(expr: &BindingExpr) -> String {
    match expr {
        BindingExpr::PropertyPath(path) => {
            if path.segments.is_empty() {
                "$".to_string()
            } else {
                path.segments.join(".")
            }
        }
        _ => "$".to_string()
    }
}
```

## Migration Path

### Phase 1: Runtime Available (Current)
- Runtime library exists and is tested
- Generated code continues to work inline
- Developers can manually use runtime for new features

### Phase 2: Hybrid Generation
- Compiler generates code that uses runtime for filters
- Data loading still inline
- Backward compatible with existing sites

### Phase 3: Full Runtime Integration
- Compiler generates minimal code
- All heavy lifting done by runtime
- Smallest possible bundle size

## Performance Considerations

### Bundle Size

**Before (inline code):**
- Filters duplicated in every site: ~5KB
- Data loading duplicated: ~3KB
- Template logic duplicated: ~4KB
- **Total per site: ~12KB**

**After (with runtime):**
- Runtime loaded once: ~20KB (cached)
- Generated code per site: ~2KB
- **Total for multiple sites: 20KB + (n × 2KB)**

Break-even point: 2 sites

### Execution Speed

Runtime adds minimal overhead:
- Function call overhead: <0.1ms
- Additional abstractions: <0.5ms per operation
- Total impact: negligible for typical sites

### Caching Benefits

Runtime library is shared across all sites:
- Loaded once
- Cached by browser
- Updated independently from site code

## Example: Full Integration

### Input (.st file)
```css
@data prints: Print[] {
    src: "/data/prints.json";
    cache: 5m;
}

.gallery {
    @each(prints) {
        template: "print-card";
        [slot="title"]: $.title;
        [slot="price"]: $.price | currency("$");
    }
}
```

### Generated Output (with runtime)
```html
<!DOCTYPE html>
<html>
<head>
  <script src="/runtime/data-binding.js"></script>
  <script src="/generated/site.js"></script>
</head>
<body>
  <div class="gallery"></div>
  <template id="print-card">
    <div>
      <h3 slot="title"></h3>
      <span slot="price"></span>
    </div>
  </template>
</body>
</html>
```

### Generated JavaScript
```javascript
// Register data
SpacetimeRuntime.registerData('prints', {
  src: '/data/prints.json',
  cache: '5m'
});

// Setup binding
document.addEventListener('data:prints:loaded', () => {
  const instances = SpacetimeRuntime.engine.instantiate(
    'print-card',
    SpacetimeRuntime.getData('prints'),
    {
      slots: { title: '$.title' }
    },
    document.querySelector('.gallery')
  );

  // Apply filters
  instances.forEach((el, i) => {
    const print = SpacetimeRuntime.getData('prints')[i];
    el.querySelector('[slot="price"]').textContent =
      SpacetimeFilters.currency(print.price, '$');
  });
});

// Load data
document.addEventListener('DOMContentLoaded', () => {
  SpacetimeRuntime.loadAll();
});
```

## Testing the Integration

1. **Unit tests**: Test runtime functions in isolation
2. **Integration tests**: Test generated code with runtime
3. **E2E tests**: Test full compilation pipeline

See `data-binding.test.html` for runtime tests.

## Next Steps

1. Update compiler to generate runtime-based code
2. Add runtime version to generated HTML
3. Setup CDN for runtime distribution
4. Create migration guide for existing sites
5. Add TypeScript definitions for runtime API
