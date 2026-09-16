# Spacetime REPL: A Vision for Live Coding

## The Core Insight

Spacetime is fundamentally **visual and temporal**. Unlike traditional REPLs that evaluate expressions and print results, a Spacetime REPL must:

1. **Show** animations and visual effects in real-time
2. **Scrub through time** - timelines are the central abstraction
3. **Expose the compilation** - show what CSS/JS gets generated
4. **Visualize reactivity** - signals, reactive classes, and data flow

This isn't a text-in-text-out REPL. It's a **live coding environment** where you sculpt motion and interaction.

---

## The Three-Pane Architecture

```
┌──────────────────────────────────────────────────────────────────────┐
│  ░░░░░░░░░░░░░░░░░░░░░░ SPACETIME REPL ░░░░░░░░░░░░░░░░░░░░░░░░░░░░  │
├──────────────────────────┬───────────────────────────────────────────┤
│                          │                                           │
│   CODE EDITOR            │   LIVE PREVIEW                            │
│   ──────────────         │   ────────────                            │
│                          │                                           │
│   .box {                 │   ┌─────────────┐                         │
│     @on hover lift {     │   │             │                         │
│       scale: 1 -> 1.1;   │   │    BOX      │  ← Live element         │
│       translate-y: 0 ->  │   │             │                         │
│         -8px;            │   └─────────────┘                         │
│     }                    │                                           │
│   }                      │   ──────●────────── Timeline: 0.45       │
│                          │   0                                    1  │
│   [LSP: completions,     │                                           │
│    hover docs, errors]   │   [Play] [Pause] [Step] [Loop]            │
│                          │                                           │
├──────────────────────────┴───────────────────────────────────────────┤
│  INSPECTOR                                                           │
│  ─────────                                                           │
│                                                                      │
│  ┌─ Signals ────────────────────────────────────────────────────────┐│
│  │  .box: { hover-progress: 0.45, scale: 1.045, translate-y: -3.6px }│
│  └──────────────────────────────────────────────────────────────────┘│
│                                                                      │
│  ┌─ Generated Code ─────────────────────────────────────────────────┐│
│  │  [CSS] [JS] [AST] [IR]                                           ││
│  │                                                                   ││
│  │  // JS Output:                                                    ││
│  │  ST.watch(box, 'hover-progress', p => {                          ││
│  │    box.style.setProperty('--st-scale', lerp(1, 1.1, p));         ││
│  │    box.style.setProperty('--st-ty', lerp(0, -8, p) + 'px');      ││
│  │  });                                                              ││
│  └──────────────────────────────────────────────────────────────────┘│
└──────────────────────────────────────────────────────────────────────┘
```

---

## Core Features

### 1. Timeline Scrubber

The **timeline scrubber** is the heart of the REPL. Spacetime normalizes everything to 0→1 progress, so:

```
Timeline Progress: ──────────●────────────
                   0        0.6          1
                            ↑
                     Current position
```

- **Drag** to scrub through animation
- **Type** exact value (0.0-1.0) for precision
- **Keyboard**: Left/Right arrows for micro-steps
- **Modes**:
  - `Manual` - freeze time, scrub manually
  - `Play` - run animation at real speed
  - `Slow-mo` - 0.25x, 0.5x speed
  - `Loop` - continuous replay

For **scroll-driven** animations: simulate scroll position.
For **hover** animations: hover state toggle + scrub.
For **loop** animations: show full cycle with time display.

### 2. Live Preview Canvas

An isolated iframe/shadow DOM containing:
- Auto-generated HTML fixture (or user-provided)
- Compiled Spacetime output
- Interactive - hover, click, scroll work naturally

**Fixture Definition** - let users define the HTML structure:

```html
<!-- Inline in REPL or separate tab -->
<div class="box">Hover me</div>
```

Or auto-generate minimal fixtures:
```
Detected selectors: .box, .hero, #modal
→ Generated fixtures for each
```

### 3. Signal Inspector

Real-time view of all reactive state:

```
┌─ Active Signals ─────────────────────────────────────────┐
│                                                          │
│  .box                                                    │
│  ├── hover-progress: 0.45 ████████░░░░░░░░░░            │
│  ├── scale: 1.045                                       │
│  ├── translate-y: -3.6px                                │
│  └── --st-hover-progress (CSS var): 0.45                │
│                                                          │
│  .hero                                                   │
│  ├── scroll-progress: 0.72 ████████████████░░░░         │
│  └── opacity: 0.72                                       │
│                                                          │
│  body.$cart: [{id: 1, qty: 2}, {id: 3, qty: 1}]         │
│  @computed cartTotal: 47.50                              │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

- Click signal to see history graph
- Filter by element, signal name, or type
- Highlight which signals changed this frame

### 4. Reactive State Visualizer

For `$`-signal driven state, show the live signal value and which reactive classes are currently active:

```
┌─ Reactive State ─────────────────────────────────────────┐
│                                                          │
│  $panelState: "expanded"                                 │
│  ├── .inspector-panel.is-expanded   ● active             │
│  ├── .inspector-panel.is-collapsed  ○ inactive           │
│                                                          │
│  $activeTab: "js"                                        │
│  ├── [css] ○  [js] ●  [ast] ○  [ir] ○                    │
│                                                          │
│  Recent values:                                          │
│  $panelState: collapsed → expanded (0.3s ago)            │
│  $activeTab:  css → js (1.2s ago)                        │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

- Visual list of signal values and active reactive classes
- Highlight which classes match right now
- Show signal value history
- Click a value to manually override the signal

### 5. Code Generation View

Tabs showing what the compiler produces:

**CSS Tab:**
```css
.box {
  --st-scale: 1;
  --st-ty: 0px;
  transform: scale(var(--st-scale)) translateY(var(--st-ty));
  transition: transform 300ms var(--st-easing);
}
```

**JS Tab:**
```javascript
const box = document.querySelector('.box');
box.addEventListener('mouseenter', () => {
  ST.animate(box, 'hover-progress', 0, 1, 300, 'ease-out');
});
ST.watch(box, 'hover-progress', p => {
  ST.set(box, 'scale', lerp(1, 1.1, p));
  ST.set(box, 'translate-y', lerp(0, -8, p));
});
```

**AST Tab:** - Parsed structure
**IR Tab:** - Intermediate representation before emit
**Expansion Tab:** - Show macro expansion steps

### 6. LSP Integration

The editor pane gets full LSP features (already built):
- **Completions** - `@` triggers directive suggestions
- **Hover** - documentation for directives
- **Diagnostics** - red squiggles for errors
- **Go to Definition** - jump to macro/primitive definitions
- **Signature Help** - parameter hints

### 7. Keyframe Timeline

For complex multi-property animations, show a visual timeline:

```
┌─ Animation Timeline ─────────────────────────────────────┐
│                                                          │
│  opacity    ○━━━━━━━━━━━━━━━━━━━━○                       │
│             0                    1                       │
│                                                          │
│  scale      ○━━━━━━━━○━━━━━━━━━━━○                       │
│             0.9      1.05       1.0                      │
│                                                          │
│  translate  ○━━━━━━━━━━━━━━━━━━━━━━━━━━━━○               │
│  -y         40px                         0               │
│                                                          │
│  ──────────────────────●─────────────────                │
│  0%        25%        50%        75%    100%             │
│                        ↑                                 │
│                   Current: 50%                           │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

- Drag keyframe handles to adjust
- Visual easing curves between keyframes
- Multi-track view for complex animations

---

## Interaction Modes

### Mode 1: Exploration (Default)

Write Spacetime code, see it live. Good for learning and prototyping.

```
┌─────────────────────┬─────────────────────┐
│  Editor             │  Preview            │
│                     │                     │
│  Type code here     │  See results here   │
│                     │                     │
└─────────────────────┴─────────────────────┘
```

### Mode 2: Debugging

Inspect a specific animation or interaction in detail.

- Timeline frozen at specific point
- All signal values visible
- Step forward/backward frame by frame
- Compare expected vs actual values

### Mode 3: Testing

Write `@test` blocks and see results:

```spacetime
@test "hover lifts box" {
  @fixture { <div class="box">Test</div> }
  @when .box hover
  @then .box should have_style "transform" containing "scale(1.1)"
}
```

- Test tree view
- Pass/fail status
- Click to see test execution

### Mode 4: Performance

Profile animations:

```
┌─ Performance ────────────────────────────────────────────┐
│                                                          │
│  Frame Budget: ████████░░ 12ms (target: 16.67ms)        │
│                                                          │
│  Breakdown:                                              │
│  ├── Layout: 2ms                                         │
│  ├── Paint: 4ms                                          │
│  ├── Composite: 1ms                                      │
│  └── JS (signals): 5ms                                   │
│                                                          │
│  ⚠️ Warning: .hero uses `width` animation (triggers      │
│     layout). Consider using `transform: scaleX()`.       │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

---

## Implementation Strategy

### Phase 1: Foundation (Build on Existing Infrastructure)

The existing codebase already has:
- **WebSocket dev server** (`src/dev_server.rs`) - hot reload infrastructure
- **LSP server** (`src/lsp/`) - editor integration
- **Test runner** (`src/test_runner.rs`) - browser-based execution
- **Compiler API** (`Compiler::from_source()` / `compile()`) - programmatic access

Build the REPL UI as a **special Spacetime site** that dogfoods the system:

```
projects/repl/
├── index.html        # REPL shell
├── editor.st         # Editor component (wrap Monaco/CodeMirror)
├── preview.st        # Iframe preview
├── inspector.st      # Signal inspection
├── timeline.st       # Scrubber controls
└── styles.st         # REPL styling
```

### Phase 2: Core REPL Loop

```
User types code
    ↓
Debounced compilation (100ms)
    ↓
Push IR to preview iframe
    ↓
Iframe executes, reports signals back
    ↓
Inspector updates
```

**WebSocket Messages:**

```typescript
// REPL → Preview
{ type: "Compile", source: string, fixture: string }
{ type: "Scrub", progress: number }
{ type: "SetSignal", name: string, value: any }

// Preview → REPL
{ type: "SignalUpdate", element: string, signals: Record<string, any> }
{ type: "SignalChange", name: string, oldValue: any, newValue: any }
{ type: "Error", message: string, span: SourceSpan }
```

### Phase 3: Timeline Control

Implement a `TimelineController` that intercepts Spacetime's runtime:

```javascript
// Injected into preview
window.SpacetimeREPL = {
  pause() { ST._paused = true; },
  resume() { ST._paused = false; },
  scrub(progress) {
    // Force all active timelines to this progress
    for (const name of ST.timelines.keys()) {
      ST.timelines.setProgress(name, progress);
    }
  },
  step(frames = 1) {
    // Advance by N frames (16.67ms each)
  }
};
```

### Phase 4: Advanced Features

- **Collaborative mode** - share REPL session via Spacetime sync
- **Snippets library** - searchable patterns
- **Export** - download standalone HTML/CSS/JS
- **Import** - load from existing .st files
- **History** - undo/redo with timeline replay

---

## UI/UX Principles

### 1. Instant Feedback
- Compile on every keystroke (debounced)
- No "Run" button - always running
- Errors appear inline, don't block preview

### 2. Progressive Disclosure
- Start simple (editor + preview)
- Inspector collapsed by default
- Advanced panels (AST, IR) in tabs

### 3. Keyboard-First
- `Cmd+Enter` - force recompile
- `Space` - play/pause
- `Left/Right` - scrub timeline
- `Cmd+Shift+I` - toggle inspector
- `Cmd+S` - save to local storage

### 4. Mobile-Friendly Preview
- Device frame selector (iPhone, Android, tablet)
- Touch event simulation
- Orientation toggle

### 5. Dark/Light Theme
- Follow system preference
- Syntax highlighting adapts
- Preview can be forced to either mode

---

## Unique REPL Affordances for Spacetime

### 1. **"Catch" an Animation Mid-Flight**

Click any animated element to freeze time at that moment. The scrubber jumps to current progress. Inspect values. Resume.

### 2. **"Record" Interaction Sequences**

Record a series of clicks/hovers/scrolls, replay as automated test.

### 3. **Visual Diff**

Change code, see before/after side-by-side at same timeline position.

### 4. **Easing Curve Editor**

Click easing function → visual bezier editor:
```
     ●──────────────●
    ╱                ╲
   ╱                  ╲
  ●                    ●

  cubic-bezier(0.4, 0, 0.2, 1)
```

### 5. **Animation Ghosting**

Show translucent "ghost" of element at keyframe positions:
```
  ┌───┐      ┌───┐      ┌───┐
  │ 0%│  →   │50%│  →   │100│
  └───┘      └───┘      └───┘
   ░░░        ▒▒▒        ███
```

### 6. **Data Playground**

For `@data` and `@each`, mock the data source:
```json
// Mock products
[
  { "id": 1, "name": "Widget", "price": 9.99 },
  { "id": 2, "name": "Gadget", "price": 19.99 }
]
```

Edit mock data, see template re-render live.

---

## Technical Considerations

### Sandboxing
- Preview runs in sandboxed iframe
- No access to parent window
- Network requests proxied through REPL server

### State Persistence
- Editor content in localStorage
- Recent files list
- Undo history survives refresh

### Sharing
- Generate shareable URL with code embedded
- Optional: GitHub Gist integration

### Offline Support
- Service worker for offline use
- Compiler runs in WASM (already exists)
- No server needed for basic use

---

## Summary

A Spacetime REPL is not a traditional text REPL—it's a **temporal visualization environment** where:

1. **Time is the primary axis** - scrub, pause, step through animations
2. **Code and visuals are unified** - no separation between "writing" and "running"
3. **Compilation is transparent** - see exactly what gets generated
4. **Reactivity is visible** - signals, reactive classes, and data flow all inspectable
5. **Built with Spacetime** - dogfoods the system itself

The existing infrastructure (LSP, dev server, test runner, WASM compiler) provides 70% of the foundation. The unique value is the **timeline-centric debugging** and **visual signal inspection** that no other REPL offers.

---

## Implementation Roadmap

### Phase 1: MVP Prototype (Foundation)

**Goal**: Working two-pane editor + preview with hot-reload

**Components**:
```
projects/repl/
├── index.html           # Shell with layout
├── index.st             # REPL state & interactions
└── styles.css           # Layout styling
```

**Features**:
1. **Code Editor** - Embed CodeMirror 6 with basic Spacetime syntax highlighting
2. **Live Preview** - Sandboxed iframe, recompiles on change
3. **Hot Reload** - Leverage existing WebSocket infrastructure
4. **Error Display** - Inline error messages from compiler

**Reuse from Codebase**:
- `src/dev_server.rs` - WebSocket message handling
- `src/watcher.rs` - File change debouncing pattern
- `public/runtime/st.js` - Full runtime already built

**New Code Required**:
- CodeMirror Spacetime mode (~200 LOC)
- REPL shell HTML/CSS (~300 LOC)
- Compile-on-type endpoint (~100 LOC in server)

**Deliverable**: `spacetime repl` command launches browser-based editor

---

### Phase 2: Timeline Scrubbing

**Goal**: Pause, scrub, and step through animations

**New Components**:
```
public/runtime/
├── repl-controller.js   # Timeline interception
└── signal-reporter.js   # Report signals to parent frame
```

**Features**:
1. **Timeline Scrubber UI** - Slider 0→1 with current progress display
2. **Play/Pause Controls** - Freeze time, step frame-by-frame
3. **Driver Selection** - Choose which timeline to control (scroll, hover, loop)
4. **PostMessage Bridge** - Preview ↔ REPL communication

**Technical Approach**:
```javascript
// Inject into preview iframe
window.STRepl = {
  pause() { ST._frozen = true; },
  setProgress(timeline, p) {
    ST.timelines.setProgress(timeline, p);
  },
  getSignals(el) {
    return Object.fromEntries(ST.signals.get(el) || []);
  }
};
```

**Deliverable**: Scrub through any animation, see values at any point

---

### Phase 3: Signal Inspector

**Goal**: Real-time visualization of all reactive state

**Features**:
1. **Signal Tree** - Hierarchical view: element → signals → values
2. **Live Updates** - Values update as animations run
3. **History Graph** - Click signal to see value over time
4. **Filter/Search** - Find specific signals

**UI Component** (in REPL shell):
```
┌─ Signals ──────────────────────────┐
│ ▼ .box                             │
│   hover-progress: 0.45 ████░░░░░   │
│   scale: 1.045                     │
│   translate-y: -3.6px              │
│ ▶ .hero (2 signals)                │
│ ▶ body.$cart (array, 3 items)      │
└────────────────────────────────────┘
```

**Deliverable**: Inspect all signals live, understand animation state

---

### Phase 4: Code Generation View

**Goal**: Show what the compiler produces

**Features**:
1. **Tab Panel** - [CSS] [JS] [AST] [IR] [Expansion]
2. **Syntax Highlighted** - Pretty-printed output
3. **Source Mapping** - Hover code → highlight source
4. **Diff View** - Before/after on change

**Leverage**:
- `Compiler` builder API - Returns structured IR
- `EmitOptions::pretty()` - Readable output

**Deliverable**: Full transparency into compilation

---

### Phase 5: Reactive State Visualizer

**Goal**: Visual debugging for `$`-signal driven state and reactive classes

**Features**:
1. **Signal Value Panel** - Show current value of each `$`-signal
2. **Reactive Class Highlight** - Indicate which `.is-*` classes are active
3. **Value History** - Log of recent signal changes
4. **Manual Override** - Click a value to force the signal

**Technical Approach**:
- Read signal values from the runtime signal graph
- Map each signal to the reactive classes it drives (`is-loading`, `is-expanded`, etc.)
- Render with simple DOM or SVG (no heavy graph library)
- Hook into ST runtime signal change events

**Deliverable**: Visual understanding of reactive state flow

---

### Phase 6: LSP Integration

**Goal**: Full IDE experience in the REPL

**Features**:
1. **Autocomplete** - `@` triggers directive suggestions
2. **Hover Docs** - Documentation on hover
3. **Error Squiggles** - Red underlines for diagnostics
4. **Go to Definition** - Jump to macro source

**Approach**:
- Run LSP server in WASM (already compiles to WASM)
- Or: WebSocket to running `spacetime lsp` process
- CodeMirror has LSP client extensions

**Deliverable**: No need to leave REPL for docs or exploration

---

### Phase 7: Advanced Features

**Features** (prioritize based on usage):
1. **Fixture Editor** - Define HTML structure for preview
2. **Test Mode** - Write/run `@test` blocks
3. **Performance Panel** - Frame budget, paint counts
4. **Share/Export** - Shareable URLs, download standalone
5. **Snippet Library** - Searchable patterns
6. **Collaborative Mode** - Real-time multiplayer via Spacetime sync

---

## File Structure (Full Implementation)

```
projects/repl/
├── index.html              # Entry point
├── index.st                # REPL orchestration
├── styles.css              # Layout
├── components/
│   ├── editor.st           # CodeMirror wrapper
│   ├── preview.st          # Iframe + controls
│   ├── timeline.st         # Scrubber UI
│   ├── inspector.st        # Signal tree
│   ├── codegen.st          # Generated code tabs
│   └── reactive-state.st   # Reactive state inspector
└── runtime/
    ├── repl-controller.js  # Preview interception
    └── signal-bridge.js    # PostMessage protocol
```

**Server-side** (minimal additions to existing):
```
src/
├── repl.rs                 # REPL-specific endpoints
└── dev_server.rs           # Add compile-on-demand
```

---

## CLI Integration

```bash
# Launch REPL
spacetime repl [--port 3030]

# Launch with initial file
spacetime repl examples/demo.st

# Launch with fixture
spacetime repl --fixture fixture.html
```

**Implementation** in `src/main.rs`:
```rust
Repl { port: Option<u16>, file: Option<PathBuf> } => {
    // Start dev server in REPL mode
    // Open browser to localhost:port
}
```

---

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Packaging** | Both (embedded first, then standalone) | Start with `spacetime repl` for fast iteration, later extract to WASM-powered web app |
| **Editor** | CodeMirror 6, abstracted | Wrap CM6 behind interface so minimal custom can replace later |
| **Dogfooding** | Full | REPL UI uses Spacetime for all animations/interactions - proves the system |

---

## Prototype Scope (What to Build First)

For a minimal viable prototype, implement:

1. ✅ **Editor pane** - CodeMirror 6 with abstraction layer
2. ✅ **Preview pane** - Iframe with hot reload
3. ✅ **Timeline scrubber** - Pause + drag to scrub
4. ✅ **Basic signal display** - Show top-level signal values
5. ✅ **Dogfood UI** - Panels use Spacetime signals & animations

Skip for prototype:
- LSP integration (use basic syntax highlighting)
- Reactive state inspector (defer)
- Code generation view (defer)
- Performance panel (defer)

---

## Editor Abstraction Layer

The editor will be wrapped to allow swapping implementations:

```typescript
// public/runtime/editor-interface.ts
interface SpacetimeEditor {
  // Core
  getValue(): string;
  setValue(code: string): void;

  // Events
  onChange(callback: (code: string) => void): void;
  onCursorChange(callback: (pos: Position) => void): void;

  // Cursor
  getCursor(): Position;
  setCursor(pos: Position): void;

  // Selection
  getSelection(): string;
  setSelection(from: Position, to: Position): void;

  // Decorations
  addDecoration(range: Range, className: string): DecorationHandle;
  removeDecoration(handle: DecorationHandle): void;

  // Markers (for errors, warnings)
  addMarker(line: number, type: 'error' | 'warning' | 'info', message: string): MarkerHandle;
  clearMarkers(): void;

  // Focus
  focus(): void;
  blur(): void;
}

// Implementations
class CodeMirrorEditor implements SpacetimeEditor { ... }
class MinimalEditor implements SpacetimeEditor { ... }  // Future: textarea-based
```

This allows:
- Start with CodeMirror 6 for full features
- Swap to minimal textarea for smaller bundle
- Add Monaco later if needed
- Test editors in isolation

---

## Dogfooding Strategy

The REPL UI will use Spacetime features:

### Panel States (Inspector, Codegen)
```spacetime
.inspector-panel {
  $panelState <- "collapsed"

  .is-collapsed: $panelState == "collapsed";
  .is-expanded: $panelState == "expanded";

  &.is-collapsed {
    height: 32px;
    .content { opacity: 0; pointer-events: none; }
  }
  &.is-expanded {
    height: 300px;
    .content { opacity: 1; pointer-events: auto; }
  }

  @on &.click:.toggle {
    $panelState <- $panelState == "collapsed" ? "expanded" : "collapsed";
  }

  @transition { height 300ms var(--ease-out-expo); }
}
```

### Timeline Scrubber Interactions
```spacetime
.timeline-scrubber {
  @on &.hover lift(200ms) {
    .thumb { scale: 1 -> 1.2; }
  }

  @drag {
    .thumb { cursor: grabbing; }
  }
}
```

### Signal Value Animations
```spacetime
.signal-value {
  @on update flash(150ms) {
    background: transparent -> var(--highlight) -> transparent;
  }
}
```

### Tab Transitions
```spacetime
$activeTab <- "js"

.codegen-tabs {
  .tab-btn.is-active: $activeTab == @this.dataset.tab;

  @view $activeTab {
    "css" => &cssPanel();
    "js"  => &jsPanel();
    "ast" => &astPanel();
    "ir"  => &irPanel();
  }
}

@on &.click:[data-tab="css"] { $activeTab <- "css"; }
@on &.click:[data-tab="js"]  { $activeTab <- "js"; }
@on &.click:[data-tab="ast"] { $activeTab <- "ast"; }
@on &.click:[data-tab="ir"]  { $activeTab <- "ir"; }
```

This proves Spacetime can build real tools, not just demos.

---

## Critical Files to Create/Modify

### New Files
```
projects/repl/
├── index.html                    # REPL shell
├── index.st                      # Main Spacetime file
├── styles.css                    # Base layout
├── editor.st                     # Editor wrapper component
├── preview.st                    # Preview iframe component
├── timeline.st                   # Scrubber component
├── inspector.st                  # Signal inspector
└── fixtures/
    └── default.html              # Default preview fixture

public/runtime/
├── editor-interface.js           # Editor abstraction
├── codemirror-impl.js            # CM6 implementation
├── repl-controller.js            # Preview interception
└── signal-bridge.js              # PostMessage protocol

src/
├── repl.rs                       # REPL server mode
└── main.rs                       # Add `repl` subcommand
```

### Modified Files
```
src/dev_server.rs                 # Add compile-on-demand endpoint
Cargo.toml                        # (if new dependencies needed)
```

---

## Next Steps After Prototype

1. **Extract to standalone** - WASM compile, host on spacetime.dev/repl
2. **Add LSP** - WebSocket to `spacetime lsp` or WASM LSP
3. **Reactive state inspector** - Signal/class visualization
4. **Share feature** - URL-encoded code + Gist integration
5. **Collaborative mode** - Multi-cursor via Spacetime sync
