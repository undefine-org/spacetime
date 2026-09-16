# Spacetime Debug API

Browser debugging tools for inspecting Spacetime animations, timelines, and driver signals in real-time.

## Quick Start

```javascript
// Enable all debugging
ST.debug.enable();

// List active timelines
ST.debug.listTimelines();

// Show visual overlay
ST.debug.showOverlay('.hero');
```

## Installation

Include the debug scripts after loading the Spacetime runtime:

```html
<script src="runtime/st.js"></script>
<script src="runtime/debug.js"></script>
<script src="runtime/debug-overlay.js"></script>
```

Or enable debug mode via URL query parameter:

```
https://yoursite.com/page.html?debug
```

## API Reference

### Core Debugging

#### `ST.debug.enable()`

Enables all debugging features including console logging and driver tracking.

```javascript
ST.debug.enable();
// => "Debugging enabled"
```

#### `ST.debug.disable()`

Disables all debugging features.

```javascript
ST.debug.disable();
// => "Debugging disabled"
```

#### `ST.debug.isEnabled()`

Check if debugging is currently enabled.

```javascript
const isDebug = ST.debug.isEnabled();
// => true or false
```

---

### Timeline Inspection

#### `ST.debug.listTimelines()`

Lists all registered timelines with their current state. Outputs both a formatted console table and returns the data as an array.

```javascript
const timelines = ST.debug.listTimelines();
// Console output:
// ┌─────────┬──────────────────┬──────────┬──────────┬──────────┬──────────┐
// │ (index) │ name             │ driver   │ selector │ elements │ progress │
// ├─────────┼──────────────────┼──────────┼──────────┼──────────┼──────────┤
// │ 0       │ 'nav-transition' │ 'scroll' │ '.hero'  │ 1        │ 0.523    │
// └─────────┴──────────────────┴──────────┴──────────┴──────────┴──────────┘

// Returns:
// [
//   {
//     name: 'nav-transition',
//     driver: 'scroll',
//     selector: '.hero',
//     elementCount: 1,
//     progress: 0.523,
//     config: { start: 0, end: 1, scrub: true }
//   }
// ]
```

#### `ST.debug.inspectTimeline(name)`

Get detailed information about a specific timeline.

```javascript
const timeline = ST.debug.inspectTimeline('nav-transition');
// Returns:
// {
//   name: 'nav-transition',
//   driver: 'scroll',
//   selector: '.hero',
//   config: { start: 0, end: 1, scrub: true },
//   registeredAt: '2025-12-23T10:30:00.000Z',
//   elements: [
//     {
//       element: <div class="hero">,
//       selector: '.hero',
//       driver: {
//         type: 'scroll',
//         progress: 0.523,
//         lastUpdate: '2025-12-23T10:35:00.000Z'
//       },
//       animation: { /* animation state */ }
//     }
//   ]
// }
```

#### `ST.debug.getTimelines()`

Get the full Map of all registered timelines.

```javascript
const allTimelines = ST.debug.getTimelines();
// => Map { 'nav-transition' => {...}, 'hero-reveal' => {...} }
```

---

### Driver Monitoring

#### `ST.debug.logDrivers([filter])`

Enable real-time logging of driver signal changes. Optionally filter by CSS selector or custom function.

**No filter (log all drivers):**
```javascript
ST.debug.logDrivers();
// Console output:
// [ST:scroll-driver] .hero $progress = 0.523 (Δ 0.003)
// [ST:hover-driver] .card $progress = 0.847 (Δ 0.052)
// [ST:time-driver] .modal $progress = 1.000 (Δ 0.154)
```

**Filter by CSS selector:**
```javascript
ST.debug.logDrivers('.hero');
// Only logs drivers for elements matching .hero
```

**Filter by custom function:**
```javascript
ST.debug.logDrivers((element, driverInfo) => {
  return driverInfo.type === 'scroll' && driverInfo.progress > 0.5;
});
// Only logs scroll drivers with progress > 50%
```

#### `ST.debug.stopLogging()`

Stop driver change logging.

```javascript
ST.debug.stopLogging();
// => "Driver logging disabled"
```

---

### State Snapshots

#### `ST.debug.snapshot()`

Capture the current state of all animations, drivers, and signals. Useful for debugging or exporting state.

```javascript
const state = ST.debug.snapshot();
// Returns:
// {
//   timestamp: '2025-12-23T10:35:00.000Z',
//   timelineCount: 5,
//   activeElements: 12,
//   timelines: [
//     { name: 'nav-transition', driver: 'scroll', selector: '.hero', elementCount: 1 },
//     { name: 'card-hover', driver: 'hover', selector: '.card', elementCount: 6 }
//   ],
//   elements: [
//     {
//       selector: '.hero',
//       driver: { type: 'scroll', progress: 0.523, config: {...} },
//       animation: { ... },
//       signals: { progress: { v: 0.523, d: Set(...) } }
//     }
//   ]
// }
```

---

### Visual Debug Overlay

#### `ST.debug.showOverlay(selector)`

Display a visual debugging overlay on elements matching the selector. Shows real-time progress, driver info, and animated properties.

```javascript
ST.debug.showOverlay('.hero');
// Shows overlay panel(s) near matching element(s)
// => 1 (number of overlays shown)
```

**Multiple elements:**
```javascript
ST.debug.showOverlay('.card');
// Shows overlays for all .card elements
// => 6
```

The overlay displays:
- Element selector
- Current animation progress (0-100%)
- Driver type and last update time
- Current CSS property values
- Active signals

#### `ST.debug.hideOverlay()`

Hide all debug overlays.

```javascript
ST.debug.hideOverlay();
```

---

### Element-Level Inspection

#### `ST.debug.getDriverInfo(element)`

Get driver information for a specific element.

```javascript
const hero = document.querySelector('.hero');
const driverInfo = ST.debug.getDriverInfo(hero);
// Returns:
// {
//   type: 'scroll',
//   config: { start: 0, end: 1, scrub: true },
//   progress: 0.523,
//   lastUpdate: 1735029300000
// }
```

#### `ST.debug.getAnimationState(element)`

Get animation state for a specific element.

```javascript
const hero = document.querySelector('.hero');
const animState = ST.debug.getAnimationState(hero);
// Returns animation state if tracked
```

---

## Usage Examples

### Debugging a Scroll Animation

```javascript
// Enable debugging
ST.debug.enable();

// Log scroll drivers only
ST.debug.logDrivers((el, info) => info.type === 'scroll');

// Show overlay on hero section
ST.debug.showOverlay('.hero');

// Inspect the timeline
ST.debug.inspectTimeline('nav-transition');
```

### Finding Performance Issues

```javascript
// Take initial snapshot
const before = ST.debug.snapshot();

// ... perform some user interaction ...

// Take second snapshot
const after = ST.debug.snapshot();

// Compare active elements
console.log('Active elements before:', before.activeElements);
console.log('Active elements after:', after.activeElements);
```

### Monitoring Hover Animations

```javascript
// Enable debugging
ST.debug.enable();

// Filter logs to only hover drivers
ST.debug.logDrivers((el, info) => info.type === 'hover');

// Console output when hovering:
// [ST:hover-driver] .card-1 $progress = 0.123 (Δ 0.123)
// [ST:hover-driver] .card-1 $progress = 0.456 (Δ 0.333)
// [ST:hover-driver] .card-1 $progress = 0.789 (Δ 0.333)
// [ST:hover-driver] .card-1 $progress = 1.000 (Δ 0.211)
```

### Auto-Enable on Specific Pages

```javascript
// In your application initialization
if (window.location.pathname.includes('/debug') ||
    window.location.search.includes('debug')) {
  ST.debug.enable();
  console.log('Debug mode active');
}
```

---

## Integration with Compiled Spacetime

The debug API automatically hooks into the Spacetime runtime when primitives are compiled. The compiler should emit calls to register timelines and drivers:

```javascript
// Generated by Spacetime compiler
if (ST._debugInternal) {
  // Register timeline
  ST._debugInternal.registerTimeline(
    'nav-transition',
    'scroll',
    '.hero',
    { start: 0, end: 1, scrub: true },
    [element]
  );

  // Register driver instance
  ST._debugInternal.registerDriver(
    element,
    'scroll',
    { start: 0, end: 1, scrub: true }
  );
}

// In animation loop
if (ST._debugInternal) {
  ST._debugInternal.updateDriverProgress(element, progress);
}
```

---

## Browser Console Tips

### Quick Access

Typing `ST.debug` in the console shows all available methods:

```javascript
ST.debug
// => {
//   enable: ƒ,
//   disable: ƒ,
//   listTimelines: ƒ,
//   inspectTimeline: ƒ,
//   logDrivers: ƒ,
//   stopLogging: ƒ,
//   snapshot: ƒ,
//   showOverlay: ƒ,
//   hideOverlay: ƒ,
//   ...
// }
```

### Bookmarklet

Create a bookmarklet for quick debugging:

```javascript
javascript:(function(){ST.debug.enable();ST.debug.showOverlay('*');})();
```

---

## Performance Considerations

- The debug API has minimal performance impact when disabled
- Driver progress logging uses passive event listeners
- Overlays update via `requestAnimationFrame` for smooth rendering
- WeakMaps ensure automatic cleanup when elements are removed
- All debug tracking is removed in production builds when `ST._debugInternal` is stripped

---

## Troubleshooting

### "Debug API not found"

Make sure `debug.js` is loaded after `st.js`:

```html
<script src="runtime/st.js"></script>
<script src="runtime/debug.js"></script>
```

### "No timelines found"

Timelines are only registered when primitives are initialized. Make sure:
1. Your Spacetime code has been compiled
2. The compiled JavaScript is executing
3. Elements matching your selectors exist in the DOM

### "Debug overlay not loaded"

Include `debug-overlay.js` to use visual overlays:

```html
<script src="runtime/debug-overlay.js"></script>
```

---

## See Also

- [Spacetime Runtime Documentation](./README.md)
- [Animation Primitives](./QUICK_REFERENCE.md)
- [Demo Page](./debug-demo.html)
