//! Purity Caching System Tests using V8
//!
//! Tests the ST._purity runtime API for element reference caching.

use super::context::V8TestContext;

/// Purity.js runtime source
const PURITY_JS: &str = include_str!("../../public/runtime/purity.js");

fn create_purity_context() -> V8TestContext {
    let mut ctx = V8TestContext::new();
    // Purity requires ST to exist, so load st.js first
    let st_js = include_str!("../../public/runtime/st.js");
    ctx.eval(st_js).expect("Failed to load st.js");
    ctx.eval(PURITY_JS).expect("Failed to load purity.js");
    ctx
}

#[test]
fn test_purity_exists() {
    let mut ctx = create_purity_context();

    let result = ctx.eval("typeof ST._purity !== 'undefined'");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

#[test]
fn test_purity_is_object() {
    let mut ctx = create_purity_context();

    let result = ctx.eval("typeof ST._purity === 'object'");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

#[test]
fn test_frame_starts_at_zero() {
    let mut ctx = create_purity_context();

    let result = ctx.eval("ST._purity.frame === 0");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

#[test]
fn test_invalidate_increments_frame() {
    let mut ctx = create_purity_context();

    ctx.eval("ST._purity.invalidate()").unwrap();
    let result = ctx.eval("ST._purity.frame === 1");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));

    ctx.eval("ST._purity.invalidate()").unwrap();
    let result = ctx.eval("ST._purity.frame === 2");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

#[test]
fn test_animating_set_exists() {
    let mut ctx = create_purity_context();

    let result = ctx.eval("ST._purity.animating instanceof Set");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

#[test]
fn test_animating_set_starts_empty() {
    let mut ctx = create_purity_context();

    let result = ctx.eval("ST._purity.animating.size === 0");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

#[test]
fn test_start_animation_adds_selector() {
    let mut ctx = create_purity_context();

    ctx.eval("ST._purity.startAnimation('.my-element')")
        .unwrap();

    let result = ctx.eval("ST._purity.animating.has('.my-element')");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

#[test]
fn test_end_animation_removes_selector() {
    let mut ctx = create_purity_context();

    ctx.eval("ST._purity.startAnimation('.my-element')")
        .unwrap();
    ctx.eval("ST._purity.endAnimation('.my-element')").unwrap();

    let result = ctx.eval("ST._purity.animating.has('.my-element')");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(false)
    ));
}

#[test]
fn test_is_stable_returns_true_for_non_animating() {
    let mut ctx = create_purity_context();

    let result = ctx.eval("ST._purity.isStable('.static-element')");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

#[test]
fn test_is_stable_returns_false_for_animating() {
    let mut ctx = create_purity_context();

    ctx.eval("ST._purity.startAnimation('.animating-element')")
        .unwrap();

    let result = ctx.eval("ST._purity.isStable('.animating-element')");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(false)
    ));
}

#[test]
fn test_multiple_selectors_can_animate() {
    let mut ctx = create_purity_context();

    ctx.eval(
        r#"
        ST._purity.startAnimation('.element-1');
        ST._purity.startAnimation('.element-2');
        ST._purity.startAnimation('.element-3');
        "#,
    )
    .unwrap();

    let result = ctx.eval("ST._purity.animating.size === 3");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));

    // All three should be not stable
    let result = ctx.eval(
        "!ST._purity.isStable('.element-1') && !ST._purity.isStable('.element-2') && !ST._purity.isStable('.element-3')",
    );
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

#[test]
fn test_ending_one_animation_leaves_others() {
    let mut ctx = create_purity_context();

    ctx.eval(
        r#"
        ST._purity.startAnimation('.element-1');
        ST._purity.startAnimation('.element-2');
        ST._purity.endAnimation('.element-1');
        "#,
    )
    .unwrap();

    // element-1 should now be stable
    let result = ctx.eval("ST._purity.isStable('.element-1')");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));

    // element-2 should still be animating
    let result = ctx.eval("ST._purity.isStable('.element-2')");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(false)
    ));
}

#[test]
fn test_animation_lifecycle() {
    let mut ctx = create_purity_context();

    // Before animation: stable
    let result = ctx.eval("ST._purity.isStable('.name-line')");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));

    // During animation: not stable
    ctx.eval("ST._purity.startAnimation('.name-line')").unwrap();
    let result = ctx.eval("ST._purity.isStable('.name-line')");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(false)
    ));

    // After animation: stable again
    ctx.eval("ST._purity.endAnimation('.name-line')").unwrap();
    let result = ctx.eval("ST._purity.isStable('.name-line')");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

// =============================================================================
// Phase 2: Cached Rect Tests
// =============================================================================

#[test]
fn test_create_cached_rect_returns_function() {
    let mut ctx = create_purity_context();

    // Create a mock element with getBoundingClientRect
    ctx.eval(
        r#"
        const mockElement = {
            getBoundingClientRect: function() {
                return { x: 0, y: 0, width: 100, height: 50 };
            }
        };
        const getCachedRect = ST._purity.createCachedRect(mockElement, '.test-element');
        "#,
    )
    .unwrap();

    let result = ctx.eval("typeof getCachedRect === 'function'");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

#[test]
fn test_cached_rect_returns_same_object_when_stable() {
    let mut ctx = create_purity_context();

    // Create a mock element that tracks how many times getBoundingClientRect is called
    ctx.eval(
        r#"
        let callCount = 0;
        const mockElement = {
            getBoundingClientRect: function() {
                callCount++;
                return { x: 0, y: 0, width: 100, height: 50, callId: callCount };
            }
        };
        const getCachedRect = ST._purity.createCachedRect(mockElement, '.stable-element');
        "#,
    )
    .unwrap();

    // First call should invoke getBoundingClientRect
    ctx.eval("const rect1 = getCachedRect();").unwrap();
    let result = ctx.eval("callCount === 1");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "First call should invoke getBoundingClientRect"
    );

    // Second call should return cached value (same callId)
    ctx.eval("const rect2 = getCachedRect();").unwrap();
    let result = ctx.eval("callCount === 1");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "Second call should use cache, not invoke getBoundingClientRect"
    );

    // Both should have the same callId (proving same cached object)
    let result = ctx.eval("rect1.callId === rect2.callId");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "Cached rect should be the same object"
    );
}

#[test]
fn test_cached_rect_recalculates_when_invalidated() {
    let mut ctx = create_purity_context();

    // Create a mock element that tracks calls
    ctx.eval(
        r#"
        let callCount = 0;
        const mockElement = {
            getBoundingClientRect: function() {
                callCount++;
                return { x: 0, y: 0, width: 100, height: 50, callId: callCount };
            }
        };
        const getCachedRect = ST._purity.createCachedRect(mockElement, '.invalidate-test');
        "#,
    )
    .unwrap();

    // First call
    ctx.eval("const rect1 = getCachedRect();").unwrap();
    let result = ctx.eval("callCount === 1");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "First call should invoke getBoundingClientRect"
    );

    // Invalidate the cache (simulates window resize)
    ctx.eval("ST._purity.invalidate();").unwrap();

    // Next call should recalculate because frame changed
    ctx.eval("const rect2 = getCachedRect();").unwrap();
    let result = ctx.eval("callCount === 2");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "After invalidate, should invoke getBoundingClientRect again"
    );

    // callIds should be different
    let result = ctx.eval("rect1.callId !== rect2.callId");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "Rects should have different callIds after invalidation"
    );
}

#[test]
fn test_cached_rect_recalculates_when_animating() {
    let mut ctx = create_purity_context();

    // Create a mock element that tracks calls
    ctx.eval(
        r#"
        let callCount = 0;
        const mockElement = {
            getBoundingClientRect: function() {
                callCount++;
                return { x: 0, y: 0, width: 100, height: 50, callId: callCount };
            }
        };
        const getCachedRect = ST._purity.createCachedRect(mockElement, '.animating-box');
        "#,
    )
    .unwrap();

    // First call while stable - should cache
    ctx.eval("const rect1 = getCachedRect();").unwrap();
    let result = ctx.eval("callCount === 1");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "First call should invoke getBoundingClientRect"
    );

    // Start animating this selector
    ctx.eval("ST._purity.startAnimation('.animating-box');")
        .unwrap();

    // Now every call should recalculate (element is moving)
    ctx.eval("const rect2 = getCachedRect();").unwrap();
    let result = ctx.eval("callCount === 2");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "During animation, should recalculate"
    );

    ctx.eval("const rect3 = getCachedRect();").unwrap();
    let result = ctx.eval("callCount === 3");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "Each call during animation should recalculate"
    );

    // Stop animation - this invalidates caches since element position changed
    ctx.eval("ST._purity.endAnimation('.animating-box');")
        .unwrap();

    // After animation ends, frame was incremented by endAnimation
    // So the next call should recalculate to get the final position
    ctx.eval("const rect4 = getCachedRect();").unwrap();
    let result = ctx.eval("callCount === 4");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "After animation ends, should recalculate (frame was invalidated)"
    );

    // Subsequent calls should use the newly cached value
    ctx.eval("const rect5 = getCachedRect();").unwrap();
    let result = ctx.eval("callCount === 4");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "Subsequent calls should use cache"
    );
}

#[test]
fn test_cached_rect_different_selectors_independent() {
    let mut ctx = create_purity_context();

    // Create two mock elements with their own call counters
    ctx.eval(
        r#"
        let callCount1 = 0;
        let callCount2 = 0;

        const mockElement1 = {
            getBoundingClientRect: function() {
                callCount1++;
                return { width: 100, callId: callCount1 };
            }
        };

        const mockElement2 = {
            getBoundingClientRect: function() {
                callCount2++;
                return { width: 200, callId: callCount2 };
            }
        };

        const getCachedRect1 = ST._purity.createCachedRect(mockElement1, '.box-1');
        const getCachedRect2 = ST._purity.createCachedRect(mockElement2, '.box-2');
        "#,
    )
    .unwrap();

    // Cache both
    ctx.eval("getCachedRect1(); getCachedRect2();").unwrap();
    let result = ctx.eval("callCount1 === 1 && callCount2 === 1");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "Both should be called once"
    );

    // Start animating only box-1
    ctx.eval("ST._purity.startAnimation('.box-1');").unwrap();

    // Call both again
    ctx.eval("getCachedRect1(); getCachedRect2();").unwrap();

    // box-1 should recalculate (animating), box-2 should use cache
    let result = ctx.eval("callCount1 === 2 && callCount2 === 1");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "Only animating selector should recalculate"
    );
}
