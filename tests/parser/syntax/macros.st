// Macro call syntax reference file
// Shows @directive invocation patterns in Spacetime
// Note: These are macro CALLS (invocations), not macro DEFINITIONS (%macro)

// ============================================
// Simple Macro Calls (No Arguments, No Block)
// ============================================

// Inline statements end with semicolon
// @fadeIn;
// @reset;
// @initialize;

// ============================================
// Macro Calls with Parenthesized Arguments
// ============================================

// Single argument
// @fadeIn(300ms)
// @delay(500ms)
// @wait(1s)

// Multiple arguments
// @slideIn("left", 500ms)
// @animate(0.3, ease-out)
// @scroll(100px, smooth)

// Named arguments
// @animation(duration: 300ms, easing: --smooth)
// @config(timeout: 500ms)
// @state(when: "active")

// Mixed positional and named
// @stagger($items, delay: 100ms)
// @on click { $active <- !$active }

// ============================================
// Macro Calls with Inline Arguments (No Parens)
// ============================================

// Selector as inline argument
// @on hover { }
// @on click .button { }
// @given .selector { }
// @when #submit click { }

// Multiple inline tokens
// @then .element should exist
// @then #counter should have_text "5"

// Identifier arguments
// @event click
// @trigger show
// @emit "custom-event"

// ============================================
// Macro Calls with Block Body
// ============================================

// ============================================
// Macro Calls with "as" Clause
// ============================================

// Variable binding
// @each($items) as $item { }
// @presence(room: "lobby") as cursors: Cursor[]

// Data binding with type
// @data("api/users") as users: User[]
// @fetch("/api/data") as response: ApiResponse

// ============================================
// Macro Calls with Variant (dot notation)
// ============================================

// @name.variant syntax
// @test.skip "disabled test" { }
// @test.only "focused test" { }
// @state.initial "idle" { }

// ============================================
// Macro Calls with Array Arguments
// ============================================

// Array literal arguments
// @stagger([1, 2, 3, 4])
// @keyframes([0%, 50%, 100%])
// @sequence(["a", "b", "c"])

// ============================================
// Macro Calls with Expression Arguments
// ============================================

// Parenthesized expressions (for assertions)
// @assert (1 + 1 === 2)
// @if ($count > 0) { }
// @when ($active && $visible) { }

// ============================================
// Macro Calls with Special References
// ============================================

// Variable references
// text <- $user
// @watch($counter)
// @each($items) as $item { }

// Preset references (RETIRED — `~` refs error loudly; easings are now
// `@form easing --name`, referenced as `easing: --name`)
// @easing(--bouncy)
// @timing(--spring-gentle)
// @animation(easing: --ease-out)

// Element references
// (element ref) &button.opacity
// @sync(&modal.visible)
// @animate(&card, transform)

// Property paths
// text <- $.user.name
// .hidden: !$.products

// ============================================
// Macro Calls with HTML Content
// ============================================

// Fixture blocks with HTML
@fixture {
    <div class="card">
        <h1>Title</h1>
        <p>Content</p>
    </div>
}

// Template definitions
@template &product-card {
    <article class="product">
        <slot name="image"></slot>
        <div class="info">
            <slot name="title"></slot>
            <slot name="price"></slot>
        </div>
    </article>
}

// ============================================
// Macro Calls with JavaScript
// ============================================

// Event handlers with JS
@on &.click {
    console.log("clicked");
}

// Test blocks with JS assertions
@test "counter increments" {
    @given .counter {
        count: 0;
    }
    @when .button click
    @then .counter should have_text "1"
}

// ============================================
// Macro Calls with Nested Macro Calls
// ============================================

// Reactive state via signal + class
.widget {
    .is-active: $active;
    opacity: 0;
}
.widget.is-active { opacity: 1; }

// Animation sequences
@sequence {
    @step(0%) {
        opacity: 0;
    }
    @step(50%) {
        opacity: 0.5;
    }
    @step(100%) {
        opacity: 1;
    }
}

// ============================================
// Macro Calls with State Transitions
// ============================================

// Transition declarations in body
@behavior {
    idle -> active on "show";
    active -> idle on "hide";
}

// Bidirectional toggle syntax (if supported)
// @toggle {
//     on <-> off
// }

// ============================================
// Macro Calls with Pseudo Blocks
// ============================================

// Animation phase pseudo-selectors
@each($items) as $item {
    :entering {
        opacity: 0;
        transform: translateY(20px);
    }
    :exiting {
        opacity: 0;
    }
    :move {
        transition: transform 300ms;
    }
}

// ============================================
// Macro Calls with Slot Bindings
// ============================================

// Text bindings
@each($products) as $product {
    [slot="title"]: $.name;
    [slot="price"]: $.price;
}

// Attribute bindings
@each($items) as $item {
    [slot="image"] {
        src: $.imageUrl;
        alt: $.title;
    }
    :host {
        data-id: $.id;
    }
}

// ============================================
// Macro Calls with Template Invocations
// ============================================

// Template calls inside body
@each($products) as $product {
    &product-card($product);
}

// Nested template with args
@gallery($images) {
    &image-tile($image, $index);
}

// ============================================
// Type and Data Macro Calls
// ============================================

// Type definitions
@type User {
    id: number;
    name: string;
    email?: string;
}

// Data sources (unified @data <kind> surface, PLAN-023)
@data fetch $products Product[] : "/api/products" { refresh: 5m }

// Computed data (query pipeline over an array source)
@data query $filteredProducts Product[] from $products {
    where: item.active === true
}

// ============================================
// Complex Real-World Examples
// ============================================

// Complete component behavior — reactive open state
.modal {
    .is-open: $modalOpen;
    display: none;
    opacity: 0;
    transition: opacity 300ms ease-out;
}
.modal.is-open {
    display: flex;
    opacity: 1;
}
.modal {
    @on &.click [data-close] { $modalOpen <- false; }
}

// Data-bound list with animations
// NOTE: This is a class bc you might use the .product-list in
// many places
.product-list {
    @each($products) as $product {
        :entering {
            opacity: 0;
            transform: translateY(20px);
        }
        [slot="name"]: $.name;
        [slot="price"]: $.price | currency;
    }
}
