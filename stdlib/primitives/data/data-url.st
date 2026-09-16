/// Data URL Module Primitive (PLAN-133 R4)
///
/// Owns `__stResolveUrl(tpl)` — the ONE template-URL→fetchable-URL transform:
/// `{locale}` substitution (via the locale module's chain) + root-relative
/// normalization (a bare "data/x.json" resolves against the site root, not
/// the current page path — the admin/CMS case where the page sits at /admin
/// but the data lives at /data). Absolute and http(s) URLs pass through.
/// Empty/non-string input yields ''.
///
/// Before this module, data-source hand-wrote the {locale} replace (three
/// sites) and reactive-source hand-wrote root-relative normalization —
/// two halves of one transform, drifting apart.
%primitive data-url() %uses locale {
  %emit prelude-js {
    window.__stResolveUrl = window.__stResolveUrl || function (tpl) {
      if (typeof tpl !== 'string' || !tpl) return '';
      var url = tpl.replace('{locale}', window.__stResolveLocale());
      if (url.startsWith('/') || url.startsWith('http://') || url.startsWith('https://')) return url;
      return '/' + url;
    };
  }
}
