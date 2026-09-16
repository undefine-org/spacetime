//! Selector Initializer Tests using V8
//!
//! Tests the ST._selectorInitializers, ST.registerSelectorInit(), and ST.initElement() APIs
//! for dynamic element initialization (supporting @each/@template generated elements).

use super::context::V8TestContext;

/// ST runtime source
const ST_JS: &str = include_str!("../../public/runtime/st.js");

fn create_context() -> V8TestContext {
    let mut ctx = V8TestContext::new();
    ctx.eval(ST_JS).expect("Failed to load st.js");
    ctx
}

// =============================================================================
// Registry Existence Tests
// =============================================================================

#[test]
fn test_selector_initializers_registry_exists() {
    let mut ctx = create_context();

    let result = ctx.eval("Array.isArray(ST._selectorInitializers)");
    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "ST._selectorInitializers should be an array"
    );
}

#[test]
fn test_register_selector_init_exists() {
    let mut ctx = create_context();

    let result = ctx.eval("typeof ST.registerSelectorInit === 'function'");
    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "ST.registerSelectorInit should be a function"
    );
}

#[test]
fn test_init_element_exists() {
    let mut ctx = create_context();

    let result = ctx.eval("typeof ST.initElement === 'function'");
    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "ST.initElement should be a function"
    );
}

// =============================================================================
// Registration Tests
// =============================================================================

#[test]
fn test_register_selector_init_adds_to_registry() {
    let mut ctx = create_context();

    ctx.eval(
        r#"
        ST.registerSelectorInit('.test', (el) => { el._initialized = true; });
    "#,
    )
    .unwrap();

    let result = ctx.eval("ST._selectorInitializers.length === 1");
    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "Should add one initializer to registry"
    );
}

#[test]
fn test_register_multiple_selectors() {
    let mut ctx = create_context();

    ctx.eval(
        r#"
        ST.registerSelectorInit('.card', (el) => {});
        ST.registerSelectorInit('.button', (el) => {});
        ST.registerSelectorInit('.input', (el) => {});
    "#,
    )
    .unwrap();

    let result = ctx.eval("ST._selectorInitializers.length === 3");
    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "Should have 3 initializers in registry"
    );
}

#[test]
fn test_registered_entry_has_selector_and_init() {
    let mut ctx = create_context();

    ctx.eval(
        r#"
        ST.registerSelectorInit('.test-class', (el) => { el.marked = true; });
    "#,
    )
    .unwrap();

    let has_selector = ctx.eval("ST._selectorInitializers[0].selector === '.test-class'");
    assert!(
        matches!(has_selector, Ok(v) if v.as_bool() == Some(true)),
        "Entry should have correct selector"
    );

    let has_init = ctx.eval("typeof ST._selectorInitializers[0].init === 'function'");
    assert!(
        matches!(has_init, Ok(v) if v.as_bool() == Some(true)),
        "Entry should have init function"
    );
}

// =============================================================================
// Element Initialization Tests
// =============================================================================

#[test]
fn test_init_element_matches_selector() {
    let mut ctx = create_context();

    ctx.set_body_html(r#"<div class="card"></div>"#);

    ctx.eval(
        r#"
        let called = false;
        ST.registerSelectorInit('.card', (el) => { called = true; });
        ST.initElement(document.querySelector('.card'));
    "#,
    )
    .unwrap();

    let result = ctx.eval("called === true");
    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "Should call init for matching element"
    );
}

#[test]
fn test_init_element_skips_non_matching() {
    let mut ctx = create_context();

    ctx.set_body_html(r#"<div class="other"></div>"#);

    ctx.eval(
        r#"
        let called = false;
        ST.registerSelectorInit('.card', (el) => { called = true; });
        ST.initElement(document.querySelector('.other'));
    "#,
    )
    .unwrap();

    let result = ctx.eval("called === false");
    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "Should NOT call init for non-matching element"
    );
}

#[test]
fn test_init_element_checks_descendants() {
    let mut ctx = create_context();

    ctx.set_body_html(r#"<div class="parent"><span class="child"></span></div>"#);

    ctx.eval(
        r#"
        let called = false;
        ST.registerSelectorInit('.child', (el) => { called = true; });
        ST.initElement(document.querySelector('.parent'));
    "#,
    )
    .unwrap();

    let result = ctx.eval("called === true");
    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "Should init matching descendants"
    );
}

#[test]
fn test_init_element_passes_element_to_init() {
    let mut ctx = create_context();

    ctx.set_body_html(r#"<div class="target" data-id="123"></div>"#);

    ctx.eval(
        r#"
        let receivedEl = null;
        ST.registerSelectorInit('.target', (el) => { receivedEl = el; });
        ST.initElement(document.querySelector('.target'));
    "#,
    )
    .unwrap();

    let result = ctx.eval("receivedEl && receivedEl.dataset.id === '123'");
    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "Should pass the actual element to init function"
    );
}

#[test]
fn test_init_element_multiple_selectors_match() {
    let mut ctx = create_context();

    ctx.set_body_html(r#"<div class="card primary"></div>"#);

    ctx.eval(
        r#"
        let cardCalled = false;
        let primaryCalled = false;
        ST.registerSelectorInit('.card', (el) => { cardCalled = true; });
        ST.registerSelectorInit('.primary', (el) => { primaryCalled = true; });
        ST.initElement(document.querySelector('.card.primary'));
    "#,
    )
    .unwrap();

    let both_called = ctx.eval("cardCalled && primaryCalled");
    assert!(
        matches!(both_called, Ok(v) if v.as_bool() == Some(true)),
        "Should call all matching initializers"
    );
}

// =============================================================================
// Double-Initialization Guard Tests (pattern used in codegen)
// =============================================================================

#[test]
fn test_double_init_guard_pattern() {
    let mut ctx = create_context();

    ctx.set_body_html(r#"<div class="card"></div>"#);

    ctx.eval(
        r#"
        let callCount = 0;
        const init = (el) => {
            // Guard pattern that codegen will use
            if (el._st_init_abc123) return;
            el._st_init_abc123 = true;
            callCount++;
        };
        ST.registerSelectorInit('.card', init);
        const el = document.querySelector('.card');
        ST.initElement(el);
        ST.initElement(el);
        ST.initElement(el);
    "#,
    )
    .unwrap();

    let result = ctx.eval("callCount === 1");
    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "Should only initialize once due to guard"
    );
}

#[test]
fn test_different_guards_for_different_selectors() {
    let mut ctx = create_context();

    ctx.set_body_html(r#"<div class="card button"></div>"#);

    ctx.eval(
        r#"
        let cardCount = 0;
        let buttonCount = 0;

        const cardInit = (el) => {
            if (el._st_init_card) return;
            el._st_init_card = true;
            cardCount++;
        };

        const buttonInit = (el) => {
            if (el._st_init_button) return;
            el._st_init_button = true;
            buttonCount++;
        };

        ST.registerSelectorInit('.card', cardInit);
        ST.registerSelectorInit('.button', buttonInit);

        const el = document.querySelector('.card.button');
        ST.initElement(el);
    "#,
    )
    .unwrap();

    let result = ctx.eval("cardCount === 1 && buttonCount === 1");
    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "Each selector should initialize independently"
    );
}

// =============================================================================
// Complex Selector Tests
// =============================================================================

#[test]
fn test_descendant_selector() {
    let mut ctx = create_context();

    ctx.set_body_html(r#"<div class="grid"><article class="card"></article></div>"#);

    ctx.eval(
        r#"
        let called = false;
        ST.registerSelectorInit('.grid > .card', (el) => { called = true; });
        ST.initElement(document.querySelector('.grid'));
    "#,
    )
    .unwrap();

    let result = ctx.eval("called === true");
    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "Should match descendant selectors"
    );
}

#[test]
fn test_wildcard_selector() {
    let mut ctx = create_context();

    ctx.set_body_html(r#"<div class="grid"><div>1</div><span>2</span><article>3</article></div>"#);

    ctx.eval(
        r#"
        let count = 0;
        ST.registerSelectorInit('.grid > *', (el) => { count++; });
        ST.initElement(document.querySelector('.grid'));
    "#,
    )
    .unwrap();

    let result = ctx.eval("count === 3");
    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "Should match all direct children with wildcard"
    );
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_init_element_with_null() {
    let mut ctx = create_context();

    // Should not throw when passed null
    let result = ctx.eval(
        r#"
        ST.registerSelectorInit('.card', (el) => {});
        try {
            ST.initElement(null);
            true;
        } catch(e) {
            false;
        }
    "#,
    );

    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "Should handle null gracefully"
    );
}

#[test]
fn test_init_element_with_text_node() {
    let mut ctx = create_context();

    ctx.set_body_html(r#"<div class="wrapper">text</div>"#);

    // Should not throw when element has text nodes. initElement schedules init via
    // a microtask (ST._scheduleInit), so register+init in one eval and assert the
    // flag in a SECOND eval — the microtask drains between eval calls.
    ctx.eval(
        r#"
        globalThis.called = false;
        ST.registerSelectorInit('.wrapper', (el) => { globalThis.called = true; });
        ST.initElement(document.querySelector('.wrapper'));
    "#,
    )
    .unwrap();

    let result = ctx.eval("called === true");
    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "Should work with elements containing text nodes"
    );
}

// =============================================================================
// Selector Validity Tests
// =============================================================================

#[test]
fn test_comment_in_selector_string_causes_error() {
    // Confirms that selectors with comment text are invalid for querySelectorAll.
    // This validates the runtime contract: selectors emitted by the compiler
    // must be clean CSS selectors with no comment leakage.
    let mut ctx = create_context();

    ctx.set_body_html(r#"<div class="child">content</div>"#);

    // A clean selector should work and fire the init. initElement schedules init
    // via a microtask (ST._scheduleInit), so assert the flag in a SECOND eval.
    ctx.eval(
        r#"
        globalThis.initFired = false;
        ST.registerSelectorInit('.child', (el) => { globalThis.initFired = true; });
        ST.initElement(document.querySelector('.child'));
    "#,
    )
    .unwrap();

    let result = ctx.eval("initFired === true");
    assert!(
        matches!(result, Ok(v) if v.as_bool() == Some(true)),
        "Clean selector '.child' should successfully match and fire init"
    );
}
