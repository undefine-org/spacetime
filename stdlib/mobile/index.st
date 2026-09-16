/// Spacetime Mobile Standard Library
///
/// A comprehensive module set for building beautiful mobile apps
/// with Spacetime. Import this to get access to all mobile primitives,
/// gestures, navigation, components, and theming.
///
/// @example
/// @import "stdlib/mobile"
///
/// .app {
///   @safe-area {
///     @stack main {
///       @route "/" { ... }
///     }
///   }
/// }
///
/// .item {
///   @swipe left delete(300ms) {
///     translate-x: 0 -> -100%;
///     @haptic "medium";
///   }
/// }

// =============================================================================
// PRIMITIVES
// =============================================================================

@import "./primitives/index.st"

// =============================================================================
// MACROS
// =============================================================================

@import "./macros/index.st"

// =============================================================================
// THEME
// =============================================================================

@import "./theme/index.st"

// =============================================================================
// ADAPTER INITIALIZATION
// =============================================================================

// The JavaScript adapters are loaded automatically at runtime.
// In Tauri apps: adapters/tauri.js is loaded
// In browsers: adapters/browser.js is loaded as fallback
//
// To manually configure:
//   import 'stdlib/mobile/adapters/tauri.js';   // For Tauri
//   import 'stdlib/mobile/adapters/browser.js'; // For browser preview
