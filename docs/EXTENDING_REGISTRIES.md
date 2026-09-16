# Extending Runtime Registries

This document explains how to add new runtime registries for filters, timelines, signals, and other runtime namespaces following the same pattern as the `functions` registry.

## Overview

Runtime registries define where runtime values live and how they're accessed. The pattern involves:

1. **Define the registry** in `stdlib/runtime/registries.st`
2. **Create a macro** that uses `%resolves` to reference the registry
3. **Create a primitive** that emits code using the registry namespace

## Current Implementation

The `functions` registry is fully implemented:

```st
// stdlib/runtime/registries.st
%runtime-registry functions {
  %target js {
    %namespace { ST.functions }
    %init { ST.functions = ST.functions || {}; }
    %register($name, $value) { ST.functions[$name] = $value; }
    %call($name, $args) { ST.functions[$name]($args) }
    %access($name) { ST.functions[$name] }
  }
}
```

The `@fn` macro uses it:

```st
// stdlib/macros/type-data.st
%macro fn {
  %creates @fn
  %form { @fn $name:ident($params:params) : $returnType:typeref { $body:expr } }

  %resolves {
    $name -> functions
  }

  %binds {
    fn-registry(name: $name, params: $params, body: $body, returnType: $returnType)
  }
}
```

---

## Adding a Filters Registry

### Step 1: Define the Registry

Add to `stdlib/runtime/registries.st`:

```st
%runtime-registry filters {
  %target js {
    %namespace { ST.filters }
    %init { ST.filters = ST.filters || {}; }
    %register($name, $fn) { ST.filters[$name] = $fn; }
    %apply($name, $value, $args) { ST.filters[$name]($value, ...$args) }
    %access($name) { ST.filters[$name] }
  }
}
```

### Step 2: Create or Update the Filter Macro

If you have a `@filter` macro:

```st
%macro filter {
  %creates @filter
  %scope file

  %form {
    @filter $name:ident($params:params) {
      $body:expr
    }
  }

  %resolves {
    $name -> filters
  }

  %binds {
    filter-registry(name: $name, params: $params, body: $body)
  }
}
```

### Step 3: Update Codegen

In `src/codegen/mod.rs`, ensure filter calls are rewritten:

```rust
// When generating filter application code
let namespace = registry.get_registry_namespace("filters")
    .unwrap_or("ST.filters");
format!("{}.{}(value, {})", namespace, filter_name, args)
```

---

## Adding a Timelines Registry

For scroll-driven animations and timeline definitions:

### Step 1: Define the Registry

```st
%runtime-registry timelines {
  %target js {
    %namespace { ST.timelines }
    %init { ST.timelines = ST.timelines || {}; }
    %register($name, $timeline) { ST.timelines[$name] = $timeline; }
    %get($name) { ST.timelines[$name] }
    %play($name) { ST.timelines[$name].play() }
    %pause($name) { ST.timelines[$name].pause() }
  }
}
```

### Step 2: Create the Primitive

In `stdlib/primitives/animation/`:

```st
%primitive timeline-registry {
  %params {
    name: ident
    driver: binding
    range: [number, number]
    keyframes: keyframes
  }

  %emits js {
    ST.timelines['{{name}}'] = new ScrollTimeline({
      driver: {{driver}},
      range: [{{range.0}}, {{range.1}}]
    });
    // Apply keyframes...
  }
}
```

---

## Adding a Signals Registry

For reactive state management:

### Step 1: Define the Registry

```st
%runtime-registry signals {
  %target js {
    %namespace { ST.signals }
    %init { ST.signals = ST.signals || {}; }
    %register($name, $initial) { ST.signals[$name] = ST.createSignal($initial); }
    %get($name) { ST.signals[$name].value }
    %set($name, $value) { ST.signals[$name].value = $value }
    %subscribe($name, $callback) { ST.signals[$name].subscribe($callback) }
  }
}
```

### Step 2: Create the Signal Macro

```st
%macro signal {
  %creates @signal
  %scope file

  %form {
    @signal $name:ident : $type:typeref = $initial:expr
  }

  %resolves {
    $name -> signals
  }

  %binds {
    signal-registry(name: $name, type: $type, initial: $initial)
  }
}
```

---

## Implementation Checklist

When adding a new registry:

- [ ] **Registry Definition** (`stdlib/runtime/registries.st`)
  - Define `%namespace`, `%init`, and relevant operations
  - Consider multi-target support if needed

- [ ] **Macro Definition** (`stdlib/macros/`)
  - Add `%resolves { $name -> registryName }`
  - Define `%form` for user syntax
  - Bind to a primitive via `%binds`

- [ ] **Primitive Definition** (`stdlib/primitives/`)
  - Emit JavaScript that uses the registry namespace
  - Use `registry.get_registry_namespace()` for dynamic lookup

- [ ] **Codegen Updates** (`src/codegen/`)
  - Ensure transform rules are applied
  - Use registry namespace instead of hardcoded strings

- [ ] **Tests**
  - Parser tests for new syntax
  - Integration tests for code generation
  - Runtime tests for behavior

---

## Multi-Target Considerations

Registries support multiple targets. When adding WASM support:

```st
%runtime-registry functions {
  %target js {
    %namespace { ST.functions }
    %call($name, $args) { ST.functions[$name]($args) }
  }

  %target wasm {
    %namespace { spacetime::functions }
    %call($name, $args) {
      (call_indirect (type $fn_sig)
        (local.get $args)
        (call $get_fn_index (i32.const $name)))
    }
  }
}
```

The compiler uses `registry.current_target` to select the appropriate target definition. Macros using `%resolves` don't need to change.

---

## Architecture Rationale

### Why Registries?

1. **Separation of concerns**: Macros declare *what* symbols mean, not *how* they're emitted
2. **Multi-target**: Same macro works for JS, WASM, Swift, etc.
3. **Customization**: Projects can override registry namespaces
4. **Consistency**: All runtime values follow the same pattern

### %resolves vs %transforms (Historical)

The old `%transforms` approach:
```st
%transforms { $name($args:expr*) => SpacetimeFunctions.$name($args) }
```

Problems:
- Hardcoded string templates
- No multi-target support
- Syntactic rewriting, not semantic resolution

The new `%resolves` approach:
```st
%resolves { $name -> functions }
```

Benefits:
- Registry owns the namespace
- Semantic symbol resolution
- Multi-target ready
- Analyzable at compile time

---

## Example: Full Filter Implementation

Here's a complete example adding a custom filter system:

```st
// stdlib/runtime/registries.st - Add to existing file
%runtime-registry filters {
  %target js {
    %namespace { ST.filters }
    %init { ST.filters = ST.filters || {}; }
    %register($name, $fn) { ST.filters[$name] = $fn; }
    %apply($name, $value, $args) { ST.filters[$name]($value, ...$args) }
  }
}

// stdlib/macros/filters.st - New file
%macro filter {
  %creates @filter
  %scope file

  %form {
    @filter $name:ident($value:ident, $params:params?) {
      $body:expr
    }
  }

  %resolves {
    $name -> filters
  }

  %binds {
    filter-registry(name: $name, value: $value, params: $params, body: $body)
  }
}

// stdlib/primitives/data/filter-registry.st - New file
%primitive filter-registry {
  %params {
    name: ident
    value: ident
    params: params?
    body: expr
  }

  %emits js {
    ST.filters['{{name}}'] = ({{value}}{{#if params}}, {{params}}{{/if}}) => {
      {{body}}
    };
  }
}
```

Usage:
```st
@filter currency(value, symbol: string = "$") {
  return symbol + value.toFixed(2);
}

.product-card {
  @each(products) {
    [slot="price"]: $.price | currency("€");
  }
}
```

Generated JavaScript:
```js
ST.filters = ST.filters || {};
ST.filters['currency'] = (value, symbol = "$") => {
  return symbol + value.toFixed(2);
};

// In template instantiation
slot_price.textContent = ST.filters.currency(item_0.price, "€");
```
