# State & Behavior in Spacetime

## Vision

Spacetime expresses dynamic web behavior as **reactive state**: lightweight `$`-signals drive CSS classes, DOM presence, and event handlers. Instead of imperative state machines, you declare what each state *looks like* (CSS) and which events *mutate* the signal. The compiler turns this into a small signal graph plus CSS.

## Architecture

### The Full Puzzle: Three Layers

```
┌─────────────────────────────────────────────────────────────┐
│  HIGH-LEVEL (Sugar)                                         │
│  @edit, @modal, @accordion, @tabs, @form                    │
└─────────────────────────────────────────────────────────────┘
                          │ compiles to
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  PRIMITIVES                                                 │
│  reactive class    - `.is-open: $open;`                     │
│  @state(when: ...) - Per-state CSS properties               │
│  @on               - Event → signal update                  │
│  @data             - Signals from data / local state        │
└─────────────────────────────────────────────────────────────┘
                          │ compiles to
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  JS RUNTIME + CSS                                           │
│  Signal graph (JS) ←→ class / attribute / style (CSS)       │
└─────────────────────────────────────────────────────────────┘
```

### Why Signals + CSS?

- **Predictable**: state is a plain signal; derived state is a plain expression.
- **Debuggable**: inspect `$open` in devtools, no opaque machine object.
- **CSS-first**: the browser does the heavy lifting for styling and transitions.
- **Composable**: a signal can drive many selectors; a selector can read many signals.

---

## DSL Syntax Examples

### 1. Text Editing (the jallete use case)

```css
@data inline $mode string : "viewing";

.jal-prose {
    .is-viewing: $mode == "viewing";
    .is-editing: $mode == "editing";
    .is-saving:  $mode == "saving";
    .is-error:   $mode == "error";

    @on dblclick { $mode <- "editing"; }
    @on blur      { $mode <- "saving"; }

    @state(when: "viewing") {
        contenteditable: false;
    }

    @state(when: "editing") {
        contenteditable: true;
        outline: 2px dashed var(--st-teal);
    }

    @state(when: "saving") {
        opacity: 0.7;
        pointer-events: none;
    }

    @state(when: "error") {
        outline: 2px solid var(--st-red);
    }
}

/* The actual persistence and highlight wrapping are provided by the
   higher-level `@edit` sugar, which compiles to the same signal primitives. */
```

### 2. Modal Dialog

```css
@data inline $modalOpen boolean : false;

.modal-overlay {
    .is-open: $modalOpen;

    @state(when: "closed") {
        opacity: 0;
        pointer-events: none;
        visibility: hidden;
    }

    @state(when: "open") {
        opacity: 1;
        pointer-events: auto;
        visibility: visible;
    }

    @on open(300ms, &ease-out) {
        opacity: 0 -> 1;
        scale: 0.95 -> 1;
    }

    @on close(200ms, &ease-in) {
        opacity: 1 -> 0;
        scale: 1 -> 0.95;
    }
}

/* Triggers placed on the relevant controls */
.modal-trigger {
    @on &.click { $modalOpen <- true; }
}

.modal-overlay {
    @on &.click:outside { $modalOpen <- false; }
    @on &.keydown:Escape { $modalOpen <- false; }
}
```

Focus trapping and body-scroll locking are handled by the `@modal` sugar directive; the primitives underneath are just the `$modalOpen` signal and reactive CSS classes.

### 3. Accordion

```css
.accordion-item {
    @data inline $expanded boolean : false;

    .is-expanded: $expanded;

    @state(when: "collapsed") {
        --content-height: 0;
    }

    @state(when: "expanded") {
        --content-height: auto;
    }

    @on expand(400ms, &ease-out) {
        --content-height: 0 -> measured;
    }

    @on collapse(300ms, &ease-in) {
        --content-height: measured -> 0;
    }

    .accordion-header {
        @on &.click { $expanded <- !$expanded; }
    }
}

.accordion-content {
    height: var(--content-height);
    overflow: hidden;
}
```

To auto-collapse siblings, the `@accordion` sugar manages a shared `$activeItem` signal; the primitive version keeps state per item.

### 4. Form Validation

```css
@data inline $emailValid boolean : false;
@data inline $emailValidating boolean : false;

input[name="email"] {
    .is-pristine:  !$emailValidating && !$emailValid && $emailValue == "";
    .is-valid:     $emailValid && !$emailValidating;
    .is-invalid:   !$emailValid && !$emailValidating && $emailValue != "";
    .is-validating: $emailValidating;

    @on blur {
        $emailValidating <- true;
        /* validate_email($emailValue) updates $emailValid /
           $emailValidating when it resolves */
    }

    @on &.input {
        $emailValidating <- true;
    }

    @state(when: "pristine") { border-color: var(--border); }
    @state(when: "valid")    { border-color: var(--green); }
    @state(when: "invalid")  { border-color: var(--red); }
    @state(when: "validating") { border-color: var(--blue); }
}
```

### 5. Sugar Syntax (@edit)

```css
/* Sugar */
.jal-prose {
    @edit {
        highlights: jal-handwritten, jal-quote;
        persist: "index.html";
        trigger: dblclick;
    }
}

/* Compiles to reactive classes, @on handlers, and @state(when:) blocks
   like the examples above. */
```

---

## Data Binding Primitives

Spacetime includes primitives for binding external data to templates with compile-time type safety.

### @type — Define Data Shapes

```css
@type Print {
    id: string;
    title: string;
    subtitle: string;
    image: url;
    prices: {
        S: number;
        M: number;
        L: number;
    };
    featured?: boolean;     /* Optional field */
    tags?: string[];        /* Optional array */
}

@type CartItem {
    printId: string;
    size: "S" | "M" | "L";  /* Union type */
    quantity: number;
}
```

### @data — Connect to Data Sources

```css
/* JSON file */
@data fetch $prints Print[] : "/data/prints.json";

/* localStorage */
@data fetch $cart CartItem[] : localStorage("zey-cart") {
    initial: [];
}

/* Inline data */
@data inline $sizes : [
    { "code": "S", "label": "Small" },
    { "code": "M", "label": "Medium" },
    { "code": "L", "label": "Large" }
];
```

Fetch sources automatically expose `$prints_loading`, `$prints_error`, etc.

### @computed — Derived Data

> Deprecated block syntax. Prefer `@data query` / `@data fold` (see [DATA_SYSTEM.md](./DATA_SYSTEM.md)).

```css
@data query $featuredPrints Print[] from $prints {
    where: $.featured == true;
    limit: 6;
}

@data fold $cartTotal number from $cart : acc + priceFor(item.printId, item.size) * item.qty;
```

### @fn — Helper Functions

```css
@fn getPrint(id: string): Print? {
    return $prints.find(p => p.id == id);
}

@fn priceFor(printId: string, size: "S" | "M" | "L"): number {
    let print = getPrint(printId);
    return print?.prices[size] ?? 0;
}
```

### @each — Template Iteration

```css
.zey-gallery {
    @each($prints as $print) {
        template: "zey-print";

        /* Slot bindings */
        [slot="image"] {
            src: $print.image;
            alt: $print.title;
        }
        [slot="title"]: $print.title;
        [slot="subtitle"]: $print.subtitle;

        /* Host (component root) bindings */
        :host {
            data-id: $print.id;
            data-prices: $print.prices | json;
        }
    }
}
```

### Reactive State from Data

Data-loading status is itself a signal. Drive `@state(when:)` through a reactive `data-state` attribute:

```css
.gallery {
    data-state: $prints_loading ? "loading" : ($prints_error ? "error" : ($prints.length == 0 ? "empty" : "ready"));

    @state(when: "loading") {
        min-height: 400px;
        opacity: 0.5;
    }

    @state(when: "ready") {
        opacity: 1;
    }

    @state(when: "empty") {
        min-height: 200px;
    }

    @state(when: "error") {
        background: #fee;
    }

    @each($prints as $print) {
        template: "gallery-item";
        /* ... bindings ... */
    }
}
```

### Cross-References with @let

```css
.cart-items {
    @each($cart as $item) {
        template: "cart-item";

        /* Look up related data */
        @let print = getPrint($item.printId);

        [slot="image"] { src: print.image; }
        [slot="title"]: print.title;
        [slot="price"]: priceFor($item.printId, $item.size) * $item.quantity | currency("$");
    }
}
```

### Filters (Pipes)

Transform data inline:

```css
[slot="price"]: $.price | currency("$");        /* "$95.00" */
[slot="date"]: $.created | date("short");       /* "Dec 16" */
[slot="data"]: $.prices | json;                 /* '{"S":95}' */
[slot="title"]: $.title | truncate(30);         /* "Between..." */
[slot="status"]: $.inStock | if("Available", "Sold Out");
```

### Nested Iteration

```css
@type Pack {
    id: string;
    title: string;
    items: PackItem[];
}

@data fetch $packs Pack[] : "/data/packs.json";

.pack-list {
    @each($packs as $pack) {
        template: "pack-card";

        [slot="title"]: $pack.title;

        /* Nested @each */
        [slot="items"] {
            @each($pack.items as $item) {
                template: "pack-item";
                [slot="name"]: $item.name;
                [slot="price"]: $item.price | currency("dhs");
            }
        }
    }
}
```

### Multi-State DOM with @view

For mutually-exclusive states, use a string signal with `@view` to swap DOM:

```css
@data inline $step string : "shipping";

.checkout {
    @view $step {
        "shipping"  => &shipping-form();
        "payment"   => &payment-form();
        "confirm"   => &confirmation();
    }
}
```

### Further Reading

- [DATA_SYSTEM.md](./DATA_SYSTEM.md) — Complete specification
- [DATA_SYSTEM_EXAMPLES.md](./DATA_SYSTEM_EXAMPLES.md) — Full site examples
- [COMPILE_TIME_ANALYSIS.md](./COMPILE_TIME_ANALYSIS.md) — Error codes and validation

---

## Compilation Flow

```
┌─────────────────────────────────────────────────────────────┐
│  INPUT: .st file with sugar                                 │
│                                                             │
│  .jal-prose {                                               │
│      @edit { highlights: jal-handwritten; persist: "..."; } │
│  }                                                          │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼ Parse (pest)
┌─────────────────────────────────────────────────────────────┐
│  AST with EditDirective                                     │
│  EditDirective { highlights: [...], persist: "...", ... }   │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼ Desugar
┌─────────────────────────────────────────────────────────────┐
│  AST with Primitives                                        │
│  SignalDeclaration { name: "$mode", initial: "viewing" }    │
│  ReactiveClass { class: "is-editing", expr: "$mode == ..." }│
│  EventHandler { on: "dblclick", body: "$mode <- ..." }      │
│  StateDirective { when: "editing", properties: [...] }      │
└─────────────────────────────────────────────────────────────┘
                          │
          ┌───────────────┼───────────────┐
          ▼               ▼               ▼
┌─────────────┐   ┌─────────────┐   ┌─────────────┐
│ Compile CSS │   │ Compile     │   │ Compile     │
│ (states →   │   │ JS runtime  │   │ DOM hooks   │
│  classes)   │   │ (signals)   │   │ (focus etc) │
└─────────────┘   └─────────────┘   └─────────────┘
          │               │               │
          ▼               ▼               ▼
┌─────────────────────────────────────────────────────────────┐
│  OUTPUT                                                     │
│  ├── styles.css         (reactive-class + state CSS)        │
│  ├── behavior.js        (signal graph)                      │
│  └── behavior-runtime.js (DOM helpers for sugar)            │
└─────────────────────────────────────────────────────────────┘
```

---

## Generated Code Examples

### Generated CSS

```css
.jal-prose {
    outline: none;
}

.jal-prose.is-editing {
    outline: 2px dashed var(--st-teal);
    outline-offset: 4px;
}

.jal-prose.is-saving {
    opacity: 0.7;
    pointer-events: none;
}

.jal-prose.is-error {
    outline: 2px solid var(--st-red);
}
```

### Generated JS Runtime (signal graph)

```javascript
import { Signal, effect } from './behavior-runtime.js';

const $mode = new Signal("viewing");

const jalProseElements = () => document.querySelectorAll('.jal-prose');

// dblclick -> editing
document.querySelectorAll('.jal-prose').forEach(el => {
    el.addEventListener('dblclick', () => $mode.set("editing"));
});

// blur -> saving
document.querySelectorAll('.jal-prose').forEach(el => {
    el.addEventListener('blur', () => $mode.set("saving"), true);
});

// reactive class: is-editing
$mode.subscribe(mode => {
    jalProseElements().forEach(el => {
        el.classList.toggle('is-editing', mode === "editing");
        el.classList.toggle('is-saving',  mode === "saving");
        el.classList.toggle('is-error',   mode === "error");
        el.dataset.stState = mode;
    });
});

// contenteditable is handled by a focused DOM helper registered by @edit
function syncContentEditable(mode) {
    jalProseElements().forEach(el => {
        el.contentEditable = (mode === "editing") ? "true" : "false";
    });
}
$mode.subscribe(syncContentEditable);
```

### Sugar Runtime Helpers

```javascript
class SpacetimeBehavior {
    constructor() {
        this.highlightClasses = ["jal-handwritten", "jal-quote"];
    }

    toggleHighlight(className) {
        const selection = window.getSelection();
        if (!selection.rangeCount) return;
        const range = selection.getRangeAt(0);

        const existing = range.commonAncestorContainer.parentElement?.closest(`.${className}`);
        if (existing) {
            const text = existing.textContent;
            existing.replaceWith(document.createTextNode(text));
        } else {
            const span = document.createElement('span');
            span.className = className;
            range.surroundContents(span);
        }
    }

    async saveToServer(target, content, elementId) {
        try {
            const res = await fetch('/__spacetime/save-html', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ file_path: target, content, elementId })
            });

            if (res.ok) {
                $mode.set("viewing");
            } else {
                throw new Error(await res.text());
            }
        } catch (e) {
            $mode.set("error");
        }
    }
}
```

---

## Retired: the imperative `@state_machine` / `@transition` (PLAN-047)

Spacetime previously shipped an imperative finite-state-machine construct:

- `@state_machine(initial: "x")`
- `@transition(from: "a", to: "b", on: event)`
- `@mutate on_enter("x")` / `@mutate on_exit(...)`

It was removed in PLAN-047 because the macro expansion never wired event listeners (BUG-117), leaving transitions dead in production. The replacement is the reactive-signal idiom shown throughout this document: `$`-signals, reactive classes, `@on` handlers, and `@state(when:)` for CSS rendering.

If you encounter old examples using these directives, rewrite them as a signal plus reactive classes. For N mutually-exclusive states, use a string signal with per-state reactive classes or `@view` for DOM dispatch.
