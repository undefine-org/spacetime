//! Template Registry Tests using V8
//!
//! Tests the Spacetime.templates runtime API using the V8 engine.

use super::context::V8TestContext;

/// Templates.js runtime source
const TEMPLATES_JS: &str = include_str!("../../public/runtime/templates.js");

fn create_template_context() -> V8TestContext {
    let mut ctx = V8TestContext::new();
    // Load templates runtime
    ctx.eval(TEMPLATES_JS).expect("Failed to load templates.js");
    ctx
}

#[test]
fn test_templates_registry_exists() {
    let mut ctx = create_template_context();

    let result = ctx.eval("typeof Spacetime.templates !== 'undefined'");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

#[test]
fn test_templates_is_map() {
    let mut ctx = create_template_context();

    let result = ctx.eval("Spacetime.templates instanceof Map");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

#[test]
fn test_register_template() {
    let mut ctx = create_template_context();

    ctx.eval(
        r#"
        Spacetime.registerTemplate('card', ($item) => {
            const el = document.createElement('div');
            el.className = 'card';
            el.textContent = $item.title;
            return el;
        });
        "#,
    )
    .expect("Failed to register template");

    let result = ctx.eval("Spacetime.hasTemplate('card')");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

#[test]
fn test_get_template() {
    let mut ctx = create_template_context();

    ctx.eval(
        r#"
        Spacetime.registerTemplate('button', ($label) => {
            const el = document.createElement('button');
            el.textContent = $label;
            return el;
        });
        "#,
    )
    .expect("Failed to register template");

    let result = ctx.eval("typeof Spacetime.getTemplate('button') === 'function'");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

#[test]
fn test_invoke_template() {
    let mut ctx = create_template_context();

    ctx.eval(
        r#"
        Spacetime.registerTemplate('greeting', ($name) => {
            const el = document.createElement('span');
            el.textContent = 'Hello, ' + $name + '!';
            return el;
        });

        const element = Spacetime.invokeTemplate('greeting', 'World');
        document.body.appendChild(element);
        "#,
    )
    .expect("Failed to invoke template");

    assert_eq!(ctx.query_text("span"), Some("Hello, World!".to_string()));
}

#[test]
fn test_invoke_template_with_object() {
    let mut ctx = create_template_context();

    ctx.eval(
        r#"
        Spacetime.registerTemplate('profile', ($user) => {
            const el = document.createElement('div');
            el.className = 'profile';
            el.innerHTML = '<h2>' + $user.name + '</h2><p>' + $user.email + '</p>';
            return el;
        });

        const element = Spacetime.invokeTemplate('profile', {
            name: 'Alice',
            email: 'alice@example.com'
        });
        document.body.appendChild(element);
        "#,
    )
    .expect("Failed to invoke template with object");

    assert!(ctx.element_exists(".profile"));
    assert_eq!(ctx.query_text(".profile h2"), Some("Alice".to_string()));
    assert_eq!(
        ctx.query_text(".profile p"),
        Some("alice@example.com".to_string())
    );
}

#[test]
fn test_invoke_nonexistent_template_returns_null() {
    let mut ctx = create_template_context();

    let result = ctx.eval("Spacetime.invokeTemplate('nonexistent', {}) === null");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

#[test]
fn test_template_names() {
    let mut ctx = create_template_context();

    ctx.eval(
        r#"
        Spacetime.registerTemplate('a', () => document.createElement('div'));
        Spacetime.registerTemplate('b', () => document.createElement('div'));
        Spacetime.registerTemplate('c', () => document.createElement('div'));
        "#,
    )
    .expect("Failed to register templates");

    let result = ctx.eval(
        r#"
        const names = Spacetime.templateNames();
        names.length === 3 && names.includes('a') && names.includes('b') && names.includes('c')
        "#,
    );
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

#[test]
fn test_clear_templates() {
    let mut ctx = create_template_context();

    ctx.eval(
        r#"
        Spacetime.registerTemplate('temp', () => document.createElement('div'));
        "#,
    )
    .expect("Failed to register template");

    let result = ctx.eval("Spacetime.hasTemplate('temp')");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));

    ctx.eval("Spacetime.clearTemplates()").unwrap();

    let result = ctx.eval("Spacetime.hasTemplate('temp')");
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(false)
    ));
}

#[test]
fn test_create_element_helper() {
    let mut ctx = create_template_context();

    ctx.eval(
        r#"
        const el = Spacetime.createElement('<div class="test">Content</div>');
        document.body.appendChild(el);
        "#,
    )
    .expect("Failed to create element");

    assert!(ctx.element_exists(".test"));
    assert_eq!(ctx.query_text(".test"), Some("Content".to_string()));
}

#[test]
fn test_interpolate_html() {
    let mut ctx = create_template_context();

    let result = ctx.eval(
        r#"
        const html = '<div>$title by $author</div>';
        const data = { title: 'Hello', author: 'World' };
        const result = Spacetime.interpolateHtml(html, data);
        result === '<div>Hello by World</div>'
        "#,
    );
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

#[test]
fn test_interpolate_html_with_nested_property() {
    let mut ctx = create_template_context();

    let result = ctx.eval(
        r#"
        const html = '<div>$user.name - $user.email</div>';
        const data = { user: { name: 'Alice', email: 'a@b.com' } };
        const result = Spacetime.interpolateHtml(html, data);
        result === '<div>Alice - a@b.com</div>'
        "#,
    );
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

#[test]
fn test_template_factory_with_multiple_args() {
    let mut ctx = create_template_context();

    ctx.eval(
        r#"
        Spacetime.registerTemplate('dialog', ($title, $content, $footer) => {
            const el = document.createElement('div');
            el.className = 'dialog';
            el.innerHTML = '<h2>' + $title + '</h2><main>' + $content + '</main><footer>' + ($footer || 'OK') + '</footer>';
            return el;
        });

        const el = Spacetime.invokeTemplate('dialog', 'Title', 'Body content', 'Cancel');
        document.body.appendChild(el);
        "#,
    )
    .expect("Failed to invoke template with multiple args");

    assert!(ctx.element_exists(".dialog"));
    assert_eq!(ctx.query_text(".dialog h2"), Some("Title".to_string()));
    assert_eq!(
        ctx.query_text(".dialog main"),
        Some("Body content".to_string())
    );
    assert_eq!(ctx.query_text(".dialog footer"), Some("Cancel".to_string()));
}

#[test]
fn test_template_with_default_footer() {
    let mut ctx = create_template_context();

    ctx.eval(
        r#"
        Spacetime.registerTemplate('dialog', ($title, $content, $footer) => {
            const el = document.createElement('div');
            el.className = 'dialog';
            el.innerHTML = '<h2>' + $title + '</h2><main>' + $content + '</main><footer>' + ($footer || 'OK') + '</footer>';
            return el;
        });

        // Invoke without footer - should use default
        const el = Spacetime.invokeTemplate('dialog', 'Title', 'Body');
        document.body.appendChild(el);
        "#,
    )
    .expect("Failed to invoke template without optional arg");

    assert_eq!(ctx.query_text(".dialog footer"), Some("OK".to_string()));
}

#[test]
fn test_escape_html_text_content() {
    let mut ctx = create_template_context();

    let result = ctx.eval(
        r#"
        Spacetime.escapeHtml('<script>alert("xss")</script>') === '&lt;script&gt;alert("xss")&lt;/script&gt;'
        "#,
    );
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

#[test]
fn test_escape_attr_quotes() {
    let mut ctx = create_template_context();

    let result = ctx.eval(
        r#"
        Spacetime.escapeAttr('Photo of "nature" & sun') === 'Photo of &quot;nature&quot; &amp; sun'
        "#,
    );
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

#[test]
fn test_interpolate_escapes_in_attribute() {
    let mut ctx = create_template_context();

    let result = ctx.eval(
        r#"
        const html = '<img alt="$desc">';
        const data = { desc: 'Image of "nature" & wildlife' };
        const result = Spacetime.interpolateHtml(html, data);
        result === '<img alt="Image of &quot;nature&quot; &amp; wildlife">'
        "#,
    );
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

#[test]
fn test_interpolate_escapes_in_text_content() {
    let mut ctx = create_template_context();

    let result = ctx.eval(
        r#"
        const html = '<div>$content</div>';
        const data = { content: '<script>alert("xss")</script>' };
        const result = Spacetime.interpolateHtml(html, data);
        result === '<div>&lt;script&gt;alert("xss")&lt;/script&gt;</div>'
        "#,
    );
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

#[test]
fn test_interpolate_null_values() {
    let mut ctx = create_template_context();

    let result = ctx.eval(
        r#"
        const html = '<div alt="$missing">$also_missing</div>';
        const data = {};
        const result = Spacetime.interpolateHtml(html, data);
        result === '<div alt=""></div>'
        "#,
    );
    assert!(matches!(
        result,
        Ok(value) if value.as_bool() == Some(true)
    ));
}

// =============================================================================
// Element Parameter Tests (&param)
// =============================================================================

/// Test that element parameters are properly inserted into templates.
/// Element parameters use the `&` prefix (e.g., `&content`, `&footer`) and
/// represent DOM elements that get inserted into the template structure.
#[test]
fn test_template_with_element_parameter() {
    let mut ctx = create_template_context();

    // Register a template that accepts an element parameter (&content)
    // Element parameters are DOM elements that get inserted into the template
    ctx.eval(
        r#"
        Spacetime.registerTemplate('modal', ($title, $content) => {
            const el = document.createElement('div');
            el.className = 'modal';

            // Create header with title
            const header = document.createElement('header');
            header.textContent = $title;
            el.appendChild(header);

            // Create body container for element parameter
            const body = document.createElement('div');
            body.className = 'modal__body';

            // Handle element parameter: if it's a DOM node, append it directly
            if ($content instanceof Node) {
                body.appendChild($content);
            } else if (typeof $content === 'string') {
                body.textContent = $content;
            }

            el.appendChild(body);
            return el;
        });

        // Create an element to pass as the &content parameter
        const contentElement = document.createElement('article');
        contentElement.className = 'content-article';
        contentElement.innerHTML = '<p>This is the modal content.</p>';

        // Invoke template with element parameter
        const modal = Spacetime.invokeTemplate('modal', 'My Modal Title', contentElement);
        document.body.appendChild(modal);
        "#,
    )
    .expect("Failed to invoke template with element parameter");

    // Verify the modal structure
    assert!(ctx.element_exists(".modal"));
    assert_eq!(
        ctx.query_text(".modal header"),
        Some("My Modal Title".to_string())
    );

    // Verify the element parameter was inserted into the body
    assert!(ctx.element_exists(".modal__body .content-article"));
    assert_eq!(
        ctx.query_text(".modal__body .content-article p"),
        Some("This is the modal content.".to_string())
    );
}

/// Test optional element parameters that may or may not be provided.
/// This simulates the behavior of `@template &card($title, &footer?) { ... }`
#[test]
#[ignore = "Hangs in V8 engine - test passes in browser, skip for CI"]
fn test_template_with_optional_element_parameter() {
    let mut ctx = create_template_context();

    // Register a template with an optional element parameter (&footer?)
    ctx.eval(
        r#"
        Spacetime.registerTemplate('card_elem', ($title, $footer) => {
            const el = document.createElement('div');
            el.className = 'card-elem';

            // Create header
            const header = document.createElement('h2');
            header.textContent = $title;
            el.appendChild(header);

            // Optionally add footer if element parameter is provided
            if ($footer) {
                const footerEl = document.createElement('footer');
                if ($footer instanceof Node) {
                    footerEl.appendChild($footer);
                } else {
                    footerEl.textContent = $footer;
                }
                el.appendChild(footerEl);
            }

            return el;
        });

        // Test 1: Invoke without footer
        const card1 = Spacetime.invokeTemplate('card_elem', 'Card Without Footer');
        card1.id = 'card1';
        document.body.appendChild(card1);

        // Test 2: Invoke with footer element
        const footerContent = document.createElement('button');
        footerContent.textContent = 'Close';
        const card2 = Spacetime.invokeTemplate('card_elem', 'Card With Footer', footerContent);
        card2.id = 'card2';
        document.body.appendChild(card2);
        "#,
    )
    .expect("Failed to invoke template with optional element parameter");

    // Verify card without footer has no footer element
    assert!(ctx.element_exists("#card1"));
    assert_eq!(
        ctx.query_text("#card1 h2"),
        Some("Card Without Footer".to_string())
    );

    let has_footer1 = ctx.eval("document.querySelector('#card1 footer') !== null");
    assert!(matches!(
        has_footer1,
        Ok(value) if value.as_bool() == Some(false)
    ));

    // Verify card with footer has the element properly inserted
    assert!(ctx.element_exists("#card2"));
    assert_eq!(
        ctx.query_text("#card2 h2"),
        Some("Card With Footer".to_string())
    );
    assert!(ctx.element_exists("#card2 footer button"));
    assert_eq!(
        ctx.query_text("#card2 footer button"),
        Some("Close".to_string())
    );
}

/// Test templates with multiple element parameters for slot-based composition.
/// This simulates layout templates like `@template &layout(&header, &main, &sidebar) { ... }`
#[test]
#[ignore = "Hangs in V8 engine - test passes in browser, skip for CI"]
fn test_template_with_multiple_element_parameters() {
    let mut ctx = create_template_context();

    // Register a template that accepts multiple element parameters
    ctx.eval(
        r#"
        Spacetime.registerTemplate('layout', ($header, $main, $sidebar) => {
            const el = document.createElement('div');
            el.className = 'layout';

            // Header slot
            const headerSlot = document.createElement('header');
            headerSlot.setAttribute('slot', 'header');
            if ($header instanceof Node) {
                headerSlot.appendChild($header);
            }
            el.appendChild(headerSlot);

            // Main content slot
            const mainSlot = document.createElement('main');
            mainSlot.setAttribute('slot', 'main');
            if ($main instanceof Node) {
                mainSlot.appendChild($main);
            }
            el.appendChild(mainSlot);

            // Sidebar slot
            const sidebarSlot = document.createElement('aside');
            sidebarSlot.setAttribute('slot', 'sidebar');
            if ($sidebar instanceof Node) {
                sidebarSlot.appendChild($sidebar);
            }
            el.appendChild(sidebarSlot);

            return el;
        });

        // Create elements for each slot
        const headerEl = document.createElement('h1');
        headerEl.textContent = 'Page Title';

        const mainEl = document.createElement('article');
        mainEl.innerHTML = '<p>Main content here</p>';

        const sidebarEl = document.createElement('nav');
        sidebarEl.innerHTML = '<ul><li>Link 1</li><li>Link 2</li></ul>';

        // Invoke template with multiple element parameters
        const layout = Spacetime.invokeTemplate('layout', headerEl, mainEl, sidebarEl);
        document.body.appendChild(layout);
        "#,
    )
    .expect("Failed to invoke template with multiple element parameters");

    // Verify all slots received their element parameters
    assert!(ctx.element_exists(".layout"));
    assert_eq!(
        ctx.query_text(".layout [slot='header'] h1"),
        Some("Page Title".to_string())
    );
    assert_eq!(
        ctx.query_text(".layout [slot='main'] article p"),
        Some("Main content here".to_string())
    );
    assert!(ctx.element_exists(".layout [slot='sidebar'] nav ul"));
}

// =============================================================================
// Optional Parameter Tests (`?` modifier)
// =============================================================================

/// Test that a template with an optional parameter can be invoked without it.
/// This simulates the behavior of `@template &greeting($name, $title?) { ... }`
/// where `$title?` is optional.
#[test]
fn test_optional_param_omitted() {
    let mut ctx = create_template_context();

    ctx.eval(
        r#"
        // Register template with optional second parameter ($subtitle?)
        // The `?` modifier in Spacetime templates means the param can be omitted
        Spacetime.registerTemplate('opt_card', ($title, $subtitle) => {
            const el = document.createElement('div');
            el.className = 'opt-card';
            // Use || for default when optional param is undefined
            el.innerHTML = '<h2>' + $title + '</h2><p>' + ($subtitle || 'No subtitle') + '</p>';
            return el;
        });

        // Invoke WITHOUT the optional parameter
        const el = Spacetime.invokeTemplate('opt_card', 'Hello');
        document.body.appendChild(el);
        "#,
    )
    .expect("Failed to invoke template without optional param");

    assert!(ctx.element_exists(".opt-card"));
    assert_eq!(ctx.query_text(".opt-card h2"), Some("Hello".to_string()));
    // When omitted, the fallback 'No subtitle' should be used
    assert_eq!(
        ctx.query_text(".opt-card p"),
        Some("No subtitle".to_string())
    );
}

/// Test that a template with an optional parameter works when the param IS provided.
/// This verifies the `?` modifier doesn't break normal invocation.
#[test]
fn test_optional_param_provided() {
    let mut ctx = create_template_context();

    ctx.eval(
        r#"
        // Register template with optional second parameter ($subtitle?)
        Spacetime.registerTemplate('opt_card2', ($title, $subtitle) => {
            const el = document.createElement('div');
            el.className = 'opt-card2';
            el.innerHTML = '<h2>' + $title + '</h2><p>' + ($subtitle || 'No subtitle') + '</p>';
            return el;
        });

        // Invoke WITH the optional parameter
        const el = Spacetime.invokeTemplate('opt_card2', 'Hello', 'World');
        document.body.appendChild(el);
        "#,
    )
    .expect("Failed to invoke template with optional param");

    assert!(ctx.element_exists(".opt-card2"));
    assert_eq!(ctx.query_text(".opt-card2 h2"), Some("Hello".to_string()));
    // When provided, the actual value should be used
    assert_eq!(ctx.query_text(".opt-card2 p"), Some("World".to_string()));
}

/// Test optional parameter with nullish coalescing (??) for default values.
/// This demonstrates the recommended pattern for handling optional params.
#[test]
#[ignore = "Hangs in V8 engine - test passes in browser, skip for CI"]
fn test_optional_param_with_nullish_coalescing() {
    let mut ctx = create_template_context();

    ctx.eval(
        r#"
        // Template using nullish coalescing for optional param default
        Spacetime.registerTemplate('counter', ($label, $count) => {
            const el = document.createElement('div');
            el.className = 'counter';
            // ?? preserves 0 as valid, unlike ||
            el.innerHTML = '<span>' + $label + ': ' + ($count ?? 0) + '</span>';
            return el;
        });

        // Invoke without count - should default to 0
        const el1 = Spacetime.invokeTemplate('counter', 'Items');
        el1.id = 'counter1';
        document.body.appendChild(el1);

        // Invoke with count = 0 - should show 0, not default
        const el2 = Spacetime.invokeTemplate('counter', 'Items', 0);
        el2.id = 'counter2';
        document.body.appendChild(el2);

        // Invoke with count = 5
        const el3 = Spacetime.invokeTemplate('counter', 'Items', 5);
        el3.id = 'counter3';
        document.body.appendChild(el3);
        "#,
    )
    .expect("Failed to test nullish coalescing with optional param");

    // Omitted param defaults to 0
    assert_eq!(
        ctx.query_text("#counter1 span"),
        Some("Items: 0".to_string())
    );
    // Explicit 0 is preserved (not replaced by default)
    assert_eq!(
        ctx.query_text("#counter2 span"),
        Some("Items: 0".to_string())
    );
    // Normal value works as expected
    assert_eq!(
        ctx.query_text("#counter3 span"),
        Some("Items: 5".to_string())
    );
}

/// Test multiple optional parameters - both can be omitted or provided.
#[test]
#[ignore = "Hangs in V8 engine - test passes in browser, skip for CI"]
fn test_multiple_optional_params() {
    let mut ctx = create_template_context();

    ctx.eval(
        r#"
        // Template with two optional params: $subtitle? and $footer?
        Spacetime.registerTemplate('panel', ($title, $subtitle, $footer) => {
            const el = document.createElement('div');
            el.className = 'panel';
            let html = '<h2>' + $title + '</h2>';
            if ($subtitle) {
                html += '<h3>' + $subtitle + '</h3>';
            }
            if ($footer) {
                html += '<footer>' + $footer + '</footer>';
            }
            el.innerHTML = html;
            return el;
        });

        // Test 1: Only required param
        const el1 = Spacetime.invokeTemplate('panel', 'Title Only');
        el1.id = 'panel1';
        document.body.appendChild(el1);

        // Test 2: Required + first optional
        const el2 = Spacetime.invokeTemplate('panel', 'With Subtitle', 'The Subtitle');
        el2.id = 'panel2';
        document.body.appendChild(el2);

        // Test 3: All params provided
        const el3 = Spacetime.invokeTemplate('panel', 'Full Panel', 'Subtitle', 'Footer Text');
        el3.id = 'panel3';
        document.body.appendChild(el3);
        "#,
    )
    .expect("Failed to test multiple optional params");

    // Panel 1: Only title
    assert_eq!(ctx.query_text("#panel1 h2"), Some("Title Only".to_string()));
    assert!(!ctx.element_exists("#panel1 h3"));
    assert!(!ctx.element_exists("#panel1 footer"));

    // Panel 2: Title + subtitle
    assert_eq!(
        ctx.query_text("#panel2 h2"),
        Some("With Subtitle".to_string())
    );
    assert_eq!(
        ctx.query_text("#panel2 h3"),
        Some("The Subtitle".to_string())
    );
    assert!(!ctx.element_exists("#panel2 footer"));

    // Panel 3: All params
    assert_eq!(ctx.query_text("#panel3 h2"), Some("Full Panel".to_string()));
    assert_eq!(ctx.query_text("#panel3 h3"), Some("Subtitle".to_string()));
    assert_eq!(
        ctx.query_text("#panel3 footer"),
        Some("Footer Text".to_string())
    );
}

// =============================================================================
// Named Argument Tests
// =============================================================================

/// Test that templates can be invoked with named arguments.
/// This verifies the syntax: &modal(title: "X", content: <div/>)
/// Named arguments are passed as an object with parameter names as keys.
///
/// The compiled factory function handles named args by checking if the first
/// argument is an object and extracting named properties from it.
#[test]
#[ignore = "Hangs in V8 engine - test passes in browser, skip for CI"]
fn test_invoke_template_with_named_arguments() {
    let mut ctx = create_template_context();

    // Register a template that accepts named parameters via destructuring.
    // This simulates the compiled factory output where the compiler generates
    // a factory that handles named arguments as an object with parameter keys.
    ctx.eval(
        r#"
        Spacetime.registerTemplate('named_modal', (args) => {
            // Extract named arguments from the object
            const { title, content, footer } = args || {};

            const el = document.createElement('div');
            el.className = 'named-modal';
            el.innerHTML = '<h2 class="modal-title">' + (title || '') + '</h2>' +
                           '<div class="modal-content">' + (content || '') + '</div>' +
                           '<footer class="modal-footer">' + (footer || 'OK') + '</footer>';
            return el;
        });
        "#,
    )
    .expect("Failed to register named_modal template");

    // Invoke with named arguments (object with parameter names as keys).
    // This simulates the compiled output of: &modal(title: "X", content: "Y")
    ctx.eval(
        r#"
        const element = Spacetime.invokeTemplate('named_modal', {
            title: 'My Dialog',
            content: 'Dialog body text',
            footer: 'Close'
        });
        document.body.appendChild(element);
        "#,
    )
    .expect("Failed to invoke template with named arguments");

    assert!(ctx.element_exists(".named-modal"));
    assert_eq!(
        ctx.query_text(".modal-title"),
        Some("My Dialog".to_string())
    );
    assert_eq!(
        ctx.query_text(".modal-content"),
        Some("Dialog body text".to_string())
    );
    assert_eq!(ctx.query_text(".modal-footer"), Some("Close".to_string()));
}
