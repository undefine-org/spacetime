/// CSS Property Primitive
///
/// Emits a single CSS property declaration within a selector.
/// This is the fundamental building block for all CSS in Spacetime.
///
/// Parameters:
///   selector - CSS selector to apply the property to
///   property - CSS property name (e.g., "display", "gap")
///   value - CSS property value (e.g., "grid", "20px")

%primitive css-property(
  selector: string,
  property: string,
  value: string
) {
  %emit css {
    %selector {
      %property: %value;
    }
  }
}
