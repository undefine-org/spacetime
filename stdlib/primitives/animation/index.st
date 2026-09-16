/// Animation Primitives Index
///
/// This module provides composable primitives for declarative animations.
///
/// ## Core Primitives
///
/// ### Easing (easing.st)
/// - `easing()` - Library of standard easing functions
///
/// ### Color Interpolation (color.st)
/// - `color-interpolate()` - RGB/HSL/OKLCH color interpolation
///
/// ### Value Interpolation (interpolate.st)
/// - `animate-property(&el, keyframes, property)` - Keyframe-based property animation
/// - `stagger(index, total, delay, from)` - Calculate stagger delays
///
/// ### Drivers (drivers.st)
/// - `scroll-driver(&el, start, end, scrub)` - Scroll-driven progress
/// - `time-driver(duration, delay, iterations)` - Time-based playback
/// - `loop-driver(duration, mode)` - Continuous looping
/// - `mouse-driver(&el, axis, invert)` - Mouse-position progress
/// - `event-driver(&el, trigger, duration)` - Event-triggered animation
/// - `load-driver(&el, duration, delay)` - On-load animation
///
/// ### Runtime Functions (functions.st)
/// - `wave(type, frequency, amplitude)` - Oscillating values
/// - `random(min, max, seed)` - Seeded random values
/// - `noise(&el, frequency, amplitude)` - Perlin-like noise
/// - `parallax(range, center)` - Parallax offset calculation
/// - `spring(stiffness, damping, mass)` - Spring physics motion
///
/// ## Usage
///
/// Primitives are typically composed via macros:
///
/// ```st
/// %macro my-animation {
///   %binds {
///     scroll-driver(&self, start: 0, end: 1) -> { $progress }
///   }
///   %binds {
///     animate-property(&self, keyframes: [...], property: "opacity")
///   }
/// }
/// ```
///
/// Or used directly in behavior blocks via the timeline macros.
