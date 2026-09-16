# Feature Request: Event Handler Blocks

## Problem Statement

Mobile gesture patterns often need to respond to specific lifecycle events or threshold crossings. For example:
- A long-press gesture needs to execute code when the press duration reaches a threshold
- A swipe gesture needs to trigger an action when the swipe completes
- A pan gesture needs to differentiate between start, move, and end phases

Currently, macros can only react to signal value changes via `%on $signal -> value { }`. There's no way to define custom event handlers that are part of the macro's form.

### Current Syntax (Does Not Compile)

```spacetime
%macro long-press {
  %form {
    @long-press(duration: $duration:time = 500ms) {
      @on threshold(0.3) { $onProgress30:block? }
      @on threshold(0.6) { $onProgress60:block? }
      @on complete { $onComplete:block? }
      @on cancel { $onCancel:block? }
    }
  }
}

// Usage
.button {
  @long-press(duration: 800ms) {
    @on threshold(0.5) {
      // Vibrate at 50%
      navigator.vibrate(10);
    }
    @on complete {
      showContextMenu();
    }
  }
}
```

**Parser Error**: The grammar does not recognize `@on eventName { }` as a form capture pattern.

## Use Cases

### 1. Threshold Events

```spacetime
// Fire when progress reaches specific thresholds
@on threshold(0.3) { hapticFeedback("light"); }
@on threshold(0.7) { hapticFeedback("medium"); }
@on threshold(1.0) { hapticFeedback("heavy"); }
```

### 2. Lifecycle Events

```spacetime
// Fire at specific lifecycle points
@on start { console.log("Gesture started"); }
@on move { updateVisuals($x, $y); }
@on end { finalizeGesture(); }
@on cancel { resetState(); }
```

### 3. State Transition Events

```spacetime
// Fire when entering/exiting specific states
@on enter("active") { addClass("active"); }
@on exit("active") { removeClass("active"); }
```

### 4. Conditional Events

```spacetime
// Fire when condition becomes true
@on condition($velocity > 500) { triggerFlick(); }
@on condition($distance > threshold) { startDrag(); }
```

## Proposed Syntax

### In `%form` Block

```spacetime
%form {
  @long-press(duration: $duration:time = 500ms) {
    // Standard property captures
    $customStates:states?

    // Event handler captures (new)
    @on start { $onStart:block? }?
    @on complete { $onComplete:block? }?
    @on cancel { $onCancel:block? }?
    @on threshold($t:number) { $onThreshold:block? }*
  }
}
```

### Event Handler Capture Types

```spacetime
// Single optional handler
@on eventName { $handler:block? }?

// Multiple handlers (for threshold events with different values)
@on threshold($t:number) { $handler:block? }*

// Required handler
@on complete { $handler:block }

// Parameterized event
@on enter($state:string) { $handler:block? }?
```

### In Macro Implementation

```spacetime
%macro long-press {
  %binds {
    long-press-driver(&self, duration: $duration) -> {
      $progress,
      $active
    }
  }

  // Invoke captured handlers when conditions are met
  %on $progress >= 0.3 {
    %if $onThreshold_0_3 {
      %invoke $onThreshold_0_3
    }
  }

  %on $active -> false {
    %if $progress >= 1.0 {
      %invoke $onComplete
    } %else {
      %invoke $onCancel
    }
  }
}
```

## Implementation Details

### 1. Grammar Changes (`src/parser/grammar.pest`)

#### Add Event Handler Form Pattern

```pest
// Form body content - add event handler captures
form_body_content = @{ form_body_part* }

// Form body can contain property captures OR event handler captures
form_body_item = {
    form_event_handler
    | form_property_capture
    | form_body_part
}

// Event handler capture: @on eventName(params?) { $handler:block? }?
form_event_handler = {
    "@on" ~ form_event_name ~ form_event_params? ~ "{" ~ form_capture ~ "}" ~ capture_modifier?
}

form_event_name = @{ identifier }

// Event parameters: (0.3), ($threshold), ("stateName")
form_event_params = { "(" ~ form_event_param_list ~ ")" }
form_event_param_list = { form_event_param ~ ("," ~ form_event_param)* }
form_event_param = { form_capture | number | string }
```

#### Update Macro Call Body

```pest
// In macro_call_body_content, add event handler blocks
macro_call_body_content = {
    generic_macro_call
    | macro_call_event_handler    // NEW
    | macro_call_html_content
    | macro_call_slot_binding
    | macro_call_host_binding
    | macro_call_property
    | macro_call_meta_subst
    | nested_scope
    | macro_call_js_statement
}

// Event handler in macro call: @on threshold(0.5) { code }
macro_call_event_handler = {
    "@on" ~ macro_call_event_name ~ macro_call_event_args? ~ macro_call_block
}

macro_call_event_name = @{ identifier }
macro_call_event_args = { "(" ~ macro_call_arg_value ~ ("," ~ macro_call_arg_value)* ~ ")" }
```

### 2. AST Changes (`src/parser/ast.rs`)

#### Add Event Handler AST Types

```rust
/// Event handler capture in a form pattern
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormEventHandler {
    /// Event name: "start", "complete", "threshold", etc.
    pub event_name: String,
    /// Event parameters (if any)
    pub params: Vec<FormEventParam>,
    /// The capture for the handler block
    pub handler_capture: FormCapture,
    /// Whether this handler is optional (?)
    pub optional: bool,
    /// Whether multiple handlers are allowed (*)
    pub multiple: bool,
    pub span: SourceSpan,
}

/// Parameter in an event handler
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FormEventParam {
    /// Captured parameter: $t:number
    Capture(FormCapture),
    /// Literal value: 0.5, "active"
    Literal(FormEventLiteral),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FormEventLiteral {
    Number(f64),
    String(String),
}
```

#### Add to MacroCallBody

```rust
/// Body of a macro call block
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MacroCallBody {
    pub macro_calls: Vec<MacroCallAst>,
    pub properties: Vec<(String, String)>,
    pub html: Option<String>,
    pub nested_scopes: Vec<NestedScope>,
    pub js_statements: Vec<String>,
    /// Event handlers provided in macro invocation (new)
    pub event_handlers: Vec<MacroEventHandler>,
}

/// Event handler in a macro call
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MacroEventHandler {
    /// Event name: "complete", "threshold", etc.
    pub event_name: String,
    /// Event arguments: 0.5, "active", etc.
    pub args: Vec<MacroCallArg>,
    /// Handler body (JavaScript code block)
    pub body: MacroCallBody,
    pub span: SourceSpan,
}
```

### 3. Meta AST Changes (`src/parser/meta_ast.rs`)

#### Update FormClause

```rust
/// %form { @directive(params) { body } }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormClause {
    pub directive_name: String,
    pub inline_elements: Vec<FormInlineElement>,
    pub params: Vec<FormParam>,
    pub body_capture: Option<String>,
    /// Event handler captures (new)
    pub event_handlers: Vec<FormEventHandler>,
    pub span: SourceSpan,
}
```

#### Add Invoke Clause

```rust
/// MacroBodyItem - add invoke for calling captured handlers
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MacroBodyItem {
    When(WhenClause),
    On(MetaOnClause),
    For(MetaForClause),
    If(MetaIfClause),
    Animates(AnimatesClause),
    Applies(AppliesClause),
    Includes(IncludesClause),
    Mutate(MetaMutateClause),
    Trigger(String),
    Emit(EmitBlock),
    Binds(Vec<BindDecl>),
    /// Invoke a captured handler: %invoke $onComplete (new)
    Invoke(InvokeClause),
}

/// %invoke $handler or %invoke $handler($arg1, $arg2)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvokeClause {
    /// Handler variable name (e.g., "onComplete")
    pub handler: String,
    /// Arguments to pass to the handler
    pub args: Vec<String>,
    pub span: SourceSpan,
}
```

### 4. Codegen Changes (`src/metasystem/codegen.rs`)

#### Generate Event Handler Registration

When a macro defines event handlers and the user provides them, generate:

```javascript
// For threshold events
const __threshold_0_3_handler = () => {
  // User's code block
  navigator.vibrate(10);
};

// Register with the gesture driver
__gestureDriver.onThreshold(0.3, __threshold_0_3_handler);

// For lifecycle events
const __onComplete_handler = () => {
  // User's code block
  showContextMenu();
};

__gestureDriver.onComplete(__onComplete_handler);
```

#### The `%invoke` Codegen

```rust
fn generate_invoke(clause: &InvokeClause) -> String {
    let handler_name = &clause.handler;

    if clause.args.is_empty() {
        format!("if (typeof __{}_handler === 'function') {{ __{}_handler(); }}\n",
            handler_name, handler_name)
    } else {
        let args = clause.args.join(", ");
        format!("if (typeof __{}_handler === 'function') {{ __{}_handler({}); }}\n",
            handler_name, handler_name, args)
    }
}
```

### 5. Macro Expansion

During macro expansion, captured event handlers are transformed:

**Input (user code)**:
```spacetime
.button {
  @long-press(duration: 800ms) {
    @on complete {
      showContextMenu();
    }
  }
}
```

**Macro definition**:
```spacetime
%macro long-press {
  %form {
    @long-press(duration: $duration:time) {
      @on complete { $onComplete:block? }?
    }
  }

  %on $progress >= 1.0 {
    %invoke $onComplete
  }
}
```

**Expanded output**:
```javascript
// Captured handler
const __onComplete_handler = () => {
  showContextMenu();
};

// Watcher for progress threshold
ST.watch(el, 'progress', (progress) => {
  if (progress >= 1.0) {
    if (typeof __onComplete_handler === 'function') {
      __onComplete_handler();
    }
  }
});
```

## Example: Complete Long-Press Implementation

### Primitive: Long Press Driver

```spacetime
%primitive long-press-driver(
  &el,
  duration: number = 500
) {
  %emit js {
    let startTime = null;
    let rafId = null;
    let lastProgress = 0;

    const thresholdCallbacks = new Map();

    // Event registration methods (exposed via exports)
    const onThreshold = (threshold, callback) => {
      if (!thresholdCallbacks.has(threshold)) {
        thresholdCallbacks.set(threshold, []);
      }
      thresholdCallbacks.get(threshold).push(callback);
    };

    const update = () => {
      if (!startTime) return;

      const elapsed = performance.now() - startTime;
      const progress = Math.min(elapsed / %duration, 1);

      %yield progress -> $progress;

      // Check thresholds
      for (const [threshold, callbacks] of thresholdCallbacks) {
        if (lastProgress < threshold && progress >= threshold) {
          callbacks.forEach(cb => cb(progress));
        }
      }
      lastProgress = progress;

      if (progress < 1) {
        rafId = requestAnimationFrame(update);
      } else {
        %yield true -> $complete;
      }
    };

    const onPointerDown = (e) => {
      startTime = performance.now();
      lastProgress = 0;
      %yield true -> $active;
      %yield 0 -> $progress;
      rafId = requestAnimationFrame(update);
      e.preventDefault();
    };

    const onPointerUp = () => {
      if (startTime) {
        cancelAnimationFrame(rafId);
        const wasComplete = lastProgress >= 1;
        startTime = null;
        %yield false -> $active;
        if (!wasComplete) {
          %yield true -> $cancelled;
        }
      }
    };

    el.addEventListener('pointerdown', onPointerDown);
    el.addEventListener('pointerup', onPointerUp);
    el.addEventListener('pointercancel', onPointerUp);

    // Expose registration function
    %yield onThreshold -> $onThreshold;

    %cleanup {
      cancelAnimationFrame(rafId);
      el.removeEventListener('pointerdown', onPointerDown);
      el.removeEventListener('pointerup', onPointerUp);
      el.removeEventListener('pointercancel', onPointerUp);
    }
  }

  %exports {
    $active: bool
    $progress: number
    $complete: bool
    $cancelled: bool
    $onThreshold: fn(number, fn) ~> void
  }
}
```

### Macro: Long Press

```spacetime
%macro long-press {
  %creates @long-press

  %form {
    @long-press(duration: $duration:time = 500ms) {
      $customStates:states?
      @on start { $onStart:block? }?
      @on complete { $onComplete:block? }?
      @on cancel { $onCancel:block? }?
      @on threshold($t:number) { $onThreshold:block? }*
    }
  }

  %binds {
    long-press-driver(&self, duration: $duration) -> {
      $active,
      $progress,
      $complete,
      $cancelled,
      $onThreshold as $registerThreshold
    }
  }

  // Register threshold handlers
  %for $handler in $onThreshold {
    %emit js {
      $registerThreshold(%$handler.t, () => {
        %$handler.block
      });
    }
  }

  %states {
    idle { opacity: 1; }
    pressing when $active {
      opacity: calc(1 - var(--progress) * 0.3);
    }
    $customStates?
  }

  %on $active -> true {
    %invoke $onStart
  }

  %on $complete -> true {
    %invoke $onComplete
  }

  %on $cancelled -> true {
    %invoke $onCancel
  }
}
```

### User Usage

```spacetime
.action-button {
  @long-press(duration: 600ms) {
    pressing {
      transform: scale(0.95);
      background: linear-gradient(to right, #4caf50 calc(var(--progress) * 100%), #eee 0);
    }

    @on threshold(0.3) {
      navigator.vibrate(5);
    }

    @on threshold(0.6) {
      navigator.vibrate(10);
    }

    @on complete {
      showContextMenu(this);
    }

    @on cancel {
      showToast("Hold longer to activate");
    }
  }
}
```

## Migration / Compatibility Notes

### Backward Compatibility

1. **Existing macros unchanged**: Macros without event handlers continue to work
2. **Existing `%on` syntax unchanged**: The `%on $signal -> value` syntax remains
3. **No breaking changes**: Event handlers are opt-in

### New Syntax Overview

| Syntax | Location | Purpose |
|--------|----------|---------|
| `@on eventName { $handler:block? }?` | `%form` | Capture event handler |
| `%invoke $handler` | `%on` body | Call captured handler |
| `@on eventName { code }` | Macro call | Provide event handler |

## Testing Checklist

- [ ] Parser accepts `@on eventName { }` in form patterns
- [ ] Parser accepts `@on eventName(params) { }` in form patterns
- [ ] Parser accepts `@on eventName { }` in macro call bodies
- [ ] AST correctly captures event handlers with parameters
- [ ] Codegen produces correct handler registration code
- [ ] `%invoke` correctly calls captured handlers
- [ ] Multiple threshold handlers work correctly
- [ ] Optional handlers (not provided) don't cause errors
- [ ] Handler parameters are passed correctly

## Files to Modify

| File | Changes |
|------|---------|
| `src/parser/grammar.pest` | Add `form_event_handler`, `macro_call_event_handler` rules |
| `src/parser/ast.rs` | Add `FormEventHandler`, `MacroEventHandler`, update `MacroCallBody` |
| `src/parser/meta_ast.rs` | Update `FormClause`, add `InvokeClause` |
| `src/parser/mod.rs` | Add parsing for event handlers |
| `src/metasystem/codegen.rs` | Add handler registration codegen, `%invoke` codegen |
| `src/metasystem/expand.rs` | Handle event handler matching during macro expansion |
