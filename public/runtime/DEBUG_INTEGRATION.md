# Integrating Spacetime Debug API

Guide for integrating the debug API into your Spacetime projects.

## Standard Integration

Add the debug scripts after loading the Spacetime runtime in your HTML:

```html
<!-- Load Spacetime Runtime -->
<script src="/__spacetime/runtime.js"></script>

<!-- Optional: Debug API (development only) -->
<script src="/__spacetime/debug.js"></script>
<script src="/__spacetime/debug-overlay.js"></script>
```

## Conditional Loading (Recommended)

Only load debug scripts in development or when a debug flag is present:

### Option 1: URL Parameter

```html
<script src="/__spacetime/runtime.js"></script>

<script>
  // Auto-load debug scripts if ?debug in URL
  if (window.location.search.includes('debug')) {
    const debugScript = document.createElement('script');
    debugScript.src = '/__spacetime/debug.js';
    document.head.appendChild(debugScript);

    debugScript.onload = () => {
      const overlayScript = document.createElement('script');
      overlayScript.src = '/__spacetime/debug-overlay.js';
      document.head.appendChild(overlayScript);
    };
  }
</script>
```

### Option 2: Environment Variable

```html
<script src="/__spacetime/runtime.js"></script>

<!-- Only in development builds -->
<% if (process.env.NODE_ENV === 'development') { %>
  <script src="/__spacetime/debug.js"></script>
  <script src="/__spacetime/debug-overlay.js"></script>
<% } %>
```

### Option 3: localhost Detection

```html
<script src="/__spacetime/runtime.js"></script>

<script>
  // Auto-load debug on localhost
  if (window.location.hostname === 'localhost' ||
      window.location.hostname === '127.0.0.1') {
    const debugScript = document.createElement('script');
    debugScript.src = '/__spacetime/debug.js';
    document.head.appendChild(debugScript);

    debugScript.onload = () => {
      const overlayScript = document.createElement('script');
      overlayScript.src = '/__spacetime/debug-overlay.js';
      document.head.appendChild(overlayScript);

      // Auto-enable debugging
      if (ST.debug) {
        ST.debug.enable();
        console.log('Spacetime debugging enabled (localhost detected)');
      }
    };
  }
</script>
```

## Compiler Integration

The Spacetime compiler should emit debug hooks when compiling primitives. Update your driver primitives to call debug tracking:

### In `stdlib/primitives/animation/drivers.st`

Add debug tracking calls to each driver primitive:

```javascript
%primitive scroll-driver(&el, start: number, end: number, scrub: bool) {
  %emit js {
    // ... existing setup code ...

    // Register driver with debug API (if available)
    if (typeof ST !== 'undefined' && ST._debugInternal) {
      ST._debugInternal.registerDriver(%&el, 'scroll', {
        start: %start,
        end: %end,
        scrub: %scrub
      });
    }

    const update = () => {
      // ... calculate progress ...

      // Update debug tracker
      if (typeof ST !== 'undefined' && ST._debugInternal) {
        ST._debugInternal.updateDriverProgress(%&el, progress);
      }

      %yield progress -> $progress;
    };

    // ... rest of driver code ...
  }
}
```

### In Timeline Macros

Register timelines when they're created:

```javascript
%macro timeline {
  // ... macro setup ...

  %binds {
    // Register timeline with debug API
    %if debug {
      js {
        if (typeof ST !== 'undefined' && ST._debugInternal) {
          const elements = document.querySelectorAll(%selector);
          ST._debugInternal.registerTimeline(
            %name,
            %driver,
            %selector,
            %config,
            Array.from(elements)
          );
        }
      }
    }

    // ... rest of bindings ...
  }
}
```

## Example: IK Architecte Integration

For the IK Architecte site, update `ikarchitecte/index.html`:

```html
<!-- Near the end of the file, after runtime.js -->
<script src="/__spacetime/runtime.js"></script>

<!-- Debug API (localhost only) -->
<script>
  if (window.location.hostname === 'localhost' ||
      window.location.search.includes('debug')) {
    const script1 = document.createElement('script');
    script1.src = '/__spacetime/debug.js';
    document.head.appendChild(script1);

    script1.onload = () => {
      const script2 = document.createElement('script');
      script2.src = '/__spacetime/debug-overlay.js';
      document.head.appendChild(script2);

      script2.onload = () => {
        console.log('Spacetime Debug API loaded');
        console.log('Use ST.debug.enable() to start debugging');
      };
    };
  }
</script>
```

Then you can debug animations:

```javascript
// Enable debugging
ST.debug.enable();

// Show overlay on hero letters
ST.debug.showOverlay('.ik-hero__letter');

// Monitor scroll animations
ST.debug.logDrivers((el, info) => info.type === 'scroll');

// List all timelines
ST.debug.listTimelines();
```

## Build-Time Stripping

For production builds, strip debug code entirely:

### Webpack

```javascript
// webpack.config.js
module.exports = {
  plugins: [
    new webpack.DefinePlugin({
      'ST._debugInternal': 'undefined',
      'process.env.DEBUG': JSON.stringify(process.env.NODE_ENV === 'development')
    })
  ]
};
```

### Rollup

```javascript
// rollup.config.js
import replace from '@rollup/plugin-replace';

export default {
  plugins: [
    replace({
      'ST._debugInternal': 'undefined',
      preventAssignment: true
    })
  ]
};
```

### esbuild

```javascript
// esbuild.config.js
require('esbuild').build({
  define: {
    'ST._debugInternal': 'undefined'
  }
});
```

## Development Workflow

### 1. Enable Debug Mode

Visit your site with `?debug` parameter:
```
http://localhost:8080/ikarchitecte/?debug
```

### 2. Open DevTools

Press `F12` or right-click > Inspect

### 3. Use Debug Commands

```javascript
// See all available commands
ST.debug

// Enable debugging
ST.debug.enable()

// View timelines
ST.debug.listTimelines()

// Show visual overlay
ST.debug.showOverlay('.ik-hero__letter')
```

### 4. Monitor Animations

```javascript
// Log all scroll animations
ST.debug.logDrivers((el, info) => info.type === 'scroll')

// Scroll the page and watch console for updates:
// [ST:scroll-driver] .hero $progress = 0.234 (Δ 0.012)
// [ST:scroll-driver] .hero $progress = 0.456 (Δ 0.222)
```

### 5. Inspect Specific Elements

```javascript
// Get driver info for an element
const hero = document.querySelector('.ik-hero');
ST.debug.getDriverInfo(hero);
// => { type: 'load', progress: 1.0, config: {...}, lastUpdate: ... }
```

## Performance Impact

The debug API is designed to have **zero performance impact** in production:

- All debug hooks are guarded by `if (ST._debugInternal)` checks
- These checks are removed by tree-shaking in production builds
- No debug code is loaded unless explicitly included
- WeakMaps ensure automatic garbage collection

**Development (with debug):**
- ~10KB additional JavaScript (debug.js + debug-overlay.js)
- Minimal runtime overhead (~0.1ms per driver update)
- Visual overlays use `requestAnimationFrame` for smooth updates

**Production (without debug):**
- 0KB overhead (scripts not loaded)
- 0ms overhead (debug hooks compiled out)

## Troubleshooting

### Debug API Not Available

**Problem:** `ST.debug is undefined`

**Solution:** Make sure `debug.js` is loaded after `st.js`:
```html
<script src="/__spacetime/runtime.js"></script>
<script src="/__spacetime/debug.js"></script> <!-- Load after runtime -->
```

### No Timelines Showing

**Problem:** `ST.debug.listTimelines()` returns empty array

**Solutions:**
1. Make sure your Spacetime animations are compiled and running
2. Check that elements exist in the DOM
3. Verify the compiler is emitting debug hooks

### Overlay Not Showing

**Problem:** `ST.debug.showOverlay()` returns error

**Solution:** Load `debug-overlay.js`:
```html
<script src="/__spacetime/debug-overlay.js"></script>
```

### Driver Updates Not Logging

**Problem:** No console logs when animations run

**Solution:** Enable driver logging:
```javascript
ST.debug.enable();         // Enable debugging
ST.debug.logDrivers();     // Enable driver logging
```

## Advanced Usage

### Custom Debug Panel

Create your own debug UI using the debug API:

```javascript
// Custom debug panel
const panel = document.createElement('div');
panel.style.cssText = 'position: fixed; top: 10px; right: 10px; background: white; padding: 20px;';

function updatePanel() {
  const timelines = ST.debug.listTimelines();
  panel.innerHTML = `
    <h3>Spacetime Debug</h3>
    <div>Active Timelines: ${timelines.length}</div>
    ${timelines.map(t => `
      <div>${t.name}: ${(t.progress * 100).toFixed(1)}%</div>
    `).join('')}
  `;
  requestAnimationFrame(updatePanel);
}

document.body.appendChild(panel);
updatePanel();
```

### Performance Monitoring

Track animation performance:

```javascript
const performanceMonitor = {
  updates: 0,
  startTime: Date.now(),

  start() {
    const originalUpdate = ST._debugInternal.updateDriverProgress;
    ST._debugInternal.updateDriverProgress = function(...args) {
      performanceMonitor.updates++;
      return originalUpdate.apply(this, args);
    };
  },

  getStats() {
    const elapsed = (Date.now() - this.startTime) / 1000;
    const fps = this.updates / elapsed;
    return {
      totalUpdates: this.updates,
      duration: elapsed.toFixed(2) + 's',
      averageFPS: fps.toFixed(2)
    };
  }
};

performanceMonitor.start();
// ... interact with page ...
console.log(performanceMonitor.getStats());
// => { totalUpdates: 1234, duration: '10.5s', averageFPS: '117.52' }
```

## See Also

- [Debug API Reference](./DEBUG_API.md)
- [Demo Page](./debug-demo.html)
- [Runtime Documentation](./README.md)
