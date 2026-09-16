//! Parameter List Capture Type Tests
//!
//! Tests for the param_list capture type migration from Rust to pure %capture_type.
//! These tests verify that templates with various parameter patterns work correctly
//! at runtime when invoked via the V8 engine.
//!
//! # Parameter Types Tested
//! - `$name` - Value parameters (strings, numbers, objects)
//! - `&content` - Element parameters (DOM nodes)
//! - `$name?` - Optional value parameters
//! - `&content?` - Optional element parameters
//!
//! # Test Categories
//! 1. Single value parameter templates
//! 2. Element parameter templates
//! 3. Optional parameter templates
//! 4. Multiple parameter templates
//! 5. Empty parameter templates
//! 6. Mixed parameter type templates
//! 7. Multiple invocation independence

use super::context::V8TestContext;

/// Templates.js runtime source
const TEMPLATES_JS: &str = include_str!("../../public/runtime/templates.js");

fn create_param_test_context() -> V8TestContext {
    let mut ctx = V8TestContext::new();
    // Load templates runtime
    ctx.eval(TEMPLATES_JS).expect("Failed to load templates.js");
    ctx
}

// =============================================================================
// Test 1: Single Value Parameter
// =============================================================================

/// Test template with a single value parameter.
/// Simulates: `@template &greet($name) { <p>Hello $name</p> }`
#[test]
fn test_template_with_single_value_param() {
    let mut ctx = create_param_test_context();

    ctx.eval(
        r#"
        Spacetime.registerTemplate('greet', ($name) => {
            const el = document.createElement('p');
            el.textContent = 'Hello ' + $name;
            return el;
        });

        const element = Spacetime.invokeTemplate('greet', 'World');
        document.body.appendChild(element);
        "#,
    )
    .expect("Failed to invoke template with single value param");

    assert!(ctx.element_exists("p"));
    assert_eq!(ctx.query_text("p"), Some("Hello World".to_string()));
}

/// Test single value parameter with empty string.
/// Edge case: empty strings should be valid parameter values.
#[test]
fn test_template_with_empty_string_param() {
    let mut ctx = create_param_test_context();

    ctx.eval(
        r#"
        Spacetime.registerTemplate('greet_empty', ($name) => {
            const el = document.createElement('p');
            el.className = 'greeting';
            el.textContent = 'Hello ' + $name;
            return el;
        });

        const element = Spacetime.invokeTemplate('greet_empty', '');
        document.body.appendChild(element);
        "#,
    )
    .expect("Failed to invoke template with empty string param");

    assert!(ctx.element_exists(".greeting"));
    assert_eq!(ctx.query_text(".greeting"), Some("Hello ".to_string()));
}

/// Test single value parameter with numeric value.
#[test]
fn test_template_with_numeric_param() {
    let mut ctx = create_param_test_context();

    ctx.eval(
        r#"
        Spacetime.registerTemplate('count_display', ($count) => {
            const el = document.createElement('span');
            el.className = 'count';
            el.textContent = 'Count: ' + $count;
            return el;
        });

        const element = Spacetime.invokeTemplate('count_display', 42);
        document.body.appendChild(element);
        "#,
    )
    .expect("Failed to invoke template with numeric param");

    assert_eq!(ctx.query_text(".count"), Some("Count: 42".to_string()));
}

// =============================================================================
// Test 2: Element Parameter
// =============================================================================

/// Test template with an element parameter.
/// Simulates: `@template &card(&content) { <div class="card">&content</div> }`
#[test]
fn test_template_with_element_param() {
    let mut ctx = create_param_test_context();

    ctx.eval(
        r#"
        Spacetime.registerTemplate('card', ($content) => {
            const el = document.createElement('div');
            el.className = 'card';

            // Handle element parameter: if it's a DOM node, append it directly
            if ($content instanceof Node) {
                el.appendChild($content);
            } else if (typeof $content === 'string') {
                el.textContent = $content;
            }

            return el;
        });

        // Create an element to pass as the &content parameter
        const innerContent = document.createElement('p');
        innerContent.className = 'inner-content';
        innerContent.textContent = 'This is card content';

        const element = Spacetime.invokeTemplate('card', innerContent);
        document.body.appendChild(element);
        "#,
    )
    .expect("Failed to invoke template with element param");

    // Verify DOM structure
    assert!(ctx.element_exists(".card"));
    assert!(ctx.element_exists(".card .inner-content"));
    assert_eq!(
        ctx.query_text(".card .inner-content"),
        Some("This is card content".to_string())
    );
}

/// Test element parameter with complex nested HTML structure.
#[test]
fn test_template_with_complex_element_param() {
    let mut ctx = create_param_test_context();

    ctx.eval(
        r#"
        Spacetime.registerTemplate('wrapper', ($content) => {
            const el = document.createElement('section');
            el.className = 'wrapper';

            if ($content instanceof Node) {
                el.appendChild($content);
            }

            return el;
        });

        // Create complex nested content
        const complexContent = document.createElement('div');
        complexContent.className = 'complex';
        complexContent.innerHTML = '<header><h1>Title</h1></header><main><p>Paragraph 1</p><p>Paragraph 2</p></main>';

        const element = Spacetime.invokeTemplate('wrapper', complexContent);
        document.body.appendChild(element);
        "#,
    )
    .expect("Failed to invoke template with complex element param");

    // Verify nested structure is preserved
    assert!(ctx.element_exists(".wrapper .complex"));
    assert!(ctx.element_exists(".wrapper .complex header h1"));
    assert_eq!(
        ctx.query_text(".wrapper .complex header h1"),
        Some("Title".to_string())
    );
    assert!(ctx.element_exists(".wrapper .complex main p"));
}

// =============================================================================
// Test 3: Optional Parameter
// =============================================================================

/// Test template with optional value parameter - without providing the optional arg.
/// Simulates: `@template &btn($label, $icon?) { <button>$label $icon</button> }`
#[test]
fn test_template_with_optional_param_omitted() {
    let mut ctx = create_param_test_context();

    ctx.eval(
        r#"
        Spacetime.registerTemplate('btn', ($label, $icon) => {
            const el = document.createElement('button');
            el.className = 'btn';
            // Use || for default when optional param is undefined
            const iconPart = $icon ? ' ' + $icon : '';
            el.textContent = $label + iconPart;
            return el;
        });

        // Invoke WITHOUT the optional icon parameter
        const element = Spacetime.invokeTemplate('btn', 'Click Me');
        document.body.appendChild(element);
        "#,
    )
    .expect("Failed to invoke template without optional param");

    assert!(ctx.element_exists(".btn"));
    assert_eq!(ctx.query_text(".btn"), Some("Click Me".to_string()));
}

/// Test template with optional value parameter - with providing the optional arg.
#[test]
fn test_template_with_optional_param_provided() {
    let mut ctx = create_param_test_context();

    ctx.eval(
        r#"
        Spacetime.registerTemplate('btn_with_icon', ($label, $icon) => {
            const el = document.createElement('button');
            el.className = 'btn-icon';
            const iconPart = $icon ? ' ' + $icon : '';
            el.textContent = $label + iconPart;
            return el;
        });

        // Invoke WITH the optional icon parameter
        const element = Spacetime.invokeTemplate('btn_with_icon', 'Save', '💾');
        document.body.appendChild(element);
        "#,
    )
    .expect("Failed to invoke template with optional param provided");

    assert!(ctx.element_exists(".btn-icon"));
    // Note: The icon should appear after the label
    let text = ctx.query_text(".btn-icon");
    assert!(text.is_some());
    let text_value = text.unwrap();
    assert!(text_value.contains("Save"));
}

/// Test optional parameter with null explicitly passed.
#[test]
fn test_template_with_optional_param_null() {
    let mut ctx = create_param_test_context();

    ctx.eval(
        r#"
        Spacetime.registerTemplate('btn_null', ($label, $icon) => {
            const el = document.createElement('button');
            el.className = 'btn-null';
            const iconPart = $icon ? ' ' + $icon : '';
            el.textContent = $label + iconPart;
            return el;
        });

        // Invoke with explicit null for optional param
        const element = Spacetime.invokeTemplate('btn_null', 'Submit', null);
        document.body.appendChild(element);
        "#,
    )
    .expect("Failed to invoke template with null optional param");

    assert!(ctx.element_exists(".btn-null"));
    assert_eq!(ctx.query_text(".btn-null"), Some("Submit".to_string()));
}

// =============================================================================
// Test 4: Multiple Parameters
// =============================================================================

/// Test template with multiple positional parameters.
/// Simulates: `@template &item($title, $desc, $price) { ... }`
#[test]
fn test_template_with_multiple_params() {
    let mut ctx = create_param_test_context();

    ctx.eval(
        r#"
        Spacetime.registerTemplate('item', ($title, $desc, $price) => {
            const el = document.createElement('div');
            el.className = 'item';
            el.innerHTML = '<h3 class="item-title">' + $title + '</h3>' +
                           '<p class="item-desc">' + $desc + '</p>' +
                           '<span class="item-price">$' + $price + '</span>';
            return el;
        });

        const element = Spacetime.invokeTemplate('item', 'Widget', 'A useful widget', '29.99');
        document.body.appendChild(element);
        "#,
    )
    .expect("Failed to invoke template with multiple params");

    assert!(ctx.element_exists(".item"));
    assert_eq!(ctx.query_text(".item-title"), Some("Widget".to_string()));
    assert_eq!(
        ctx.query_text(".item-desc"),
        Some("A useful widget".to_string())
    );
    assert_eq!(ctx.query_text(".item-price"), Some("$29.99".to_string()));
}

/// Test template with many parameters (stress test).
#[test]
fn test_template_with_many_params() {
    let mut ctx = create_param_test_context();

    ctx.eval(
        r#"
        Spacetime.registerTemplate('full_card', ($a, $b, $c, $d, $e) => {
            const el = document.createElement('div');
            el.className = 'full-card';
            el.innerHTML = '<span class="p1">' + $a + '</span>' +
                           '<span class="p2">' + $b + '</span>' +
                           '<span class="p3">' + $c + '</span>' +
                           '<span class="p4">' + $d + '</span>' +
                           '<span class="p5">' + $e + '</span>';
            return el;
        });

        const element = Spacetime.invokeTemplate('full_card', 'one', 'two', 'three', 'four', 'five');
        document.body.appendChild(element);
        "#,
    )
    .expect("Failed to invoke template with many params");

    assert!(ctx.element_exists(".full-card"));
    assert_eq!(ctx.query_text(".p1"), Some("one".to_string()));
    assert_eq!(ctx.query_text(".p2"), Some("two".to_string()));
    assert_eq!(ctx.query_text(".p3"), Some("three".to_string()));
    assert_eq!(ctx.query_text(".p4"), Some("four".to_string()));
    assert_eq!(ctx.query_text(".p5"), Some("five".to_string()));
}

// =============================================================================
// Test 5: Empty Parameters
// =============================================================================

/// Test template with no parameters.
/// Simulates: `@template &divider() { <hr/> }`
#[test]
fn test_template_empty_params() {
    let mut ctx = create_param_test_context();

    ctx.eval(
        r#"
        Spacetime.registerTemplate('divider', () => {
            const el = document.createElement('hr');
            el.className = 'divider';
            return el;
        });

        const element = Spacetime.invokeTemplate('divider');
        document.body.appendChild(element);
        "#,
    )
    .expect("Failed to invoke template with no params");

    assert!(ctx.element_exists("hr.divider"));
}

/// Test template with no parameters invoked multiple times.
#[test]
fn test_template_empty_params_multiple_invocations() {
    let mut ctx = create_param_test_context();

    ctx.eval(
        r#"
        Spacetime.registerTemplate('spacer', () => {
            const el = document.createElement('div');
            el.className = 'spacer';
            return el;
        });

        // Invoke multiple times
        document.body.appendChild(Spacetime.invokeTemplate('spacer'));
        document.body.appendChild(Spacetime.invokeTemplate('spacer'));
        document.body.appendChild(Spacetime.invokeTemplate('spacer'));
        "#,
    )
    .expect("Failed to invoke empty param template multiple times");

    let result = ctx.eval("document.querySelectorAll('.spacer').length === 3");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

// =============================================================================
// Test 6: Mixed Parameter Types
// =============================================================================

/// Test template with mixed parameter types (value, element, optional).
/// Simulates: `@template &modal($title, &content, $footer?) { ... }`
#[test]
fn test_template_with_mixed_param_types() {
    let mut ctx = create_param_test_context();

    ctx.eval(
        r#"
        Spacetime.registerTemplate('modal', ($title, $content, $footer) => {
            const el = document.createElement('div');
            el.className = 'modal';

            // Header with title (value param)
            const header = document.createElement('header');
            header.className = 'modal-header';
            header.textContent = $title;
            el.appendChild(header);

            // Body with content (element param)
            const body = document.createElement('div');
            body.className = 'modal-body';
            if ($content instanceof Node) {
                body.appendChild($content);
            } else if (typeof $content === 'string') {
                body.textContent = $content;
            }
            el.appendChild(body);

            // Footer (optional param)
            if ($footer) {
                const footerEl = document.createElement('footer');
                footerEl.className = 'modal-footer';
                footerEl.textContent = $footer;
                el.appendChild(footerEl);
            }

            return el;
        });

        // Create element for &content param
        const contentEl = document.createElement('article');
        contentEl.className = 'modal-content';
        contentEl.innerHTML = '<p>Modal body text here.</p>';

        const element = Spacetime.invokeTemplate('modal', 'My Modal', contentEl, 'Close');
        document.body.appendChild(element);
        "#,
    )
    .expect("Failed to invoke template with mixed param types");

    // Verify structure
    assert!(ctx.element_exists(".modal"));
    assert_eq!(
        ctx.query_text(".modal-header"),
        Some("My Modal".to_string())
    );
    assert!(ctx.element_exists(".modal-body .modal-content"));
    assert_eq!(
        ctx.query_text(".modal-body .modal-content p"),
        Some("Modal body text here.".to_string())
    );
    assert_eq!(ctx.query_text(".modal-footer"), Some("Close".to_string()));
}

/// Test mixed param template without the optional footer.
#[test]
fn test_template_with_mixed_params_no_optional() {
    let mut ctx = create_param_test_context();

    ctx.eval(
        r#"
        Spacetime.registerTemplate('modal_no_footer', ($title, $content, $footer) => {
            const el = document.createElement('div');
            el.className = 'modal-nf';

            const header = document.createElement('header');
            header.className = 'modal-nf-header';
            header.textContent = $title;
            el.appendChild(header);

            const body = document.createElement('div');
            body.className = 'modal-nf-body';
            if ($content instanceof Node) {
                body.appendChild($content);
            }
            el.appendChild(body);

            if ($footer) {
                const footerEl = document.createElement('footer');
                footerEl.className = 'modal-nf-footer';
                footerEl.textContent = $footer;
                el.appendChild(footerEl);
            }

            return el;
        });

        const contentEl = document.createElement('div');
        contentEl.textContent = 'Just content';

        // Invoke WITHOUT optional footer
        const element = Spacetime.invokeTemplate('modal_no_footer', 'Title', contentEl);
        document.body.appendChild(element);
        "#,
    )
    .expect("Failed to invoke mixed param template without optional");

    assert!(ctx.element_exists(".modal-nf"));
    assert_eq!(
        ctx.query_text(".modal-nf-header"),
        Some("Title".to_string())
    );
    // Footer should NOT exist
    let has_footer = ctx.eval("document.querySelector('.modal-nf-footer') !== null");
    assert!(matches!(
        has_footer,
        Ok(value) if value.as_bool() == Some(false)
    ));
}

// =============================================================================
// Test 7: Multiple Invocations Independence
// =============================================================================

/// Test that multiple invocations of the same template are independent.
/// Each invocation should create separate DOM elements with their own data.
#[test]
fn test_template_multiple_invocations() {
    let mut ctx = create_param_test_context();

    ctx.eval(
        r#"
        Spacetime.registerTemplate('user_badge', ($name, $role) => {
            const el = document.createElement('div');
            el.className = 'user-badge';
            el.innerHTML = '<span class="name">' + $name + '</span>' +
                           '<span class="role">' + $role + '</span>';
            return el;
        });

        // Invoke multiple times with different data
        const badge1 = Spacetime.invokeTemplate('user_badge', 'Alice', 'Admin');
        badge1.id = 'badge1';
        document.body.appendChild(badge1);

        const badge2 = Spacetime.invokeTemplate('user_badge', 'Bob', 'User');
        badge2.id = 'badge2';
        document.body.appendChild(badge2);

        const badge3 = Spacetime.invokeTemplate('user_badge', 'Charlie', 'Guest');
        badge3.id = 'badge3';
        document.body.appendChild(badge3);
        "#,
    )
    .expect("Failed to create multiple template invocations");

    // Verify each badge has its own independent data
    assert_eq!(ctx.query_text("#badge1 .name"), Some("Alice".to_string()));
    assert_eq!(ctx.query_text("#badge1 .role"), Some("Admin".to_string()));

    assert_eq!(ctx.query_text("#badge2 .name"), Some("Bob".to_string()));
    assert_eq!(ctx.query_text("#badge2 .role"), Some("User".to_string()));

    assert_eq!(ctx.query_text("#badge3 .name"), Some("Charlie".to_string()));
    assert_eq!(ctx.query_text("#badge3 .role"), Some("Guest".to_string()));

    // Verify we have exactly 3 badges
    let count_result = ctx.eval("document.querySelectorAll('.user-badge').length === 3");
    assert!(matches!(
        count_result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

/// Test that modifying one invocation doesn't affect others.
#[test]
fn test_template_invocations_isolated() {
    let mut ctx = create_param_test_context();

    ctx.eval(
        r#"
        Spacetime.registerTemplate('counter_box', ($initial) => {
            const el = document.createElement('div');
            el.className = 'counter-box';
            el.setAttribute('data-count', $initial);
            el.textContent = 'Count: ' + $initial;
            return el;
        });

        const box1 = Spacetime.invokeTemplate('counter_box', '10');
        box1.id = 'box1';
        document.body.appendChild(box1);

        const box2 = Spacetime.invokeTemplate('counter_box', '20');
        box2.id = 'box2';
        document.body.appendChild(box2);

        // Modify box1
        box1.setAttribute('data-count', '100');
        box1.textContent = 'Count: 100';
        "#,
    )
    .expect("Failed to test invocation isolation");

    // box1 should be modified
    assert_eq!(ctx.query_text("#box1"), Some("Count: 100".to_string()));

    // box2 should be unchanged
    assert_eq!(ctx.query_text("#box2"), Some("Count: 20".to_string()));
}

// =============================================================================
// Edge Case Tests
// =============================================================================

/// Test parameter with special characters.
#[test]
fn test_template_param_with_special_chars() {
    let mut ctx = create_param_test_context();

    ctx.eval(
        r#"
        Spacetime.registerTemplate('special', ($text) => {
            const el = document.createElement('div');
            el.className = 'special';
            el.textContent = $text;
            return el;
        });

        const element = Spacetime.invokeTemplate('special', 'Hello <World> & "Friends"');
        document.body.appendChild(element);
        "#,
    )
    .expect("Failed to invoke template with special chars");

    // textContent should preserve the literal text
    assert_eq!(
        ctx.query_text(".special"),
        Some("Hello <World> & \"Friends\"".to_string())
    );
}

/// Test parameter with Unicode characters.
#[test]
fn test_template_param_with_unicode() {
    let mut ctx = create_param_test_context();

    ctx.eval(
        r#"
        Spacetime.registerTemplate('unicode', ($text) => {
            const el = document.createElement('div');
            el.className = 'unicode';
            el.textContent = $text;
            return el;
        });

        const element = Spacetime.invokeTemplate('unicode', '你好世界 🌍 مرحبا');
        document.body.appendChild(element);
        "#,
    )
    .expect("Failed to invoke template with unicode");

    let text = ctx.query_text(".unicode");
    assert!(text.is_some());
    // Should contain the Unicode text
    let text_val = text.unwrap();
    assert!(text_val.contains("你好世界"));
}

/// Test template invocation with object parameter.
#[test]
fn test_template_with_object_param() {
    let mut ctx = create_param_test_context();

    ctx.eval(
        r#"
        Spacetime.registerTemplate('profile', ($user) => {
            const el = document.createElement('div');
            el.className = 'profile';
            el.innerHTML = '<h2 class="profile-name">' + $user.name + '</h2>' +
                           '<p class="profile-email">' + $user.email + '</p>';
            return el;
        });

        const element = Spacetime.invokeTemplate('profile', {
            name: 'John Doe',
            email: 'john@example.com'
        });
        document.body.appendChild(element);
        "#,
    )
    .expect("Failed to invoke template with object param");

    assert!(ctx.element_exists(".profile"));
    assert_eq!(
        ctx.query_text(".profile-name"),
        Some("John Doe".to_string())
    );
    assert_eq!(
        ctx.query_text(".profile-email"),
        Some("john@example.com".to_string())
    );
}

/// Test template with array parameter.
#[test]
fn test_template_with_array_param() {
    let mut ctx = create_param_test_context();

    ctx.eval(
        r#"
        Spacetime.registerTemplate('list', ($items) => {
            const el = document.createElement('ul');
            el.className = 'item-list';
            for (const item of $items) {
                const li = document.createElement('li');
                li.textContent = item;
                el.appendChild(li);
            }
            return el;
        });

        const element = Spacetime.invokeTemplate('list', ['Apple', 'Banana', 'Cherry']);
        document.body.appendChild(element);
        "#,
    )
    .expect("Failed to invoke template with array param");

    assert!(ctx.element_exists(".item-list"));

    let count_result = ctx.eval("document.querySelectorAll('.item-list li').length === 3");
    assert!(matches!(
        count_result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

/// Test that undefined and null are handled gracefully.
#[test]
fn test_template_param_undefined_handling() {
    let mut ctx = create_param_test_context();

    ctx.eval(
        r#"
        Spacetime.registerTemplate('safe_display', ($value) => {
            const el = document.createElement('div');
            el.className = 'safe-display';
            // Handle undefined/null gracefully
            el.textContent = $value != null ? String($value) : '(empty)';
            return el;
        });

        const el1 = Spacetime.invokeTemplate('safe_display', undefined);
        el1.id = 'display1';
        document.body.appendChild(el1);

        const el2 = Spacetime.invokeTemplate('safe_display', null);
        el2.id = 'display2';
        document.body.appendChild(el2);

        const el3 = Spacetime.invokeTemplate('safe_display', 'actual value');
        el3.id = 'display3';
        document.body.appendChild(el3);
        "#,
    )
    .expect("Failed to test undefined/null handling");

    assert_eq!(ctx.query_text("#display1"), Some("(empty)".to_string()));
    assert_eq!(ctx.query_text("#display2"), Some("(empty)".to_string()));
    assert_eq!(
        ctx.query_text("#display3"),
        Some("actual value".to_string())
    );
}

/// Test template param with boolean false (falsy value).
#[test]
fn test_template_param_boolean_false() {
    let mut ctx = create_param_test_context();

    ctx.eval(
        r#"
        Spacetime.registerTemplate('bool_display', ($value) => {
            const el = document.createElement('div');
            el.className = 'bool-display';
            // Use strict comparison to preserve false
            el.textContent = $value === false ? 'false' : ($value === true ? 'true' : 'other');
            return el;
        });

        const el1 = Spacetime.invokeTemplate('bool_display', false);
        el1.id = 'bool1';
        document.body.appendChild(el1);

        const el2 = Spacetime.invokeTemplate('bool_display', true);
        el2.id = 'bool2';
        document.body.appendChild(el2);
        "#,
    )
    .expect("Failed to test boolean param");

    assert_eq!(ctx.query_text("#bool1"), Some("false".to_string()));
    assert_eq!(ctx.query_text("#bool2"), Some("true".to_string()));
}

/// Test template param with zero (falsy value).
#[test]
fn test_template_param_zero() {
    let mut ctx = create_param_test_context();

    ctx.eval(
        r#"
        Spacetime.registerTemplate('num_display', ($num) => {
            const el = document.createElement('div');
            el.className = 'num-display';
            // Use nullish coalescing to preserve 0
            el.textContent = 'Value: ' + ($num ?? 'default');
            return el;
        });

        const el1 = Spacetime.invokeTemplate('num_display', 0);
        el1.id = 'num1';
        document.body.appendChild(el1);

        const el2 = Spacetime.invokeTemplate('num_display', 42);
        el2.id = 'num2';
        document.body.appendChild(el2);
        "#,
    )
    .expect("Failed to test zero param");

    assert_eq!(ctx.query_text("#num1"), Some("Value: 0".to_string()));
    assert_eq!(ctx.query_text("#num2"), Some("Value: 42".to_string()));
}
