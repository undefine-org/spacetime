/// Spacetime Mobile Motion Presets
///
/// CSS custom properties for consistent animations and transitions.
/// Provides duration, easing, and spring presets for smooth mobile UX.
///
/// @example
/// .card {
///   @mobile-motion
///   transition: transform var(--st-duration-normal) var(--st-ease-out);
/// }
///
/// @example spring animation
/// .sheet {
///   @mobile-motion
///   transition: transform var(--st-duration-slow) var(--st-spring);
/// }

%macro mobile-motion {

  %form {
    @mobile-motion
  }

  %states {
    base {
      // Duration scale
      --st-duration-fast: 150ms
      --st-duration-normal: 250ms
      --st-duration-slow: 400ms

      // Easing curves
      --st-ease-out: cubic-bezier(0.16, 1, 0.3, 1)
      --st-ease-in: cubic-bezier(0.7, 0, 0.84, 0)
      --st-ease-in-out: cubic-bezier(0.87, 0, 0.13, 1)

      // Spring and bounce (for playful interactions)
      --st-spring: cubic-bezier(0.34, 1.56, 0.64, 1)
      --st-bounce: cubic-bezier(0.68, -0.55, 0.265, 1.55)

      // Linear for progress indicators
      --st-linear: linear
    }
  }
}
