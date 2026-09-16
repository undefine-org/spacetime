# Feature Request: Macro Event Syntax in `%form`

## Problem Statement

Mobile gestures often need to expose lifecycle hooks and event callbacks to users. The macro author wants to define event handler slots in the `%form` that users can optionally provide.

Currently, `%form` patterns can capture:
- Parameters: `axis: $axis:("x" | "y") = "both"`
- Properties: `$customStates:states?`
- Blocks: `$body:block?`

But there's no way to capture inline event handlers that users might provide:

### Desired Syntax (Does Not Compile)

```spacetime
%macro long-press {
  %form {
    @long-press(duration: $duration:time = 500ms) {
      // Standard property captures
      $customStates:states?

      // Event handler captures (NEW)
      @on start { $onStart:block? }?
      @on complete { $onComplete:block? }?
      @on cancel { $onCancel:block? }?
    }
  }
}

// User invokes with:
.button {
  @long-press(duration: 600ms) {
    pressing { scale: 0.95; }

    @on start {
      console.log("Started!");
    }

    @on complete {
      showMenu();
    }
  }
}
```

**Parser Error**: `@on` inside `%form` body is not recognized as a capture pattern.

## Proposed Syntax

### In `%form` Pattern Definition

```spacetime
%form {
  @directive-name(params) {
    // Property captures (existing)
    $properties:properties?

    // State captures (existing)
    $states:states?

    // Event handler captures (NEW)
    @on eventName { $handlerVar:block? }?       // Optional single handler
    @on eventName { $handlerVar:block }         // Required single handler
    @on eventName($param:type) { $handler:block? }*   // Multiple handlers with param
  }
}
```

### Capture Modifiers

| Modifier | Meaning |
|----------|---------|
| `?` | Optional (0 or 1) |
| `*` | Zero or more |
| `+` | One or more |
| (none) | Required (exactly 1) |

### Event Parameter Syntax

```spacetime
// Simple event (no parameters)
@on complete { $onComplete:block? }?

// Event with literal parameter (matched exactly)
@on threshold(0.5) { $onThreshold50:block? }?

// Event with captured parameter
@on threshold($t:number) { $onThreshold:block? }*

// Event with multiple parameters
@on drag($x:number, $y:number) { $onDrag:block? }?

// Event with string parameter (for named states)
@on enter($state:string) { $onEnter:block? }*
```

## Implementation Details

### 1. Grammar Changes (`src/parser/grammar.pest`)

#### Add Event Handler Capture Rules

```pest
// Form body content - can contain captures or event handler patterns
form_body_content = @{ form_body_part* }

// Explicit form body parsing (alternative to raw capture)
form_body_explicit = { form_body_item* }
form_body_item = {
    form_event_capture       // @on eventName { $handler:block? }?
    | form_property_capture  // $name:type?
}

// Event handler capture in form pattern
form_event_capture = {
    "@on" ~ form_event_name ~ form_event_params? ~
    "{" ~ form_capture ~ "}" ~ capture_modifier?
}

form_event_name = @{ identifier }

// Event parameters: (0.5), ($threshold:number), ("active")
form_event_params = { "(" ~ form_event_param_list ~ ")" }
form_event_param_list = { form_event_param ~ ("," ~ form_event_param)* }
form_event_param = {
    form_capture         // $name:type for captured params
    | number             // literal number like 0.5
    | string             // literal string like "active"
}

// Property capture (existing, renamed for clarity)
form_property_capture = { "$" ~ identifier ~ ":" ~ capture_type ~ capture_modifier? }

// Capture modifier (existing)
capture_modifier = { "?" | "*" | "+" }
```

#### Update Macro Call Body Rules

```pest
// Macro call body content - add event handler invocations
macro_call_body_content = {
    generic_macro_call
    | macro_call_event_handler    // @on eventName { code }
    | macro_call_html_content
    | macro_call_slot_binding
    | macro_call_host_binding
    | macro_call_property
    | macro_call_meta_subst
    | nested_scope
    | macro_call_js_statement
}

// Event handler in macro invocation
macro_call_event_handler = {
    "@on" ~ macro_call_event_name ~ macro_call_event_args? ~ macro_call_block
}

macro_call_event_name = @{ identifier }

// Event arguments (literal values)
macro_call_event_args = { "(" ~ macro_call_event_arg_list ~ ")" }
macro_call_event_arg_list = { macro_call_event_arg ~ ("," ~ macro_call_event_arg)* }
macro_call_event_arg = { number | string | "$" ~ identifier }
```

### 2. AST Changes (`src/parser/meta_ast.rs`)

#### Add Event Capture Types

```rust
/// Event handler capture in a form pattern
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormEventCapture {
    /// Event name: "start", "complete", "threshold", etc.
    pub event_name: String,
    /// Event parameters (captured or literal)
    pub params: Vec<FormEventParam>,
    /// The capture specification for the handler body
    pub handler_capture: FormCapture,
    /// Capture modifier: Optional(?), ZeroOrMore(*), OneOrMore(+), Required
    pub modifier: CaptureModifier,
    pub span: SourceSpan,
}

/// Parameter in an event capture pattern
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FormEventParam {
    /// Captured parameter: $threshold:number
    Capture(FormCapture),
    /// Literal number: 0.5, 100
    Number(f64),
    /// Literal string: "active", "idle"
    String(String),
}

impl FormEventParam {
    /// Check if this is a captured (variable) parameter
    pub fn is_capture(&self) -> bool {
        matches!(self, FormEventParam::Capture(_))
    }

    /// Get the capture if this is a captured parameter
    pub fn as_capture(&self) -> Option<&FormCapture> {
        match self {
            FormEventParam::Capture(c) => Some(c),
            _ => None,
        }
    }
}
```

#### Update FormClause

```rust
/// %form { @directive(params) { body } }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormClause {
    /// The directive name
    pub directive_name: String,
    /// Inline elements before parentheses
    pub inline_elements: Vec<FormInlineElement>,
    /// Parameters in parentheses
    pub params: Vec<FormParam>,
    /// Raw body capture (if any) - for simple cases
    pub body_capture: Option<String>,
    /// Event handler captures (NEW)
    pub event_captures: Vec<FormEventCapture>,
    /// Property captures in body (for explicit parsing)
    pub property_captures: Vec<FormCapture>,
    pub span: SourceSpan,
}
```

#### Add Event Handler Invocation to MacroCallBody

```rust
/// Body of a macro call block
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MacroCallBody {
    /// Nested macro calls
    pub macro_calls: Vec<MacroCallAst>,
    /// Property declarations
    pub properties: Vec<(String, String)>,
    /// Raw HTML content
    pub html: Option<String>,
    /// Nested scopes
    pub nested_scopes: Vec<NestedScope>,
    /// JavaScript statements
    pub js_statements: Vec<String>,
    /// Event handlers (NEW)
    pub event_handlers: Vec<MacroCallEventHandler>,
}

/// Event handler provided in a macro invocation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MacroCallEventHandler {
    /// Event name: "start", "complete", "threshold"
    pub event_name: String,
    /// Event arguments (literal values)
    pub args: Vec<MacroCallArg>,
    /// Handler body
    pub body: MacroCallBody,
    pub span: SourceSpan,
}
```

### 3. Parser Implementation (`src/parser/mod.rs`)

Add parsing functions:

```rust
fn parse_form_event_capture(pair: pest::iterators::Pair<Rule>) -> Result<FormEventCapture, ParseError> {
    let span = SourceSpan::from_pest(pair.as_span());
    let mut inner = pair.into_inner();

    let event_name = inner.next()
        .ok_or_else(|| pest_error("Expected event name"))?
        .as_str()
        .to_string();

    let mut params = Vec::new();
    let mut handler_capture = None;
    let mut modifier = CaptureModifier::Required;

    for item in inner {
        match item.as_rule() {
            Rule::form_event_params => {
                params = parse_form_event_params(item)?;
            }
            Rule::form_capture => {
                handler_capture = Some(parse_form_capture(item)?);
            }
            Rule::capture_modifier => {
                modifier = parse_capture_modifier(item);
            }
            _ => {}
        }
    }

    Ok(FormEventCapture {
        event_name,
        params,
        handler_capture: handler_capture.ok_or_else(|| pest_error("Expected handler capture"))?,
        modifier,
        span,
    })
}

fn parse_form_event_params(pair: pest::iterators::Pair<Rule>) -> Result<Vec<FormEventParam>, ParseError> {
    let mut params = Vec::new();

    for item in pair.into_inner() {
        match item.as_rule() {
            Rule::form_capture => {
                params.push(FormEventParam::Capture(parse_form_capture(item)?));
            }
            Rule::number => {
                let n: f64 = item.as_str().parse().unwrap_or(0.0);
                params.push(FormEventParam::Number(n));
            }
            Rule::string => {
                let s = item.as_str();
                params.push(FormEventParam::String(s[1..s.len()-1].to_string()));
            }
            _ => {}
        }
    }

    Ok(params)
}

fn parse_macro_call_event_handler(pair: pest::iterators::Pair<Rule>) -> Result<MacroCallEventHandler, ParseError> {
    let span = SourceSpan::from_pest(pair.as_span());
    let mut inner = pair.into_inner();

    let event_name = inner.next()
        .ok_or_else(|| pest_error("Expected event name"))?
        .as_str()
        .to_string();

    let mut args = Vec::new();
    let mut body = None;

    for item in inner {
        match item.as_rule() {
            Rule::macro_call_event_args => {
                args = parse_macro_call_args(item)?;
            }
            Rule::macro_call_block => {
                body = Some(parse_macro_call_body(item)?);
            }
            _ => {}
        }
    }

    Ok(MacroCallEventHandler {
        event_name,
        args,
        body: body.ok_or_else(|| pest_error("Expected handler body"))?,
        span,
    })
}
```

### 4. Macro Expansion (`src/metasystem/expand.rs`)

During macro expansion, match event handlers:

```rust
fn expand_macro_with_events(
    macro_def: &MacroDefAst,
    call: &MacroCallAst,
    bindings: &mut HashMap<String, PatternValue>,
) -> Result<ExpandedMacro, String> {
    // ... existing expansion logic ...

    // Match event handlers from call to form
    if let (Some(form), Some(call_body)) = (&macro_def.form, &call.body) {
        for event_capture in &form.event_captures {
            let matching_handlers: Vec<_> = call_body.event_handlers
                .iter()
                .filter(|h| event_matches(&event_capture, h))
                .collect();

            match event_capture.modifier {
                CaptureModifier::Required => {
                    if matching_handlers.is_empty() {
                        return Err(format!(
                            "Required event handler @on {} not provided",
                            event_capture.event_name
                        ));
                    }
                    if matching_handlers.len() > 1 {
                        return Err(format!(
                            "Multiple handlers for @on {}, expected exactly one",
                            event_capture.event_name
                        ));
                    }
                    bind_event_handler(&event_capture, &matching_handlers[0], bindings)?;
                }
                CaptureModifier::Optional => {
                    if matching_handlers.len() > 1 {
                        return Err(format!(
                            "Multiple handlers for optional @on {}, expected 0 or 1",
                            event_capture.event_name
                        ));
                    }
                    if let Some(handler) = matching_handlers.first() {
                        bind_event_handler(&event_capture, handler, bindings)?;
                    } else {
                        // Bind null/empty handler
                        bindings.insert(
                            event_capture.handler_capture.var_name.clone(),
                            PatternValue::Expr("".to_string())
                        );
                    }
                }
                CaptureModifier::ZeroOrMore | CaptureModifier::OneOrMore => {
                    if matches!(event_capture.modifier, CaptureModifier::OneOrMore)
                        && matching_handlers.is_empty()
                    {
                        return Err(format!(
                            "At least one @on {} handler required",
                            event_capture.event_name
                        ));
                    }
                    bind_event_handlers_array(&event_capture, &matching_handlers, bindings)?;
                }
            }
        }
    }

    Ok(expanded)
}

fn event_matches(capture: &FormEventCapture, handler: &MacroCallEventHandler) -> bool {
    if capture.event_name != handler.event_name {
        return false;
    }

    // For parameterized events, check literal params match
    for (i, param) in capture.params.iter().enumerate() {
        match param {
            FormEventParam::Number(n) => {
                if let Some(MacroCallArg::Number(arg_n)) = handler.args.get(i) {
                    if (*n - *arg_n).abs() > 0.0001 {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            FormEventParam::String(s) => {
                if let Some(MacroCallArg::String(arg_s)) = handler.args.get(i) {
                    if s != arg_s {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            FormEventParam::Capture(_) => {
                // Captured params match anything (bound at expansion)
            }
        }
    }

    true
}

fn bind_event_handler(
    capture: &FormEventCapture,
    handler: &MacroCallEventHandler,
    bindings: &mut HashMap<String, PatternValue>,
) -> Result<(), String> {
    // Bind captured parameters
    for (i, param) in capture.params.iter().enumerate() {
        if let FormEventParam::Capture(cap) = param {
            if let Some(arg) = handler.args.get(i) {
                let value = match arg {
                    MacroCallArg::Number(n) => PatternValue::Number(*n),
                    MacroCallArg::String(s) => PatternValue::String(s.clone()),
                    _ => PatternValue::Expr(format!("{:?}", arg)),
                };
                bindings.insert(cap.var_name.clone(), value);
            }
        }
    }

    // Bind handler body
    let body_js = generate_handler_body(&handler.body);
    bindings.insert(
        capture.handler_capture.var_name.clone(),
        PatternValue::Expr(body_js),
    );

    Ok(())
}

fn bind_event_handlers_array(
    capture: &FormEventCapture,
    handlers: &[&MacroCallEventHandler],
    bindings: &mut HashMap<String, PatternValue>,
) -> Result<(), String> {
    let entries: Vec<PatternValue> = handlers.iter().map(|h| {
        let mut entry = HashMap::new();

        // Include captured params
        for (i, param) in capture.params.iter().enumerate() {
            if let FormEventParam::Capture(cap) = param {
                if let Some(arg) = h.args.get(i) {
                    entry.insert(cap.var_name.clone(), arg_to_pattern_value(arg));
                }
            }
        }

        // Include handler body
        entry.insert(
            "block".to_string(),
            PatternValue::Expr(generate_handler_body(&h.body)),
        );

        PatternValue::Properties(entry.into_iter().collect())
    }).collect();

    bindings.insert(
        capture.handler_capture.var_name.clone(),
        PatternValue::List(entries),
    );

    Ok(())
}
```

### 5. Codegen (`src/metasystem/codegen.rs`)

Generate JavaScript for captured event handlers:

```rust
fn generate_event_handler_registration(
    event_name: &str,
    params: &[FormEventParam],
    handler_code: &str,
    element_expr: &str,
) -> String {
    let param_args: Vec<String> = params.iter()
        .filter_map(|p| p.as_capture().map(|c| c.var_name.clone()))
        .collect();

    let args_str = if param_args.is_empty() {
        String::new()
    } else {
        format!("({})", param_args.join(", "))
    };

    format!(r#"
        const __on_{event}_handler = {args} => {{
            {code}
        }};
        {el}.__st_event_{event} = __on_{event}_handler;
    "#,
        event = event_name,
        args = args_str,
        code = handler_code,
        el = element_expr
    )
}

fn generate_event_invocation(
    event_name: &str,
    args: &[String],
    element_expr: &str,
) -> String {
    let args_str = args.join(", ");
    format!(r#"
        if ({el}.__st_event_{event}) {{
            {el}.__st_event_{event}({args});
        }}
    "#,
        el = element_expr,
        event = event_name,
        args = args_str
    )
}
```

### 6. Add `%invoke` Clause

Add a way to invoke captured handlers from macro body:

```pest
// In macro_body_item
invoke_clause = { "%invoke" ~ "$" ~ identifier ~ invoke_args? }
invoke_args = { "(" ~ (invoke_arg ~ ("," ~ invoke_arg)*)? ~ ")" }
invoke_arg = { "$" ~ identifier | string | number | identifier }
```

```rust
/// %invoke $onComplete or %invoke $onComplete($x, $y)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvokeClause {
    /// Handler variable name
    pub handler: String,
    /// Arguments to pass
    pub args: Vec<String>,
    pub span: SourceSpan,
}
```

## Complete Example

### Macro Definition

```spacetime
%macro long-press {
  %creates @long-press

  %form {
    @long-press(duration: $duration:time = 500ms) {
      // Standard captures
      $customStates:states?

      // Event handler captures
      @on start { $onStart:block? }?
      @on progress($p:number) { $onProgress:block? }?
      @on threshold($t:number) { $onThreshold:block? }*
      @on complete { $onComplete:block? }?
      @on cancel { $onCancel:block? }?
    }
  }

  %binds {
    long-press-driver(&self, duration: $duration) -> {
      $active,
      $progress,
      $complete,
      $cancelled
    }
  }

  // Register threshold handlers
  %for $handler in $onThreshold {
    %emit js {
      ST.watch(el, 'progress', (p) => {
        if (p >= %$handler.t && lastProgress < %$handler.t) {
          %$handler.block
        }
        lastProgress = p;
      });
    }
  }

  %states {
    idle { }
    pressing when $active { }
    $customStates?
  }

  // Invoke captured handlers at appropriate times
  %on $active -> true {
    %invoke $onStart
    %if $onProgress {
      ST.watch(el, 'progress', (p) => { %invoke $onProgress(p) });
    }
  }

  %on $complete -> true {
    %invoke $onComplete
  }

  %on $cancelled -> true {
    %invoke $onCancel
  }
}
```

### User Invocation

```spacetime
.action-button {
  @long-press(duration: 800ms) {
    pressing {
      transform: scale(0.95);
      background: linear-gradient(
        to right,
        #4caf50 calc(var(--st-progress) * 100%),
        #eee 0
      );
    }

    @on start {
      console.log("Long press started");
      this.classList.add("pressing");
    }

    @on threshold(0.3) {
      navigator.vibrate(5);
    }

    @on threshold(0.6) {
      navigator.vibrate(10);
    }

    @on threshold(0.9) {
      navigator.vibrate(20);
    }

    @on complete {
      showContextMenu(this);
      navigator.vibrate(50);
    }

    @on cancel {
      showToast("Hold longer to activate");
    }
  }
}
```

### Generated JavaScript

```javascript
// Element: .action-button
document.querySelectorAll('.action-button').forEach(el => {
  // Long press driver setup...
  let lastProgress = 0;

  // Register threshold handlers
  ST.watch(el, 'progress', (p) => {
    if (p >= 0.3 && lastProgress < 0.3) {
      navigator.vibrate(5);
    }
    if (p >= 0.6 && lastProgress < 0.6) {
      navigator.vibrate(10);
    }
    if (p >= 0.9 && lastProgress < 0.9) {
      navigator.vibrate(20);
    }
    lastProgress = p;
  });

  // On start handler
  const __on_start_handler = () => {
    console.log("Long press started");
    el.classList.add("pressing");
  };

  // On complete handler
  const __on_complete_handler = () => {
    showContextMenu(el);
    navigator.vibrate(50);
  };

  // On cancel handler
  const __on_cancel_handler = () => {
    showToast("Hold longer to activate");
  };

  // Watch for state changes
  ST.watch(el, 'active', (active) => {
    if (active) {
      if (typeof __on_start_handler === 'function') __on_start_handler();
    }
  });

  ST.watch(el, 'complete', (complete) => {
    if (complete) {
      if (typeof __on_complete_handler === 'function') __on_complete_handler();
    }
  });

  ST.watch(el, 'cancelled', (cancelled) => {
    if (cancelled) {
      if (typeof __on_cancel_handler === 'function') __on_cancel_handler();
    }
  });
});
```

## Testing Checklist

- [ ] `@on eventName { $handler:block? }?` parses in `%form`
- [ ] `@on eventName($p:type) { $handler:block? }*` parses with params
- [ ] `@on eventName { code }` parses in macro call body
- [ ] Required event handlers error when missing
- [ ] Optional event handlers work when omitted
- [ ] Multiple handlers (`*`) collected into array
- [ ] Event params are captured correctly
- [ ] Literal params match correctly (0.3, "active")
- [ ] `%invoke $handler` generates correct code
- [ ] `%invoke $handler($arg)` passes arguments
- [ ] `%for $h in $handlers` iterates array handlers
- [ ] Generated JS is syntactically correct
- [ ] Handlers are properly scoped to element

## Files to Modify

| File | Changes |
|------|---------|
| `src/parser/grammar.pest` | Add `form_event_capture`, `macro_call_event_handler`, `invoke_clause` |
| `src/parser/meta_ast.rs` | Add `FormEventCapture`, `MacroCallEventHandler`, `InvokeClause` |
| `src/parser/ast.rs` | Update `MacroCallBody` with `event_handlers` |
| `src/parser/mod.rs` | Add parsing functions for event handlers |
| `src/metasystem/expand.rs` | Add event handler matching and binding |
| `src/metasystem/codegen.rs` | Add handler registration and invocation codegen |
