// Scope block syntax reference file
// Shows .selector { content } structure

// ============================================
// Basic Scope Blocks
// ============================================

.card {
    // CSS declarations
    opacity: 1;
    transform: none;

    // Spacetime value declarations
    $opacity number: 1;
    $scale number: 1;
}

#header {
    background: #fff;
    $height number: 60;
}

button {
    cursor: pointer;
    $pressed boolean: false;
}

// ============================================
// Element Reference Declarations
// ============================================

.modal {
    // Simple element ref
    &backdrop .modal-backdrop;

    // Element ref with selector
    &closeBtn .modal > .close-button;

    // Element ref with inline scope
    &content .modal-content {
        $visible boolean: false;
        opacity: 0;
    }
}

// ============================================
// Value Declarations
// ============================================

.counter {
    // Typed value with initial
    $count number: 0;

    // Uninitialized typed value
    $result number?;

    // Boolean value
    $active boolean: false;

    // String value
    $label string: "Count";

    // Array value
    $items Item[]: [];

    // Object-like initial value
    $config object: { timeout: 1000, retries: 3 };
}

// ============================================
// Nested Scopes
// ============================================

.card {
    background: white;

    // Child combinator
    > .header {
        font-weight: bold;
    }

    // Descendant
    .content {
        padding: 16px;
    }

    // Class modifier
    &.featured {
        border: 2px solid gold;
    }

    // Pseudo-class
    :hover {
        transform: scale(1.02);
    }

    // Pseudo-element
    ::before {
        content: "";
    }
}

// ============================================
// @on Mutation Blocks
// ============================================

.button {
    $count number: 0;

    @on &.click {
        $count <- $count + 1;
    }

    @on &.mouseenter {
        $hovered <- true;
    }

    @on &.mouseleave {
        $hovered <- false;
    }
}

// ============================================
// Mixed Content
// ============================================

.interactive-card {
    // CSS
    opacity: 1;
    transform: none;
    transition: all 0.3s;

    // Values
    $expanded boolean: false;
    $progress number: 0;

    // Element refs
    &header .card-header;
    &body .card-body;

    // Nested scopes
    > .header {
        cursor: pointer;

        @on &.click {
            $expanded <- !$expanded;
        }
    }

    // Macro calls
    @on visible expand(300ms) {
        height: 0 -> auto;
    }

    @state-machine idle {
        @state idle { }
        @state expanded { }
    }
}

// ============================================
// Behavior Directives (in scope)
// ============================================

.toggle-button {
    @state-machine off {
        on, off
    }

    @state on {
        background: green;
    }

    @state off {
        background: gray;
    }

    @transition on <-> off {
        duration: 200ms;
    }

    @on &.click {
        $toggled <- true;
    }
}

// ============================================
// Value Declaration Variants
// ============================================

.value-examples {
    // Untyped value (inferred)
    $simple: 42;

    // Typed with value
    $typed number: 100;

    // Optional (uninitialized)
    $optional number?;

    // Complex types
    $list string[]: ["a", "b", "c"];
}

// ============================================
// Selector List (Multiple Selectors)
// ============================================

h1, h2, h3 {
    font-weight: bold;
    $level number: 1;
}

.btn, .button, [role="button"] {
    cursor: pointer;
    user-select: none;
}
