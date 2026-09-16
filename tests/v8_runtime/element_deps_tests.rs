//! Element Dependency Runtime Tests using V8
//!
//! Tests the ST._purity dependency-aware methods for targeted invalidation.
//! These tests verify the Phase 4 purity enhancements that use ElementDepGraph
//! for compile-time dependency analysis.

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

// =============================================================================
// setDependents Tests
// =============================================================================

#[test]
fn test_purity_dependents_property_exists() {
    let mut ctx = create_purity_context();

    let result = ctx.eval("typeof ST._purity.dependents === 'object'");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

#[test]
fn test_purity_dependents_starts_empty() {
    let mut ctx = create_purity_context();

    let result = ctx.eval("Object.keys(ST._purity.dependents).length === 0");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

#[test]
fn test_set_dependents_works() {
    let mut ctx = create_purity_context();

    ctx.eval(
        r#"
        ST._purity.setDependents({
            '.header': ['.content', '.footer'],
            '.sidebar': ['.main-panel']
        });
        "#,
    )
    .unwrap();

    // Check that dependents were set
    let result = ctx.eval("ST._purity.dependents['.header'].length === 2");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "Should have 2 dependents for .header"
    );

    let result = ctx.eval("ST._purity.dependents['.sidebar'].length === 1");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "Should have 1 dependent for .sidebar"
    );

    // Check specific values
    let result = ctx.eval("ST._purity.dependents['.header'].includes('.content')");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        ".header should have .content as dependent"
    );
}

// =============================================================================
// getAffectedSelectors Tests
// =============================================================================

#[test]
fn test_get_affected_selectors_exists() {
    let mut ctx = create_purity_context();

    let result = ctx.eval("typeof ST._purity.getAffectedSelectors === 'function'");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

#[test]
fn test_get_affected_selectors_no_deps() {
    let mut ctx = create_purity_context();

    // With no dependents set, should return just the selector itself
    ctx.eval("const affected = ST._purity.getAffectedSelectors('.isolated')")
        .unwrap();

    let result = ctx.eval("affected.length === 1");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "Should return array with just the input selector"
    );

    let result = ctx.eval("affected[0] === '.isolated'");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "Should contain the input selector"
    );
}

#[test]
fn test_get_affected_selectors_direct_deps() {
    let mut ctx = create_purity_context();

    ctx.eval(
        r#"
        ST._purity.setDependents({
            '.header': ['.content', '.footer']
        });
        const affected = ST._purity.getAffectedSelectors('.header');
        "#,
    )
    .unwrap();

    let result = ctx.eval("affected.length === 3");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "Should include .header and its 2 dependents"
    );

    let result = ctx.eval("affected.includes('.header')");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "Should include the input selector"
    );

    let result = ctx.eval("affected.includes('.content')");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "Should include .content as a dependent"
    );

    let result = ctx.eval("affected.includes('.footer')");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "Should include .footer as a dependent"
    );
}

#[test]
fn test_get_affected_selectors_transitive_deps() {
    let mut ctx = create_purity_context();

    // A -> B -> C (transitive chain)
    ctx.eval(
        r#"
        ST._purity.setDependents({
            '.a': ['.b'],
            '.b': ['.c']
        });
        const affected = ST._purity.getAffectedSelectors('.a');
        "#,
    )
    .unwrap();

    let result = ctx.eval("affected.length === 3");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "Should follow transitive dependencies: .a -> .b -> .c"
    );

    let result =
        ctx.eval("affected.includes('.a') && affected.includes('.b') && affected.includes('.c')");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "Should include all selectors in the chain"
    );
}

#[test]
fn test_get_affected_selectors_handles_cycles() {
    let mut ctx = create_purity_context();

    // A -> B -> A (cycle)
    ctx.eval(
        r#"
        ST._purity.setDependents({
            '.a': ['.b'],
            '.b': ['.a']
        });
        const affected = ST._purity.getAffectedSelectors('.a');
        "#,
    )
    .unwrap();

    // Should not infinite loop, should contain both selectors exactly once
    let result = ctx.eval("affected.length === 2");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "Should handle cycles without infinite loop"
    );

    let result = ctx.eval("affected.includes('.a') && affected.includes('.b')");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "Should include both selectors in the cycle"
    );
}

// =============================================================================
// startAnimationWithDeps Tests
// =============================================================================

#[test]
fn test_start_animation_with_deps_exists() {
    let mut ctx = create_purity_context();

    let result = ctx.eval("typeof ST._purity.startAnimationWithDeps === 'function'");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

#[test]
fn test_start_animation_with_deps_marks_all_affected() {
    let mut ctx = create_purity_context();

    ctx.eval(
        r#"
        ST._purity.setDependents({
            '.slider': ['.label', '.track']
        });
        const affected = ST._purity.startAnimationWithDeps('.slider');
        "#,
    )
    .unwrap();

    // All three should now be marked as animating
    let result = ctx.eval("ST._purity.animating.has('.slider')");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        ".slider should be marked as animating"
    );

    let result = ctx.eval("ST._purity.animating.has('.label')");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        ".label (dependent) should be marked as animating"
    );

    let result = ctx.eval("ST._purity.animating.has('.track')");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        ".track (dependent) should be marked as animating"
    );

    // Should return the affected array
    let result = ctx.eval("affected.length === 3");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "Should return array of affected selectors"
    );
}

#[test]
fn test_start_animation_with_deps_returns_affected_for_end() {
    let mut ctx = create_purity_context();

    ctx.eval(
        r#"
        ST._purity.setDependents({
            '.panel': ['.content']
        });
        const affected = ST._purity.startAnimationWithDeps('.panel');
        "#,
    )
    .unwrap();

    // Verify we can use the returned array with endAnimationWithDeps
    let result = ctx.eval("Array.isArray(affected)");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "Should return an array"
    );

    let result = ctx.eval("affected.includes('.panel') && affected.includes('.content')");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "Returned array should include all affected selectors"
    );
}

// =============================================================================
// endAnimationWithDeps Tests
// =============================================================================

#[test]
fn test_end_animation_with_deps_exists() {
    let mut ctx = create_purity_context();

    let result = ctx.eval("typeof ST._purity.endAnimationWithDeps === 'function'");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

#[test]
fn test_end_animation_with_deps_clears_all_affected() {
    let mut ctx = create_purity_context();

    ctx.eval(
        r#"
        ST._purity.setDependents({
            '.slider': ['.label', '.track']
        });
        const affected = ST._purity.startAnimationWithDeps('.slider');
        "#,
    )
    .unwrap();

    // All should be animating now
    let result = ctx.eval("ST._purity.animating.size === 3");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "All three should be animating before end"
    );

    // End the animation
    ctx.eval("ST._purity.endAnimationWithDeps('.slider', affected)")
        .unwrap();

    // All should be cleared now
    let result = ctx.eval("ST._purity.animating.size === 0");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "All selectors should be cleared after end"
    );
}

#[test]
fn test_end_animation_with_deps_increments_frame() {
    let mut ctx = create_purity_context();

    ctx.eval(
        r#"
        ST._purity.setDependents({
            '.box': ['.shadow']
        });
        const initialFrame = ST._purity.frame;
        const affected = ST._purity.startAnimationWithDeps('.box');
        ST._purity.endAnimationWithDeps('.box', affected);
        "#,
    )
    .unwrap();

    let result = ctx.eval("ST._purity.frame === initialFrame + 1");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "Frame should be incremented after endAnimationWithDeps"
    );
}

// =============================================================================
// Integration Tests
// =============================================================================

#[test]
fn test_full_animation_lifecycle_with_deps() {
    let mut ctx = create_purity_context();

    ctx.eval(
        r#"
        // Set up dependency graph
        ST._purity.setDependents({
            '.header': ['.content'],
            '.content': ['.footer']
        });

        // Initial state: all stable
        const allStableBefore =
            ST._purity.isStable('.header') &&
            ST._purity.isStable('.content') &&
            ST._purity.isStable('.footer');

        // Start animation on header
        const affected = ST._purity.startAnimationWithDeps('.header');

        // During animation: all affected should be animating
        const allAnimating =
            !ST._purity.isStable('.header') &&
            !ST._purity.isStable('.content') &&
            !ST._purity.isStable('.footer');

        // End animation
        ST._purity.endAnimationWithDeps('.header', affected);

        // After animation: all stable again
        const allStableAfter =
            ST._purity.isStable('.header') &&
            ST._purity.isStable('.content') &&
            ST._purity.isStable('.footer');
        "#,
    )
    .unwrap();

    let result = ctx.eval("allStableBefore");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "All should be stable before animation"
    );

    let result = ctx.eval("allAnimating");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "All should be animating during animation"
    );

    let result = ctx.eval("allStableAfter");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "All should be stable after animation"
    );
}

#[test]
fn test_deps_with_cached_rect() {
    let mut ctx = create_purity_context();

    ctx.eval(
        r#"
        // Set up dependencies
        ST._purity.setDependents({
            '.slider': ['.label']
        });

        // Create mock elements with call tracking
        let sliderCalls = 0;
        let labelCalls = 0;

        const mockSlider = {
            getBoundingClientRect: function() {
                sliderCalls++;
                return { x: 0, y: 0, width: 100 + sliderCalls, height: 50 };
            }
        };

        const mockLabel = {
            getBoundingClientRect: function() {
                labelCalls++;
                return { x: 100, y: 0, width: 50, height: 20 };
            }
        };

        const getSliderRect = ST._purity.createCachedRect(mockSlider, '.slider');
        const getLabelRect = ST._purity.createCachedRect(mockLabel, '.label');

        // Initial calls - both should hit the real function
        getSliderRect();
        getLabelRect();
        const initialCalls = sliderCalls === 1 && labelCalls === 1;

        // Cached calls - neither should increment
        getSliderRect();
        getLabelRect();
        const cachedCalls = sliderCalls === 1 && labelCalls === 1;

        // Start animation with deps - both become animating
        const affected = ST._purity.startAnimationWithDeps('.slider');

        // Calls during animation - both should recalculate
        getSliderRect();
        getLabelRect();
        const animatingCalls = sliderCalls === 2 && labelCalls === 2;

        // Multiple calls during animation
        getSliderRect();
        getLabelRect();
        const moreAnimatingCalls = sliderCalls === 3 && labelCalls === 3;

        // End animation
        ST._purity.endAnimationWithDeps('.slider', affected);

        // Next call should recalculate (frame was incremented)
        getSliderRect();
        getLabelRect();
        const postAnimCalls = sliderCalls === 4 && labelCalls === 4;

        // Subsequent calls should be cached again
        getSliderRect();
        getLabelRect();
        const reCachedCalls = sliderCalls === 4 && labelCalls === 4;
        "#,
    )
    .unwrap();

    let result = ctx.eval("initialCalls");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "Initial calls should hit the real function"
    );

    let result = ctx.eval("cachedCalls");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "Subsequent calls should use cache"
    );

    let result = ctx.eval("animatingCalls");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "Calls during animation should recalculate"
    );

    let result = ctx.eval("moreAnimatingCalls");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "Multiple calls during animation should each recalculate"
    );

    let result = ctx.eval("postAnimCalls");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "First call after animation should recalculate (frame changed)"
    );

    let result = ctx.eval("reCachedCalls");
    assert!(
        matches!(result, Ok(value) if value.as_bool() == Some(true)),
        "Subsequent calls after animation should be cached again"
    );
}
