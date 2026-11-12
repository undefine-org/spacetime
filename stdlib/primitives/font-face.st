/// Font Face Primitive
///
/// Emits a @font-face CSS at-rule for web font registration.
/// This is a CSS-only primitive (no JS, no exports).
///
/// Parameters:
///   family - Font family name (e.g., "Aspekta")
///   styles - CSS declarations for the font face (src, font-weight, etc.)
///
/// Used via the @font macro.

%primitive font-face-emit(
  family: string,
  styles: any
) {
  %emit css {
    @font-face {
      font-family: %family;
      %styles
    }
  }
}
