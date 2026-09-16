/// Spacetime Tasks - Mobile Demo App
///
/// A task management app showcasing stdlib/mobile features.
/// This file defines the behaviors and states; visual CSS is in styles.css.
///
/// @example Build with:
/// spacetime build examples/mobile-tasks/app.st

@import "stdlib/mobile"


// ============================================================================
// Theme - Apply design tokens, typography, and color scheme
// ============================================================================

:root {
  @mobile-tokens
  @mobile-typography
  @mobile-theme
}


// ============================================================================
// App Shell - Safe area handling for notches and home indicators
// ============================================================================

.app {
  @safe-area
}


// ============================================================================
// Scrollable content with momentum and pull-to-refresh
// ============================================================================

.task-scroll {
  @scroll-view
  @pull-refresh
}


// ============================================================================
// Navigation - Tab bar with item states
// ============================================================================

.tab-item {
  @tab-item
}


// ============================================================================
// Form Input - Mobile-optimized keyboard handling
// ============================================================================

.task-input {
  @mobile-input
}


// ============================================================================
// Loading indicator
// ============================================================================

.loading-spinner {
  @loader
}
