/// Spacetime Mobile Primitives
///
/// Low-level primitives that wrap native mobile APIs.
/// These provide signals and events for mobile-specific functionality.

@import "./platform.st"      // Platform detection (iOS, Android, browser)
@import "./haptic.st"        // Haptic feedback
@import "./location.st"      // GPS/geolocation
@import "./notification.st"  // Push notifications
@import "./camera.st"        // Camera/barcode scanning
@import "./biometric.st"     // Biometric auth
@import "./share.st"         // Share sheet

// Future primitives (to be implemented in Phase 4):
// @import "./safe-area.st"     // Safe area insets
// @import "./keyboard.st"      // Keyboard events
// @import "./storage.st"       // Secure storage
