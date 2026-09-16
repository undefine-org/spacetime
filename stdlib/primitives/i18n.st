/// Locale Build Primitive
///
/// Generates per-locale static HTML at build time. Each locale gets its own
/// directory (e.g. /en/index.html, /fr/index.html) with translated content,
/// plus a root index.html that redirects to the default locale.
///
/// Translation works via the `data-t` attribute convention: any HTML element
/// with `data-t="key"` has its textContent replaced with the matching value
/// from the locale's JSON dictionary.
///
/// @param default - Default locale identifier (e.g. `en`). Used for the
///   root redirect and the hreflang x-default.
/// @param available - Array of locale identifiers to generate (e.g. `[en, fr]`).
/// @param src - Path pattern for locale JSON files. Use `{locale}` as
///   placeholder (e.g. `"locales/{locale}.json"`).
///
/// Build-time output:
///   - `<locale>/index.html` for each locale in `available`
///   - `index.html` root redirect to `/<default>/`
///   - `<link rel="alternate" hreflang="...">` tags injected into each page
///   - `lang` and `dir` attributes set on `<html>` element
///
/// Runtime API (injected via %emit js):
///   - `window.__st_locale` — current locale string
///   - `window.__st_setLocale(locale)` — navigates to the given locale prefix
///
/// @example
/// @locale(
///   default: en,
///   available: [en, fr],
///   src: "locales/{locale}.json"
/// )
///
/// <!-- In HTML, mark translatable elements: -->
/// <h1 data-t="hero_title">Welcome</h1>
/// <p data-t="hero_sub">AI tools built for you.</p>
%primitive locale-build(
  default: ident = en,
  available: array,
  src: string
) {
  %emit build-js {
    const locales = %available;
    const srcPattern = %src;
    const defaultLocale = %default;
    const rtlLocales = ['ar', 'he', 'fa', 'ur'];

    for (const locale of locales) {
      // Parse HTML with LinkeDOM
      var document = parseHTML(getHtml()).document;

      // Load translations
      const dictPath = srcPattern.replace('{locale}', locale);
      const dict = readJson(dictPath);

      // Regex-based HTML sanitizer for locale values (build-js safe, no DOMParser)
      var __stHtmlRe = new RegExp('<[a-z][\\s\\S]*>', 'i');
      function __stSanitizeLocaleHtml(html) {
        // Strip script/style/iframe/object/embed/form tags and contents
        html = html.replace(new RegExp('<(script|style|iframe|object|embed|form)[^>]*>[\\s\\S]*?<\\/\\1>', 'gi'), '');
        // Strip self-closing dangerous tags
        html = html.replace(new RegExp('<(script|style|iframe|object|embed|form|input|textarea|select)[^>]*\\/?>', 'gi'), '');
        // Strip on* event handler attributes
        html = html.replace(new RegExp('\\s+on\\w+\\s*=\\s*(?:"[^"]*"|' + "'[^']*'" + '|[^\\s>]+)', 'gi'), '');
        // Strip javascript: protocol from href/src/action
        html = html.replace(new RegExp('(href|src|action)\\s*=\\s*"javascript:[^"]*"', 'gi'), '$1=""');
        html = html.replace(new RegExp("(href|src|action)\\s*=\\s*'javascript:[^']*'", 'gi'), "$1=''");
        return html;
      }

      // Replace data-t element content
      document.querySelectorAll('[data-t]').forEach(function(el) {
        // Skip elements being actively edited in dev mode — their content
        // is managed by dev-editable and must not be overwritten.
        if (el.hasAttribute('data-st-editing')) return;
        const key = el.getAttribute('data-t');
        if (dict[key] !== undefined) {
          var val = dict[key];
          if (__stHtmlRe.test(val)) {
            el.innerHTML = __stSanitizeLocaleHtml(val);
          } else {
            el.textContent = val;
          }
        } else {
          warn('Missing translation key "' + key + '" for locale "' + locale + '"');
        }
      });

      // Set lang and dir attributes
      document.documentElement.setAttribute('lang', locale);
      if (rtlLocales.indexOf(locale) >= 0) {
        document.documentElement.setAttribute('dir', 'rtl');
      }

      // Inject hreflang links
      locales.forEach(function(l) {
        var link = document.createElement('link');
        link.setAttribute('rel', 'alternate');
        link.setAttribute('hreflang', l);
        link.setAttribute('href', '/' + l + '/');
        document.head.appendChild(link);
      });
      var xdefault = document.createElement('link');
      xdefault.setAttribute('rel', 'alternate');
      xdefault.setAttribute('hreflang', 'x-default');
      xdefault.setAttribute('href', '/' + defaultLocale + '/');
      document.head.appendChild(xdefault);

      // Embed dictionary for runtime translation of template-generated content
      var dictScript = document.createElement('script');
      dictScript.setAttribute('type', 'application/json');
      dictScript.setAttribute('id', '__st-locale-dict');
      dictScript.textContent = JSON.stringify(dict);
      document.head.appendChild(dictScript);

      // Write locale-specific HTML
      writeFile(locale + '/index.html',
        '<!DOCTYPE html>\n' + document.documentElement.outerHTML);
      log('Generated /' + locale + '/index.html (' +
        Object.keys(dict).length + ' keys)');
    }

    // Root redirect
    writeFile('index.html',
      '<!DOCTYPE html>\n<html>\n<head>\n' +
      '  <meta http-equiv="refresh" content="0; url=/' + defaultLocale + '/">\n' +
      '  <link rel="canonical" href="/' + defaultLocale + '/" />\n' +
      '</head>\n<body>\n' +
      '  <p>Redirecting to <a href="/' + defaultLocale + '/">' +
      defaultLocale + '</a>...</p>\n' +
      '</body>\n</html>');
  }

  %emit js {
    // Minimal runtime: current locale + switcher
    window.__st_locale = document.documentElement.lang || %default;
    window.__st_setLocale = function(locale) {
      var segs = location.pathname.split('/').filter(Boolean);
      if (segs.length > 0) segs[0] = locale;
      else segs = [locale];
      location.pathname = '/' + segs.join('/') + '/';
    };

    // Runtime locale translation for dynamically-created content (@template, @each)
    (function() {
      var dictEl = document.getElementById('__st-locale-dict');
      if (!dictEl) return;
      var dict;
      try { dict = JSON.parse(dictEl.textContent); } catch(e) { return; }

      var htmlRe = /<[a-z][\s\S]*>/i;

      function translateEl(el) {
        var key = el.getAttribute('data-t');
        if (key && dict[key] !== undefined) {
          var val = dict[key];
          if (htmlRe.test(val)) {
            el.innerHTML = val;
          } else {
            el.textContent = val;
          }
        }
        // Translate attribute bindings: data-t-{attr} → set attr to dict value
        for (var i = 0; i < el.attributes.length; i++) {
          var name = el.attributes[i].name;
          if (name.startsWith('data-t-') && name !== 'data-t') {
            var attrKey = el.attributes[i].value;
            if (dict[attrKey] !== undefined) {
              el.setAttribute(name.slice(7), dict[attrKey]);
            }
          }
        }
      }

      function translateTree(root) {
        var els = root.querySelectorAll('[data-t]');
        for (var i = 0; i < els.length; i++) translateEl(els[i]);
      }

      // Translate any elements already in the DOM (template content that beat us)
      translateTree(document);

      // Watch for future dynamic content (templates, @each)
      new MutationObserver(function(mutations) {
        for (var m = 0; m < mutations.length; m++) {
          var added = mutations[m].addedNodes;
          for (var n = 0; n < added.length; n++) {
            var node = added[n];
            if (node.nodeType === 1) {
              if (node.hasAttribute('data-t')) translateEl(node);
              translateTree(node);
            }
          }
        }
      }).observe(document.documentElement, { childList: true, subtree: true });
    })();
  }
}
