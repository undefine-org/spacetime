/// Template Macro
///
/// Defines reusable templates with value and element parameters.
/// Templates compile to factory functions in Spacetime.templates registry.
///
/// @example basic value params
/// @template &service-card($s) {
///   <div class="service" data-id="`$s.id`">
///     <span>`$s.number`</span>
///     <h3>`$s.title`</h3>
///   </div>
/// }
///
/// @example with element params (composition)
/// @template &modal($title, &content, &footer?) {
///   <div class="modal">
///     <h2>`$title`</h2>
///     <div class="modal__body">`&content`</div>
///     <div class="modal__footer">`&footer`</div>
///   </div>
/// }
///
/// NOTE — `@if` does not work ANYWHERE yet (BUG-362), so a template cannot
/// render conditionally. `%macro conditional` declares a `%form` but has no
/// `%emit` block and no primitive behind it; outside a template the compiler
/// says so directly (`E0956: unknown primitive: conditional`), and even a
/// literal `@if(true)` renders nothing. Inside a template's markup it is worse
/// — the source leaks as page TEXT — which is reported as W0712 (BUG-358).
///
/// So: render the slot UNCONDITIONALLY, as above. An omitted optional param
/// renders as empty, which is the intended shape anyway. Verified, not
/// assumed: `tests/template_body_directive_test.rs`.
///
/// @example with scoped animations
/// @template &card($item) {
///   <article class="card">
///     <h3>`$item.title`</h3>
///     <p>`$item.desc`</p>
///   </article>
///
///   @on visible(300ms) {
///     & { opacity: 0 -> 1; translate-y: 20px -> 0; }
///   }
/// }
///
/// Param types:
/// - `$name` - value param (string, number, object)
/// - &name - element param (DOM element/fragment)
/// - `$name`? / &name? - optional param (see Optional Parameters below)
/// - `$name` = default - param with default value
///
/// ## Optional Parameters (`?` modifier)
///
/// The `?` modifier marks a parameter as optional, allowing template invocation
/// without providing that argument. This works for both value params ($) and
/// element params (&).
///
/// ### How optional parameters work:
/// - When a `$param?` is not provided, its value is `undefined`
/// - When an `&param?` is not provided, its value is `null`
/// - Optional params can be combined with @if for conditional rendering
/// - Optional params should typically come after required params
///
/// ### Example: Optional value parameter
/// ```spacetime
/// @template &greeting($name, $title?) {
///   <div class="greeting">
///     <span class="title">`$title`</span>
///     <span class="name">`$name`</span>
///   </div>
/// }
/// // An omitted `$title?` renders as empty — see the BUG-362 note above for
/// // why this is not written with `@if`.
///
/// // Usage:
/// &greeting("Alice")                    // Works - $title is undefined
/// &greeting("Alice", "Dr.")             // Works - $title is "Dr."
/// &greeting(name: "Alice")              // Works - named param style
/// &greeting(name: "Alice", title: "Dr.") // Works - both named
/// ```
///
/// ### Example: Optional element parameter
/// ```spacetime
/// @template &card($title, &actions?) {
///   <article class="card">
///     <h2>`$title`</h2>
///     <div class="card__actions">`&actions`</div>
///   </article>
/// }
///
/// // Usage:
/// &card("Title")                        // Works - no actions rendered
/// &card("Title", <button>Edit</button>) // Works - actions slot filled
/// ```
///
/// ### Runtime behavior:
/// - The factory function receives `undefined` for omitted optional args
/// - Use nullish coalescing (??) or OR (||) for defaults in interpolation
/// - The @if directive treats undefined/null as falsy for conditional rendering
///
/// Invocation styles:
/// - Positional: &modal("Title", <div>Content</div>)
/// - Named: &modal(title: "Title", content: <div>Content</div>)
/// - Mixed: &modal("Title", content: <div>Content</div>)

%macro template {
  %scope file

  %form {
    @template &$name:ident($params:param_list) {
      $body:component_body
    }
  }

  // Register template in the compile-time registry for validation
  %registers template($name) {
    params: $params
    body: $body
  }

  // Bind to the runtime primitive that creates the factory
  %binds {
    register-template(
      name: $name,
      body: $body,
      params: $params
    ) -> { $factory }
  }
}

/// Unlinked ref: &name(`$params`) { html }
///
/// Same as @template but without the directive prefix. Defines a template
/// as an element reference that can be invoked inline.
///
/// @example
/// &greeting(`$name`) {
///   <p>Hello, `$name`!</p>
/// }
%macro template-inline {
  %scope file

  %form {
    &$name:ident($params:param_list) {
      $body:component_body
    }
  }

  %registers template($name) {
    params: $params
    body: $body
  }

  %binds {
    register-template(
      name: $name,
      body: $body,
      params: $params
    ) -> { $factory }
  }
}

/// Template invocation with body: &name(args) { body_html }
///
/// Invokes a registered template inside a selector body, passing
/// inline body HTML as the last argument (fills the first unbound &param).
/// Body is required — use regular &name(args); for invocations without body.
///
/// @example
/// .card-list {
///   &card("Title") {
///     <p>Body content</p>
///   }
/// }
%macro template-invoke {
  %scope selector
  %order 200

  %form {
    &$name:ident($args:expr*) {
      $body:html_block
    }
  }

  %binds {
    invoke-template(
      &self,
      name: $name,
      args: $args,
      body: $body
    )
  }
}

/// Bare template invocation: &name(args);
///
/// Invokes a registered template inside a selector body without an inline
/// body block.  The semicolon terminator (or end-of-block) distinguishes
/// this from the body-form `&name(args) { body }`.
///
/// @example
/// .nav-wrap {
///   &psr-nav();
/// }
///
/// @example positional args
/// .header {
///   &greeting("World");
/// }
%macro template-invoke-bare {
  %scope selector
  %order 199

  %form {
    &$name:ident($args:expr*);
  }

  %binds {
    invoke-template(
      &self,
      name: $name,
      args: $args
    )
  }
}

///
/// Invokes a registered template with a named ref inside a selector body.
/// The created element is stored under the ref name for later access.
///
/// @example
/// .counter-wrap {
///   &counter &psr-counter();
/// }
///
%macro template-invoke-named {
  %scope selector
  %order 198

  %form {
    &$ref:ident &$name:ident($args:expr*);
  }

  %binds {
    invoke-template(
      &self,
      name: $name,
      args: $args,
      ref: $ref
    )
  }
}
