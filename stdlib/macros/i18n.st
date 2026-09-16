/// Locale Setup Macro
///
/// Sugar for the `locale-build` primitive. Maps the `@locale(...)` parenthesized
/// syntax to the underlying primitive call.
///
/// @example basic — English + French
/// @locale(
///   default: en,
///   available: [en, fr],
///   src: "locales/{locale}.json"
/// )
///
/// @example three locales with Spanish default
/// @locale(
///   default: es,
///   available: [es, en, fr],
///   src: "i18n/{locale}.json"
/// )
%macro locale-setup {
  %form {
    @locale(
      default: $default:ident = en,
      available: $available:array,
      src: $src:string
    )
  }

  %binds {
    locale-build(
      default: $default,
      available: $available,
      src: $src
    )
  }
}
