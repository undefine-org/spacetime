# Spacetime Debug API - Quick Reference

One-page cheat sheet for the Spacetime Debug API.

## Setup

```html
<!-- After loading st.js -->
<script src="/__spacetime/debug.js"></script>
<script src="/__spacetime/debug-overlay.js"></script>
```

## Quick Start

```javascript
ST.debug.enable()                  // Enable debugging
ST.debug.listTimelines()           // Show all timelines
ST.debug.showOverlay('.hero')      // Show visual overlay
```

## Core Commands

| Command | Description | Returns |
|---------|-------------|---------|
| `ST.debug.enable()` | Enable all debugging | `"Debugging enabled"` |
| `ST.debug.disable()` | Disable debugging | `"Debugging disabled"` |
| `ST.debug.isEnabled()` | Check if enabled | `true/false` |

## Timeline Inspection

| Command | Description | Returns |
|---------|-------------|---------|
| `ST.debug.listTimelines()` | List all timelines | `Array<Timeline>` |
| `ST.debug.inspectTimeline(name)` | Inspect timeline | `Timeline` or `null` |
| `ST.debug.getTimelines()` | Get timeline Map | `Map<string, Timeline>` |

## Driver Monitoring

| Command | Description |
|---------|-------------|
| `ST.debug.logDrivers()` | Log all driver changes |
| `ST.debug.logDrivers('.hero')` | Log drivers matching selector |
| `ST.debug.logDrivers((el, info) => ...)` | Log drivers with custom filter |
| `ST.debug.stopLogging()` | Stop driver logging |

## State Inspection

| Command | Description | Returns |
|---------|-------------|---------|
| `ST.debug.snapshot()` | Capture current state | `Snapshot` |
| `ST.debug.getDriverInfo(el)` | Get element driver | `DriverInfo` or `null` |
| `ST.debug.getAnimationState(el)` | Get element animation | `AnimState` or `null` |

## Visual Overlay

| Command | Description | Returns |
|---------|-------------|---------|
| `ST.debug.showOverlay(selector)` | Show overlay(s) | `number` (count shown) |
| `ST.debug.hideOverlay()` | Hide all overlays | `void` |

## Common Workflows

### Debug Scroll Animation

```javascript
ST.debug.enable()
ST.debug.logDrivers((el, info) => info.type === 'scroll')
ST.debug.showOverlay('.parallax')
```

### Find Slow Animations

```javascript
ST.debug.enable()
const before = ST.debug.snapshot()
// ... interact with page ...
const after = ST.debug.snapshot()
console.log('Elements:', before.activeElements, '->', after.activeElements)
```

### Monitor Specific Timeline

```javascript
ST.debug.inspectTimeline('hero-reveal')
ST.debug.showOverlay('.hero')
```

### Export State

```javascript
const state = ST.debug.snapshot()
console.log(JSON.stringify(state, null, 2))
// or
localStorage.setItem('debug', JSON.stringify(state))
```

## Auto-Enable Debug

```javascript
// Via URL parameter
// http://localhost/?debug

// Via code
if (location.hostname === 'localhost') {
  ST.debug.enable()
}
```

## Filter Examples

### By Driver Type

```javascript
ST.debug.logDrivers((el, info) => info.type === 'scroll')
```

### By CSS Selector

```javascript
ST.debug.logDrivers('.hero, .card')
```

### By Progress

```javascript
ST.debug.logDrivers((el, info) => info.progress > 0.5)
```

### By Element ID

```javascript
ST.debug.logDrivers((el) => el.id === 'hero')
```

## Data Structures

### Timeline

```typescript
{
  name: string
  driver: 'scroll' | 'time' | 'hover' | 'click' | 'load'
  selector: string
  elementCount: number
  progress: number
  config: object
}
```

### DriverInfo

```typescript
{
  type: string
  config: object
  progress: number
  lastUpdate: number
}
```

### Snapshot

```typescript
{
  timestamp: string
  timelineCount: number
  activeElements: number
  timelines: Timeline[]
  elements: ElementInfo[]
}
```

## Console Output Examples

### listTimelines()

```
┌─────────┬──────────────────┬──────────┬──────────┬──────────┬──────────┐
│ (index) │ name             │ driver   │ selector │ elements │ progress │
├─────────┼──────────────────┼──────────┼──────────┼──────────┼──────────┤
│ 0       │ 'nav-transition' │ 'scroll' │ '.hero'  │ 1        │ 0.523    │
│ 1       │ 'card-hover'     │ 'hover'  │ '.card'  │ 6        │ 0.000    │
└─────────┴──────────────────┴──────────┴──────────┴──────────┴──────────┘
```

### logDrivers()

```
[ST:scroll-driver] .hero $progress = 0.234 (Δ 0.012)
[ST:scroll-driver] .hero $progress = 0.456 (Δ 0.222)
[ST:hover-driver] .card-1 $progress = 0.123 (Δ 0.123)
[ST:hover-driver] .card-1 $progress = 1.000 (Δ 0.877)
```

### inspectTimeline()

```javascript
{
  name: 'nav-transition',
  driver: 'scroll',
  selector: '.hero',
  config: { start: 0, end: 1, scrub: true },
  registeredAt: '2025-12-23T10:30:00.000Z',
  elements: [
    {
      element: <div class="hero">,
      selector: '.hero',
      driver: { type: 'scroll', progress: 0.523, ... },
      animation: { ... }
    }
  ]
}
```

## Keyboard Shortcuts

Create bookmarklets for quick access:

### Enable Debug

```javascript
javascript:(function(){ST.debug.enable()})();
```

### Show All Overlays

```javascript
javascript:(function(){ST.debug.enable();ST.debug.showOverlay('*')})();
```

### Snapshot

```javascript
javascript:(function(){console.log(ST.debug.snapshot())})();
```

## Performance Tips

- Disable debug in production (0KB overhead)
- Use filters to reduce console noise
- Hide overlays when not needed
- Limit overlay count on dense UIs
- Snapshots can be large - serialize carefully

## Troubleshooting

| Problem | Solution |
|---------|----------|
| `ST.debug undefined` | Load `debug.js` after `st.js` |
| `No timelines found` | Make sure animations are running |
| `Overlay not showing` | Load `debug-overlay.js` |
| `No logs` | Call `ST.debug.enable()` first |

## Links

- [Full API Docs](./DEBUG_API.md)
- [Integration Guide](./DEBUG_INTEGRATION.md)
- [Demo Page](./debug-demo.html)
- [Test Suite](./debug-test.js)

---

**Print this page for quick reference while debugging!**
