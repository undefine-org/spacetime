# Spacetime Debug API

**A comprehensive browser debugging toolkit for Spacetime animations.**

Real-time inspection of timelines, drivers, and animation state with zero production overhead.

## Features

✅ **Timeline Inspection** - List and inspect all active animation timelines
✅ **Driver Monitoring** - Real-time logging of scroll, hover, time, and event drivers
✅ **Visual Debugging** - Overlay panels showing live progress and properties
✅ **State Snapshots** - Export complete animation state as JSON
✅ **Performance Tracking** - Monitor update frequency and driver activity
✅ **Zero Production Cost** - Completely removed in production builds

## Quick Start

### 1. Load Debug Scripts

```html
<script src="/__spacetime/runtime.js"></script>
<script src="/__spacetime/debug.js"></script>
<script src="/__spacetime/debug-overlay.js"></script>
```

### 2. Enable Debugging

```javascript
ST.debug.enable()
```

### 3. Explore

```javascript
// List all timelines
ST.debug.listTimelines()

// Show visual overlay
ST.debug.showOverlay('.hero')

// Monitor scroll animations
ST.debug.logDrivers((el, info) => info.type === 'scroll')
```

## Files Overview

### Core Implementation (23KB total)

| File | Size | Description |
|------|------|-------------|
| `debug.js` | 11KB | Main debug API, timeline/driver tracking |
| `debug-overlay.js` | 9KB | Visual debugging overlay with live updates |
| `debug-test.js` | 5KB | Automated test suite (20+ tests) |

### Documentation (38KB total)

| File | Size | Description |
|------|------|-------------|
| `DEBUG_API.md` | 10KB | Complete API reference with examples |
| `DEBUG_INTEGRATION.md` | 10KB | Integration guide for projects |
| `DEBUG_SUMMARY.md` | 11KB | Implementation overview and architecture |
| `DEBUG_QUICK_REFERENCE.md` | 7KB | One-page cheat sheet |

### Examples (27KB total)

| File | Size | Description |
|------|------|-------------|
| `debug-demo.html` | 15KB | Interactive demo with animations |
| `ikarchitecte-debug-example.html` | 12KB | Real-world integration example |

**Total: ~88KB (3,472 lines)**

## API Overview

### Core Methods

```javascript
ST.debug.enable()                        // Enable debugging
ST.debug.disable()                       // Disable debugging
ST.debug.isEnabled()                     // Check if enabled
```

### Timeline Inspection

```javascript
ST.debug.listTimelines()                 // List all timelines
ST.debug.inspectTimeline('hero-reveal')  // Inspect specific timeline
ST.debug.getTimelines()                  // Get timeline Map
```

### Driver Monitoring

```javascript
ST.debug.logDrivers()                    // Log all driver changes
ST.debug.logDrivers('.hero')             // Log specific selector
ST.debug.logDrivers((el, info) => ...)   // Custom filter
ST.debug.stopLogging()                   // Stop logging
```

### State Inspection

```javascript
ST.debug.snapshot()                      // Capture current state
ST.debug.getDriverInfo(element)          // Get element's driver
ST.debug.getAnimationState(element)      // Get animation state
```

### Visual Overlay

```javascript
ST.debug.showOverlay('.hero')            // Show overlay
ST.debug.hideOverlay()                   // Hide all overlays
```

## Documentation

### Getting Started
- **[Quick Reference](./DEBUG_QUICK_REFERENCE.md)** - One-page cheat sheet
- **[Demo Page](./debug-demo.html)** - Interactive examples
- **[IK Architecte Example](./ikarchitecte-debug-example.html)** - Real-world usage

### Reference
- **[API Documentation](./DEBUG_API.md)** - Complete method reference
- **[Integration Guide](./DEBUG_INTEGRATION.md)** - How to add to your project
- **[Implementation Summary](./DEBUG_SUMMARY.md)** - Architecture details

### Testing
- **[Test Suite](./debug-test.js)** - Automated tests

## Usage Examples

### Debug Scroll Animation

```javascript
ST.debug.enable()
ST.debug.logDrivers((el, info) => info.type === 'scroll')
ST.debug.showOverlay('.parallax')

// Scroll the page, watch console:
// [ST:scroll-driver] .parallax $progress = 0.234 (Δ 0.012)
// [ST:scroll-driver] .parallax $progress = 0.567 (Δ 0.333)
```

### Inspect Timeline

```javascript
const timeline = ST.debug.inspectTimeline('nav-transition')
console.log(timeline)
// {
//   name: 'nav-transition',
//   driver: 'scroll',
//   selector: '.hero',
//   elements: [...],
//   config: { start: 0, end: 1, scrub: true }
// }
```

### Monitor Hover Effects

```javascript
ST.debug.logDrivers((el, info) => {
  return info.type === 'hover' && el.matches('.card')
})

// Hover over cards:
// [ST:hover-driver] .card-1 $progress = 0.123 (Δ 0.123)
// [ST:hover-driver] .card-1 $progress = 1.000 (Δ 0.877)
```

### Export State

```javascript
const state = ST.debug.snapshot()
localStorage.setItem('debug', JSON.stringify(state))
// Later:
const restored = JSON.parse(localStorage.getItem('debug'))
console.log('Captured', restored.timelineCount, 'timelines')
```

## Integration

### Standard Setup

```html
<script src="/__spacetime/runtime.js"></script>
<script src="/__spacetime/debug.js"></script>
<script src="/__spacetime/debug-overlay.js"></script>
```

### Conditional Loading (Recommended)

Only load debug in development:

```html
<script src="/__spacetime/runtime.js"></script>
<script>
  if (location.hostname === 'localhost' || location.search.includes('debug')) {
    const loadScript = (src) => {
      const script = document.createElement('script')
      script.src = src
      document.head.appendChild(script)
    }
    loadScript('/__spacetime/debug.js')
    loadScript('/__spacetime/debug-overlay.js')
  }
</script>
```

### URL Parameter

Enable via URL:
```
http://localhost:8080/?debug
```

The debug API auto-enables when `?debug` is present.

## Architecture

### Data Storage

```javascript
// Timeline registry
timelines: Map<string, TimelineInfo>

// Driver tracking (auto-cleanup via WeakMap)
driverInstances: WeakMap<Element, DriverInfo>

// Animation state (auto-cleanup via WeakMap)
animationStates: WeakMap<Element, AnimationState>

// Active overlays
activeOverlays: Map<Element, OverlayData>
```

### Compiler Integration

Drivers should emit debug hooks:

```javascript
// In driver primitive
if (ST._debugInternal) {
  ST._debugInternal.registerDriver(element, 'scroll', config)
  ST._debugInternal.updateDriverProgress(element, progress)
}
```

### Production Build

Strip debug code for zero overhead:

```javascript
// webpack.config.js
new webpack.DefinePlugin({
  'ST._debugInternal': 'undefined'
})

// Tree-shaking removes all debug checks
// Result: 0KB, 0ms overhead
```

## Performance

**Development:**
- ~23KB JavaScript (debug.js + debug-overlay.js)
- ~0.1ms per driver update
- Overlays update via rAF (60fps)

**Production:**
- 0KB overhead (not loaded)
- 0ms overhead (hooks removed)

## Browser Support

✅ Chrome 90+
✅ Firefox 88+
✅ Safari 14+
✅ Edge 90+

Requires: ES6+, WeakMap, requestAnimationFrame

## Testing

### Run Demo

```bash
cd public/runtime
python3 -m http.server 8888
open http://localhost:8888/debug-demo.html
```

### Run Tests

Load in browser:
```html
<script src="st.js"></script>
<script src="debug.js"></script>
<script src="debug-overlay.js"></script>
<script src="debug-test.js"></script>
```

Or in console:
```javascript
runDebugTests()
// => Tests: 20, Passed: 20, Failed: 0
```

## Troubleshooting

### Debug API not found

```javascript
// Problem: ST.debug is undefined
// Solution: Load debug.js after st.js
```

### No timelines showing

```javascript
// Problem: ST.debug.listTimelines() returns []
// Solution: Make sure animations are running and compiler emits debug hooks
```

### Overlay not showing

```javascript
// Problem: ST.debug.showOverlay() returns error
// Solution: Load debug-overlay.js
```

## Advanced Usage

### Custom Filter

```javascript
// Only log drivers with high progress
ST.debug.logDrivers((el, info) => info.progress > 0.8)

// Only log specific element IDs
ST.debug.logDrivers((el) => el.id.startsWith('hero-'))

// Combine conditions
ST.debug.logDrivers((el, info) => {
  return info.type === 'scroll' &&
         el.matches('.parallax') &&
         info.progress > 0.5
})
```

### Performance Monitor

```javascript
let updates = 0
const start = Date.now()

const original = ST._debugInternal.updateDriverProgress
ST._debugInternal.updateDriverProgress = function(...args) {
  updates++
  return original.apply(this, args)
}

// After some time:
const fps = updates / ((Date.now() - start) / 1000)
console.log(`Average FPS: ${fps.toFixed(2)}`)
```

### Custom Debug UI

```javascript
const panel = document.createElement('div')
panel.style.cssText = 'position: fixed; top: 10px; right: 10px; ...'

function update() {
  const timelines = ST.debug.listTimelines()
  panel.innerHTML = timelines.map(t =>
    `<div>${t.name}: ${(t.progress * 100).toFixed(1)}%</div>`
  ).join('')
  requestAnimationFrame(update)
}

document.body.appendChild(panel)
update()
```

## Bookmarklets

Quick access from browser bookmarks:

**Enable Debug:**
```javascript
javascript:(function(){ST.debug.enable()})();
```

**Show Overlays:**
```javascript
javascript:(function(){ST.debug.enable();ST.debug.showOverlay('*')})();
```

**Snapshot:**
```javascript
javascript:(function(){console.log(ST.debug.snapshot())})();
```

## Contributing

The debug API is designed to be:
- **Non-invasive** - Guards all hooks with `if (ST._debugInternal)`
- **Performant** - Uses WeakMaps for auto-cleanup
- **Tree-shakeable** - Completely removed in production
- **Extensible** - Easy to add new tracking features

When adding new drivers or primitives:
1. Add registration call: `ST._debugInternal.registerDriver(...)`
2. Add progress updates: `ST._debugInternal.updateDriverProgress(...)`
3. Document in DEBUG_API.md

## License

Part of the Spacetime project.

## Links

- [API Reference](./DEBUG_API.md)
- [Quick Reference](./DEBUG_QUICK_REFERENCE.md)
- [Integration Guide](./DEBUG_INTEGRATION.md)
- [Implementation Summary](./DEBUG_SUMMARY.md)
- [Demo Page](./debug-demo.html)
- [IK Architecte Example](./ikarchitecte-debug-example.html)

---

**Questions?** Check the [API docs](./DEBUG_API.md) or [integration guide](./DEBUG_INTEGRATION.md).

**Ready to debug?** Load the [demo page](./debug-demo.html) and try it out!
