# Runtime debugging

Loaded on demand from `spacetime-design` SKILL.md when the symptom is
runtime-side (animation, CSS, browser, console).
# Debug Spacetime Animations

## When to use

- Animations not starting (especially `@load` timelines)
- CSS transitions/keyframes not applied
- Elements not animating despite correct DSL
- Need to inspect runtime state (timelines, progress values)
- Verifying DSL compiled correctly

## Diagnostic Methods (Built into Runtime)

The Spacetime runtime exposes these methods on `window.Spacetime`:

### `Spacetime.diagnose()`
Quick health check returning:
- `initialized` - Is runtime initialized?
- `timelineCount` - Total timelines
- `timelines[]` - Each timeline's id, driver, progress, started/completed/destroyed state, elementCount
- `cssVars` - All `--st-*-progress` CSS variables and values
- `readyState` - Document load state

### `Spacetime.inspectTimeline(id)`
Deep inspection of a specific timeline:
- `driver` - Full driver config (type, duration, threshold, immediate, etc.)
- `state` - started/completed/destroyed/active flags
- `elements` - Cached DOM elements per selector with visibility info
- `animations` - Animation configs with properties and keyframes
- `cleanup` - RAF/timeout/observer tracking

### `Spacetime.inspectElement(selector)`
Check CSS application on elements:
- `computedStyles` - opacity, transform, filter, clip-path, stroke-dashoffset
- `inlineStyles` - Styles set by runtime
- `spacetimeState` - Internal `_stTransforms`, `_stFilters` accumulators
- `rect` - Bounding box
- `visible`, `inViewport` - Visibility flags

### `Spacetime.forcePlay(id, durationMs)`
Manually trigger a timeline to verify it CAN animate:
- Resets progress to 0
- Animates to 1 over specified duration (default 1000ms)
- Useful to test if interpolation works

### `Spacetime.review()` (FEAT-015)

5-dimension audit of the loaded page. Returns:

```js
{
  score: 0..10,                       // = min over dim scores
  dims: { idiomatic, animation, brand, a11y, compile },
  findings: [
    { dim, severity: 'error'|'warn'|'info', message, selector?, value? }
  ],
  generatedAt: number                  // Date.now()
}
```

Per-dim score formula: `clamp(10 - errors*2 - warns, 0, 10)`.

Only defined when the dev runtime is loaded (i.e. `cargo run -- serve`).
Production builds never expose `Spacetime.review`. The walker is
implemented in `stdlib/__dev__/primitives/dev-review.st`.

## Chrome DevTools MCP Workflow

### Step 1: Health Check
```
mcp__chrome-devtools__evaluate_script({
  function: "() => window.Spacetime.diagnose()"
})
```

Check:
- `initialized: true`
- `timelineCount` matches expected
- All timelines have `elementCount > 0`
- `cssVars` contains expected `--st-*-progress` variables

### Step 2: Inspect Problem Timeline
```
mcp__chrome-devtools__evaluate_script({
  function: "(id) => window.Spacetime.inspectTimeline(id)",
  args: [{ uid: "timeline-id" }]
})
```

Look for:
- `state.started: false` - Driver never triggered
- `elements: {}` empty - Selectors didn't match DOM
- `state.destroyed: true` - Premature cleanup

### Step 3: Check Target Elements
```
mcp__chrome-devtools__evaluate_script({
  function: "(sel) => window.Spacetime.inspectElement(sel)",
  args: [{ uid: ".my-selector" }]
})
```

Verify:
- Elements exist (not empty array)
- `visible: true`, `inViewport: true`
- `computedStyles` shows expected initial values
- `inlineStyles` populated by runtime

### Step 4: Check Console for Errors
```
mcp__chrome-devtools__list_console_messages({ types: ["error", "warn"] })
```

### Step 5: Force Play (Test Interpolation)
```
mcp__chrome-devtools__evaluate_script({
  function: "(id) => window.Spacetime.forcePlay(id, 2000)",
  args: [{ uid: "timeline-id" }]
})
```

Then take screenshot to see if animation happened:
```
mcp__chrome-devtools__take_screenshot()
```

## Console Debug Logs

When `Spacetime.debug = true` (default in dev), the runtime logs to console:

```
[ST] timeline-id: initLoadDriver { durationMs, delayMs, threshold, once, immediate, scope }
[ST] timeline-id: immediate start (readyState=complete)
[ST] timeline-id: waiting for load event
[ST] timeline-id: setting up IntersectionObserver with threshold=0.1
[ST] timeline-id: intersection { isIntersecting: true, ratio: "0.500" }
[ST] timeline-id: skipped (already played, once=true)
[ST] timeline-id: applying delay 500ms
[ST] timeline-id: delay complete, starting animation
[ST] timeline-id: playAnimation startTime=12345, duration=1000
[ST] timeline-id: frame timestamp=..., elapsed=..., progress=...
[ST] timeline-id: animation complete
```

Use `mcp__chrome-devtools__list_console_messages({ types: ["log"] })` to see these.

---

## Common Issues

### `@load immediate: true` not playing
- Check `document.readyState` in diagnose output
- Race condition: custom elements may not be defined when Spacetime.init() runs
- Solution: Wait for custom element registration before init

### elementCount: 0
- Selector doesn't match any DOM elements
- Elements created dynamically after timeline init
- Solution: Ensure DOM elements exist before Spacetime.init()

### started: false, progress: 0
- Driver never triggered
- For `@load`: element not in viewport (if not immediate) or intersection threshold not met
- For `@scroll`: trigger element not found or out of scroll range

### destroyed: true
- Timeline was cleaned up (page navigation, destroyAll call)
- Check if components unmount and re-mount

### stroke animations (SVG lines)
- Check `stroke-dasharray` is set (required for draw effect)
- Verify `stroke-dashoffset` initial value matches dasharray
- Use `inspectElement` to see computed stroke values

## Files Reference

| File | Purpose |
|------|---------|
| `public/runtime/debug.js` | `ST.debug.*` diagnostics implementation (the historical `Spacetime.diagnose/forcePlay` notes refer to this file). |
| `stdlib/__dev__/primitives/dev-review.st` | `Spacetime.review()` — the 5-dim audit added by FEAT-015. |
| `src/server.rs::dev_runtime_js_handler` | Compiles + serves the dev runtime JS bundle. |
