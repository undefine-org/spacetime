/// Locale Setup Macro
///
/// Sugar for the `locale-build` primitive. Maps the `@locale(...)` parenthesized
/// syntax to the underlying primitive call.
%macro locale-setup {
  %form {
    @locale(
      /// identifier (e.g. `en`). Used for the
      /// root redirect and the hreflang x-default.
      default: $default:ident = en,
      /// Array of locale identifiers to generate (e.g. `[en, fr]`).
      available: $available:array,
      /// Path pattern for locale JSON files. Use `{locale}` as
      /// placeholder (e.g. `"locales/{locale}.json"`).
      src: $src:string
    )
  }

  %binds {
    locale-build(...)
  }

  %example "basic — English + French" {
    @locale(
      default: en,
      available: [en, fr],
      src: "locales/{locale}.json"
    )
  }

  %example "three locales with Spanish default" {
    @locale(
      default: es,
      available: [es, en, fr],
      src: "i18n/{locale}.json"
    )
  }

  %example "@locale-setup (3)" {
    @locale(
      default: en,
      available: [en, fr],
      src: "locales/{locale}.json"
    )
  }
}
