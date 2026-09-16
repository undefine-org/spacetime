/// Element Reference Caching Demo
///
/// Demonstrates the purity caching system with element references.
/// This file shows how element refs can be used with cached rect accessors.

// Element references scoped to body (accessible everywhere)
// When compiled, creates cached rect accessors via ST._purity.createCachedRect
body {
    &box .box;
}

// The container holds our animated box
.container {
  position: relative;
  width: 100%;
  height: 400px;
  background: #f0f0f0;
  overflow: hidden;
}

// The box that will be animated
.box {
  position: absolute;
  width: 100px;
  height: 100px;
  background: #3b82f6;
  border-radius: 8px;
  left: 50%;
  top: 50%;
  transform: translate(-50%, -50%);

  // Hover animation demonstrates purity tracking
  // During this animation, &box.rect will recalculate on each frame
  // After animation completes, &box.rect is cached until next resize/invalidation
  @on hover grow-box(300ms ease-out) {
    width: 100px -> 200px;
    height: 100px -> 150px;
  }
}

// Target element that animates TO the box's current size
// Uses element reference to read the box's rect
.target {
  position: absolute;
  bottom: 20px;
  left: 20px;
  width: 50px;
  height: 50px;
  background: #10b981;
  border-radius: 4px;

  // This animation references &box.rect.width
  // The purity system ensures we get:
  // - Cached value when .box is stable (not animating)
  // - Fresh value when .box is animating (width is changing)
  @on click match-box-width(400ms ease-in-out) {
    width: 50px -> &box.rect.width;
  }
}

// Info panel showing cache status
.info-panel {
  position: absolute;
  top: 10px;
  left: 10px;
  padding: 12px;
  background: rgba(0, 0, 0, 0.8);
  color: white;
  font-family: monospace;
  font-size: 12px;
  border-radius: 4px;

  // Content would be populated by JS to show:
  // - Current frame counter
  // - Number of animating selectors
  // - Whether .box is stable
}

// Demonstration of multiple refs with independent caching
// Element refs scoped to body for cross-scope access
body {
    &sidebar .sidebar;
    &header .header;
}

.sidebar {
  width: 250px;
  background: #1e293b;

  @on hover sidebar-expand(200ms) {
    width: 250px -> 300px;
  }
}

.header {
  height: 60px;
  background: #0f172a;

  // Reference sidebar width for responsive layout
  // Cached independently - only recalculates when sidebar animates
  padding-left: 20px;

  @on load header-init(0ms) {
    // Initial padding based on sidebar width
    padding-left: 20px -> calc(&sidebar.rect.width + 20px);
  }
}

// Main content adjusts based on both sidebar and header
.main-content {
  position: absolute;

  @on load content-layout(0ms) {
    // Uses cached rects - only recalcs when referenced elements animate
    top: 0 -> &header.rect.height;
    left: 0 -> &sidebar.rect.width;
  }
}
