/// CSS Property Registration Primitive
///
/// Emits a @property CSS at-rule for custom property registration.
/// This enables smooth CSS transitions on custom properties (e.g., colors).
/// This is a CSS-only primitive (no JS, no exports).
///
/// Parameters:
///   name - Custom property name (e.g., "--nav-fg")
///   styles - CSS descriptors (syntax, initial-value, inherits)
///
/// @example
/// Used via the @property macro:
///   @property("--nav-fg") {
///       syntax: '<color>';
///       initial-value: #1A1A1A;
///       inherits: true;
///   }

%primitive property-registration-emit(
  name: string,
  styles: any
) {
  %emit css {
    @property %name {
      %styles
    }
  }
}
