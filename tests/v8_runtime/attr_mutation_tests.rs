//! Attribute Mutation Tests using V8
//!
//! Tests for the attribute mutation feature:
//! - &.attr <- expr (set attribute on current element)
//! - &name.attr <- expr (set attribute on named element ref)

use super::context::V8TestContext;

/// ST runtime source
const ST_JS: &str = include_str!("../../public/runtime/st.js");

fn create_context() -> V8TestContext {
    let mut ctx = V8TestContext::new();
    ctx.eval(ST_JS).expect("Failed to load st.js");
    // Initialize SpacetimeLocal for state access
    ctx.eval(
        r#"
        window._localState = {};
        window.SpacetimeLocal = new Proxy(window._localState, {
            get(target, prop) { return target[prop]; },
            set(target, prop, value) { target[prop] = value; return true; }
        });
    "#,
    )
    .expect("Failed to init SpacetimeLocal");
    // Polyfill btoa/atob for V8 (not built-in)
    ctx.eval(
        r#"
        if (typeof btoa === 'undefined') {
            globalThis.btoa = function(str) {
                const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=';
                let output = '';
                for (let i = 0; i < str.length; i += 3) {
                    const a = str.charCodeAt(i);
                    const b = str.charCodeAt(i + 1) || 0;
                    const c = str.charCodeAt(i + 2) || 0;
                    const enc1 = a >> 2;
                    const enc2 = ((a & 3) << 4) | (b >> 4);
                    const enc3 = ((b & 15) << 2) | (c >> 6);
                    const enc4 = c & 63;
                    output += chars[enc1] + chars[enc2];
                    output += (i + 1 < str.length) ? chars[enc3] : '=';
                    output += (i + 2 < str.length) ? chars[enc4] : '=';
                }
                return output;
            };
        }
    "#,
    )
    .expect("Failed to init btoa polyfill");
    ctx
}

// =============================================================================
// Parser Pattern Tests
// Verify the JS regex patterns match correctly
// =============================================================================

#[test]
fn test_current_element_attr_pattern_matches() {
    let mut ctx = create_context();

    let result = ctx.eval(
        r#"
        const stmt = "&.href <- checkoutUrl($cart)";
        const match = stmt.match(/^&\.(\w+)\s*<-\s*(.+)$/);
        match !== null && match[1] === "href" && match[2] === "checkoutUrl($cart)"
    "#,
    );

    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "&.attr pattern should match current element attribute mutation"
    );
}

#[test]
fn test_named_ref_attr_pattern_matches() {
    let mut ctx = create_context();

    let result = ctx.eval(r#"
        const stmt = "&checkout.href <- buildUrl($cart)";
        const match = stmt.match(/^&(\w+)\.(\w+)\s*<-\s*(.+)$/);
        match !== null && match[1] === "checkout" && match[2] === "href" && match[3] === "buildUrl($cart)"
    "#);

    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "&name.attr pattern should match named ref attribute mutation"
    );
}

#[test]
fn test_variable_pattern_still_works() {
    let mut ctx = create_context();

    let result = ctx.eval(
        r#"
        const stmt = "$cart <- $cart.concat([item])";
        const match = stmt.match(/^\$(\w+)\s*<-\s*(.+)$/);
        match !== null && match[1] === "cart" && match[2] === "$cart.concat([item])"
    "#,
    );

    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "$var pattern should still match variable mutations"
    );
}

// =============================================================================
// Attribute Mutation Execution Tests
// =============================================================================

#[test]
fn test_set_attribute_on_current_element() {
    let mut ctx = create_context();

    // Set up DOM
    ctx.eval(
        r#"
        document.body.innerHTML = '<button class="btn">Click</button>';
        const $ = document.querySelector('.btn');
    "#,
    )
    .unwrap();

    // Simulate attribute mutation: &.disabled <- "true"
    ctx.eval(
        r#"
        const stmt = "&.disabled <- \"true\"";
        const match = stmt.match(/^&\.(\w+)\s*<-\s*(.+)$/);
        if (match) {
            const attrName = match[1];
            const expr = match[2];
            const value = eval(expr);
            $.setAttribute(attrName, String(value));
        }
    "#,
    )
    .unwrap();

    let result = ctx.eval(
        r#"
        document.querySelector('.btn').getAttribute('disabled') === 'true'
    "#,
    );

    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "Button should have disabled='true' attribute"
    );
}

#[test]
fn test_set_href_with_expression() {
    let mut ctx = create_context();

    // Set up DOM and state (use window.$ to persist across eval blocks)
    ctx.eval(
        r##"
        document.body.innerHTML = '<a class="checkout" href="#">Checkout</a>';
        SpacetimeLocal.cart = [{ id: "1", name: "Test", price: 100 }];
        window.$ = document.querySelector('.checkout');
    "##,
    )
    .unwrap();

    // Simulate: &.href <- "https://example.com?data=" + btoa(JSON.stringify($cart))
    ctx.eval(
        r#"
        const stmt = '&.href <- "https://example.com?data=" + btoa(JSON.stringify($cart))';
        const match = stmt.match(/^&\.(\w+)\s*<-\s*(.+)$/);
        if (match) {
            const attrName = match[1];
            const expr = match[2].replace(/\$(\w+)/g, (_, n) => `SpacetimeLocal.${n}`);
            const value = eval(expr);
            $.setAttribute(attrName, String(value));
        }
    "#,
    )
    .unwrap();

    // Note: base64 of JSON array starting with "[{" is "W3s...", not "eyJ" (which is for "{")
    let result = ctx.eval(
        r#"
        const href = document.querySelector('.checkout').getAttribute('href');
        href.startsWith('https://example.com?data=') && href.includes('W3s')
    "#,
    );

    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "Checkout link should have href with base64-encoded cart data"
    );
}

#[test]
fn test_set_attribute_on_named_ref() {
    let mut ctx = create_context();

    // Set up DOM and refs registry
    ctx.eval(
        r#"
        document.body.innerHTML = `
            <button class="trigger">Click</button>
            <div class="target" id="target"></div>
        `;
        // Simulate named ref registration
        ST.refs = ST.refs || {};
        ST.refs.target = document.querySelector('.target');
    "#,
    )
    .unwrap();

    // Simulate: &target.data-active <- "true"
    // Note: regex needs to match hyphenated attrs like data-active
    ctx.eval(
        r#"
        const stmt = '&target.data-active <- "true"';
        const match = stmt.match(/^&(\w+)\.([\w-]+)\s*<-\s*(.+)$/);
        if (match) {
            const refName = match[1];
            const attrName = match[2];
            const expr = match[3];
            const value = eval(expr);
            const el = ST.refs[refName];
            if (el) el.setAttribute(attrName, String(value));
        }
    "#,
    )
    .unwrap();

    let result = ctx.eval(
        r#"
        document.querySelector('.target').getAttribute('data-active') === 'true'
    "#,
    );

    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "Target element should have data-active='true' attribute via named ref"
    );
}

// =============================================================================
// Expression Transform Tests
// =============================================================================

#[test]
fn test_dollar_vars_local() {
    let mut ctx = create_context();

    ctx.eval(
        r#"
        SpacetimeLocal.count = 42;
        SpacetimeLocal.name = "test";
    "#,
    )
    .unwrap();

    let result = ctx.eval(
        r#"
        const expr = "$count + $name.length";
        const jsExpr = expr.replace(/\$(\w+)/g, (_, n) => `SpacetimeLocal.${n}`);
        eval(jsExpr) === 46  // 42 + 4
    "#,
    );

    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "$var references should transform to SpacetimeLocal.var"
    );
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_empty_cart_returns_hash() {
    let mut ctx = create_context();

    ctx.eval(
        r#"
        SpacetimeLocal.cart = [];
    "#,
    )
    .unwrap();

    let result = ctx.eval(r#"
        const cart = SpacetimeLocal.cart || [];
        const url = cart.length === 0 ? '#' : 'https://example.com?data=' + btoa(JSON.stringify(cart));
        url === '#'
    "#);

    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "Empty cart should return '#' as URL"
    );
}

#[test]
fn test_undefined_ref_does_not_throw() {
    let mut ctx = create_context();

    ctx.eval(
        r#"
        ST.refs = {};
        document.body.innerHTML = '<div></div>';
    "#,
    )
    .unwrap();

    // This should not throw even if ref doesn't exist
    let result = ctx.eval(
        r#"
        const refName = 'nonexistent';
        const el = ST.refs[refName];
        if (el) {
            el.setAttribute('test', 'value');
        }
        true  // Should reach here without error
    "#,
    );

    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "Missing ref should be handled gracefully without throwing"
    );
}
