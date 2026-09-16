/// Spacetime Mobile Colors
///
/// Color palette with automatic dark mode support.
/// Uses CSS custom properties for consistent theming.
///
/// @example
/// .app {
///   @mobile-colors
///   background: var(--st-color-background);
///   color: var(--st-color-text);
/// }
///
/// .card {
///   background: var(--st-color-surface);
///   border: 1px solid var(--st-color-border);
/// }
///
/// .button-primary {
///   background: var(--st-color-primary);
///   color: var(--st-color-on-primary);
/// }

%macro mobile-colors {

  %form {
    @mobile-colors
  }

  %states {
    base {
      --st-color-primary: #3b82f6
      --st-color-primary-hover: #2563eb
      --st-color-primary-active: #1d4ed8
      --st-color-on-primary: #ffffff

      --st-color-secondary: #64748b
      --st-color-secondary-hover: #475569
      --st-color-secondary-active: #334155
      --st-color-on-secondary: #ffffff

      --st-color-accent: #8b5cf6
      --st-color-accent-hover: #7c3aed
      --st-color-accent-active: #6d28d9
      --st-color-on-accent: #ffffff

      --st-color-background: #ffffff
      --st-color-surface: #ffffff
      --st-color-surface-elevated: #ffffff
      --st-color-surface-overlay: rgba(0, 0, 0, 0.5)

      --st-color-border: #e5e7eb
      --st-color-border-strong: #d1d5db
      --st-color-divider: #f3f4f6

      --st-color-text: #1f2937
      --st-color-text-secondary: #6b7280
      --st-color-text-muted: #9ca3af
      --st-color-text-inverse: #ffffff

      --st-color-success: #22c55e
      --st-color-success-bg: #dcfce7
      --st-color-on-success: #ffffff

      --st-color-warning: #f59e0b
      --st-color-warning-bg: #fef3c7
      --st-color-on-warning: #ffffff

      --st-color-error: #ef4444
      --st-color-error-bg: #fee2e2
      --st-color-on-error: #ffffff

      --st-color-info: #3b82f6
      --st-color-info-bg: #dbeafe
      --st-color-on-info: #ffffff
    }
  }
}


/// Mobile Colors Dark Mode
///
/// Dark mode color palette that applies when system prefers dark mode.
/// Uses the colorScheme primitive binding to detect dark mode preference.
///
/// @example
/// .app {
///   @mobile-colors
///   @mobile-colors-dark
/// }

%macro mobile-colors-dark {

  %form {
    @mobile-colors-dark
  }

  %binds {
    colorScheme() -> { $dark }
  }

  %states {
    dark when $dark {
      --st-color-primary: #60a5fa
      --st-color-primary-hover: #3b82f6
      --st-color-primary-active: #2563eb
      --st-color-on-primary: #0f172a

      --st-color-secondary: #94a3b8
      --st-color-secondary-hover: #cbd5e1
      --st-color-secondary-active: #e2e8f0
      --st-color-on-secondary: #0f172a

      --st-color-accent: #a78bfa
      --st-color-accent-hover: #8b5cf6
      --st-color-accent-active: #7c3aed
      --st-color-on-accent: #0f172a

      --st-color-background: #0f172a
      --st-color-surface: #1e293b
      --st-color-surface-elevated: #334155
      --st-color-surface-overlay: rgba(0, 0, 0, 0.7)

      --st-color-border: #334155
      --st-color-border-strong: #475569
      --st-color-divider: #1e293b

      --st-color-text: #f8fafc
      --st-color-text-secondary: #cbd5e1
      --st-color-text-muted: #94a3b8
      --st-color-text-inverse: #0f172a

      --st-color-success: #4ade80
      --st-color-success-bg: #14532d
      --st-color-on-success: #0f172a

      --st-color-warning: #fbbf24
      --st-color-warning-bg: #713f12
      --st-color-on-warning: #0f172a

      --st-color-error: #f87171
      --st-color-error-bg: #7f1d1d
      --st-color-on-error: #0f172a

      --st-color-info: #60a5fa
      --st-color-info-bg: #1e3a5f
      --st-color-on-info: #0f172a
    }
  }
}


/// Mobile Theme Macro
///
/// Convenience macro that applies light colors with automatic dark mode support.
/// Combines @mobile-colors base with @mobile-colors-dark responsive behavior.
///
/// @example
/// :root {
///   @mobile-theme
/// }

%macro mobile-theme {

  %form {
    @mobile-theme
  }

  %binds {
    colorScheme() -> { $dark }
  }

  %derives {
    $light: !$dark
  }

  %states {
    light when $light {
      --st-color-primary: #3b82f6
      --st-color-primary-hover: #2563eb
      --st-color-primary-active: #1d4ed8
      --st-color-on-primary: #ffffff

      --st-color-secondary: #64748b
      --st-color-secondary-hover: #475569
      --st-color-secondary-active: #334155
      --st-color-on-secondary: #ffffff

      --st-color-accent: #8b5cf6
      --st-color-accent-hover: #7c3aed
      --st-color-accent-active: #6d28d9
      --st-color-on-accent: #ffffff

      --st-color-background: #ffffff
      --st-color-surface: #ffffff
      --st-color-surface-elevated: #ffffff
      --st-color-surface-overlay: rgba(0, 0, 0, 0.5)

      --st-color-border: #e5e7eb
      --st-color-border-strong: #d1d5db
      --st-color-divider: #f3f4f6

      --st-color-text: #1f2937
      --st-color-text-secondary: #6b7280
      --st-color-text-muted: #9ca3af
      --st-color-text-inverse: #ffffff

      --st-color-success: #22c55e
      --st-color-success-bg: #dcfce7
      --st-color-on-success: #ffffff

      --st-color-warning: #f59e0b
      --st-color-warning-bg: #fef3c7
      --st-color-on-warning: #ffffff

      --st-color-error: #ef4444
      --st-color-error-bg: #fee2e2
      --st-color-on-error: #ffffff

      --st-color-info: #3b82f6
      --st-color-info-bg: #dbeafe
      --st-color-on-info: #ffffff
    }

    dark when $dark {
      --st-color-primary: #60a5fa
      --st-color-primary-hover: #3b82f6
      --st-color-primary-active: #2563eb
      --st-color-on-primary: #0f172a

      --st-color-secondary: #94a3b8
      --st-color-secondary-hover: #cbd5e1
      --st-color-secondary-active: #e2e8f0
      --st-color-on-secondary: #0f172a

      --st-color-accent: #a78bfa
      --st-color-accent-hover: #8b5cf6
      --st-color-accent-active: #7c3aed
      --st-color-on-accent: #0f172a

      --st-color-background: #0f172a
      --st-color-surface: #1e293b
      --st-color-surface-elevated: #334155
      --st-color-surface-overlay: rgba(0, 0, 0, 0.7)

      --st-color-border: #334155
      --st-color-border-strong: #475569
      --st-color-divider: #1e293b

      --st-color-text: #f8fafc
      --st-color-text-secondary: #cbd5e1
      --st-color-text-muted: #94a3b8
      --st-color-text-inverse: #0f172a

      --st-color-success: #4ade80
      --st-color-success-bg: #14532d
      --st-color-on-success: #0f172a

      --st-color-warning: #fbbf24
      --st-color-warning-bg: #713f12
      --st-color-on-warning: #0f172a

      --st-color-error: #f87171
      --st-color-error-bg: #7f1d1d
      --st-color-on-error: #0f172a

      --st-color-info: #60a5fa
      --st-color-info-bg: #1e3a5f
      --st-color-on-info: #0f172a
    }
  }
}
