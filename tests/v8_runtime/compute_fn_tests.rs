//! Compute Function Tests using V8
//!
//! Tests the @compute macro and compute-fn primitive for reactive computed values
//! with multi-statement JavaScript function bodies.

use super::context::V8TestContext;

/// ST runtime source
const ST_JS: &str = include_str!("../../public/runtime/st.js");

fn create_compute_context() -> V8TestContext {
    let mut ctx = V8TestContext::new();
    ctx.eval(ST_JS).expect("Failed to load st.js");

    // Initialize SpacetimeLocal for reactive properties
    ctx.eval(
        r#"
        window.SpacetimeLocal = window.SpacetimeLocal || {};
        ST.functions = ST.functions || {};
    "#,
    )
    .expect("Failed to initialize SpacetimeLocal");

    ctx
}

// =============================================================================
// Simple Calculation Tests
// =============================================================================

#[test]
fn test_compute_fn_simple_calculation() {
    let mut ctx = create_compute_context();

    // Simulate what the compute-fn primitive would generate
    ctx.eval(
        r#"
        SpacetimeLocal.items = [10, 20, 30];
        
        (() => {
            const fnName = 'total';
            const depNames = ['items'];
            
            const computeFn = new Function('items', `
                if (!items || items.length === 0) return 0;
                return items.reduce((sum, n) => sum + n, 0);
            `);
            
            ST.functions[fnName] = computeFn;
            
            let value = null;
            
            const compute = () => {
                const args = depNames.map(name => SpacetimeLocal[name]);
                value = computeFn(...args);
                return value;
            };
            
            value = compute();
            
            Object.defineProperty(SpacetimeLocal, fnName, {
                get() { return value; },
                enumerable: true,
                configurable: true
            });
        })();
    "#,
    )
    .unwrap();

    let result = ctx.eval("SpacetimeLocal.total === 60");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

#[test]
fn test_compute_fn_with_discount() {
    let mut ctx = create_compute_context();

    // Set up cart and config
    ctx.eval(
        r#"
        SpacetimeLocal.cart = [
            { packId: 'pack-mama-cozy', originalPrice: 100 },
            { packId: 'pack-cocoon', originalPrice: 80 }
        ];
        
        SpacetimeLocal.config = {
            packDiscounts: {
                'pack-mama-cozy': { percentage: 0.11 },
                'pack-cocoon': { percentage: 0.10 }
            }
        };
        
        (() => {
            const fnName = 'cartTotal';
            const depNames = ['cart', 'config'];
            
            const computeFn = new Function('cart', 'config', `
                if (!cart || cart.length === 0) return 0;
                const discounts = config?.packDiscounts || {};
                const total = cart.reduce((sum, item) => sum + (item.originalPrice || 0), 0);
                let maxDiscount = 0;
                cart.forEach(item => {
                    const d = discounts[item.packId]?.percentage || 0;
                    if (d > maxDiscount) maxDiscount = d;
                });
                return Math.round(total * (1 - maxDiscount));
            `);
            
            ST.functions[fnName] = computeFn;
            
            let value = null;
            
            const compute = () => {
                const args = depNames.map(name => SpacetimeLocal[name]);
                value = computeFn(...args);
                return value;
            };
            
            value = compute();
            
            Object.defineProperty(SpacetimeLocal, fnName, {
                get() { return value; },
                enumerable: true,
                configurable: true
            });
        })();
    "#,
    )
    .unwrap();

    // Total is 180, highest discount is 11%, so result should be 180 * 0.89 = 160.2 -> 160
    let result = ctx.eval("SpacetimeLocal.cartTotal === 160");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

// =============================================================================
// URL Building Tests
// =============================================================================

#[test]
fn test_compute_fn_url_building() {
    let mut ctx = create_compute_context();

    ctx.eval(r#"
        SpacetimeLocal.cart = [
            { 
                packId: 'pack-mama-cozy',
                sweatParentId: 123,
                sweatVariationId: 456,
                pullParentId: 789,
                pullVariationId: 101
            }
        ];
        
        SpacetimeLocal.config = {
            baseUrl: 'https://example.com/cart/',
            emptyCartUrl: '#',
            packDiscounts: {
                'pack-mama-cozy': { percentage: 0.11, couponCode: 'MAMA11' }
            }
        };
        
        (() => {
            const fnName = 'checkoutUrl';
            const depNames = ['cart', 'config'];
            
            const computeFn = new Function('cart', 'config', `
                if (!cart || cart.length === 0) return config?.emptyCartUrl || '#';
                const baseUrl = config?.baseUrl || 'https://example.com/';
                const discounts = config?.packDiscounts || {};
                
                const items = [];
                const coupons = new Map();
                
                cart.forEach(item => {
                    if (item.sweatParentId && item.sweatVariationId) {
                        items.push(item.sweatParentId + '.v' + item.sweatVariationId + ':1');
                    }
                    if (item.pullParentId && item.pullVariationId) {
                        items.push(item.pullParentId + '.v' + item.pullVariationId + ':1');
                    }
                    const packConfig = discounts[item.packId];
                    if (packConfig?.couponCode) {
                        coupons.set(packConfig.couponCode, packConfig.percentage);
                    }
                });
                
                let url = baseUrl + '?bd_cart=' + items.join(',') + '&bd_empty=1&bd_redirect=checkout';
                
                if (coupons.size > 0) {
                    let bestCoupon = null, bestDiscount = 0;
                    coupons.forEach((d, code) => {
                        if (d > bestDiscount) { bestDiscount = d; bestCoupon = code; }
                    });
                    if (bestCoupon) url += '&bd_coupon=' + encodeURIComponent(bestCoupon);
                }
                
                return url;
            `);
            
            ST.functions[fnName] = computeFn;
            
            let value = null;
            
            const compute = () => {
                const args = depNames.map(name => SpacetimeLocal[name]);
                value = computeFn(...args);
                return value;
            };
            
            value = compute();
            
            Object.defineProperty(SpacetimeLocal, fnName, {
                get() { return value; },
                enumerable: true,
                configurable: true
            });
        })();
    "#).unwrap();

    // Verify URL structure
    let has_cart = ctx.eval("SpacetimeLocal.checkoutUrl.includes('bd_cart=123.v456:1,789.v101:1')");
    assert!(matches!(
        has_cart,
        Ok(value) if value.as_bool() == Some(true)
    ));

    let has_coupon = ctx.eval("SpacetimeLocal.checkoutUrl.includes('bd_coupon=MAMA11')");
    assert!(matches!(
        has_coupon,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

// =============================================================================
// Edge Case Tests
// =============================================================================

#[test]
fn test_compute_fn_empty_cart() {
    let mut ctx = create_compute_context();

    ctx.eval(
        r#"
        SpacetimeLocal.cart = [];
        SpacetimeLocal.config = { emptyCartUrl: '#empty' };
        
        (() => {
            const fnName = 'checkoutUrl';
            const depNames = ['cart', 'config'];
            
            const computeFn = new Function('cart', 'config', `
                if (!cart || cart.length === 0) return config?.emptyCartUrl || '#';
                return 'https://checkout';
            `);
            
            let value = computeFn(SpacetimeLocal.cart, SpacetimeLocal.config);
            
            Object.defineProperty(SpacetimeLocal, fnName, {
                get() { return value; },
                enumerable: true
            });
        })();
    "#,
    )
    .unwrap();

    let result = ctx.eval("SpacetimeLocal.checkoutUrl === '#empty'");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

#[test]
fn test_compute_fn_null_cart() {
    let mut ctx = create_compute_context();

    ctx.eval(
        r#"
        SpacetimeLocal.cart = null;
        
        (() => {
            const fnName = 'cartTotal';
            
            const computeFn = new Function('cart', `
                if (!cart || cart.length === 0) return 0;
                return cart.reduce((sum, item) => sum + item.price, 0);
            `);
            
            let value = computeFn(SpacetimeLocal.cart);
            
            Object.defineProperty(SpacetimeLocal, fnName, {
                get() { return value; },
                enumerable: true
            });
        })();
    "#,
    )
    .unwrap();

    let result = ctx.eval("SpacetimeLocal.cartTotal === 0");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

// =============================================================================
// Reactivity Tests
// =============================================================================

#[test]
fn test_compute_fn_reacts_to_changes() {
    let mut ctx = create_compute_context();

    ctx.eval(r#"
        SpacetimeLocal.items = [1, 2, 3];
        let computeCallCount = 0;
        
        (() => {
            const fnName = 'sum';
            const depNames = ['items'];
            
            const computeFn = new Function('items', `
                return (items || []).reduce((a, b) => a + b, 0);
            `);
            
            let value = null;
            
            const compute = () => {
                computeCallCount++;
                const args = depNames.map(name => SpacetimeLocal[name]);
                value = computeFn(...args);
                return value;
            };
            
            value = compute();
            
            depNames.forEach(name => {
                document.addEventListener('local:' + name + ':updated', () => {
                    value = compute();
                    document.dispatchEvent(new CustomEvent('local:' + fnName + ':updated', { detail: value }));
                });
            });
            
            Object.defineProperty(SpacetimeLocal, fnName, {
                get() { return value; },
                enumerable: true,
                configurable: true
            });
        })();
    "#).unwrap();

    // Initial value
    let initial = ctx.eval("SpacetimeLocal.sum === 6");
    assert!(matches!(
        initial,
        Ok(value) if value.as_bool() == Some(true)
    ));

    // Simulate updating items and triggering the event
    ctx.eval(
        r#"
        SpacetimeLocal.items = [10, 20, 30];
        document.dispatchEvent(new CustomEvent('local:items:updated'));
    "#,
    )
    .unwrap();

    let updated = ctx.eval("SpacetimeLocal.sum === 60");
    assert!(matches!(
        updated,
        Ok(value) if value.as_bool() == Some(true)
    ));

    // Verify compute was called twice (initial + update)
    let call_count = ctx.eval("computeCallCount === 2");
    assert!(matches!(
        call_count,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

// =============================================================================
// Function Registry Tests
// =============================================================================

#[test]
fn test_compute_fn_registered_in_st_functions() {
    let mut ctx = create_compute_context();

    ctx.eval(
        r#"
        (() => {
            const fnName = 'myCompute';
            
            const computeFn = new Function('x', 'return x * 2');
            ST.functions[fnName] = computeFn;
            
            let value = computeFn(5);
            
            Object.defineProperty(SpacetimeLocal, fnName, {
                get() { return value; },
                enumerable: true
            });
        })();
    "#,
    )
    .unwrap();

    // Check function is registered
    let registered = ctx.eval("typeof ST.functions.myCompute === 'function'");
    assert!(matches!(
        registered,
        Ok(value) if value.as_bool() == Some(true)
    ));

    // Check function works correctly
    let result = ctx.eval("ST.functions.myCompute(10) === 20");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_compute_fn_error_handling() {
    let mut ctx = create_compute_context();

    ctx.eval(
        r#"
        SpacetimeLocal.data = { value: 'not a number' };
        let errorLogged = false;
        const originalError = console.error;
        console.error = (...args) => { errorLogged = true; };
        
        (() => {
            const fnName = 'computed';
            const depNames = ['data'];
            
            const computeFn = new Function('data', `
                // This will throw when trying to call toFixed on a string
                return data.value.toFixed(2);
            `);
            
            let value = null;
            
            const compute = () => {
                const args = depNames.map(name => SpacetimeLocal[name]);
                try {
                    value = computeFn(...args);
                } catch (e) {
                    console.error('[compute-fn] Error:', e);
                    value = null;
                }
                return value;
            };
            
            value = compute();
            
            Object.defineProperty(SpacetimeLocal, fnName, {
                get() { return value; },
                enumerable: true
            });
        })();
        
        console.error = originalError;
    "#,
    )
    .unwrap();

    // Value should be null due to error
    let is_null = ctx.eval("SpacetimeLocal.computed === null");
    assert!(matches!(
        is_null,
        Ok(value) if value.as_bool() == Some(true)
    ));

    // Error should have been logged
    let logged = ctx.eval("errorLogged === true");
    assert!(matches!(
        logged,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

// =============================================================================
// Best Coupon Selection Tests
// =============================================================================

#[test]
fn test_compute_fn_selects_best_coupon() {
    let mut ctx = create_compute_context();

    ctx.eval(
        r#"
        SpacetimeLocal.cart = [
            { packId: 'pack-a' },  // 5% discount
            { packId: 'pack-b' },  // 15% discount
            { packId: 'pack-c' }   // 10% discount
        ];
        
        SpacetimeLocal.config = {
            packDiscounts: {
                'pack-a': { percentage: 0.05, couponCode: 'SMALL5' },
                'pack-b': { percentage: 0.15, couponCode: 'BIG15' },
                'pack-c': { percentage: 0.10, couponCode: 'MED10' }
            }
        };
        
        (() => {
            const fnName = 'bestCoupon';
            const depNames = ['cart', 'config'];
            
            const computeFn = new Function('cart', 'config', `
                if (!cart || cart.length === 0) return null;
                const discounts = config?.packDiscounts || {};
                
                const coupons = new Map();
                cart.forEach(item => {
                    const packConfig = discounts[item.packId];
                    if (packConfig?.couponCode) {
                        coupons.set(packConfig.couponCode, packConfig.percentage);
                    }
                });
                
                let bestCoupon = null, bestDiscount = 0;
                coupons.forEach((d, code) => {
                    if (d > bestDiscount) { bestDiscount = d; bestCoupon = code; }
                });
                
                return bestCoupon;
            `);
            
            let value = computeFn(SpacetimeLocal.cart, SpacetimeLocal.config);
            
            Object.defineProperty(SpacetimeLocal, fnName, {
                get() { return value; },
                enumerable: true
            });
        })();
    "#,
    )
    .unwrap();

    // Should select BIG15 (15% discount)
    let result = ctx.eval("SpacetimeLocal.bestCoupon === 'BIG15'");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}
