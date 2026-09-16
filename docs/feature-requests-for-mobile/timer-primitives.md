# Feature Request: Timer Primitives

## Problem Statement

Mobile gesture patterns often require time-based behavior:
- **Long-press detection**: Progress updates during a hold
- **Debouncing**: Prevent rapid repeated triggers
- **Momentum animation**: Physics-based decay after release
- **Auto-hide**: Elements that fade after inactivity
- **Delayed actions**: Execute after a timeout

Currently, Spacetime primitives can use `requestAnimationFrame` and `setTimeout` in their `%emit js` blocks, but there's no standardized way to:
1. Integrate timer callbacks with the `%yield` system
2. Handle cleanup of timers automatically
3. Coordinate multiple timers within a primitive

### Current Workaround

```spacetime
%primitive debounced-input(&el, delay: number = 300) {
  %emit js {
    let timeoutId = null;

    const handler = (e) => {
      if (timeoutId) clearTimeout(timeoutId);
      timeoutId = setTimeout(() => {
        %yield e.target.value -> $value;
      }, %delay);
    };

    el.addEventListener('input', handler);

    %cleanup {
      if (timeoutId) clearTimeout(timeoutId);
      el.removeEventListener('input', handler);
    }
  }

  %exports {
    $value: string
  }
}
```

This works but is verbose and error-prone. We need better patterns for timer-based primitives.

## Proposed Solutions

### Solution 1: Timer Helper Functions

Add built-in timer helpers that integrate with cleanup automatically.

#### Proposed Syntax

```spacetime
%primitive long-press-driver(&el, duration: number = 500) {
  %emit js {
    let progress = 0;

    const onPointerDown = (e) => {
      // %timer creates a managed timer that auto-cleans up
      %timer.start("progress", () => {
        const elapsed = performance.now() - startTime;
        progress = Math.min(elapsed / %duration, 1);
        %yield progress -> $progress;
        return progress < 1; // Return true to continue, false to stop
      }, "raf");  // "raf" for requestAnimationFrame, or milliseconds for setTimeout

      %yield true -> $active;
    };

    const onPointerUp = () => {
      %timer.stop("progress");
      %yield false -> $active;
    };

    el.addEventListener('pointerdown', onPointerDown);
    el.addEventListener('pointerup', onPointerUp);
  }

  %exports {
    $active: bool
    $progress: number
  }
}
```

#### Timer Helper API

```javascript
// Generated runtime helpers
const __timers = new Map();

%timer.start(name, callback, interval) {
  // interval can be:
  //   "raf" - requestAnimationFrame
  //   number - setTimeout/setInterval with ms
  //   { type: "interval", ms: 100 } - setInterval
  //   { type: "timeout", ms: 100 } - setTimeout (one-shot)
}

%timer.stop(name) {
  // Cancel the named timer
}

%timer.stopAll() {
  // Cancel all timers (called automatically on cleanup)
}
```

### Solution 2: Timer Primitive

Create a reusable timer primitive that other primitives can bind to.

```spacetime
%primitive raf-loop(&el) {
  %emit js {
    let running = false;
    let rafId = null;
    let startTime = null;

    const start = () => {
      if (running) return;
      running = true;
      startTime = performance.now();
      rafId = requestAnimationFrame(tick);
      %yield true -> $running;
    };

    const stop = () => {
      if (!running) return;
      running = false;
      cancelAnimationFrame(rafId);
      %yield false -> $running;
    };

    const tick = () => {
      if (!running) return;
      const elapsed = performance.now() - startTime;
      %yield elapsed -> $elapsed;
      %yield performance.now() -> $now;
      rafId = requestAnimationFrame(tick);
    };

    // Export control functions
    %yield start -> $start;
    %yield stop -> $stop;

    %cleanup {
      stop();
    }
  }

  %exports {
    $running: bool
    $elapsed: number
    $now: number
    $start: fn() ~> void
    $stop: fn() ~> void
  }
}
```

Usage in another primitive:

```spacetime
%primitive long-press-progress(&el, duration: number = 500) {
  // Bind to the raf-loop primitive
  %binds {
    raf-loop(&self) -> { $elapsed, $start, $stop, $running }
  }

  %emit js {
    const onPointerDown = () => {
      $start();
      %yield true -> $active;
    };

    const onPointerUp = () => {
      $stop();
      %yield false -> $active;
    };

    el.addEventListener('pointerdown', onPointerDown);
    el.addEventListener('pointerup', onPointerUp);

    %cleanup {
      el.removeEventListener('pointerdown', onPointerDown);
      el.removeEventListener('pointerup', onPointerUp);
    }
  }

  %derives {
    $progress: Math.min($elapsed / %duration, 1)
    $complete: $progress >= 1 && !$running
  }

  %exports {
    $active: bool
    $progress: number
    $complete: bool
  }
}
```

### Solution 3: Async/Timer Block Syntax

Add special blocks for timer-based code that integrates with cleanup.

```spacetime
%primitive auto-hide(&el, timeout: number = 3000) {
  %emit js {
    const show = () => {
      el.classList.add('visible');
      %yield true -> $visible;
    };

    const hide = () => {
      el.classList.remove('visible');
      %yield false -> $visible;
    };

    // Movement resets the timer
    el.addEventListener('mousemove', show);
    el.addEventListener('touchmove', show);
  }

  // New: %after block for delayed execution
  %after $visible -> true, %timeout {
    hide();
  }

  // Alternative: %debounce block
  %debounce "activity", 3000 {
    // Runs 3s after last activity
    hide();
  }

  %exports {
    $visible: bool
  }
}
```

## Implementation Details

### 1. Grammar Changes (`src/parser/grammar.pest`)

#### Add Timer Syntax

```pest
// Timer helper calls in emit blocks
emit_timer_call = {
    "%timer." ~ timer_method ~ "(" ~ timer_args ~ ")"
}

timer_method = { "start" | "stop" | "stopAll" }
timer_args = { (timer_arg ~ ("," ~ timer_arg)*)? }
timer_arg = { string | number | timer_callback | identifier }
timer_callback = { "(" ~ ")" ~ "=>" ~ "{" ~ emit_content ~ "}" }

// After block (post-emit)
after_block = {
    "%after" ~ after_condition ~ "," ~ after_delay ~ "{" ~ emit_main_content ~ "}"
}

after_condition = { "$" ~ identifier ~ "->" ~ meta_on_value }
after_delay = { "%"? ~ identifier | number ~ time_unit? }

// Debounce block
debounce_block = {
    "%debounce" ~ string ~ "," ~ number ~ "{" ~ emit_main_content ~ "}"
}
```

### 2. AST Changes (`src/parser/meta_ast.rs`)

```rust
/// Timer call in emit block
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TimerCall {
    Start {
        name: String,
        callback: String,  // JS callback code
        interval: TimerInterval,
    },
    Stop {
        name: String,
    },
    StopAll,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TimerInterval {
    Raf,                    // requestAnimationFrame
    Timeout(u32),           // setTimeout with ms
    Interval(u32),          // setInterval with ms
}

/// After block - delayed execution after signal change
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AfterBlock {
    pub signal: String,
    pub value: String,
    pub delay_ms: u32,
    pub body: String,
    pub span: SourceSpan,
}

/// Debounce block - execute after inactivity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebounceBlock {
    pub name: String,
    pub delay_ms: u32,
    pub body: String,
    pub span: SourceSpan,
}

/// Update PrimitiveBody
pub struct PrimitiveBody {
    pub emit_blocks: Vec<EmitBlock>,
    pub cleanup: Option<String>,
    pub exports: Vec<ExportDecl>,
    pub if_blocks: Vec<PrimitiveIfBlock>,
    /// Timer-based blocks (new)
    pub after_blocks: Vec<AfterBlock>,
    pub debounce_blocks: Vec<DebounceBlock>,
}
```

### 3. Codegen Changes (`src/metasystem/codegen.rs`)

#### Transform Timer Calls

```rust
static TIMER_START_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"%timer\.start\("([^"]+)",\s*(.+?),\s*"?(raf|\d+)"?\)"#).unwrap()
});

static TIMER_STOP_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"%timer\.stop\("([^"]+)"\)"#).unwrap()
});

fn transform_timer_calls(code: &str) -> String {
    let mut result = code.to_string();

    // Transform %timer.start
    result = TIMER_START_REGEX.replace_all(&result, |caps: &regex::Captures| {
        let name = &caps[1];
        let callback = &caps[2];
        let interval = &caps[3];

        if interval == "raf" {
            format!(r#"
                (function() {{
                    const __timer_{name} = {{ id: null, running: false }};
                    __timers.set('{name}', __timer_{name});
                    const __tick_{name} = () => {{
                        if (!__timer_{name}.running) return;
                        const __continue = ({callback})();
                        if (__continue) {{
                            __timer_{name}.id = requestAnimationFrame(__tick_{name});
                        }} else {{
                            __timer_{name}.running = false;
                        }}
                    }};
                    __timer_{name}.running = true;
                    __timer_{name}.id = requestAnimationFrame(__tick_{name});
                }})()"#, name = name, callback = callback)
        } else {
            let ms: u32 = interval.parse().unwrap_or(0);
            format!(r#"
                (function() {{
                    const __timer_{name} = {{ id: null }};
                    __timers.set('{name}', __timer_{name});
                    __timer_{name}.id = setInterval(() => {{
                        ({callback})();
                    }}, {ms});
                }})()"#, name = name, callback = callback, ms = ms)
        }
    }).to_string();

    // Transform %timer.stop
    result = TIMER_STOP_REGEX.replace_all(&result, |caps: &regex::Captures| {
        let name = &caps[1];
        format!(r#"
            (function() {{
                const __t = __timers.get('{name}');
                if (__t) {{
                    if (__t.running !== undefined) {{
                        __t.running = false;
                        cancelAnimationFrame(__t.id);
                    }} else {{
                        clearInterval(__t.id);
                        clearTimeout(__t.id);
                    }}
                    __timers.delete('{name}');
                }}
            }})()"#, name = name)
    }).to_string();

    result
}

/// Generate cleanup code for timers
fn generate_timer_cleanup() -> String {
    r#"
    // Clean up all timers
    for (const [name, timer] of __timers) {
        if (timer.running !== undefined) {
            timer.running = false;
            cancelAnimationFrame(timer.id);
        } else {
            clearInterval(timer.id);
            clearTimeout(timer.id);
        }
    }
    __timers.clear();
    "#.to_string()
}
```

#### Generate After Block

```rust
fn generate_after_block(block: &AfterBlock) -> String {
    format!(r#"
        ST.watch(el, '{}', (v) => {{
            if (v === {}) {{
                const __afterId = setTimeout(() => {{
                    {}
                }}, {});
                ST.onCleanup(el, () => clearTimeout(__afterId));
            }}
        }});
    "#, block.signal, block.value, block.body, block.delay_ms)
}
```

#### Generate Debounce Block

```rust
fn generate_debounce_block(block: &DebounceBlock) -> String {
    format!(r#"
        (function() {{
            let __debounce_{name}_id = null;
            const __debounce_{name} = () => {{
                if (__debounce_{name}_id) clearTimeout(__debounce_{name}_id);
                __debounce_{name}_id = setTimeout(() => {{
                    {body}
                }}, {delay});
            }};
            // Store for external triggering
            el.__st_debounce_{name} = __debounce_{name};
            ST.onCleanup(el, () => {{
                if (__debounce_{name}_id) clearTimeout(__debounce_{name}_id);
            }});
        }})();
    "#, name = block.name, body = block.body, delay = block.delay_ms)
}
```

### 4. Runtime Helpers (`public/runtime/st-minimal.js`)

Add timer management to the runtime:

```javascript
// Timer management for primitives
const ST = {
  // ... existing methods ...

  // Timer registry per element
  timers: new WeakMap(),

  // Get or create timer registry for element
  getTimers(el) {
    if (!this.timers.has(el)) {
      this.timers.set(el, new Map());
    }
    return this.timers.get(el);
  },

  // Start a RAF-based timer
  rafTimer(el, name, callback) {
    const timers = this.getTimers(el);
    const timer = { running: true, id: null };

    const tick = () => {
      if (!timer.running) return;
      const shouldContinue = callback();
      if (shouldContinue && timer.running) {
        timer.id = requestAnimationFrame(tick);
      }
    };

    timer.id = requestAnimationFrame(tick);
    timers.set(name, timer);

    return () => {
      timer.running = false;
      cancelAnimationFrame(timer.id);
      timers.delete(name);
    };
  },

  // Start an interval timer
  intervalTimer(el, name, callback, ms) {
    const timers = this.getTimers(el);
    const id = setInterval(callback, ms);
    timers.set(name, { id, type: 'interval' });

    return () => {
      clearInterval(id);
      timers.delete(name);
    };
  },

  // Start a timeout timer
  timeoutTimer(el, name, callback, ms) {
    const timers = this.getTimers(el);
    const id = setTimeout(() => {
      callback();
      timers.delete(name);
    }, ms);
    timers.set(name, { id, type: 'timeout' });

    return () => {
      clearTimeout(id);
      timers.delete(name);
    };
  },

  // Stop a specific timer
  stopTimer(el, name) {
    const timers = this.getTimers(el);
    const timer = timers.get(name);
    if (timer) {
      if (timer.running !== undefined) {
        timer.running = false;
        cancelAnimationFrame(timer.id);
      } else if (timer.type === 'interval') {
        clearInterval(timer.id);
      } else {
        clearTimeout(timer.id);
      }
      timers.delete(name);
    }
  },

  // Stop all timers for an element
  stopAllTimers(el) {
    const timers = this.getTimers(el);
    for (const [name] of timers) {
      this.stopTimer(el, name);
    }
  },
};
```

## Example: Complete Timer-Based Primitives

### Long Press with Progress

```spacetime
%primitive long-press-progress(&el, duration: number = 500) {
  %emit js {
    const timers = ST.getTimers(el);
    let startTime = null;
    let progress = 0;

    const onPointerDown = (e) => {
      startTime = performance.now();
      progress = 0;
      %yield true -> $active;
      %yield 0 -> $progress;

      // Start RAF timer for progress updates
      ST.rafTimer(el, 'progress', () => {
        const elapsed = performance.now() - startTime;
        progress = Math.min(elapsed / %duration, 1);
        %yield progress -> $progress;

        if (progress >= 1) {
          %yield true -> $complete;
          return false; // Stop timer
        }
        return true; // Continue timer
      });

      e.preventDefault();
    };

    const onPointerUp = () => {
      ST.stopTimer(el, 'progress');
      %yield false -> $active;

      if (progress < 1) {
        %yield true -> $cancelled;
      }
      progress = 0;
    };

    el.addEventListener('pointerdown', onPointerDown);
    el.addEventListener('pointerup', onPointerUp);
    el.addEventListener('pointercancel', onPointerUp);
  }

  %cleanup {
    ST.stopAllTimers(el);
    el.removeEventListener('pointerdown', onPointerDown);
    el.removeEventListener('pointerup', onPointerUp);
    el.removeEventListener('pointercancel', onPointerUp);
  }

  %exports {
    $active: bool
    $progress: number
    $complete: bool
    $cancelled: bool
  }
}
```

### Debounced Search Input

```spacetime
%primitive debounced-input(&el, delay: number = 300) {
  %emit js {
    let debounceId = null;

    const handler = (e) => {
      %yield e.target.value -> $immediateValue;
      %yield true -> $typing;

      if (debounceId) clearTimeout(debounceId);
      debounceId = ST.timeoutTimer(el, 'debounce', () => {
        %yield e.target.value -> $debouncedValue;
        %yield false -> $typing;
      }, %delay);
    };

    el.addEventListener('input', handler);
  }

  %cleanup {
    ST.stopAllTimers(el);
    el.removeEventListener('input', handler);
  }

  %exports {
    $immediateValue: string
    $debouncedValue: string
    $typing: bool
  }
}
```

### Auto-Hide Panel

```spacetime
%primitive auto-hide(&el, timeout: number = 3000) {
  %emit js {
    let visible = true;

    const show = () => {
      if (!visible) {
        el.classList.add('visible');
        visible = true;
        %yield true -> $visible;
      }
      resetTimer();
    };

    const hide = () => {
      el.classList.remove('visible');
      visible = false;
      %yield false -> $visible;
    };

    const resetTimer = () => {
      ST.stopTimer(el, 'autohide');
      ST.timeoutTimer(el, 'autohide', hide, %timeout);
    };

    // Track activity
    el.addEventListener('mousemove', show);
    el.addEventListener('touchstart', show);

    // Start initial timer
    resetTimer();
    %yield true -> $visible;
  }

  %cleanup {
    ST.stopAllTimers(el);
    el.removeEventListener('mousemove', show);
    el.removeEventListener('touchstart', show);
  }

  %exports {
    $visible: bool
  }
}
```

## Migration / Compatibility Notes

### Backward Compatibility

1. **Existing timer code works**: Manual `setTimeout`/`requestAnimationFrame` still work
2. **No breaking changes**: New helpers are additive
3. **Cleanup is enhanced**: `ST.stopAllTimers` is called automatically in generated cleanup

### Best Practices

1. Always use named timers for easy cleanup
2. Use `ST.rafTimer` for animation-related updates
3. Use `ST.timeoutTimer` for delayed actions
4. Use `ST.intervalTimer` for periodic polling
5. Call `ST.stopAllTimers(el)` in cleanup for completeness

## Testing Checklist

- [ ] `ST.rafTimer` creates and runs RAF loop
- [ ] `ST.rafTimer` stops when callback returns false
- [ ] `ST.stopTimer` cancels specific timer
- [ ] `ST.stopAllTimers` cancels all timers for element
- [ ] Timer cleanup happens on element removal
- [ ] Multiple timers can run simultaneously
- [ ] Timer state persists across re-renders
- [ ] `%timer.start` syntax parses correctly
- [ ] `%timer.stop` syntax parses correctly
- [ ] `%after` block generates correct code
- [ ] `%debounce` block generates correct code

## Files to Modify

| File | Changes |
|------|---------|
| `src/parser/grammar.pest` | Add timer syntax rules (optional) |
| `src/parser/meta_ast.rs` | Add `AfterBlock`, `DebounceBlock` (optional) |
| `src/metasystem/codegen.rs` | Add timer transformation functions |
| `public/runtime/st-minimal.js` | Add `ST.rafTimer`, `ST.intervalTimer`, etc. |
