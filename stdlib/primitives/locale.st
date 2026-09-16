/// Locale Module Primitive (PLAN-133 W2.5, FUP-175 R3)
///
/// A yield-less, param-free MODULE primitive: its only job is installing the
/// shared locale resolver once per page via `%emit prelude-js`. Consumers
/// declare `%uses locale` in their header; the expansion closure pulls this
/// prelude onto the page even when `locale` itself is never invoked.
///
/// Before this module, the resolution chain was hand-copied across source.st
/// (twice), dev-editable.st (twice — and already DRIFTED, missing the
/// html-lang fallback), and dev-inspect.st (a third variant) — the BUG-274
/// habitat. One owner now.
///
/// The chain: explicit runtime override (`window.__st_locale`, written by
/// i18n.st) → the document's declared language → 'en'.
%primitive locale() {
  %emit prelude-js {
    window.__stResolveLocale = window.__stResolveLocale || function () {
      return window.__st_locale || document.documentElement.lang || 'en';
    };
  }
}
