# Spacetime Architecture

**A comprehensive guide to understanding the entire Spacetime system**

---

## Table of Contents

1. [The Puzzle: Why Spacetime?](#1-the-puzzle-why-spacetime)
2. [The Three Layers](#2-the-three-layers)
3. [The Data Binding Layer](#3-the-data-binding-layer)
4. [The Core Insight](#4-the-core-insight)
5. [The Hybrid Architecture](#5-the-hybrid-architecture)
6. [The Compilation Pipeline](#6-the-compilation-pipeline)
7. [Key Abstractions](#7-key-abstractions)
8. [How Sugar Works](#8-how-sugar-works)
9. [Timeline Types Deep Dive](#9-timeline-types-deep-dive)
10. [Runtime Execution Model](#10-runtime-execution-model)
11. [Performance Considerations](#11-performance-considerations)
12. [Internal: Data Binding Metasystem](#12-internal-data-binding-metasystem)

---

## 1. The Puzzle: Why Spacetime?

### The Problem

Web developers face a fundamental challenge: **dynamic behavior is scattered across multiple paradigms**:

- **Animations**: CSS transitions, CSS animations, WAAPI, GSAP, Framer Motion
- **State management**: React state, Vue reactivity, Svelte stores, vanilla JS
- **DOM mutations**: Imperative JavaScript, framework-specific patterns
- **Async operations**: Promises, async/await, callbacks, signal-driven updates
- **User interactions**: Event listeners, click handlers, form validation

Each paradigm has its own mental model, API surface, and debugging strategy. This fragmentation leads to:
- **Cognitive overhead**: Switching between paradigms constantly
- **Boilerplate**: Repetitive code for common patterns (modals, accordions, editing)
- **Bugs**: State synchronization issues, race conditions, memory leaks
- **Poor DX**: Debugging requires jumping between browser devtools, React devtools, etc.

### The Vision

**Spacetime unifies all dynamic behavior under a single declarative paradigm: timelines.**

From animations (continuous progress 0→1) to behaviors (discrete state transitions), everything is expressed as a function of timeline progress. This isn't just about animations—it's about creating a **universal abstraction for dynamic web behavior**.

```
Current fragmented approach:
  CSS animations + React state + event listeners + async logic + DOM APIs

Spacetime unified approach:
  .st files → Compiled CSS/WASM/JS → Declarative behavior
```

---

## 2. The Three Layers

Spacetime's architecture is organized into three distinct layers, each serving a specific purpose:

```
┌─────────────────────────────────────────────────────────────┐
│  LAYER 1: HIGH-LEVEL SUGAR (Developer Experience)          │
│                                                             │
│  @edit      - Inline content editing with persistence      │
│  @modal     - Modal dialogs with focus management          │
│  @accordion - Expandable sections with auto-collapse       │
│  @tabs      - Tab interfaces with keyboard navigation      │
│  @form      - Form validation with async operations        │
│                                                             │
│  These are HIGH-LEVEL directives that compile down         │
│  to primitive operations. Optimized for common patterns.   │
└─────────────────────────────────────────────────────────────┘
                          │
┌─────────────────────────────────────────────────────────────┐
│  LAYER 2: PRIMITIVES (Core Building Blocks)                │
│                                                             │
│  reactive class     - `.is-open: $open;`                   │
│  @state(when: ...)  - CSS properties per state             │
│  @on                - Event → signal update                │
│  @data              - Signals from data / local state      │
│                                                             │
│  These are the FUNDAMENTAL operations. All behavior        │
│  can be expressed using these primitives.                  │
└─────────────────────────────────────────────────────────────┘
                          │
                          │ Compiles to (Code Generation)
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  LAYER 3: RUNTIME (Execution)                              │
│                                                             │
│  ┌───────────────────┐          ┌──────────────────────┐   │
│  │   CSS Output      │          │   JS Signal Graph    │   │
│  │                   │          │                      │   │
│  │ • Reactive classes│          │ • $-signals          │   │
│  │ • Keyframes       │    ↔     │ • Event handlers     │   │
│  │ • CSS variables   │          │ • DOM helpers        │   │
│  └───────────────────┘          └──────────────────────┘   │
│                                           ↕                 │
│                          ┌──────────────────────┐           │
│                          │   DOM Runtime Glue   │           │
│                          │                      │           │
│                          │ • class toggling     │           │
│                          │ • Event listeners    │           │
│                          │ • Focus management   │           │
│                          │ • contenteditable    │           │
│                          └──────────────────────┘           │
└─────────────────────────────────────────────────────────────┘
```

### Why This Layering?

1. **Layer 1 (Sugar)** provides ergonomic APIs for common patterns
2. **Layer 2 (Primitives)** ensures power users can build anything
3. **Layer 3 (Runtime)** optimizes for performance and browser compatibility

You can work at any layer depending on your needs:
- **Beginners**: Use sugar directives like `@edit` and `@modal`
- **Advanced**: Compose primitives for custom behavior
- **Power users**: Extend the compiler with new sugar directives

---

## 3. The Data Binding Layer

Spacetime extends the three-layer model with a **data binding system** that connects external data to templates.

### The Problem

Dynamic content often requires repetitive HTML:

```html
<!-- Repeated 23 times for each print -->
<zey-print data-id="IN-003" data-prices='{"S":95}'>
  <img slot="image" src="/prints/IN-003.jpg" alt="Between Worlds">
  <span slot="title">Between Worlds</span>
  <span slot="subtitle">Surfer in morning mist</span>
</zey-print>
```

This is error-prone, hard to maintain, and not scalable.

### The Solution

Data binding adds a fourth concern to Spacetime:
```
┌─────────────────────────────────────────────────────────────┐
│  LAYER 0: DATA BINDING (Type-Safe Templates)               │
│                                                             │
│  @type       - Define data shapes with compile-time safety │
│  @data       - Connect to JSON, localStorage, APIs         │
│  @data query - Derived / filtered arrays                   │
│  @fn         - Helper functions for cross-references       │
│  @each       - Template iteration with slot binding        │
│                                                             │
│  These integrate with Layer 1-3 for full reactive UI.      │
└─────────────────────────────────────────────────────────────┘
                          │
                          │ Feeds data into
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  LAYER 1: SUGAR (+ data-aware directives)                  │
│                                                             │
│  @edit, @modal, etc. can now bind to data sources          │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  LAYER 2: PRIMITIVES (+ reactive status)                   │
│                                                             │
│  `$data_loading` / `$data_error` drive reactive classes     │
│  and `@state(when:)` without imperative state machines.    │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  LAYER 3: RUNTIME (+ data loading & template instantiation)│
│                                                             │
│  • Fetch JSON at runtime                                   │
│  • Instantiate templates for each data item                │
│  • Apply slot bindings                                     │
│  • Update signals that feed reactive classes               │
└─────────────────────────────────────────────────────────────┘

### How It Works

```css
/* 1. Define the data shape */
@type Print {
    id: string;
    title: string;
    subtitle: string;
    image: url;
    prices: { S: number; M: number; L: number; };
}

/* 2. Connect to data source */
@data prints: Print[] {
    src: "/data/prints.json";
}

/* 3. Bind data to templates */
.zey-gallery {
    $loadState <- "loading";

    .is-loading: $loadState == "loading";
    .is-ready:   $loadState == "ready";

    .is-loading { opacity: 0.5; }
    .is-ready   { opacity: 1; }

    @on data:prints:loaded { $loadState <- "ready"; }

    @each($prints as $print) {
        template: "zey-print";

        [slot="image"] { src: $print.image; alt: $print.title; }
        [slot="title"]: $print.title;
        [slot="subtitle"]: $print.subtitle;

        :host {
            data-id: $print.id;
            data-prices: $print.prices | json;
        }
    }

    /* 4. Animations work on generated elements */
    > zey-print {
        @scroll reveal(&quick-reveal) {
            opacity: 0 -> 1;
            stagger: 0.08 first;
        }
    }
}
```

### Compilation Flow

```
┌──────────────────────────────────────────────────────────┐
│  INPUT: .st file with @type, @data, @each               │
└──────────────────────────────────────────────────────────┘
                          │
                          │ Step 1: Parse types
                          ▼
┌──────────────────────────────────────────────────────────┐
│  TYPE REGISTRY                                           │
│  • Print { id: string, title: string, ... }             │
│  • Curation { id: string, slug: string, ... }           │
└──────────────────────────────────────────────────────────┘
                          │
                          │ Step 2: Validate JSON
                          ▼
┌──────────────────────────────────────────────────────────┐
│  JSON SCHEMA VALIDATION                                  │
│  • /data/prints.json validates against Print[]          │
│  • Missing fields → compile error                       │
│  • Wrong types → compile error                          │
└──────────────────────────────────────────────────────────┘
                          │
                          │ Step 3: Validate bindings
                          ▼
┌──────────────────────────────────────────────────────────┐
│  BINDING VALIDATION                                      │
│  • $.title exists on Print ✓                            │
│  • $.price does not exist → error                       │
│  • template "zey-print" exists ✓                        │
│  • slot "image" exists in template ✓                    │
└──────────────────────────────────────────────────────────┘
                          │
                          │ Step 4: Generate code
                          ▼
┌──────────────────────────────────────────────────────────┐
│  OUTPUT                                                  │
│                                                          │
│  CSS: Reactive class rules, animations                   │
│  JS: Data loading, template instantiation, binding       │
│  Report: Types, sources, bindings, warnings              │
└──────────────────────────────────────────────────────────┘
```

### Key Benefits

1. **Type Safety**: Catch property typos at compile time
2. **Schema Validation**: JSON files validated against types
3. **Orphan Detection**: Unused templates/data sources flagged
4. **Single Source of Truth**: Data in JSON, templates in HTML, behavior in .st
5. **Animation Integration**: Generated elements participate in scroll/hover animations

### Further Reading

- [DATA_SYSTEM.md](./DATA_SYSTEM.md) - Complete data binding specification
- [DATA_SYSTEM_EXAMPLES.md](./DATA_SYSTEM_EXAMPLES.md) - Examples for each site
- [COMPILE_TIME_ANALYSIS.md](./COMPILE_TIME_ANALYSIS.md) - Error codes and validation

---

## 4. The Core Insight

### Animations and Behaviors Share the Same Paradigm

At first glance, animations and signal-driven state seem fundamentally different:

**Animations** (Continuous):
```
progress: 0 ────────────────────────────► 1
           ↓
        opacity: 0 → 0.5 → 1
        scale: 0.9 → 1
```

**Behaviors** (Discrete):
```
State: VIEWING → EDITING → SAVING → VIEWING
         ↓         ↓         ↓
       CSS      CSS +     CSS +
       rules   mutations  async
```

**But both are functions of timeline progress!**
```javascript
// Animation: continuous interpolation
value = interpolate(keyframes, progress)

// Behavior: signal-driven state
$open = false -> true   // instant jump
       ↓
    progress jumps from 0 → 1 (state change)
```

The key insight: **signal changes are just animations with instant progress jumps**. A click event doesn't gradually move from 0→1; it updates a `$`-signal immediately. But the *result* of that jump can still be animated:

```css
.modal {
    $open <- false;

    .is-open: $open;

    /* Closed state (progress = 0) */
    opacity: 0;
    pointer-events: none;
    transform: scale(0.95);

    /* Open state (progress = 1) */
    &.is-open {
        opacity: 1;
        pointer-events: auto;
        transform: scale(1);
    }

    /* Animate the properties driven by the reactive class */
    @transition { opacity 300ms, transform 300ms; }

    /* Trigger: click toggles the signal */
    @on &.click { $open <- !$open; }
}
```

This unification means:
- One mental model for all dynamic behavior
- Reuse the same runtime for animations and reactive state
- Debug everything with signals and timeline progress as the common primitives

---

## 5. The Hybrid Architecture

### Why Hybrid? (WASM + JS)

Spacetime uses a **hybrid architecture** that plays to each platform's strengths:

```
┌──────────────────────────────────────────────────────────────┐
│  WASM Module (Rust compiled to WebAssembly)                  │
│                                                              │
│  PURE LOGIC (No side effects):                              │
│  • State machine transitions                                │
│  • Event routing                                            │
│  • Mutation rule matching                                   │
│  • Async operation coordination                             │
│                                                              │
│  Emits COMMANDS:                                            │
│    { type: "set_state", elementId, state: "editing" }       │
│    { type: "apply_focus", elementId, selector: "input" }    │
│    { type: "save_content", url, data }                      │
└──────────────────────────────────────────────────────────────┘
                         │                    ▲
                         │ Commands           │ Events
                         ▼                    │
┌──────────────────────────────────────────────────────────────┐
│  JavaScript Runtime (Thin Glue Layer)                        │
│                                                              │
│  SIDE EFFECTS (DOM, Network):                               │
│  • DOM mutations (contenteditable, classList, innerHTML)    │
│  • Focus management (element.focus(), focus trap)           │
│  • Event listeners (click, blur, keydown)                   │
│  • Selection API (window.getSelection())                    │
│  • Network requests (fetch for persistence)                 │
│                                                              │
│  Sends EVENTS to WASM:                                      │
│    wasm.on_click(elementId)                                 │
│    wasm.on_blur(elementId, content)                         │
│    wasm.on_save_success(elementId)                          │
└──────────────────────────────────────────────────────────────┘
```

### Why This Split?

| Concern | WASM | JavaScript |
|---------|------|------------|
| **State machine logic** | ✅ Pure functions, testable | ❌ Complex imperative code |
| **DOM manipulation** | ❌ FFI overhead | ✅ Native, fast |
| **Type safety** | ✅ Rust type system | ❌ Runtime errors |
| **Debugging** | ❌ Limited devtools | ✅ Browser devtools |
| **Performance** | ✅ Fast compute | ✅ Fast DOM access |

**Benefits:**
- **Testability**: State machines are pure Rust functions (unit testable)
- **Performance**: No FFI overhead for frequent DOM updates
- **Debugging**: Browser devtools work naturally for DOM/events
- **Separation of concerns**: Logic vs. effects

### Command/Event Flow

1. **User clicks element**
   ```
   Browser → JS event listener → wasm.on_click(id)
   ```

2. **WASM processes event**
   ```rust
   pub fn on_click(&mut self, id: String) -> Option<Command> {
       if self.state == State::Closed {
           self.state = State::Open;
           Some(Command::SetState { id, state: "open" })
       } else {
           None
       }
   }
   ```

3. **JS executes command**
   ```javascript
   function executeCommand(cmd) {
       const el = document.querySelector(`[data-st-id="${cmd.id}"]`);
       el.dataset.stState = cmd.state;
       el.focus(); // Side effect
   }
   ```

---

## 6. The Compilation Pipeline

The compiler transforms high-level `.st` files into executable CSS, WASM, and JS:

```
┌─────────────────────────────────────────────────────────────┐
│  INPUT: .st file                                            │
│                                                             │
│  .jal-prose {                                               │
│      @edit {                                                │
│          highlights: jal-handwritten, jal-quote;            │
│          persist: "index.html";                             │
│          trigger: dblclick;                                 │
│      }                                                      │
│  }                                                          │
└─────────────────────────────────────────────────────────────┘
                          │
                          │ Step 1: Parse (pest grammar)
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  AST (Abstract Syntax Tree)                                 │
│                                                             │
│  StFile {                                                   │
│    scopes: [                                                │
│      ScopeBlock {                                           │
│        selector: ".jal-prose",                              │
│        timelines: [                                         │
│          EditDirective {                                    │
│            highlights: ["jal-handwritten", "jal-quote"],    │
│            persist: "index.html",                           │
│            trigger: Dblclick                                │
│          }                                                  │
│        ]                                                    │
│      }                                                      │
│    ]                                                        │
│  }                                                          │
└─────────────────────────────────────────────────────────────┘
                          │
                          │ Step 2: Desugar (transform sugar → primitives)
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  Expanded AST (Primitives)                                  │
│                                                             │
│  • Signal { name: "$mode", initial: "viewing" }             │
│  • ReactiveClass { class: "is-editing", expr: ... }         │
│  • OnHandler { event: "dblclick", body: "$mode <- ..." }    │
│  • OnHandler { event: "blur",   body: "$mode <- ..." }      │
│  • DomHelper { name: "focus", ... }                         │
└─────────────────────────────────────────────────────────────┘
                          │
          ┌───────────────┼────────────────┐
          │               │                │
          ▼               ▼                ▼
     ┌─────────┐    ┌──────────┐    ┌──────────┐
     │  CSS    │    │  WASM    │    │    JS    │
     │ Codegen │    │  Codegen │    │  Codegen │
     └─────────┘    └──────────┘    └──────────┘
          │               │                │
          ▼               ▼                ▼
┌─────────────────────────────────────────────────────────────┐
│  OUTPUT                                                     │
│                                                             │
│  styles.css:                                                │
│    .jal-prose.is-viewing { ... }                            │
│    .jal-prose.is-editing {                                  │
│        outline: 2px dashed var(--st-teal);                  │
│    }                                                        │
│                                                             │
│  behavior.wasm:                                             │
│    Signal graph + event handler dispatch                    │
│                                                             │
│  runtime.js:                                                │
│    SpacetimeBehavior class that binds DOM events           │
└─────────────────────────────────────────────────────────────┘
```

### Parsing (pest)

Spacetime uses the [pest](https://pest.rs/) parser generator with a PEG grammar defined in `src/parser/grammar.pest`:

```pest
// Simplified example
st_file = { SOI ~ (import_stmt | preset_def | scope_block)* ~ EOI }

scope_block = { selector ~ "{" ~ timeline* ~ "}" }

timeline = {
    "@scroll" ~ id ~ config? ~ "{" ~ animation* ~ "}" |
    "@edit" ~ config? ~ "{" ~ edit_options* ~ "}"
}

animation = { selector ~ "{" ~ property* ~ "}" }
property = { ident ~ ":" ~ value ~ "->" ~ value ~ ";" }
```

The parser produces an AST (`StFile`, `ScopeBlock`, etc.) with source location spans for error reporting.

### Desugaring

The `transform` module expands sugar directives into primitives:

```rust
// Pseudo-code
fn desugar_edit(edit: EditDirective) -> Vec<Primitive> {
    vec![
        Primitive::Signal {
            name: "$mode",
            initial: "viewing"
        },
        Primitive::ReactiveClass {
            class: "is-editing",
            expr: "$mode == \"editing\""
        },
        Primitive::ReactiveClass {
            class: "is-saving",
            expr: "$mode == \"saving\""
        },
        Primitive::ReactiveClass {
            class: "is-error",
            expr: "$mode == \"error\""
        },
        Primitive::OnHandler {
            event: edit.trigger, // e.g., "dblclick"
            body: "$mode <- \"editing\""
        },
        Primitive::OnHandler {
            event: "blur",
            body: "$mode <- \"saving\""
        },
        Primitive::DomHelper { name: "focus", ... },
        Primitive::DomHelper { name: "toggleHighlight", ... },
    ]
}
```


### Code Generation

Three separate codegen modules produce the final output:

1. **CSS Codegen** (`compiler.rs`):
   - Generates reactive-class CSS rules
   - Creates `@keyframes` for animations
   - Sets up CSS custom properties for timeline progress

2. **WASM Codegen** (future):
   - Generates signal graph and event dispatch
   - Compiles reactive expressions to `.wasm`

3. **JS Codegen** (current `compiler.rs`):
   - Creates runtime that manages timelines
   - Binds event listeners to signal updates
   - Executes DOM helpers
   - Updates CSS variables based on scroll/time/events

---

## 7. Key Abstractions

### Timeline

**The fundamental unit of all dynamic behavior.**

```rust
pub struct Timeline {
    pub id: String,
    pub driver: TimelineDriver,  // What controls progress
    pub scope: Option<String>,   // CSS selector for scope element
    pub animations: Vec<Animation>,
    pub children: Vec<Timeline>, // Nested timelines
}
```

**Timeline Progress**: Always normalized to 0→1
- `0.0` = start of timeline
- `0.5` = halfway through
- `1.0` = complete

**Exposed as CSS variable**: `--st-{id}-progress`

### TimelineDriver

**What drives the timeline's progress:**

```rust
pub enum TimelineDriver {
    Scroll { start, end, trigger, scrub },
    Time { duration_ms, delay_ms, iterations, alternate },
    Loop { period_ms, phase },
    Mouse { relative_to, axis },
    Event { trigger, duration_ms },
    Load { duration_ms, delay_ms, threshold, once },
    After { trigger_timeline, delay_ms, trigger_event },
    ValueChange { duration_ms, char_stagger_ms },
}
```

Each driver type updates timeline progress differently:
- **Scroll**: Progress based on element visibility in viewport
- **Time**: Progress based on elapsed milliseconds
- **Loop**: Sinusoidal progress that oscillates
- **Mouse**: Progress based on cursor position
- **Event**: Discrete 0→1 or 1→0 on user events
- **Load**: One-shot 0→1 when element enters viewport
- **After**: Triggered by another timeline's completion
- **ValueChange**: Per-character flip animations on text changes

### Animation

**Transforms CSS properties based on timeline progress:**

```rust
pub struct Animation {
    pub targets: String,              // CSS selector
    pub properties: Vec<PropertyAnimation>,
    pub offset: f64,                  // When to start (0-1)
    pub span: f64,                    // Duration as % of timeline (0-1)
    pub stagger: Option<StaggerConfig>,
    pub color_space: Option<String>,  // rgb, hsl, oklch
}
```

**Example:**
```css
.hero {
    @scroll entrance {
        .title {
            opacity: 0 -> 1;
            translate-y: 40px -> 0;
            range: 0.2 to 0.7;  // offset: 0.2, span: 0.5
        }
    }
}
```

This means:
- When timeline progress = 0.2, title opacity = 0
- When timeline progress = 0.7, title opacity = 1
- Outside that range, values are clamped to 0 or 1

### PropertyAnimation

**Defines how a single CSS property changes:**

```rust
pub struct PropertyAnimation {
    pub property: String,        // "opacity", "translateY", etc.
    pub keyframes: Vec<Keyframe>,
    pub easing: Easing,
}

pub struct Keyframe {
    pub at: f64,          // Progress point (0-1)
    pub value: Value,     // Number, String, or Calc expression
    pub easing: Option<Easing>,  // Per-keyframe easing override
}
```

**Multi-step animations:**
```css
translate-y: 0 -> -100px -> 0;
// Becomes:
// Keyframe { at: 0.0, value: "0" }
// Keyframe { at: 0.5, value: "-100px" }
// Keyframe { at: 1.0, value: "0" }
```

### Stagger

**Delays each element in a selection:**

```rust
pub struct StaggerConfig {
    pub delay: f64,              // Delay per element (in progress units)
    pub from: StaggerFrom,       // Direction
    pub grid: Option<(u32, u32)>, // Grid dimensions
}

pub enum StaggerFrom {
    First,   // First to last
    Last,    // Last to first
    Center,  // Center outward
    Random,  // Random order
    Index(usize), // From specific index
}
```

**Runtime calculation:**
```javascript
function calculateStagger(index, total, stagger) {
    let position;
    switch (stagger.from) {
        case 'first': position = index / total; break;
        case 'center': position = Math.abs(index - total/2) / (total/2); break;
        // ...
    }
    return position * stagger.delay;
}
```

### Signal

**A reactive value that can be read in expressions and updated by event handlers:**

```rust
pub struct Signal {
    pub name: String,       // e.g. "$open"
    pub initial: Expr,      // initial value
    pub source: SignalSource,
}

pub enum SignalSource {
    Inline,          // @data inline $x : ...
    Fetch,           // @data fetch $x ...
    Query,           // @data query $x ...
    Fold,            // @data fold $x ...
}
```

**Example:**
```css
@data inline $open boolean : false;

.modal {
    @on &.click { $open <- !$open; }
}
```

### ReactiveClass

**A CSS class that is toggled based on a signal expression:**

```rust
pub struct ReactiveClass {
    pub class: String,   // e.g. "is-open"
    pub expr: Expr,      // e.g. "$open" or "$mode == 'editing'"
}
```

**Example:**
```css
.modal {
    .is-open: $open;
    .is-editing: $mode == "editing";
}
```

### EventHandler

**An `@on` block that updates one or more signals when an event fires:**

```rust
pub struct EventHandler {
    pub event: EventTrigger,  // e.g. "click", "click:.trigger", "keydown:Escape"
    pub body: Vec<Stmt>,      // signal assignments
}
```

**Example:**
```css
.modal-trigger {
    @on &.click { $modalOpen <- true; }
}

.modal-overlay {
    @on &.click:outside { $modalOpen <- false; }
    @on &.keydown:Escape { $modalOpen <- false; }
}
```

### StateDirective

**Per-state CSS properties rendered when an element's `data-state` matches:**

```rust
pub struct StateDirective {
    pub when: String,  // State name
    pub properties: Vec<(String, String)>, // CSS properties
}
```

**Example:**
```css
.modal {
    @state(when: "closed") {
        opacity: 0;
        visibility: hidden;
    }

    @state(when: "open") {
        opacity: 1;
        visibility: visible;
    }
}
```

### ViewDirective (multi-state DOM)

**For mutually-exclusive states, swap DOM based on a signal value:**

```rust
pub struct ViewDirective {
    pub signal: String,
    pub arms: Vec<(String, TemplateCall)>,
}
```

**Example:**
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

> **Retired construct (PLAN-047).** Spacetime previously had an imperative finite-state-machine directive set. It was removed because the macro expansion never wired event listeners (BUG-117). The reactive-signal primitives above are the replacement.

---

## 8. How Sugar Works

Let's trace how `@edit` expands to signal-driven primitives:

### Input (Sugar)

```css
.jal-prose {
    @edit {
        highlights: jal-handwritten, jal-quote;
        persist: "index.html";
        trigger: dblclick;
    }
}
```

### Desugared (Primitives)

```css
.jal-prose {
    // Signal: current editing mode
    $mode <- "viewing";

    // Reactive classes derived from the signal
    .is-viewing: $mode == "viewing";
    .is-editing: $mode == "editing";
    .is-saving:  $mode == "saving";
    .is-error:   $mode == "error";

    // State styles applied via reactive classes
    &.is-viewing { cursor: text; }
    &.is-editing {
        outline: 2px dashed var(--st-teal);
        outline-offset: 4px;
    }
    &.is-saving {
        opacity: 0.7;
        pointer-events: none;
    }
    &.is-error {
        outline: 2px solid var(--st-red);
    }

    // Event handlers update the signal
    @on dblclick { $mode <- "editing"; }
    @on blur     { $mode <- "saving"; }

    // Async completion events are emitted by the runtime glue
    @on save:success { $mode <- "viewing"; }
    @on save:error   { $mode <- "error"; }
}
```

### Generated CSS

```css
.jal-prose.is-viewing { cursor: text; }

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

### Generated Runtime (Signal Graph)

```rust
pub struct JalProseBehavior {
    element_id: String,
    mode: Signal<String>,
}

impl JalProseBehavior {
    pub fn new(element_id: String) -> Self {
        Self {
            element_id,
            mode: Signal::new("viewing"),
        }
    }

    pub fn on_dblclick(&self) {
        self.mode.set("editing");
    }

    pub fn on_blur(&self) {
        self.mode.set("saving");
        // Runtime glue calls save_content() and emits save:success / save:error
    }

    pub fn on_save_success(&self) {
        self.mode.set("viewing");
    }

    pub fn on_save_error(&self) {
        self.mode.set("error");
    }
}
```

### Generated JS Glue

```javascript
class Signal {
    constructor(initial) {
        this._value = initial;
        this._listeners = new Set();
    }
    get() { return this._value; }
    set(value) {
        if (this._value === value) return;
        this._value = value;
        this._listeners.forEach(cb => cb(value));
    }
    subscribe(cb) {
        this._listeners.add(cb);
        return () => this._listeners.delete(cb);
    }
}

class SpacetimeBehavior {
    constructor() {
        this.elements = new Map();
    }

    bindElements() {
        document.querySelectorAll('[data-st-edit]').forEach(el => {
            const id = el.dataset.stId;
            const $mode = new Signal('viewing');
            this.elements.set(id, { element: el, $mode });

            // Reactive class bindings
            $mode.subscribe(mode => {
                el.classList.toggle('is-viewing', mode === 'viewing');
                el.classList.toggle('is-editing', mode === 'editing');
                el.classList.toggle('is-saving',  mode === 'saving');
                el.classList.toggle('is-error',   mode === 'error');
                el.contentEditable = mode === 'editing' ? 'true' : 'false';
                if (mode === 'editing') this.showToolbar(el);
                if (mode === 'viewing') this.hideToolbar();
            });

            // Bind event listeners
            el.addEventListener('dblclick', () => {
                $mode.set('editing');
                el.focus();
            });

            el.addEventListener('blur', () => {
                $mode.set('saving');
                this.saveToServer('index.html', el.innerHTML, id, $mode);
            });
        });
    }

    async saveToServer(target, content, elementId, $mode) {
        try {
            const res = await fetch('/__spacetime/save-html', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ file_path: target, content, elementId })
            });

            if (res.ok) {
                $mode.set('viewing');
                this.hideToolbar();
            } else {
                throw new Error(await res.text());
            }
        } catch (e) {
            $mode.set('error');
            console.error('Save failed:', e);
        }
    }

    showToolbar(el) { /* mount formatting toolbar */ }
    hideToolbar()  { /* remove formatting toolbar */ }

    toggleHighlight(className) {
        const selection = window.getSelection();
        if (!selection.rangeCount) return;
        const range = selection.getRangeAt(0);

        const existing = range.commonAncestorContainer.parentElement
            ?.closest(`.${className}`);
        if (existing) {
            const text = existing.textContent;
            existing.replaceWith(document.createTextNode(text));
        } else {
            const span = document.createElement('span');
            span.className = className;
            range.surroundContents(span);
        }
    }
}

new SpacetimeBehavior().bindElements();
```

---
## 9. Timeline Types Deep Dive

### Scroll Timelines

**Progress based on element visibility in viewport:**

```
Viewport:
┌──────────────────┐  ← top (1.0)
│                  │
│                  │  ← center (0.5)
│                  │
│                  │
└──────────────────┘  ← bottom (0.0)

Element scrolling up:
     ▲
     │ scroll direction
     │
┌────────────┐
│  element   │  ← element.top
│            │
│            │
└────────────┘  ← element.bottom
```

**Configuration:**
- `start`: When animation begins (element position in viewport)
- `end`: When animation completes
- `scrub`: Sync to scroll position (true) or trigger once (false)

**Progress calculation:**
```javascript
const rect = element.getBoundingClientRect();
const vh = window.innerHeight;

const startTrigger = vh - (start * vh);
const endTrigger = vh - (end * vh);

const totalRange = (startTrigger - endTrigger) + rect.height;
const scrolled = startTrigger - rect.top;
const progress = scrolled / totalRange;
```

**Example:**
```css
.hero {
    @scroll entrance(&quick-reveal) {
        /* start: 0, end: 0.5 */
        .title {
            opacity: 0 -> 1;
            translate-y: 40px -> 0;
        }
    }
}
```

### Time Timelines

**Progress based on elapsed milliseconds:**

```javascript
const animate = (timestamp) => {
    const elapsed = timestamp - startTime;
    const progress = Math.min(1, elapsed / durationMs);
    setProgress(progress);

    if (progress < 1) {
        requestAnimationFrame(animate);
    }
};
```

**Supports:**
- `iterations`: Number of loops (infinite if omitted)
- `alternate`: Reverse direction on each iteration
- `delay_ms`: Initial delay before starting

**Example:**
```css
.modal {
    @on visible entrance(800ms) {
        opacity: 0 -> 1;
        scale: 0.95 -> 1;
    }
}
```

### Loop Timelines

**Continuous cyclical motion:**

```javascript
const animate = () => {
    const elapsed = performance.now() - startTime;
    const t = (elapsed / periodMs) * Math.PI * 2;
    const progress = (Math.sin(t) + 1) / 2; // 0→1→0
    setProgress(progress);
    requestAnimationFrame(animate);
};
```

**Example:**
```css
.orb {
    @loop float(8s) {
        translate-y: 0 -> -30px -> 0;
        easing: &ease-in-out-quad;
    }
}
```

### Event Timelines

**Triggered by user interactions:**

```javascript
element.addEventListener('mouseenter', () => {
    // Animate from current progress to 1
    animateTo(1, durationMs);
});

element.addEventListener('mouseleave', () => {
    // Animate from current progress to 0
    animateTo(0, durationMs);
});
```

**Supported events:**
- `hover`: Mouse enter/leave
- `click`: Toggle on click
- `focus`: Focus/blur

**Example:**
```css
.button {
    @on &.hover lift(300ms) {
        scale: 1 -> 1.05;
        translate-y: 0 -> -4px;
    }
}
```

### After Timelines

**Triggered by another timeline's completion:**

```javascript
timeline.on('complete', () => {
    setTimeout(() => {
        dependentTimeline.start();
    }, delayMs);
});
```

**Example:**
```css
.sequence {
    @on click step-1 {
        .box-1 { opacity: 0 -> 1; }
    }

    @after step-1 step-2(delay: 200ms) {
        .box-2 { opacity: 0 -> 1; }
    }
}
```

### ValueChange Timelines

**Per-character flip animations on text changes:**

```javascript
const observer = new MutationObserver(() => {
    const oldText = lastValue;
    const newText = element.textContent;

    // Diff characters
    const changes = diffChars(oldText, newText);

    // Animate each changed character
    changes.forEach((char, index) => {
        if (char.changed) {
            const delay = index * charStaggerMs;
            animateChar(char, delay, durationMs);
        }
    });
});
```

**Example:**
```css
.counter {
    @value_change flip(400ms, stagger: 50ms) {
        :exiting { translate-y: 0 -> -100%; opacity: 1 -> 0; }
        :entering { translate-y: 100% -> 0; opacity: 0 -> 1; }
    }
}
```

---

## 10. Runtime Execution Model

### Initialization

```javascript
// 1. Load compiled output
<link rel="stylesheet" href="styles.css">
<script src="runtime.js"></script>
<script type="module">
    import config from './timeline-config.json';
    Spacetime.init(config);
</script>

// 2. Runtime creates timeline instances
class SpacetimeTimeline {
    constructor(config) {
        this.id = config.id;
        this.driver = config.driver;
        this.progress = 0;
        this.init();
    }
}

// 3. Set up drivers (scroll, time, etc.)
initScrollDriver() {
    window.addEventListener('scroll', () => {
        requestAnimationFrame(() => {
            const progress = calculateScrollProgress();
            this.setProgress(progress);
        });
    }, { passive: true });
}
```

### Frame Loop

```javascript
setProgress(progress) {
    // 1. Update CSS variable
    document.documentElement.style.setProperty(
        `--st-${this.id}-progress`,
        progress
    );

    // 2. Update all animations
    for (const anim of this.animations) {
        this.updateAnimation(anim, progress);
    }

    // 3. Update children
    for (const child of this.children) {
        child.setProgress(progress);
    }
}

updateAnimation(anim, timelineProgress) {
    const elements = document.querySelectorAll(anim.targets);

    elements.forEach((el, index) => {
        // Calculate stagger offset
        const staggerOffset = calculateStagger(index, elements.length, anim.stagger);

        // Calculate local progress
        const localStart = anim.offset + staggerOffset;
        const localEnd = localStart + anim.span;
        const localProgress = clamp((timelineProgress - localStart) / anim.span, 0, 1);

        // Interpolate each property
        for (const prop of anim.properties) {
            const value = interpolate(prop.keyframes, localProgress, prop.easing);
            applyProperty(el, prop.property, value);
        }
    });
}
```

### Property Application

```javascript
function applyProperty(el, property, value) {
    if (property.startsWith('--')) {
        // CSS variable
        el.style.setProperty(property, value);
    } else if (isTransform(property)) {
        // Accumulate transforms
        el._stTransforms = el._stTransforms || {};
        el._stTransforms[property] = value;
        el.style.transform = Object.entries(el._stTransforms)
            .map(([k, v]) => `${k}(${v})`)
            .join(' ');
    } else if (isFilter(property)) {
        // Accumulate filters
        el._stFilters = el._stFilters || {};
        el._stFilters[property] = value;
        el.style.filter = Object.entries(el._stFilters)
            .map(([k, v]) => `${k}(${v})`)
            .join(' ');
    } else {
        // Direct CSS property
        el.style[property] = value;
    }
}
```

### Interpolation

```javascript
function interpolate(keyframes, progress, easing) {
    // Find surrounding keyframes
    let prev = keyframes[0];
    let next = keyframes[keyframes.length - 1];

    for (let i = 0; i < keyframes.length - 1; i++) {
        if (progress >= keyframes[i].at && progress <= keyframes[i + 1].at) {
            prev = keyframes[i];
            next = keyframes[i + 1];
            break;
        }
    }

    // Local progress between keyframes
    const localProgress = (progress - prev.at) / (next.at - prev.at);
    const easedProgress = applyEasing(localProgress, easing);

    // Interpolate value
    if (isColor(prev.value)) {
        return interpolateColor(prev.value, next.value, easedProgress);
    } else if (isNumeric(prev.value)) {
        return interpolateNumeric(prev.value, next.value, easedProgress);
    } else {
        // Snap at 50%
        return easedProgress < 0.5 ? prev.value : next.value;
    }
}
```

### Color Interpolation

```javascript
function interpolateColor(c1, c2, t, space = 'rgb') {
    if (space === 'oklch') {
        const oklch1 = rgbToOklch(c1);
        const oklch2 = rgbToOklch(c2);

        const l = lerp(oklch1.l, oklch2.l, t);
        const c = lerp(oklch1.c, oklch2.c, t);
        const h = lerpHue(oklch1.h, oklch2.h, t);

        return oklchToRgb(l, c, h);
    } else {
        // RGB interpolation
        const r = lerp(c1.r, c2.r, t);
        const g = lerp(c1.g, c2.g, t);
        const b = lerp(c1.b, c2.b, t);

        return `rgb(${r}, ${g}, ${b})`;
    }
}
```

---

## 11. Performance Considerations

### Optimization Strategies

#### 1. CSS-First Approach

Wherever possible, use CSS for animations instead of JS interpolation:

```css
/* Good: CSS handles the animation */
.element {
    transition: opacity 300ms ease-out;
}
.element.is-visible {
    opacity: 1;
}

/* Avoid: JS interpolation on every frame */
element.style.opacity = interpolate(0, 1, progress);
```

#### 2. RequestAnimationFrame Batching

Group all DOM updates into a single RAF callback:

```javascript
// Good: Batched updates
requestAnimationFrame(() => {
    for (const timeline of timelines) {
        timeline.update();
    }
});

// Avoid: Multiple RAF calls
for (const timeline of timelines) {
    requestAnimationFrame(() => timeline.update());
}
```

#### 3. Passive Event Listeners

Use `{ passive: true }` for scroll/touch listeners:

```javascript
window.addEventListener('scroll', handler, { passive: true });
```

#### 4. IntersectionObserver for Visibility

Use IntersectionObserver instead of scroll listeners for load animations:

```javascript
const observer = new IntersectionObserver((entries) => {
    for (const entry of entries) {
        if (entry.isIntersecting) {
            playAnimation(entry.target);
        }
    }
}, { threshold: 0.1 });
```

#### 5. Element Caching

Cache DOM queries instead of repeating them:

```javascript
// Good: Cache queries
this.elements = new Map();
this.elements.set('.title', document.querySelectorAll('.title'));

// Avoid: Query on every frame
document.querySelectorAll('.title').forEach(el => ...);
```

#### 6. Transform Accumulation

Accumulate transforms instead of overwriting:

```javascript
// Good: Preserve all transforms
el._stTransforms = { translateY: '50px', scale: '1.1' };
el.style.transform = 'translateY(50px) scale(1.1)';

// Avoid: Overwrite
el.style.transform = 'translateY(50px)'; // Lost scale!
```

#### 7. Stagger Calculation Memoization

Calculate stagger offsets once, not per frame:

```javascript
// Good: Memoize
const staggerOffsets = elements.map((_, i) =>
    calculateStagger(i, elements.length, config)
);

// Avoid: Calculate every frame
elements.forEach((el, i) => {
    const offset = calculateStagger(i, elements.length, config);
});
```

### Performance Metrics

| Operation | Target | Notes |
|-----------|--------|-------|
| RAF callback | < 16ms | 60fps |
| Scroll handler | < 8ms | Leave time for browser |
| DOM query cache hit | < 0.1ms | Map lookup |
| Interpolation | < 0.5ms | Per property |
| Color interpolation | < 1ms | OKLCH conversion |

### Memory Management

#### Cleanup Protocol

```javascript
class SpacetimeTimeline {
    destroy() {
        // Cancel RAF
        if (this._rafId) cancelAnimationFrame(this._rafId);

        // Abort event listeners (uses AbortController)
        this._abortController.abort();

        // Disconnect observers
        this._observers.forEach(obs => obs.disconnect());

        // Clear element cache
        this.elements.clear();

        // Destroy children
        this.children.forEach(child => child.destroy());
    }
}
```

#### Avoiding Leaks

1. **Use AbortController for event listeners:**
   ```javascript
   const controller = new AbortController();
   el.addEventListener('click', handler, { signal: controller.signal });
   controller.abort(); // Removes all listeners
   ```

2. **Disconnect observers:**
   ```javascript
   const observer = new IntersectionObserver(callback);
   observer.observe(element);
   observer.disconnect(); // Cleanup
   ```

3. **Cancel Web Animations:**
   ```javascript
   const anim = element.animate(keyframes, options);
   anim.cancel(); // Release resources
   ```

---

## 12. Internal: Data Binding Metasystem

> **For developers working on the compiler internals.**

### The Single Entry Point

All data binding code generation flows through a single function:

```rust
// src/metasystem/compile.rs
pub fn compile_macro_data_bindings(
    file: &StFile,
    registry: &MetaRegistry,
) -> CompiledOutput
```

This function processes data binding features in 7 ordered phases:

| Phase | Feature | Example Syntax | Output |
|-------|---------|----------------|--------|
| 1 | SpacetimeLocal | `$count number: 0;` | `window.SpacetimeLocal = {...}` |
| 2 | Element refs | `#header .header;` | CSS vars + JS observers |
| 3 | @data | `@data products: T[] { src: "..." }` | `fetch()` + events |
| 4 | @computed | `@computed featured: T[] { from: ... }` | Derived data |
| 5 | @fn | `@fn formatPrice(n): string {...}` | `ST.functions.*` |
| 6 | @each | `@each($items as $i) {...}` | Template instantiation |
| 7 | @on | `@on &.click(.btn) { $count += 1; }` | Event handlers |

### Phase Dependencies

The ordering is critical - later phases depend on earlier ones:

```
Phase 1 (SpacetimeLocal)
    └── Phase 3 (@data) and Phase 6 (@each) need SpacetimeLocal defined first

Phase 3 (@data)
    └── Phase 6 (@each) listens for data:NAME:loaded events

Phase 5 (@fn)
    └── Phase 6 (@each) can call helper functions in bindings
```

### Codegen Function Reuse

The metasystem reuses existing codegen functions from `src/codegen/mod.rs`:

```rust
use crate::codegen::{
    generate_local_definitions,    // Phase 1
    generate_element_ref_code,     // Phase 2
    generate_data_loading_js,      // Phase 3
    generate_computed_js,          // Phase 4
    generate_functions_js,         // Phase 5
    generate_each_block_code,      // Phase 6
    generate_mutation_handler,     // Phase 7
};
```

### Test Coverage

Integration tests for each phase are in `tests/integration/*_metasystem.rs`:

- `local_state_metasystem.rs` - Phase 1
- `element_refs_metasystem.rs` - Phase 2
- `data_loading_metasystem.rs` - Phase 3
- `computed_metasystem.rs` - Phase 4
- `fn_metasystem.rs` - Phase 5
- `each_metasystem.rs` - Phase 6
- `mutations_metasystem.rs` - Phase 7

### Further Reading

For detailed internal documentation:
- [src/metasystem/ARCHITECTURE.md](../src/metasystem/ARCHITECTURE.md) - Complete 7-phase pipeline
- [src/metasystem/compile.rs](../src/metasystem/compile.rs) - Implementation

---

## Conclusion

Spacetime represents a paradigm shift in how we think about dynamic web behavior:

**From:** Fragmented APIs (CSS, JS, WAAPI, frameworks)
**To:** Unified timeline-based abstraction

**From:** Imperative state management
**To:** Declarative reactive signals

**From:** Side-effect-heavy JavaScript
**To:** Pure WASM logic + thin JS glue

The three-layer architecture (Sugar → Primitives → Runtime) provides:
- **Ergonomics** for common patterns
- **Power** for advanced use cases
- **Performance** through hybrid WASM/JS execution

By understanding these architectural principles, you can:
- **Use** Spacetime effectively for animations and behaviors
- **Extend** Spacetime with custom sugar directives
- **Debug** Spacetime timelines with confidence
- **Contribute** to the Spacetime compiler and runtime

---

**Further Reading:**
- [SPEC.md](../SPEC.md) - Complete DSL reference
- [BEHAVIOR_SYSTEM.md](./BEHAVIOR_SYSTEM.md) - State & Behavior in Spacetime
- [src/compiler.rs](../src/compiler.rs) - Compilation implementation
- [src/parser/](../src/parser/) - Parser and AST definitions
