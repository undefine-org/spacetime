/// CSS At-Rule Macros
///
/// Provides SpaceTime macros for CSS at-rules that need to be emitted
/// as static CSS (no runtime behavior).
///
/// @font - Emits @font-face declarations
/// @property - Emits @property custom property registration


/// Font Face Macro
///
/// Registers a web font via @font-face CSS at-rule.
///
/// @example
/// @font("Aspekta") {
///     src: url("/fonts/AspektaVF.woff2") format("woff2-variations");
///     font-weight: 50 1000;
///     font-style: normal;
///     font-display: swap;
/// }

%macro font-face {
    %form {
        @font($family:string) {
            $styles:properties
        }
    }

    %binds {
        font-face-emit($family, styles: $styles) -> {}
    }
}


/// CSS Property Registration Macro
///
/// Registers a CSS custom property with @property for smooth transitions.
/// Enables CSS to interpolate custom properties (e.g., colors) that would
/// otherwise snap between values.
///
/// @example
/// @property("--nav-fg") {
///     syntax: '<color>';
///     initial-value: #1A1A1A;
///     inherits: true;
/// }

%macro css-property-registration {
    %form {
        @property($name:string) {
            $styles:properties
        }
    }

    %binds {
        property-registration-emit($name, styles: $styles) -> {}
    }
}
