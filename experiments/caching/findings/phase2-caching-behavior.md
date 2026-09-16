# Phase 2: Caching Behavior Findings

## Overview

This document describes the purity caching system implemented in Phase 2, which provides intelligent caching for element rect calculations in Spacetime animations.

## How the Caching Works

### Core Concept: Purity Tracking

The purity system tracks whether a selector is "stable" (not currently animating). When a selector is stable, its computed geometry values (like `getBoundingClientRect()`) can be safely cached because they won't change.

### Key Components

1. **Frame Counter (`ST._purity.frame`)**
   - Incremented on window resize or explicit invalidation
   - Used to bust all caches when layout changes globally

2. **Animating Set (`ST._purity.animating`)**
   - Set of CSS selectors currently being animated
   - Elements in this set are "impure" - their geometry is changing

3. **Cached Rect Accessor (`ST._purity.createCachedRect`)**
   - Creates a closure that caches `getBoundingClientRect()` results
   - Returns cached value when selector is stable and frame hasn't changed
   - Recalculates when animating or after invalidation

### Cache Decision Logic

```javascript
function getCachedRect() {
  // Always recalculate if element is animating
  if (purity.animating.has(selector)) {
    return el.getBoundingClientRect();
  }

  // Recalculate if frame changed (e.g., window resize)
  if (cache.frame !== purity.frame) {
    cache.rect = el.getBoundingClientRect();
    cache.frame = purity.frame;
  }

  // Return cached value
  return cache.rect;
}
```

## Scenarios That Trigger Recalculation

### 1. Element is Animating

When `ST._purity.startAnimation(selector)` is called:
- The selector is added to the `animating` set
- All subsequent rect queries bypass the cache
- Every call to `getCachedRect()` invokes `getBoundingClientRect()`

When `ST._purity.endAnimation(selector)` is called:
- The selector is removed from the `animating` set
- The frame counter is incremented (invalidating all caches)
- This ensures the next rect query gets the element's final position

This ensures animations always have fresh geometry data during animation,
and other elements referencing the animated element get updated values
after the animation completes.

### 2. Window Resize

When the window is resized:
- `ST._purity.invalidate()` is called automatically
- The frame counter increments
- Next rect query for any selector will recalculate
- New values are cached until next invalidation

### 3. Explicit Invalidation

Code can manually call `ST._purity.invalidate()` to:
- Bust all caches when layout changes programmatically
- Force recalculation after dynamic content changes
- Sync caches after third-party DOM modifications

## Cache Usage Scenarios

### Scenario A: Static Reference

```
// .box is not animating
width: 0 -> &box.rect.width;
```

- First access: `getBoundingClientRect()` called, result cached
- Subsequent accesses: Return cached value (no DOM query)
- On resize: Cache invalidated, fresh value on next access

### Scenario B: Animating Target

```
// .box is currently animating
@on hover grow-box(300ms) {
  width: 100px -> 200px;
}
```

- While animating: Every rect query bypasses cache
- When animation ends: `endAnimation()` increments frame counter
- Next query: Recalculates with final position, caches result
- Subsequent stable queries: Return cached value

### Scenario C: Cross-Reference During Animation

```
// .target references .box while .box animates
width: 50px -> &box.rect.width;
```

- If .box is animating: .target gets fresh rect each frame
- If .box is stable: .target uses cached rect
- Independence: .target's animation state doesn't affect .box's cache

## Performance Implications

### Positive Impacts

1. **Reduced Layout Thrashing**
   - Stable elements avoid repeated `getBoundingClientRect()` calls
   - Single measurement per invalidation cycle
   - Particularly beneficial for multiple references to same element

2. **Animation Performance**
   - No stale data during animations
   - Clean invalidation when animation ends
   - Independent caching per selector

3. **Resize Handling**
   - Automatic cache bust on resize
   - Fresh measurements after layout reflow
   - No manual cache management needed

### Considerations

1. **Memory Usage**
   - Each cached rect accessor holds a closure
   - Cache object per element reference
   - Minimal impact (rect objects are small)

2. **Correctness vs Performance Trade-off**
   - Animating elements always pay `getBoundingClientRect()` cost
   - This is necessary for correct animation behavior
   - Stable elements benefit from caching

3. **Edge Cases**
   - Third-party DOM changes may need manual `invalidate()`
   - CSS transitions without Spacetime animation won't auto-track
   - Consider explicit invalidation for dynamic layouts

## Integration with Element References

The caching system integrates with Spacetime's element reference syntax:

```
&box .box;
```

When compiled, this generates:
```javascript
const boxRect = ST._purity.createCachedRect(
  document.querySelector('.box'),
  '.box'
);
```

Animation blocks automatically call `startAnimation`/`endAnimation`:
```javascript
// Animation start
ST._purity.startAnimation('.box');

// ... animation runs ...

// Animation end
ST._purity.endAnimation('.box');
```

## Testing

The caching behavior is verified by V8 tests in `tests/v8_runtime/purity_tests.rs`:

- `test_create_cached_rect_returns_function` - Verifies accessor creation
- `test_cached_rect_returns_same_object_when_stable` - Confirms caching works
- `test_cached_rect_recalculates_when_invalidated` - Confirms frame-based invalidation
- `test_cached_rect_recalculates_when_animating` - Confirms animation bypass
- `test_cached_rect_different_selectors_independent` - Confirms isolation

## Future Considerations

1. **Batch Invalidation**
   - Could coalesce multiple rapid invalidations
   - Useful for resize debouncing

2. **Partial Invalidation**
   - Invalidate specific selectors instead of global frame bump
   - More granular cache control

3. **Derived Value Caching**
   - Cache computed values like `width + 20px`
   - Currently only raw rect is cached

4. **Debug Tooling**
   - Cache hit/miss statistics
   - Visualization of animating selectors
   - Performance profiling integration
