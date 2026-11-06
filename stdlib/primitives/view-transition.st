/// View Transition Primitive
///
/// Emits a `@view-transition` CSS at-rule (cross-document view transitions).
/// This is a CSS-only primitive (no JS, no exports).
///
/// Parameters:
///   styles - descriptors for the rule (`navigation: auto;`, `types: …;`)
///
/// @example
/// Used via the @view-transition macro:
///   @view-transition { navigation: auto; }

%primitive view-transition-emit(
  styles: any
) {
  %emit css {
    @view-transition {
      %styles
    }
  }
}
