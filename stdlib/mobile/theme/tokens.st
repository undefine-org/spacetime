/// Spacetime Mobile Design Tokens
///
/// CSS custom properties for consistent design across mobile apps.
/// Provides spacing, radius, shadows, and touch target values.
///
/// @example
/// .container {
///   @mobile-tokens
///   padding: var(--st-space-md);
///   border-radius: var(--st-radius-md);
/// }

%macro mobile-tokens {

  %form {
    @mobile-tokens
  }

  %states {
    base {
      // Spacing scale
      --st-space-xs: 4px
      --st-space-sm: 8px
      --st-space-md: 16px
      --st-space-lg: 24px
      --st-space-xl: 32px

      // Border radius
      --st-radius-sm: 8px
      --st-radius-md: 12px
      --st-radius-lg: 16px
      --st-radius-full: 9999px

      // Shadows
      --st-shadow-sm: 0 1px 2px rgba(0, 0, 0, 0.05)
      --st-shadow-md: 0 4px 6px rgba(0, 0, 0, 0.07), 0 2px 4px rgba(0, 0, 0, 0.05)
      --st-shadow-lg: 0 10px 15px rgba(0, 0, 0, 0.1), 0 4px 6px rgba(0, 0, 0, 0.05)

      // Touch target minimums (iOS and Android guidelines)
      --st-touch-target-min: 44px
      --st-touch-target-comfortable: 48px
    }
  }
}
