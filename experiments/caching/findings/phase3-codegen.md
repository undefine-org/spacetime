# Phase 3: Codegen Integration for Purity Caching

## Overview

This document describes the changes made to `src/codegen/mod.rs` to integrate the purity caching system into Spacetime's element reference code generation. The caching system from Phase 1-2 is now automatically used by all element refs.

## What Changed

### Location
- **File:** `src/codegen/mod.rs`
- **Function:** `generate_element_ref_code` (lines ~954-1210)

### Changes Made

The `generate_element_ref_code` function was modified to:

1. **Generate cached rect accessor** - Uses `ST._purity.createCachedRect()` instead of direct `getBoundingClientRect()` calls
2. **Use cached accessor in update function** - The `update_*` functions now call the cached accessor
3. **ResizeObserver invalidates cache** - Before calling the update function, ResizeObserver calls `ST._purity.invalidate()`

## Before/After Comparison

### Before (Old Code)

```javascript
// &header .header
waitForElement('.header', (scope) => {
  const root = scope.shadowRoot || scope;
  waitForElement('.header', (el) => {
    const update_header = () => {
      const rect = el.getBoundingClientRect();  // <-- Called every time!
      setVar('--st-header-rect-width', rect.width + 'px');
      setVar('--st-header-rect-height', rect.height + 'px');
      setVar('--st-header-rect-top', rect.top + 'px');
      setVar('--st-header-rect-right', rect.right + 'px');
      setVar('--st-header-rect-bottom', rect.bottom + 'px');
      setVar('--st-header-rect-left', rect.left + 'px');
      setVar('--st-header-rect-x', rect.x + 'px');
      setVar('--st-header-rect-y', rect.y + 'px');
    };
    new ResizeObserver(update_header).observe(el);  // <-- No cache invalidation
    window.addEventListener('scroll', update_header, { passive: true });
    update_header();
  }, root);
});
```

**Problems with old approach:**
- `getBoundingClientRect()` called on every scroll event
- No caching of stable element rects
- Scroll handlers could cause layout thrashing

### After (New Code)

```javascript
// &header .header
waitForElement('.header', (scope) => {
  const root = scope.shadowRoot || scope;
  waitForElement('.header', (el) => {
    // Create cached rect accessor using purity system
    const getRect_header = ST._purity.createCachedRect(el, '.header');

    const update_header = () => {
      const rect = getRect_header();  // <-- Uses cache when stable
      setVar('--st-header-rect-width', rect.width + 'px');
      setVar('--st-header-rect-height', rect.height + 'px');
      setVar('--st-header-rect-top', rect.top + 'px');
      setVar('--st-header-rect-right', rect.right + 'px');
      setVar('--st-header-rect-bottom', rect.bottom + 'px');
      setVar('--st-header-rect-left', rect.left + 'px');
      setVar('--st-header-rect-x', rect.x + 'px');
      setVar('--st-header-rect-y', rect.y + 'px');
    };
    // ResizeObserver invalidates the cache before updating
    new ResizeObserver(() => { ST._purity.invalidate(); update_header(); }).observe(el);
    window.addEventListener('scroll', update_header, { passive: true });
    update_header();
  }, root);
});
```

**Benefits of new approach:**
- Cached rect returned when element is stable (not animating)
- Scroll handlers use cached values - no layout thrashing
- ResizeObserver properly invalidates cache before update
- Animation state tracking allows bypass during active animations

## Selector Types Modified

The following selector patterns were all updated to use purity caching:

### 1. Regular Selectors
```
&header .header;
```
- Uses the full CSS selector for cache key

### 2. Self Reference (&self)
```
.card {
    &card &self;
}
```
- Uses the scope selector as cache key

### 3. Parent Reference (&parent)
```
.child {
    &container &parent;
}
```
- Uses the scope selector as cache key

## Testing

Integration tests were added in `tests/integration/purity_codegen.rs`:

| Test | Purpose |
|------|---------|
| `test_element_ref_generates_purity_cached_rect` | Verifies `ST._purity.createCachedRect` is generated |
| `test_element_ref_uses_cached_accessor` | Verifies `getRect_*()` accessor is used |
| `test_resize_observer_invalidates_cache` | Verifies `ST._purity.invalidate()` is called |
| `test_multiple_element_refs_independent_caching` | Verifies each ref has its own accessor |
| `test_self_reference_purity_caching` | Verifies &self uses purity caching |
| `test_parent_reference_purity_caching` | Verifies &parent uses purity caching |
| `test_scroll_listener_uses_cached_accessor` | Verifies scroll events benefit from caching |

## Runtime Requirements

For the generated code to work, the runtime must include `public/runtime/purity.js` which provides:

- `ST._purity.frame` - Cache invalidation counter
- `ST._purity.animating` - Set of animating selectors
- `ST._purity.startAnimation(selector)` - Mark selector as animating
- `ST._purity.endAnimation(selector)` - Mark selector as stable, invalidate cache
- `ST._purity.isStable(selector)` - Check if selector is stable
- `ST._purity.invalidate()` - Increment frame counter to bust all caches
- `ST._purity.createCachedRect(el, selector)` - Create cached rect accessor

## Performance Impact

### Expected Improvements

1. **Scroll Performance**
   - Scroll handlers now return cached rects
   - No layout thrashing during rapid scroll
   - Single `getBoundingClientRect()` per invalidation cycle

2. **Multiple References**
   - If multiple elements reference the same source
   - Source rect is cached after first access
   - Subsequent accesses are cache hits

3. **Animation Awareness**
   - During animation: Fresh rects (correctness)
   - After animation: Cached rects (performance)

### Metrics to Monitor

- Cache hit ratio during scroll
- Time spent in `getBoundingClientRect()`
- Frame rate during heavy scroll with element refs

## Next Steps (Phase 4+)

1. **Animation Integration** - Modify animation codegen to call `startAnimation`/`endAnimation`
2. **Debug Tooling** - Add cache statistics to dev tools
3. **Batch Invalidation** - Coalesce multiple resize events
